//! `protocell` — the P0 command line.
//!
//! Subcommands:
//!
//! - `budget`  — the memory and event-log arithmetic (`docs/BUDGET.md`)
//! - `l0`      — the analytic validation ladder
//! - `kill`    — the kill test (invariant 4)
//! - `arms`    — counted vs individuated, the two-arm cost comparison
//! - `dod`     — check the P0 Definition of Done end to end

use protocell_chem::{
    formula::Formula,
    network::{Network, Tier},
    ssa::Simulation,
};
use protocell_validate::{
    budget::{human_bytes, log_budget, memory_budget, syn3a_budget_inputs},
    ladder::{cell_unit, kill_test, l0a_birth_death, l0b_reversible_isomerisation,
             l0c_reversible_dimerisation, run_ensemble, LadderEnsemble, T_CELL},
    replay::{draws_at, EntityHistory},
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("dod");
    let ok = match cmd {
        "budget" => {
            cmd_budget();
            true
        }
        "l0" => cmd_l0(),
        "kill" => cmd_kill(),
        "arms" => {
            cmd_arms();
            true
        }
        "dod" => cmd_dod(),
        other => {
            eprintln!("unknown subcommand `{other}`");
            eprintln!("usage: protocell [budget|l0|kill|arms|dod]");
            std::process::exit(2);
        }
    };
    if !ok {
        std::process::exit(1);
    }
}

fn rule(title: &str) {
    println!("\n=== {title} {}", "=".repeat(66usize.saturating_sub(title.len())));
}

fn cmd_budget() {
    let t = syn3a_budget_inputs();

    rule("Inputs");
    println!("{:<32} {:>14}  {:<10} provenance", "parameter", "value", "units");
    for p in t.iter() {
        println!(
            "{:<32} {:>14.4e}  {:<10} {}",
            p.name,
            p.value,
            p.units,
            p.method.label()
        );
    }
    let est = t.iter().filter(|p| p.method.label() == "estimated").count();
    println!(
        "\n{est} of {} inputs are estimates, labelled as such. None are fitted.",
        t.len()
    );

    rule("Finding 4 — resident memory for individuation");
    let m = memory_budget(&t, 64);
    println!("live T2 entities (Syn3A)      {:>12.3e}", m.live_entities);
    println!("bytes per entity              {:>12}", m.bytes_per_entity);
    println!("resident, slots recycled      {:>12}", human_bytes(m.resident_bytes));
    println!("cumulative births per cycle   {:>12.3e}", m.cumulative_entities);
    println!(
        "resident if never evicted     {:>12}   <- the naive design",
        human_bytes(m.cumulative_bytes)
    );
    println!(
        "\nThe HPC role's veto threshold was 1e7 entities. Syn3A's T2 population is\n\
         {:.1e}, three orders of magnitude below it. Individuation does not have a\n\
         memory problem; it may still have a throughput problem, which is what P3\n\
         must measure instead.",
        m.live_entities
    );

    rule("Finding 3 — event log volume per cell cycle");
    let l = log_budget(&t, 32);
    println!("RDME steps per cycle          {:>12.3e}", l.steps_per_cycle);
    println!("bytes per event               {:>12}", l.bytes_per_event);
    println!("diffusion hops per cycle      {:>12.3e}", l.naive_hops_per_cycle);
    println!(
        "log size IF hops were logged  {:>12}   <- {:.0}x worse than the 80 TB dump",
        human_bytes(l.naive_bytes),
        l.naive_bytes / t.value("frame_dump_estimate_bytes")
    );
    println!("non-reconstructible events    {:>12.3e}", l.events_per_cycle);
    println!("actual log size               {:>12}", human_bytes(l.log_bytes));
    println!(
        "as a fraction of 80 TB        {:>12.3e}",
        l.fraction_of_frame_dump
    );
    println!(
        "\nThe gap between those two lines is the architecture. It exists only because\n\
         every reconstructible outcome is omitted from the log and recomputed from its\n\
         RNG address. See crates/core/src/rng.rs for the two rules that keep that true."
    );
}

fn report(e: &LadderEnsemble) -> bool {
    let pass = e.passes();
    println!(
        "{:<5} {:<61} {}",
        e.name,
        e.description,
        if pass { "PASS" } else { "FAIL" }
    );
    println!(
        "      combined over {} seeds: chi2 = {:.2}, p = {:.4}  ({} individually below 1%)",
        e.p_values.len(),
        e.combined_chi2,
        e.combined_p,
        e.individually_low()
    );
    println!(
        "      mean {:.4} (expected {:.4}); per-seed p = {}",
        e.observed_mean,
        e.expected_mean,
        e.p_values
            .iter()
            .map(|p| format!("{p:.3}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    pass
}

fn cmd_l0() -> bool {
    rule("L0 — stochastic solvers against closed-form answers");
    let a = run_ensemble(|s| l0a_birth_death(s, 20.0, 1.0, 20_000));
    let b = run_ensemble(|s| l0b_reversible_isomerisation(s, 40, 1.0, -2.0, 20_000));
    let c = run_ensemble(|s| l0c_reversible_dimerisation(s, 60, 0.05, -32.0, 20_000));
    let ok = report(&a) & report(&b) & report(&c);
    println!(
        "\nEach level is judged on eight fixed seeds combined by Fisher's method, not on\n\
         one run: a correct benchmark fails a single-seed test 1% of the time, and the\n\
         reflex when it does is to re-roll the seed. That is seed-shopping, and it is the\n\
         same move as tuning a parameter until the plot looks right.\n\n\
         L0b carries invariant 3: k_r is never supplied, it is derived from dG. The\n\
         binomial parameter is a prediction of the thermodynamics, not an input."
    );
    ok
}

fn kill_fixture(seed: u64) -> Simulation {
    let mut net = Network::new(T_CELL, cell_unit());
    let a = net.add_species("A", Formula::parse("C2H6O", 0).unwrap(), Tier::Counted);
    let b = net.add_species("B", Formula::parse("C2H6O", 0).unwrap(), Tier::Counted);
    net.add_reversible("iso", vec![(a, 1)], vec![(b, 1)], 1.0, -2.0);
    Simulation::new(net, vec![400, 0], 0xBEEF + seed)
}

fn cmd_kill_quiet() -> bool {
    kill_test(kill_fixture, 24, 12.0, 24).passes()
}

fn cmd_kill() -> bool {
    rule("Invariant 4 — kill test");
    let r = kill_test(kill_fixture, 24, 12.0, 24);
    println!("irreversible reactions:  {:?}", r.irreversible);
    println!("monotone decrease:       {}", r.monotone);
    println!("relaxed to equilibrium:  {}", r.relaxed);
    println!("spontaneous restart:     {}", r.restarted);
    println!(
        "\nsigma(t)/sigma(0): {}",
        r.sigma
            .iter()
            .step_by(4)
            .map(|s| format!("{:.3}", s / r.sigma[0]))
            .collect::<Vec<_>>()
            .join(" ")
    );
    println!(
        "\n{}",
        if r.passes() {
            "PASS"
        } else {
            "FAIL — see docs/RELAXATIONS.md before relaxing anything"
        }
    );
    r.passes()
}

fn build_arm(tier: Tier, seed: u64) -> Simulation {
    let mut net = Network::new(T_CELL, cell_unit());
    let a = net.add_species("A", Formula::parse("C6H11O9P", -2).unwrap(), tier);
    let b = net.add_species("B", Formula::parse("C6H11O9P", -2).unwrap(), tier);
    net.add_reversible("iso", vec![(a, 1)], vec![(b, 1)], 1.0, -2.0);
    Simulation::new(net, vec![500, 0], seed)
}

fn cmd_arms() {
    rule("Two arms — counted vs individuated, same seed");
    const STEPS: u64 = 50_000;
    let seed = 0xA11CE;

    let mut counted = build_arm(Tier::Counted, seed);
    let mut individuated = build_arm(Tier::Individuated, seed);
    counted.run_steps(STEPS);
    individuated.run_steps(STEPS);

    let identical = counted.counts() == individuated.counts()
        && counted.time().to_bits() == individuated.time().to_bits();

    println!("steps                    {STEPS}");
    println!("counted   final counts   {:?}", counted.counts());
    println!("individuated final       {:?}", individuated.counts());
    println!(
        "trajectories identical   {}",
        if identical { "yes" } else { "NO — the overlay perturbs the dynamics" }
    );
    println!();
    println!("{:<26} {:>14} {:>14}", "arm", "counted", "individuated");
    println!(
        "{:<26} {:>14} {:>14}",
        "resident (individuation)",
        human_bytes(counted.individuation_resident_bytes() as f64),
        human_bytes(individuated.individuation_resident_bytes() as f64)
    );
    println!(
        "{:<26} {:>14} {:>14}",
        "log events",
        counted.log().len(),
        individuated.log().len()
    );
    println!(
        "{:<26} {:>14} {:>14}",
        "log bytes",
        human_bytes(counted.log().bytes() as f64),
        human_bytes(individuated.log().bytes() as f64)
    );

    // What the individuated arm can answer and the counted arm cannot.
    if let Some(id) = individuated.table().iter_live().next() {
        let h = EntityHistory::extract(individuated.log(), id);
        println!(
            "\nA question only the individuated arm can answer:\n  entity {} born at step {:?}, \
             {} events in its history, still alive at step {}",
            h.id,
            h.birth_step,
            h.events.len(),
            individuated.step_number()
        );
    }
    println!(
        "\nThis is the control the thesis needs in miniature. Any claim of the form\n\
         \"individuation buys X\" must be run against a non-individuated arm that is\n\
         otherwise identical — see docs/PREREGISTRATION.md."
    );
    assert!(identical, "the two arms diverged; that is a bug in the overlay");
}

fn cmd_dod() -> bool {
    let mut ok = true;

    rule("P0 Definition of Done");

    // Invariant 1: atom and charge balance, checked at load.
    let mut bad = Network::new(T_CELL, cell_unit());
    let x = bad.add_species("X", Formula::parse("CH4", 0).unwrap(), Tier::Counted);
    let y = bad.add_species("Y", Formula::parse("CH3", 0).unwrap(), Tier::Counted);
    bad.add_reversible("unbalanced", vec![(x, 1)], vec![(y, 1)], 1.0, 0.0);
    let inv1 = bad.validate().is_err();
    println!("invariant 1  atom/charge balance rejects an unbalanced reaction   {}", pf(inv1));
    ok &= inv1;

    // Invariant 2: mass conservation in a closed network.
    let mut s = build_arm(Tier::Individuated, 1234);
    s.run_steps(20_000);
    s.assert_mass_conservation();
    println!("invariant 2  mass conserved over 20000 steps                      {}", pf(true));

    // Invariant 3: reverse rates are derived, and L0b would catch it if they were not.
    let l0b = run_ensemble(|s| l0b_reversible_isomerisation(s, 40, 1.0, -2.0, 20_000));
    println!("invariant 3  equilibrium matches the free energy (L0b)            {}", pf(l0b.passes()));
    ok &= l0b.passes();

    // Invariant 7: determinism.
    let digest = |seed| {
        let mut s = build_arm(Tier::Individuated, seed);
        s.run_steps(20_000);
        (s.log().digest(), s.time().to_bits())
    };
    let det = digest(99) == digest(99) && digest(99) != digest(100);
    println!("invariant 7  same seed => bit-identical trajectory                {}", pf(det));
    ok &= det;

    // Invariant 8: identity conservation.
    s.assert_identity_conservation();
    println!("invariant 8  identity conserved; every entity born and buried     {}", pf(true));

    // Standalone replay.
    let d = draws_at(1234, 12_345);
    let replay_ok = d == draws_at(1234, 12_345) && (0.0..1.0).contains(&d.channel_uniform);
    println!("             a single event's draws recover in O(1)               {}", pf(replay_ok));
    ok &= replay_ok;

    // L0.
    let l0a = run_ensemble(|s| l0a_birth_death(s, 20.0, 1.0, 20_000));
    let l0c = run_ensemble(|s| l0c_reversible_dimerisation(s, 60, 0.05, -32.0, 20_000));
    println!("L0           analytic benchmarks a/b/c, 8 seeds each              {}",
             pf(l0a.passes() && l0b.passes() && l0c.passes()));
    ok &= l0a.passes() && l0c.passes();

    // Kill test.
    let kill = cmd_kill_quiet();
    println!("invariant 4  kill test on a closed reversible system              {}", pf(kill));
    ok &= kill;

    rule("Honest caveats");
    println!(
        "- The P0 networks are two conformers of one real formula. Invariant 1 is\n\
        \x20 exercised but proves nothing about Syn3A chemistry; that starts at P1.\n\
        - The kill test passes on a closed reversible system. Real metabolic models\n\
        \x20 contain irreversible steps, which make invariant 4 as originally written\n\
        \x20 unreachable. See docs/RELAXATIONS.md R-0001.\n\
        - Determinism is asserted single-threaded. GPU reductions are not order-stable\n\
        \x20 by default and this will cost throughput at P2. See docs/DECISIONS.md ADR-0005."
    );

    println!("\n{}", if ok { "P0 DoD: GREEN" } else { "P0 DoD: RED" });
    ok
}

fn pf(b: bool) -> &'static str {
    if b {
        "PASS"
    } else {
        "FAIL"
    }
}
