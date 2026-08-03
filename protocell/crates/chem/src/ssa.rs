//! Well-stirred stochastic simulation (Gillespie direct method), with individuation as
//! an overlay rather than a rewrite.
//!
//! # The two arms
//!
//! The same network can be run with species marked [`Tier::Counted`] or
//! [`Tier::Individuated`]. The counted arm stores integers. The individuated arm gives
//! every molecule an `EntityId`, a birth event and a death event.
//!
//! **The count trajectories are bit-identical between the two arms.** That is asserted,
//! not hoped for, and it matters for two reasons:
//!
//! - It makes individuation a *measurable overhead* rather than a confound. Any
//!   difference in the observables between arms would be a bug in the overlay, and the
//!   cost comparison in `docs/BUDGET.md` would be comparing two different models.
//! - It is the miniature form of the control the thesis needs. `docs/REVIEW.md` finding 1
//!   is that the project's stated falsification (L4) does not isolate individuation,
//!   because polysomes can be had from a counted representation with a ribosome
//!   occupancy integer. Every claim of the form "individuation buys X" has to be run
//!   against a non-individuated arm that is otherwise identical. This is the machinery
//!   for doing that, established at P0 so nothing is built without it.
//!
//! Bit-identity across arms is only possible because the RNG is counter-based: the
//! individuated arm spends extra draws on [`Stream::ENTITY_PICK`], and addressed draws
//! mean spending them shifts nothing else.

use crate::network::{Network, Tier};
use crate::thermo::entropy_production;
use protocell_core::{
    rng::{Rng, Stream, ENTITY_GLOBAL},
    EntityId, EntityTable, Event, EventLog, SpeciesId,
};

/// Why a run stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// Requested step budget reached.
    StepLimit,
    /// Requested simulated time reached.
    TimeLimit,
    /// Total propensity is zero — the system is absorbed and nothing further can occur.
    Absorbed,
}

#[derive(Debug)]
pub struct Simulation {
    net: Network,
    counts: Vec<u64>,
    /// Live entities per species, ascending species index. Empty for counted species.
    live: Vec<Vec<EntityId>>,
    table: EntityTable,
    log: EventLog,
    rng: Rng,
    step: u64,
    time: f64,
    initial_atoms: i64,
    /// Scratch buffer for propensities, reused so the hot loop does not allocate.
    propensities: Vec<f64>,
}

impl Simulation {
    pub fn new(net: Network, initial_counts: Vec<u64>, seed: u64) -> Self {
        assert_eq!(
            initial_counts.len(),
            net.species().len(),
            "initial count vector must cover every species"
        );
        net.validate().expect("network must pass invariant 1 before it can be simulated");

        let nspecies = net.species().len();
        let nreactions = net.reactions().len();
        let initial_atoms = net.total_atoms(&initial_counts);

        let mut sim = Simulation {
            counts: initial_counts,
            live: vec![Vec::new(); nspecies],
            table: EntityTable::new(),
            log: EventLog::new(),
            rng: Rng::new(seed),
            step: 0,
            time: 0.0,
            initial_atoms,
            propensities: vec![0.0; nreactions],
            net,
        };

        // Individuated species get their initial population instantiated, each with a
        // birth event. Invariant 8 admits no entity that was never born, including the
        // ones that exist at t = 0.
        for i in 0..nspecies {
            let sid = SpeciesId(i as u32);
            if sim.net.tier_of(sid) == Tier::Individuated {
                for _ in 0..sim.counts[i] {
                    let id = sim.table.spawn(sid, 0, &mut sim.log);
                    sim.live[i].push(id);
                }
            }
        }
        sim
    }

    pub fn counts(&self) -> &[u64] {
        &self.counts
    }

    pub fn count(&self, s: SpeciesId) -> u64 {
        self.counts[s.0 as usize]
    }

    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn step_number(&self) -> u64 {
        self.step
    }

    pub fn log(&self) -> &EventLog {
        &self.log
    }

    pub fn table(&self) -> &EntityTable {
        &self.table
    }

    pub fn network(&self) -> &Network {
        &self.net
    }

    /// Live entities of an individuated species, in allocation order.
    pub fn live_entities(&self, s: SpeciesId) -> &[EntityId] {
        &self.live[s.0 as usize]
    }

    /// Stochastic propensity `a_j = k_j · Π C(n_i, ν_i)`.
    ///
    /// Combinatorial rather than `k·Πn^ν`: for `A + A → …` the number of distinct
    /// reacting pairs is `n(n−1)/2`, and using `n²` overcounts by a factor that only
    /// disappears at large `n`. At Syn3A copy numbers — tens to thousands — it does not
    /// disappear.
    fn propensity(&self, r: usize) -> f64 {
        let rx = &self.net.reactions()[r];
        let mut a = rx.k;
        for (s, nu) in &rx.reactants {
            let n = self.counts[s.0 as usize];
            let nu = *nu as u64;
            if n < nu {
                return 0.0;
            }
            // C(n, nu), computed exactly in the small-nu regime we ever use.
            let mut c = 1.0f64;
            for i in 0..nu {
                c *= (n - i) as f64;
            }
            for i in 1..=nu {
                c /= i as f64;
            }
            a *= c;
        }
        a
    }

    /// All propensities, in fixed reaction order. Order is part of the trajectory:
    /// summing in a different order changes the floating-point total and therefore the
    /// selected channel, so it must never be parallelised without a fixed-order
    /// reduction. See `docs/DECISIONS.md` ADR-0005.
    fn refresh_propensities(&mut self) -> f64 {
        let n = self.net.reactions().len();
        let mut total = 0.0;
        for r in 0..n {
            let a = self.propensity(r);
            self.propensities[r] = a;
            total += a;
        }
        total
    }

    /// One SSA step. Returns `false` when the system is absorbed.
    pub fn advance_one(&mut self) -> bool {
        let a0 = self.refresh_propensities();
        // NaN-safe: a propensity that has gone NaN must halt the run, not select a channel.
        if a0.is_nan() || a0 <= 0.0 {
            return false;
        }

        // Exactly one draw from each of two streams, unconditionally. The count of
        // draws per address must not depend on the branch taken — see `core::rng`.
        let tau = self.rng.exponential(Stream::SSA_TIME, self.step, ENTITY_GLOBAL, 0) / a0;
        let u = self.rng.uniform(Stream::SSA_CHOICE, self.step, ENTITY_GLOBAL, 0);

        // Linear search over the cumulative sum, in fixed order.
        let target = u * a0;
        let mut acc = 0.0;
        let mut chosen = self.propensities.len() - 1;
        for (j, &a) in self.propensities.iter().enumerate() {
            acc += a;
            if target < acc {
                chosen = j;
                break;
            }
        }

        self.fire(chosen);
        self.log.push(Event::reaction(self.step, chosen as u16, ENTITY_GLOBAL));
        self.time += tau;
        self.step += 1;
        true
    }

    fn fire(&mut self, r: usize) {
        let rx = self.net.reactions()[r].clone();

        // Consume reactants. Kill before spawning so a species that is both reactant
        // and product cannot have a product entity selected for destruction.
        for (s, nu) in &rx.reactants {
            let si = s.0 as usize;
            self.counts[si] -= *nu as u64;
            if self.net.tier_of(*s) == Tier::Individuated {
                for ordinal in 0..*nu {
                    self.kill_one_of(*s, r, ordinal);
                }
            }
        }

        for (s, nu) in &rx.products {
            let si = s.0 as usize;
            self.counts[si] += *nu as u64;
            if self.net.tier_of(*s) == Tier::Individuated {
                for _ in 0..*nu {
                    let id = self.table.spawn(*s, self.step, &mut self.log);
                    self.live[si].push(id);
                }
            }
        }
    }

    /// Select and destroy one individuated molecule of a species.
    ///
    /// The pick is addressed by `(step, reaction, ordinal)`, so it is reproducible in
    /// isolation: given the log entry for a death, the choice that produced it can be
    /// recomputed without replaying the trajectory.
    fn kill_one_of(&mut self, s: SpeciesId, reaction: usize, ordinal: u32) {
        let si = s.0 as usize;
        let n = self.live[si].len();
        assert!(n > 0, "count/entity desynchronisation for species {si}");

        let stream = Stream::ENTITY_PICK.sub((reaction as u64) << 8 | ordinal as u64);
        let u = self.rng.uniform(stream, self.step, ENTITY_GLOBAL, 0);
        let idx = ((u * n as f64) as usize).min(n - 1);

        // `swap_remove` is O(1) and deterministic: the resulting order is a function of
        // the trajectory so far, which is itself deterministic.
        let victim = self.live[si].swap_remove(idx);
        self.table.kill(victim, self.step, &mut self.log);
    }

    pub fn run_steps(&mut self, steps: u64) -> StopReason {
        for _ in 0..steps {
            if !self.advance_one() {
                return StopReason::Absorbed;
            }
        }
        StopReason::StepLimit
    }

    pub fn run_until(&mut self, t_end: f64) -> StopReason {
        while self.time < t_end {
            if !self.advance_one() {
                return StopReason::Absorbed;
            }
        }
        StopReason::TimeLimit
    }

    /// A network is closed when no reaction creates or destroys matter — every reaction
    /// has both reactants and products. Only closed networks can conserve mass.
    pub fn is_closed(&self) -> bool {
        self.net
            .reactions()
            .iter()
            .all(|r| !r.reactants.is_empty() && !r.products.is_empty())
    }

    /// Invariant 2. Only meaningful for closed networks; the caller is told rather than
    /// silently given a pass.
    pub fn assert_mass_conservation(&self) {
        assert!(
            self.is_closed(),
            "mass conservation is only defined for a closed network; this one exchanges \
             matter with its surroundings"
        );
        let now = self.net.total_atoms(&self.counts);
        assert_eq!(
            now, self.initial_atoms,
            "invariant 2: atom count drifted from {} to {now}",
            self.initial_atoms
        );
    }

    /// Invariant 8, delegated to the entity table.
    pub fn assert_identity_conservation(&self) {
        self.table.assert_identity_conservation(&self.log);
        for (i, ids) in self.live.iter().enumerate() {
            if self.net.tier_of(SpeciesId(i as u32)) == Tier::Individuated {
                assert_eq!(
                    ids.len() as u64, self.counts[i],
                    "species {i}: {} live entities but count says {}",
                    ids.len(),
                    self.counts[i]
                );
            }
        }
    }

    /// Total entropy production rate over all reversible pairs, in units of `R`.
    ///
    /// Each pair is counted once. Irreversible reactions contribute `+∞` by
    /// construction — they cannot equilibrate — and the kill test reports that rather
    /// than quietly returning a finite number.
    pub fn entropy_production_rate(&self) -> f64 {
        let mut sigma = 0.0;
        for (j, rx) in self.net.reactions().iter().enumerate() {
            match rx.reverse {
                Some(rev) if rev > j => {
                    sigma += entropy_production(self.propensity(j), self.propensity(rev));
                }
                Some(_) => {}
                None => {
                    if self.propensity(j) > 0.0 {
                        return f64::INFINITY;
                    }
                }
            }
        }
        sigma
    }

    /// Overwrite the count vector for **off-trajectory analysis only** — evaluating a
    /// rate expression at an ensemble-mean state, for instance.
    ///
    /// This is not a simulation operation: it logs nothing and advances nothing. It
    /// refuses to run on a network with individuated species, because setting counts
    /// behind the entity table's back is precisely the desynchronisation invariant 8
    /// exists to forbid.
    pub fn set_counts_for_analysis(&mut self, counts: &[f64]) {
        assert_eq!(counts.len(), self.counts.len());
        assert!(
            self.net
                .species()
                .iter()
                .all(|s| s.tier == Tier::Counted),
            "set_counts_for_analysis would desynchronise the entity table; \
             build the analysis probe from a counted network"
        );
        for (dst, src) in self.counts.iter_mut().zip(counts) {
            *dst = src.round().max(0.0) as u64;
        }
    }

    /// Resident bytes attributable to individuation: entity components plus the
    /// per-species live lists. Excludes the log, which is on disk.
    pub fn individuation_resident_bytes(&self) -> usize {
        self.table.component_bytes()
            + self
                .live
                .iter()
                .map(|v| v.capacity() * core::mem::size_of::<EntityId>())
                .sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::Formula;
    use crate::thermo::ConcentrationUnit;
    use protocell_core::event::EventKind;

    const T: f64 = 310.15;

    /// `A ⇌ B` with `k_r` derived from ΔG°′, in the requested tier.
    fn isomerisation(tier: Tier, n_a: u64, dg: f64) -> Simulation {
        let mut net = Network::new(T, ConcentrationUnit::sphere_nm(200.0));
        let a = net.add_species("A", Formula::parse("C2H6O", 0).unwrap(), tier);
        let b = net.add_species("B", Formula::parse("C2H6O", 0).unwrap(), tier);
        net.add_reversible("iso", vec![(a, 1)], vec![(b, 1)], 1.0, dg);
        Simulation::new(net, vec![n_a, 0], 0xC0FFEE)
    }

    /// The central claim of the overlay: individuation changes cost, not dynamics.
    #[test]
    fn counted_and_individuated_arms_produce_identical_trajectories() {
        let mut counted = isomerisation(Tier::Counted, 200, -2.0);
        let mut individuated = isomerisation(Tier::Individuated, 200, -2.0);

        let mut trace_c = Vec::new();
        let mut trace_i = Vec::new();
        for _ in 0..5_000 {
            counted.advance_one();
            individuated.advance_one();
            trace_c.push(counted.counts().to_vec());
            trace_i.push(individuated.counts().to_vec());
        }

        assert_eq!(
            trace_c, trace_i,
            "individuation must be a pure overlay: identical seeds must give identical counts"
        );
        assert_eq!(counted.time(), individuated.time(), "times must match bitwise");

        // ...and the cost differs, which is the whole point of measuring it.
        assert_eq!(counted.individuation_resident_bytes(), 0);
        assert!(individuated.individuation_resident_bytes() > 0);
        assert!(individuated.log().len() > counted.log().len());
    }

    #[test]
    fn same_seed_gives_a_bit_identical_trajectory() {
        let digest = |seed: u64| {
            let mut net = Network::new(T, ConcentrationUnit::sphere_nm(200.0));
            let a = net.add_species("A", Formula::parse("CH4", 0).unwrap(), Tier::Individuated);
            let b = net.add_species("B", Formula::parse("CH4", 0).unwrap(), Tier::Individuated);
            net.add_reversible("iso", vec![(a, 1)], vec![(b, 1)], 1.0, -1.0);
            let mut s = Simulation::new(net, vec![50, 50], seed);
            s.run_steps(3_000);
            (s.log().digest(), s.time().to_bits(), s.counts().to_vec())
        };
        assert_eq!(digest(7), digest(7), "invariant 7: same seed, same trajectory");
        assert_ne!(digest(7), digest(8), "different seeds must actually differ");
    }

    #[test]
    fn mass_is_conserved_in_a_closed_network() {
        let mut s = isomerisation(Tier::Individuated, 300, -3.0);
        assert!(s.is_closed());
        for _ in 0..2_000 {
            s.advance_one();
            s.assert_mass_conservation();
        }
        s.assert_identity_conservation();
    }

    #[test]
    fn every_entity_has_a_birth_and_the_dead_have_a_death() {
        let mut s = isomerisation(Tier::Individuated, 100, 0.5);
        s.run_steps(4_000);
        s.assert_identity_conservation();

        let births = s.log().count_of(EventKind::Birth);
        let deaths = s.log().count_of(EventKind::Death);
        assert_eq!(births - deaths, s.table().live_count() as u64);
        assert_eq!(s.table().births(), births);
    }

    #[test]
    fn equilibrium_matches_the_free_energy_that_was_specified() {
        // A <=> B, unimolecular: at equilibrium <B>/<A> = K_eq = exp(-dG/RT).
        let dg = -2.0;
        let mut s = isomerisation(Tier::Counted, 4_000, dg);
        s.run_steps(200_000);
        let (na, nb) = (s.counts()[0] as f64, s.counts()[1] as f64);
        let observed = nb / na;
        let expected = crate::thermo::k_eq_thermo(dg, T);
        assert!(
            (observed / expected - 1.0).abs() < 0.10,
            "equilibrium ratio {observed:.4} should approach K_eq {expected:.4}"
        );
    }

    #[test]
    fn absorbed_systems_stop_rather_than_spinning() {
        let mut net = Network::new(T, ConcentrationUnit::sphere_nm(200.0));
        let a = net.add_species("A", Formula::abstract_species(), Tier::Counted);
        net.add_irreversible("sink", vec![(a, 1)], vec![], 1.0, "test fixture: absorbing state")
            .unwrap();
        let mut s = Simulation::new(net, vec![10], 1);
        assert_eq!(s.run_steps(1_000), StopReason::Absorbed);
        assert_eq!(s.count(SpeciesId(0)), 0);
    }
}
