//! Append-only event log — the alternative to an 80 TB frame dump.
//!
//! # What goes in the log, and what must never go in it
//!
//! The log records **only non-reconstructible facts**: which reaction actually fired,
//! which entities were born and died, which bonds formed. Anything that is a pure
//! function of `(seed, stream, step, entity_id)` is *not logged* — it is recomputed
//! from the RNG address on demand (see `crate::rng`).
//!
//! This distinction is the entire cost argument, and getting it wrong inverts the
//! result. Naive arithmetic: a 105-minute cell cycle at a 50 µs RDME timestep is
//! 1.26e8 steps; logging even 1e3 events per step at 32 B/event is ~4 PB, fifty times
//! *worse* than the 80 TB dense dump the project exists to avoid. Diffusion hops alone
//! are ~1e14 per cycle and can never be logged. They are replayed instead.
//!
//! The rule, stated so it can be checked in review:
//!
//! > If an outcome can be recomputed from its RNG address plus the state implied by
//! > earlier log entries, it does not belong in the log.
//!
//! `cargo run -p protocell-cli -- budget` computes the resulting volume from the
//! Syn3A event rates; `docs/BUDGET.md` records the answer.

use crate::entity::{EntityId, SpeciesId};

/// Fixed-width event record: 32 bytes, little-endian, no padding surprises.
///
/// Fixed width is deliberate. It makes the log seekable by index (`get(i)` is O(1)),
/// which is what makes a single event replayable standalone rather than by scanning.
pub const EVENT_BYTES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum EventKind {
    Birth = 1,
    Death = 2,
    Reaction = 3,
    Bind = 4,
    Unbind = 5,
}

impl EventKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            1 => EventKind::Birth,
            2 => EventKind::Death,
            3 => EventKind::Reaction,
            4 => EventKind::Bind,
            5 => EventKind::Unbind,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            EventKind::Birth => "birth",
            EventKind::Death => "death",
            EventKind::Reaction => "reaction",
            EventKind::Bind => "bind",
            EventKind::Unbind => "unbind",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    /// Simulation step. Together with `subject` this is the RNG address of the event,
    /// which is what makes standalone replay possible.
    pub step: u64,
    pub kind: EventKind,
    /// Reaction-rule index for `Reaction`, unused otherwise.
    pub rule: u16,
    /// Kind-specific: species index for `Birth`, free for others.
    pub payload: u32,
    /// The entity this event is about. `0` for events with no single subject.
    pub subject: u64,
    /// Second participant for `Bind`/`Unbind`/bimolecular `Reaction`. `0` if unused.
    pub object: u64,
}

impl Event {
    pub fn birth(step: u64, id: EntityId, species: SpeciesId) -> Self {
        Event {
            step,
            kind: EventKind::Birth,
            rule: 0,
            payload: species.0,
            subject: id.get(),
            object: 0,
        }
    }

    pub fn death(step: u64, id: EntityId) -> Self {
        Event {
            step,
            kind: EventKind::Death,
            rule: 0,
            payload: 0,
            subject: id.get(),
            object: 0,
        }
    }

    /// A reaction firing. `subject` is the RNG address the propensity draw used —
    /// `ENTITY_GLOBAL` for a well-stirred SSA, the reacting entity once P2 makes
    /// reactions local.
    pub fn reaction(step: u64, rule: u16, subject: u64) -> Self {
        Event {
            step,
            kind: EventKind::Reaction,
            rule,
            payload: 0,
            subject,
            object: 0,
        }
    }

    pub fn bind(step: u64, a: EntityId, b: EntityId, rule: u16) -> Self {
        Event {
            step,
            kind: EventKind::Bind,
            rule,
            payload: 0,
            subject: a.get(),
            object: b.get(),
        }
    }

    pub fn unbind(step: u64, a: EntityId, b: EntityId, rule: u16) -> Self {
        Event {
            step,
            kind: EventKind::Unbind,
            rule,
            payload: 0,
            subject: a.get(),
            object: b.get(),
        }
    }

    pub fn to_bytes(self) -> [u8; EVENT_BYTES] {
        let mut b = [0u8; EVENT_BYTES];
        b[0..8].copy_from_slice(&self.step.to_le_bytes());
        b[8] = self.kind as u8;
        b[9] = 0; // reserved; keeps `rule` 2-byte aligned in the on-disk form
        b[10..12].copy_from_slice(&self.rule.to_le_bytes());
        b[12..16].copy_from_slice(&self.payload.to_le_bytes());
        b[16..24].copy_from_slice(&self.subject.to_le_bytes());
        b[24..32].copy_from_slice(&self.object.to_le_bytes());
        b
    }

    pub fn from_bytes(b: &[u8; EVENT_BYTES]) -> Option<Self> {
        Some(Event {
            step: u64::from_le_bytes(b[0..8].try_into().ok()?),
            kind: EventKind::from_u8(b[8])?,
            rule: u16::from_le_bytes(b[10..12].try_into().ok()?),
            payload: u32::from_le_bytes(b[12..16].try_into().ok()?),
            subject: u64::from_le_bytes(b[16..24].try_into().ok()?),
            object: u64::from_le_bytes(b[24..32].try_into().ok()?),
        })
    }
}

/// Append-only, index-addressable event log.
#[derive(Debug, Default, Clone)]
pub struct EventLog {
    events: Vec<Event>,
    counts: [u64; 6],
}

impl EventLog {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn push(&mut self, e: Event) {
        self.counts[e.kind as usize] += 1;
        self.events.push(e);
    }

    /// O(1) access by index — the property that makes single-event replay standalone.
    #[inline]
    pub fn get(&self, i: usize) -> Option<Event> {
        self.events.get(i).copied()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    #[inline]
    pub fn count_of(&self, k: EventKind) -> u64 {
        self.counts[k as usize]
    }

    pub fn iter(&self) -> impl Iterator<Item = &Event> {
        self.events.iter()
    }

    /// On-disk size of the log as written. Reported by the budget tool.
    #[inline]
    pub fn bytes(&self) -> usize {
        self.events.len() * EVENT_BYTES
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.bytes());
        for e in &self.events {
            out.extend_from_slice(&e.to_bytes());
        }
        out
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() % EVENT_BYTES != 0 {
            return None;
        }
        let mut log = EventLog::new();
        for chunk in bytes.chunks_exact(EVENT_BYTES) {
            let arr: &[u8; EVENT_BYTES] = chunk.try_into().ok()?;
            log.push(Event::from_bytes(arr)?);
        }
        Some(log)
    }

    /// Content hash of the whole log — the determinism check for invariant 7.
    ///
    /// FNV-1a over the encoded bytes. Not cryptographic; it only has to detect a
    /// trajectory that differs, and any difference at all changes the digest.
    pub fn digest(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for e in &self.events {
            for b in e.to_bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityTable, SpeciesId};

    #[test]
    fn events_round_trip_through_bytes_exactly() {
        let mut t = EntityTable::new();
        let mut log = EventLog::new();
        let a = t.spawn(SpeciesId(3), 10, &mut log);
        let b = t.spawn(SpeciesId(4), 11, &mut log);
        log.push(Event::bind(12, a, b, 7));
        log.push(Event::reaction(13, 2, a.get()));
        log.push(Event::unbind(14, a, b, 7));
        t.kill(a, 15, &mut log);

        let bytes = log.encode();
        assert_eq!(bytes.len(), log.len() * EVENT_BYTES);
        let back = EventLog::decode(&bytes).expect("log must decode");
        assert_eq!(back.len(), log.len());
        assert_eq!(back.digest(), log.digest());
        for i in 0..log.len() {
            assert_eq!(back.get(i), log.get(i), "event {i} did not round-trip");
        }
    }

    #[test]
    fn digest_detects_any_divergence() {
        let mut a = EventLog::new();
        let mut b = EventLog::new();
        for s in 0..100u64 {
            a.push(Event::reaction(s, 1, 0));
            b.push(Event::reaction(s, 1, 0));
        }
        assert_eq!(a.digest(), b.digest());
        b.push(Event::reaction(100, 1, 0));
        assert_ne!(a.digest(), b.digest());

        let mut c = EventLog::new();
        for s in 0..100u64 {
            // Same length, one rule index differs.
            c.push(Event::reaction(s, if s == 50 { 2 } else { 1 }, 0));
        }
        assert_ne!(a.digest(), c.digest());
    }

    #[test]
    fn malformed_logs_are_rejected_not_guessed_at() {
        assert!(EventLog::decode(&[0u8; 7]).is_none(), "short read must fail");
        let mut bad = [0u8; EVENT_BYTES];
        bad[8] = 99; // not a valid EventKind
        assert!(EventLog::decode(&bad).is_none(), "bad kind must fail");
    }
}
