//! Identity: `EntityId` allocation, the structure-of-arrays entity table, and the
//! enforcement of invariant 8.
//!
//! # Identity is a kernel property, not a phase
//!
//! The original scaffold scheduled individuation as phase P3, after the central dogma
//! at P4. That ordering is circular — P3's Definition of Done was "a named mRNA is
//! traceable end-to-end", and mRNA does not exist until P4. Identity lives here, in the
//! kernel, from P0. What P3 retains is the *measurement*: what individuation costs in
//! resident bytes and in wall clock. See `docs/DECISIONS.md` ADR-0004.
//!
//! # Ids are never reused; slots are
//!
//! Invariant 8 says an `EntityId` is unique for life and never reused. It does *not*
//! say the memory holding a dead entity must be retained — and the distinction is worth
//! roughly two orders of magnitude in resident set over a cell cycle, because mRNA
//! turnover means cumulative births vastly exceed the standing population.
//!
//! So: the id counter is monotonic and never rewinds, while the *slot* a dead entity
//! occupied returns to a free list. Resident memory tracks the live population;
//! the event log carries the history. See `docs/BUDGET.md`.

use crate::event::{Event, EventKind, EventLog};

/// A unique-for-life entity identifier. Never reused, never recycled, never zero.
///
/// Zero is reserved as a niche so `Option<EntityId>` costs no extra bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(core::num::NonZeroU64);

impl EntityId {
    #[inline]
    pub fn get(self) -> u64 {
        self.0.get()
    }

    /// Reconstruct an id from its raw value — for reading an event log back in.
    ///
    /// Returns `None` for 0, which is not a valid id.
    #[inline]
    pub fn from_raw(v: u64) -> Option<Self> {
        core::num::NonZeroU64::new(v).map(EntityId)
    }
}

impl core::fmt::Display for EntityId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "e{}", self.0.get())
    }
}

/// Species index. P0 carries species as an opaque index; `protocell-chem` owns the
/// formula, charge and provenance behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpeciesId(pub u32);

/// Never-allocated slot marker.
const NO_SLOT: u32 = u32::MAX;

/// Structure-of-arrays entity table.
///
/// One `Vec` per component, not one struct per entity: this is the layout the RDME and
/// BD kernels will want at P2/P4, and retrofitting it later is a rewrite.
#[derive(Debug, Default)]
pub struct EntityTable {
    /// Monotonic id counter. Only ever increases. Invariant 8.
    next_id: u64,

    // --- Per-slot components (structure of arrays) ---
    slot_id: Vec<u64>,
    slot_species: Vec<SpeciesId>,
    slot_birth_step: Vec<u64>,
    slot_live: Vec<bool>,

    /// Slots whose entity has died and whose storage may be recycled.
    /// Ids are never recycled; slots are. Ordered as a stack for determinism.
    free_slots: Vec<u32>,

    /// Live entity id -> slot. `BTreeMap` not `HashMap`: iteration order is part of the
    /// trajectory, and `HashMap` order is not reproducible across runs.
    index: std::collections::BTreeMap<u64, u32>,

    // --- Running census, for invariant 8 and the budget instrumentation ---
    births: u64,
    deaths: u64,
    peak_live: u64,
}

impl EntityTable {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            ..Default::default()
        }
    }

    /// Allocate a new entity and write its birth event. Invariant 8: no entity exists
    /// without a birth event, so birth and logging are one operation and cannot be
    /// separated by a caller who forgets.
    pub fn spawn(
        &mut self,
        species: SpeciesId,
        step: u64,
        log: &mut EventLog,
    ) -> EntityId {
        self.next_id += 1;
        let raw = self.next_id;
        let id = EntityId(
            core::num::NonZeroU64::new(raw).expect("id counter starts at 1 and only grows"),
        );

        let slot = match self.free_slots.pop() {
            Some(s) => {
                let i = s as usize;
                self.slot_id[i] = raw;
                self.slot_species[i] = species;
                self.slot_birth_step[i] = step;
                self.slot_live[i] = true;
                s
            }
            None => {
                let s = self.slot_id.len() as u32;
                assert_ne!(s, NO_SLOT, "entity slot space exhausted");
                self.slot_id.push(raw);
                self.slot_species.push(species);
                self.slot_birth_step.push(step);
                self.slot_live.push(true);
                s
            }
        };

        let prev = self.index.insert(raw, slot);
        assert!(prev.is_none(), "invariant 8 violated: {id} allocated twice");

        self.births += 1;
        let live = self.live_count() as u64;
        if live > self.peak_live {
            self.peak_live = live;
        }

        log.push(Event::birth(step, id, species));
        id
    }

    /// Kill an entity and write its death event. The slot is recycled; the id is not.
    ///
    /// Panics if the entity is already dead or was never alive — invariant 8 says no
    /// entity vanishes silently, and a double death is exactly that failure mode.
    pub fn kill(&mut self, id: EntityId, step: u64, log: &mut EventLog) {
        let slot = self
            .index
            .remove(&id.get())
            .unwrap_or_else(|| panic!("invariant 8 violated: {id} died without being alive"));

        let i = slot as usize;
        debug_assert!(self.slot_live[i]);
        self.slot_live[i] = false;
        self.free_slots.push(slot);
        self.deaths += 1;

        log.push(Event::death(step, id));
    }

    #[inline]
    pub fn is_alive(&self, id: EntityId) -> bool {
        self.index.contains_key(&id.get())
    }

    #[inline]
    pub fn species_of(&self, id: EntityId) -> Option<SpeciesId> {
        self.index.get(&id.get()).map(|&s| self.slot_species[s as usize])
    }

    #[inline]
    pub fn birth_step_of(&self, id: EntityId) -> Option<u64> {
        self.index
            .get(&id.get())
            .map(|&s| self.slot_birth_step[s as usize])
    }

    /// Live entities in ascending id order. Deterministic by construction.
    pub fn iter_live(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.index
            .keys()
            .map(|&raw| EntityId(core::num::NonZeroU64::new(raw).expect("live ids are nonzero")))
    }

    #[inline]
    pub fn live_count(&self) -> usize {
        self.index.len()
    }

    #[inline]
    pub fn births(&self) -> u64 {
        self.births
    }

    #[inline]
    pub fn deaths(&self) -> u64 {
        self.deaths
    }

    /// Highest simultaneous live population seen. This — not cumulative births — is
    /// what sizes the resident entity table. See `docs/BUDGET.md`.
    #[inline]
    pub fn peak_live(&self) -> u64 {
        self.peak_live
    }

    /// Slots ever allocated. With slot recycling this tracks `peak_live`, not births;
    /// asserting that relationship is the point of the budget argument.
    #[inline]
    pub fn slots_allocated(&self) -> usize {
        self.slot_id.len()
    }

    /// Resident bytes of the per-entity components, excluding the index.
    pub fn component_bytes(&self) -> usize {
        let n = self.slot_id.len();
        n * (core::mem::size_of::<u64>()      // id
            + core::mem::size_of::<SpeciesId>() // species
            + core::mem::size_of::<u64>()     // birth step
            + core::mem::size_of::<bool>()) // live flag
    }

    /// Invariant 8, checkable at any point: every id ever issued is either live or has
    /// a recorded death, and no id was issued twice.
    pub fn assert_identity_conservation(&self, log: &EventLog) {
        let births = log.count_of(EventKind::Birth);
        let deaths = log.count_of(EventKind::Death);
        assert_eq!(
            births, self.births,
            "invariant 8: birth events ({births}) != births recorded ({})",
            self.births
        );
        assert_eq!(
            deaths, self.deaths,
            "invariant 8: death events ({deaths}) != deaths recorded ({})",
            self.deaths
        );
        assert_eq!(
            self.births - self.deaths,
            self.live_count() as u64,
            "invariant 8: births - deaths != live population"
        );
        assert_eq!(
            self.next_id, self.births,
            "invariant 8: id counter diverged from birth count (id reuse or a silent spawn)"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_never_reused_even_though_slots_are() {
        let mut t = EntityTable::new();
        let mut log = EventLog::new();
        let s = SpeciesId(0);

        let a = t.spawn(s, 0, &mut log);
        let b = t.spawn(s, 0, &mut log);
        t.kill(a, 1, &mut log);
        t.kill(b, 1, &mut log);
        let c = t.spawn(s, 2, &mut log);
        let d = t.spawn(s, 2, &mut log);

        assert_ne!(a, c);
        assert_ne!(a, d);
        assert_ne!(b, c);
        assert_ne!(b, d);
        assert_eq!([a.get(), b.get(), c.get(), d.get()], [1, 2, 3, 4]);

        // ...but the storage was recycled: 4 entities, only 2 slots ever allocated.
        assert_eq!(t.slots_allocated(), 2, "slots must be recycled");
        t.assert_identity_conservation(&log);
    }

    #[test]
    fn resident_memory_tracks_live_population_not_cumulative_births() {
        let mut t = EntityTable::new();
        let mut log = EventLog::new();
        let s = SpeciesId(1);

        // 10_000 births, but never more than 10 alive at once — the mRNA turnover
        // regime. Resident cost must follow the 10, not the 10_000.
        let mut live = Vec::new();
        for step in 0..10_000u64 {
            live.push(t.spawn(s, step, &mut log));
            if live.len() > 10 {
                let victim = live.remove(0);
                t.kill(victim, step, &mut log);
            }
        }

        assert_eq!(t.births(), 10_000);
        assert_eq!(t.peak_live(), 11);
        assert_eq!(
            t.slots_allocated(),
            11,
            "resident slots must track peak live population, not cumulative births"
        );
        t.assert_identity_conservation(&log);
    }

    #[test]
    #[should_panic(expected = "invariant 8")]
    fn double_death_is_a_crash_not_a_log_line() {
        let mut t = EntityTable::new();
        let mut log = EventLog::new();
        let a = t.spawn(SpeciesId(0), 0, &mut log);
        t.kill(a, 1, &mut log);
        t.kill(a, 2, &mut log);
    }

    #[test]
    fn live_iteration_is_ordered_and_therefore_reproducible() {
        let mut t = EntityTable::new();
        let mut log = EventLog::new();
        let s = SpeciesId(0);
        let ids: Vec<_> = (0..64).map(|i| t.spawn(s, i, &mut log)).collect();
        for (i, id) in ids.iter().enumerate() {
            if i % 3 == 0 {
                t.kill(*id, 100, &mut log);
            }
        }
        let seen: Vec<u64> = t.iter_live().map(|e| e.get()).collect();
        let mut sorted = seen.clone();
        sorted.sort_unstable();
        assert_eq!(seen, sorted, "live iteration must be in ascending id order");
    }
}
