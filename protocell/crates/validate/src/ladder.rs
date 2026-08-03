//! The validation ladder, level L0: stochastic solvers against closed-form answers.
//!
//! `PROMPT.md` §5 requires every stochastic solver to be validated against an analytic
//! case before it touches biology. Three cases are implemented here, in increasing
//! strength:
//!
//! | Case | System | Analytic target | What it can catch |
//! |---|---|---|---|
//! | **L0a** | `∅ → X → ∅` | Poisson(k_b/k_d) | A broken SSA sampler; wrong waiting-time distribution |
//! | **L0b** | `A ⇌ B` closed | Binomial(N, k_f/(k_f+k_r)) | All of the above, **plus** a wrongly derived reverse rate |
//! | **L0c** | `2A ⇌ B` closed | exact chain stationary distribution | All of the above, **plus** a wrong combinatorial propensity |
//!
//! L0b is the one that carries invariant 3: `k_r` is never supplied, it is derived from
//! ΔG°′, so the binomial parameter `p` is a prediction of the thermodynamics. If the
//! standard-state factor were dropped the equilibrium would sit in the wrong place and
//! this test would fail.
//!
//! # Sampling in time, not in steps
//!
//! Samples are taken on a fixed *time* grid. Sampling every N SSA steps would weight
//! each state by how quickly it is left, giving a distribution biased towards
//! short-lived states — an error that looks like a small discrepancy rather than an
//! obvious failure, and that would then be "fixed" by tuning something else.

use protocell_chem::{
    formula::Formula,
    network::{Network, Tier},
    ssa::Simulation,
    thermo::{self, ConcentrationUnit},
};
use protocell_core::SpeciesId;

use crate::stats::{
    binomial_pmf, birth_death_chain_stationary, chi_square_gof, poisson_pmf, GofResult,
};

/// 37 °C, the temperature Syn3A is cultured and modelled at.
pub const T_CELL: f64 = 310.15;

/// Syn3A at birth: 200 nm radius. Sets `c°` for count-basis rate constants.
pub fn cell_unit() -> ConcentrationUnit {
    ConcentrationUnit::sphere_nm(200.0)
}

#[derive(Debug, Clone)]
pub struct LadderResult {
    pub name: &'static str,
    pub description: String,
    pub gof: GofResult,
    /// Observed and expected first moment, for a human-readable sanity line.
    pub observed_mean: f64,
    pub expected_mean: f64,
    /// The raw histogram, retained so the same samples can be re-scored against a
    /// deliberately wrong hypothesis. A goodness-of-fit test that has never been shown
    /// to reject anything is decoration.
    pub histogram: Vec<u64>,
}

impl LadderResult {
    /// Whether this single run survives at the 1% level.
    ///
    /// Prefer [`LadderEnsemble`] for anything that gates a Definition of Done: a single
    /// run fails 1% of the time by construction, and the temptation when it does is to
    /// try another seed.
    pub fn passes(&self) -> bool {
        self.gof.passes()
    }
}

/// The same benchmark across a fixed set of seeds, combined by Fisher's method.
///
/// This is what a ladder level is actually judged on. See [`crate::stats::fisher_combine`]
/// for why: one seed cannot distinguish a 1-in-300 unlucky draw from a real bias, and
/// re-rolling the seed until it passes is the same move as tuning a parameter until the
/// plot looks right.
#[derive(Debug, Clone)]
pub struct LadderEnsemble {
    pub name: &'static str,
    pub description: String,
    pub p_values: Vec<f64>,
    pub combined_chi2: f64,
    pub combined_p: f64,
    pub observed_mean: f64,
    pub expected_mean: f64,
}

impl LadderEnsemble {
    /// Combined-p threshold of 0.001: loose enough that the whole suite flakes rarely,
    /// tight enough to catch the systematic drift a single run would miss.
    pub fn passes(&self) -> bool {
        self.combined_p > 0.001
    }

    /// Seeds whose individual run fell below the 1% level. Reported, not acted on —
    /// one or two out of eight is expected.
    pub fn individually_low(&self) -> usize {
        self.p_values.iter().filter(|&&p| p <= 0.01).count()
    }
}

/// The eight seeds every ladder level is evaluated at. Fixed in the source so they
/// cannot drift, and so a reviewer can see that they were not chosen after the fact.
pub const LADDER_SEEDS: [u64; 8] = [
    0x0000_1111,
    0x2222_3333,
    0x4444_5555,
    0x6666_7777,
    0x8888_9999,
    0xAAAA_BBBB,
    0xCCCC_DDDD,
    0xEEEE_FFFF,
];

/// Run a ladder level at every seed in [`LADDER_SEEDS`] and combine.
pub fn run_ensemble(f: impl Fn(u64) -> LadderResult) -> LadderEnsemble {
    let runs: Vec<LadderResult> = LADDER_SEEDS.iter().map(|&s| f(s)).collect();
    let p_values: Vec<f64> = runs.iter().map(|r| r.gof.p_value).collect();
    let (combined_chi2, combined_p) = crate::stats::fisher_combine(&p_values);
    let n = runs.len() as f64;
    LadderEnsemble {
        name: runs[0].name,
        description: runs[0].description.clone(),
        p_values,
        combined_chi2,
        combined_p,
        observed_mean: runs.iter().map(|r| r.observed_mean).sum::<f64>() / n,
        expected_mean: runs[0].expected_mean,
    }
}

/// The count *in effect at* time `t`, which is not the same as the count after the
/// first jump past `t`.
///
/// # The bias this exists to avoid
///
/// After an SSA step, `sim.time()` is the instant the new state took effect, and that
/// state holds until the next event. Running until `sim.time() >= t` therefore lands on
/// the state that begins *after* `t`, not the one that spans it — every sample is
/// pulled one jump forward. The result is a distribution weighted towards states that
/// are easy to jump into, which shifts the equilibrium by a few percent: small enough to
/// look like a modelling discrepancy, large enough to fail a goodness-of-fit test.
///
/// This was a live bug. L0b caught it — chi² = 45 on 20 degrees of freedom, while the
/// first-moment check passed and would have let it through. That is the argument for
/// distributional benchmarks over "the mean looks about right".
fn sample_at(sim: &mut Simulation, species: SpeciesId, t: f64) -> u64 {
    // The state currently in effect began at `sim.time()` and holds until the next event.
    let mut in_effect = sim.count(species);
    while sim.time() < t {
        in_effect = sim.count(species);
        if !sim.advance_one() {
            // Absorbed: this state now holds forever, including at `t`.
            return in_effect;
        }
    }
    in_effect
}

/// Collect a species' count on a fixed time grid after a burn-in.
fn sample_on_time_grid(
    sim: &mut Simulation,
    species: SpeciesId,
    burn_in: f64,
    interval: f64,
    samples: usize,
) -> Vec<u64> {
    sim.run_until(burn_in);
    let mut out = Vec::with_capacity(samples);
    let mut t = burn_in;
    for _ in 0..samples {
        t += interval;
        out.push(sample_at(sim, species, t));
    }
    out
}

fn histogram(samples: &[u64], max: u64) -> Vec<u64> {
    let mut h = vec![0u64; max as usize + 1];
    for &s in samples {
        if s <= max {
            h[s as usize] += 1;
        }
    }
    h
}

/// **L0a** — immigration–death, `∅ →k_b→ X →k_d→ ∅`. Stationary: Poisson(k_b/k_d).
///
/// The network is open by construction, so mass conservation does not apply and the
/// reactions are declared irreversible with that justification recorded.
pub fn l0a_birth_death(seed: u64, k_b: f64, k_d: f64, samples: usize) -> LadderResult {
    let lambda = k_b / k_d;

    let mut net = Network::new(T_CELL, cell_unit());
    let x = net.add_species("X", Formula::abstract_species(), Tier::Counted);
    net.add_irreversible(
        "immigration",
        vec![],
        vec![(x, 1)],
        k_b,
        "L0a analytic benchmark: constant-rate source is the definition of the \
         immigration-death process; not a claim about chemistry",
    )
    .unwrap();
    net.add_irreversible(
        "death",
        vec![(x, 1)],
        vec![],
        k_d,
        "L0a analytic benchmark: first-order sink, as above",
    )
    .unwrap();

    let mut sim = Simulation::new(net, vec![0], seed);
    let relax = 1.0 / k_d;
    let obs = sample_on_time_grid(&mut sim, x, 20.0 * relax, 5.0 * relax, samples);

    let max = (lambda + 8.0 * lambda.sqrt()).ceil() as u64;
    let h = histogram(&obs, max);
    let mean = obs.iter().map(|&v| v as f64).sum::<f64>() / obs.len() as f64;

    LadderResult {
        name: "L0a",
        description: format!("immigration-death, lambda = {lambda:.3}"),
        gof: chi_square_gof(&h, |k| poisson_pmf(k, lambda), 5.0),
        observed_mean: mean,
        expected_mean: lambda,
        histogram: h,
    }
}

/// **L0b** — closed isomerisation `A ⇌ B`, `k_r` derived from ΔG°′.
///
/// Stationary distribution of `n_B` is Binomial(N, p) with `p = k_f/(k_f + k_r)`.
/// This is the level at which invariant 3 becomes falsifiable: nothing in the setup
/// supplies `p`, it follows from the free energy.
pub fn l0b_reversible_isomerisation(
    seed: u64,
    n_total: u64,
    k_f: f64,
    dg_kj_per_mol: f64,
    samples: usize,
) -> LadderResult {
    let mut net = Network::new(T_CELL, cell_unit());
    // A real formula, so the balance check is not vacuous: two conformers of the same
    // molecule differ in no atom, which is exactly what makes the test meaningful.
    let a = net.add_species("A", Formula::parse("C6H11O9P", -2).unwrap(), Tier::Counted);
    let b = net.add_species("B", Formula::parse("C6H11O9P", -2).unwrap(), Tier::Counted);
    net.add_reversible("iso", vec![(a, 1)], vec![(b, 1)], k_f, dg_kj_per_mol);

    // Δn = 0 here, so the standard-state factor is unity and p depends only on ΔG.
    let k_r = thermo::reverse_rate(k_f, dg_kj_per_mol, T_CELL, 0, cell_unit());
    let p = k_f / (k_f + k_r);

    let mut sim = Simulation::new(net, vec![n_total, 0], seed);
    let relax = 1.0 / (k_f + k_r);
    let obs = sample_on_time_grid(&mut sim, b, 20.0 * relax, 5.0 * relax, samples);

    let h = histogram(&obs, n_total);
    let mean = obs.iter().map(|&v| v as f64).sum::<f64>() / obs.len() as f64;

    LadderResult {
        name: "L0b",
        description: format!(
            "closed A<=>B, N = {n_total}, dG = {dg_kj_per_mol} kJ/mol, p = {p:.5} (derived)"
        ),
        gof: chi_square_gof(&h, |k| binomial_pmf(k, n_total, p), 5.0),
        observed_mean: mean,
        expected_mean: n_total as f64 * p,
        histogram: h,
    }
}

/// **L0c** — closed dimerisation `2A ⇌ B`.
///
/// One-dimensional in `n_B`, so the exact stationary distribution follows from detailed
/// balance. This is the level that exercises the combinatorial propensity `C(n,2)`;
/// using `n²` instead shifts the equilibrium by a factor that vanishes only at large
/// copy number, and Syn3A copy numbers are not large.
pub fn l0c_reversible_dimerisation(
    seed: u64,
    n_a_initial: u64,
    k_f: f64,
    dg_kj_per_mol: f64,
    samples: usize,
) -> LadderResult {
    let mut net = Network::new(T_CELL, cell_unit());
    let a = net.add_species("A", Formula::parse("CH3", 0).unwrap(), Tier::Counted);
    let b = net.add_species("B", Formula::parse("C2H6", 0).unwrap(), Tier::Counted);
    net.add_reversible("dimerise", vec![(a, 2)], vec![(b, 1)], k_f, dg_kj_per_mol);

    let k_r = thermo::reverse_rate(k_f, dg_kj_per_mol, T_CELL, -1, cell_unit());

    let max_b = (n_a_initial / 2) as usize;
    let pi = birth_death_chain_stationary(
        max_b + 1,
        |nb| {
            let na = n_a_initial - 2 * nb as u64;
            k_f * (na as f64) * ((na as f64) - 1.0) / 2.0
        },
        |nb| k_r * nb as f64,
    );

    let mut sim = Simulation::new(net, vec![n_a_initial, 0], seed);
    // Relaxation is dominated by the slower of the two directions at this population.
    let a_f = k_f * (n_a_initial as f64) * (n_a_initial as f64 - 1.0) / 2.0;
    let relax = 1.0 / (a_f / n_a_initial as f64 + k_r).max(1e-12);
    let obs = sample_on_time_grid(&mut sim, b, 20.0 * relax, 5.0 * relax, samples);

    let h = histogram(&obs, max_b as u64);
    let mean = obs.iter().map(|&v| v as f64).sum::<f64>() / obs.len() as f64;
    let expected_mean: f64 = pi.iter().enumerate().map(|(i, p)| i as f64 * p).sum();

    LadderResult {
        name: "L0c",
        description: format!("closed 2A<=>B, N_A0 = {n_a_initial}, dG = {dg_kj_per_mol} kJ/mol"),
        gof: chi_square_gof(&h, |k| pi.get(k as usize).copied().unwrap_or(0.0), 5.0),
        observed_mean: mean,
        expected_mean,
        histogram: h,
    }
}

/// Outcome of the kill test (invariant 4).
#[derive(Debug, Clone)]
pub struct KillTestResult {
    /// Entropy production rate over time, in units of `R`, from the ensemble mean state.
    pub sigma: Vec<f64>,
    /// Reactions that were declared irreversible, and therefore cannot equilibrate.
    pub irreversible: Vec<String>,
    pub monotone: bool,
    pub relaxed: bool,
    pub restarted: bool,
}

impl KillTestResult {
    pub fn passes(&self) -> bool {
        self.irreversible.is_empty() && self.monotone && self.relaxed && !self.restarted
    }
}

/// **Invariant 4 — the kill test.** With no input, a closed system must relax
/// monotonically to equilibrium and stay there.
///
/// # Why this is run on an ensemble
///
/// A single stochastic trajectory's entropy production is not monotone — it fluctuates,
/// and at equilibrium it fluctuates around zero. The second law constrains the
/// *ensemble*. So the test averages the state over replicates and evaluates σ on the
/// mean state, which is the quantity the law actually speaks about.
///
/// # Why irreversible reactions are reported rather than tolerated
///
/// The scaffold's invariant 4 required monotone relaxation with σ → 0. Any strictly
/// irreversible reaction makes that unreachable: it runs to completion instead of
/// equilibrating, and σ stays pinned at infinity. Whole-cell models are full of steps
/// declared irreversible for convenience, so this is not a hypothetical. The test
/// therefore *names* them instead of silently failing, and the fix is either to supply
/// the reverse ΔG°′ or to record a relaxation in `docs/RELAXATIONS.md`.
pub fn kill_test(
    build: impl Fn(u64) -> Simulation,
    replicates: u64,
    t_end: f64,
    grid: usize,
) -> KillTestResult {
    let probe = build(0);
    let irreversible: Vec<String> = probe
        .network()
        .irreversible_declarations()
        .iter()
        .map(|d| probe.network().reactions()[d.reaction].name.clone())
        .collect();

    let nspecies = probe.network().species().len();
    let mut sims: Vec<Simulation> = (0..replicates).map(&build).collect();
    let mut sigma = Vec::with_capacity(grid);

    for g in 1..=grid {
        let t = t_end * g as f64 / grid as f64;
        let mut mean = vec![0.0f64; nspecies];
        for s in sims.iter_mut() {
            s.run_until(t);
            for (i, c) in s.counts().iter().enumerate() {
                mean[i] += *c as f64;
            }
        }
        for m in mean.iter_mut() {
            *m /= replicates as f64;
        }

        // Evaluate sigma on the ensemble-mean state, via a scratch simulation whose
        // counts are set to the (rounded) mean.
        let mut probe = build(0);
        probe.set_counts_for_analysis(&mean);
        sigma.push(probe.entropy_production_rate());
    }

    // Monotone within a tolerance that accounts for the finite ensemble: a rise of more
    // than 5% of the initial value is a real violation, smaller ones are sampling noise.
    let tol = sigma.first().copied().unwrap_or(0.0).abs() * 0.05 + 1e-9;
    let monotone = sigma.windows(2).all(|w| w[1] <= w[0] + tol);
    let last = sigma.last().copied().unwrap_or(f64::INFINITY);
    let first = sigma.first().copied().unwrap_or(0.0);
    let relaxed = last.abs() <= first.abs() * 0.01 + 1e-9;
    // A restart is a late excursion above the tolerance after having relaxed.
    let tail = &sigma[sigma.len().saturating_sub(grid / 4).max(1)..];
    let restarted = tail.iter().any(|&s| s > first.abs() * 0.05 + 1e-9);

    KillTestResult {
        sigma,
        irreversible,
        monotone,
        relaxed,
        restarted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_ensemble(e: &LadderEnsemble) {
        assert!(
            e.passes(),
            "{} failed across {} seeds: combined chi2 = {:.2}, p = {:.3e}, individual p = {:?}",
            e.name,
            e.p_values.len(),
            e.combined_chi2,
            e.combined_p,
            e.p_values
                .iter()
                .map(|p| format!("{p:.4}"))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn l0a_matches_the_poisson_stationary_distribution() {
        let e = run_ensemble(|s| l0a_birth_death(s, 20.0, 1.0, 20_000));
        assert_ensemble(&e);
        assert!((e.observed_mean - e.expected_mean).abs() < 0.2);
    }

    #[test]
    fn l0b_matches_the_binomial_predicted_by_the_free_energy() {
        let e = run_ensemble(|s| l0b_reversible_isomerisation(s, 40, 1.0, -2.0, 20_000));
        assert_ensemble(&e);
        assert!((e.observed_mean - e.expected_mean).abs() < 0.2);
    }

    /// The ladder's own control: at one seed a correct benchmark fails 1% of the time,
    /// and that is exactly the failure that gets "fixed" by re-rolling. Documenting the
    /// case that motivated the ensemble keeps the reasoning attached to the code.
    #[test]
    fn a_single_seed_can_fail_a_correct_benchmark() {
        let unlucky = l0b_reversible_isomerisation(0x2222, 40, 1.0, -2.0, 20_000);
        assert!(
            !unlucky.passes(),
            "seed 0x2222 is retained because it is the counter-example: it fails at the \
             1% level while the model is correct"
        );
        // ...and the ensemble containing eight other seeds is unmoved by it.
        let e = run_ensemble(|s| l0b_reversible_isomerisation(s, 40, 1.0, -2.0, 20_000));
        assert!(e.passes());
    }

    /// The negative control for L0b. If the reverse rate were set independently rather
    /// than derived from ΔG°′, the equilibrium would sit elsewhere. Re-scoring the *same
    /// samples* against that wrong hypothesis must reject it — otherwise L0b passing
    /// tells us nothing about invariant 3, only that some binomial fits.
    #[test]
    fn l0b_would_reject_an_independently_chosen_reverse_rate() {
        let (n, k_f, dg) = (40u64, 1.0, -2.0);
        let k_r_true = thermo::reverse_rate(k_f, dg, T_CELL, 0, cell_unit());
        let p_true = k_f / (k_f + k_r_true);
        // Someone sets both rates by hand and lands 30% off on the reverse.
        let p_wrong = k_f / (k_f + k_r_true * 1.3);
        assert!((p_true - p_wrong).abs() > 1e-3, "hypotheses must actually differ");

        let truth = run_ensemble(|s| l0b_reversible_isomerisation(s, n, k_f, dg, 20_000));
        assert!(truth.passes(), "the true hypothesis must fit");

        // The same samples, scored against the wrong reverse rate, across the same seeds.
        let wrong_ps: Vec<f64> = LADDER_SEEDS
            .iter()
            .map(|&s| {
                let r = l0b_reversible_isomerisation(s, n, k_f, dg, 20_000);
                chi_square_gof(&r.histogram, |k| binomial_pmf(k, n, p_wrong), 5.0).p_value
            })
            .collect();
        let (_, combined) = crate::stats::fisher_combine(&wrong_ps);
        assert!(
            combined < 1e-6,
            "a 30% error in the derived reverse rate must be overwhelmingly rejected, \
             got combined p = {combined:.3e}"
        );
    }

    #[test]
    fn l0c_matches_the_exact_chain_stationary_distribution() {
        // ΔG chosen so the equilibrium sits near n_B ≈ 10 of a possible 30: a
        // degenerate equilibrium pinned at 0 or at the ceiling would test nothing.
        let e = run_ensemble(|s| l0c_reversible_dimerisation(s, 60, 0.05, -32.0, 20_000));
        assert_ensemble(&e);
        assert!((e.observed_mean - e.expected_mean).abs() < 0.2);
    }

    /// The deep check: under a correct sampler the p-value is uniform on [0,1], so 40
    /// independent seeds should show a median near 0.5 and about two results below 0.05.
    ///
    /// This is strictly stronger than the eight-seed gate — it can detect a bias too
    /// small for any single run — but it takes ~30 s, so it is opt-in rather than run on
    /// every commit:
    ///
    /// ```text
    /// cargo test --release -p protocell-validate -- --ignored --nocapture
    /// ```
    ///
    /// Last run (recorded in `docs/STATE.md`): medians 0.58 / 0.54 / 0.41, combined
    /// p 0.38 / 0.81 / 0.20. No level shows bias.
    #[test]
    #[ignore = "slow: 120 SSA runs; run explicitly with --ignored"]
    fn p_values_are_uniform_across_forty_seeds() {
        let cases: [(&str, &dyn Fn(u64) -> LadderResult); 3] = [
            ("L0a", &|s| l0a_birth_death(s, 20.0, 1.0, 20_000)),
            ("L0b", &|s| l0b_reversible_isomerisation(s, 40, 1.0, -2.0, 20_000)),
            ("L0c", &|s| l0c_reversible_dimerisation(s, 60, 0.05, -32.0, 20_000)),
        ];
        for (name, f) in cases {
            let ps: Vec<f64> = (0..40u64)
                .map(|i| f(0x9E37_79B9_7F4A_7C15u64.wrapping_mul(i + 1)).gof.p_value)
                .collect();
            let (x, combined) = crate::stats::fisher_combine(&ps);
            let mut sorted = ps.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let median = sorted[20];
            println!("{name}: median p = {median:.3}, Fisher X = {x:.1}, combined p = {combined:.4}");
            assert!(
                combined > 0.01,
                "{name} shows systematic bias across 40 seeds: combined p = {combined:.4}"
            );
            assert!(
                (0.2..0.8).contains(&median),
                "{name} p-value median {median:.3} is far from uniform"
            );
        }
    }

    #[test]
    fn kill_test_passes_for_a_closed_reversible_system() {
        let build = |seed: u64| {
            let mut net = Network::new(T_CELL, cell_unit());
            let a = net.add_species("A", Formula::parse("C2H6O", 0).unwrap(), Tier::Counted);
            let b = net.add_species("B", Formula::parse("C2H6O", 0).unwrap(), Tier::Counted);
            net.add_reversible("iso", vec![(a, 1)], vec![(b, 1)], 1.0, -2.0);
            Simulation::new(net, vec![400, 0], 0xBEEF + seed)
        };
        let r = kill_test(build, 24, 12.0, 24);
        assert!(r.irreversible.is_empty());
        assert!(r.monotone, "entropy production must not increase: {:?}", r.sigma);
        assert!(r.relaxed, "must reach equilibrium: {:?}", r.sigma);
        assert!(!r.restarted, "must not restart: {:?}", r.sigma);
        assert!(r.passes());
    }

    /// The kill test must *fail* in the presence of an irreversible reaction, and say
    /// which one. A test that cannot fail is not a test.
    #[test]
    fn kill_test_names_irreversible_reactions_instead_of_passing_quietly() {
        let build = |seed: u64| {
            let mut net = Network::new(T_CELL, cell_unit());
            let a = net.add_species("A", Formula::abstract_species(), Tier::Counted);
            let b = net.add_species("B", Formula::abstract_species(), Tier::Counted);
            net.add_reversible("iso", vec![(a, 1)], vec![(b, 1)], 1.0, -2.0);
            net.add_irreversible(
                "leak",
                vec![(b, 1)],
                vec![(a, 1)],
                0.1,
                "deliberately unsound: fixture for the kill test's failure path",
            )
            .unwrap();
            Simulation::new(net, vec![200, 0], seed)
        };
        let r = kill_test(build, 8, 5.0, 8);
        assert_eq!(r.irreversible, vec!["leak".to_string()]);
        assert!(!r.passes(), "a system with an irreversible step must not pass");
    }
}
