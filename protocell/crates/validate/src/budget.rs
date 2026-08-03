//! The two de-risking calculations, done before the code that depends on them.
//!
//! `docs/REVIEW.md` findings 3 and 4: the project's entire cost argument rested on
//! "event log instead of an 80 TB frame dump", with no arithmetic anywhere, and its
//! stated project-killing risk — that individuation costs more memory than it buys —
//! was deferred to phase P3. Both are an hour of arithmetic, and both can end the
//! project cheaply if the answer is bad. So they are done here, first, as code that
//! reruns rather than prose that goes stale.
//!
//! Inputs carry provenance like every other parameter. Several are `Estimated`, which
//! is legal and labelled — see `provenance::Method`. An estimate you can see is a
//! scientific statement; the same number with a fabricated DOI is not.

use protocell_chem::provenance::{Method, Param, ParamTable, Uncertainty};

/// Syn3A cell-cycle and population figures used by both calculations.
pub fn syn3a_budget_inputs() -> ParamTable {
    let mut t = ParamTable::new();
    let mut add = |p: Param| t.insert(p).expect("budget inputs must carry provenance");

    add(Param::new(
        "cell_cycle_s",
        105.0 * 60.0,
        "s",
        Method::Measured {
            doi: "10.7554/eLife.36842".into(),
            technique: "growth curve, Breuer et al. 2019 (JCVI-syn3A doubling time)".into(),
        },
        Uncertainty::Relative(0.10),
    )
    .unwrap());

    add(Param::new(
        "rdme_timestep_s",
        50e-6,
        "s",
        Method::Inherited {
            source: "4DWCM (doi:10.1101/2025.06.10.658899)".into(),
            note: "RDME timestep; adopted so the comparison is like for like".into(),
        },
        Uncertainty::Unquantified,
    )
    .unwrap());

    add(Param::new(
        "ribosomes_at_division",
        881.0,
        "molecules",
        Method::Inherited {
            source: "4DWCM".into(),
            note: "predicted count at division; 500 at birth".into(),
        },
        Uncertainty::Relative(0.15),
    )
    .unwrap());

    add(Param::new(
        "rnap_at_division",
        176.0,
        "molecules",
        Method::Inherited {
            source: "4DWCM".into(),
            note: "predicted count at division".into(),
        },
        Uncertainty::Relative(0.15),
    )
    .unwrap());

    add(Param::new(
        "degradosomes_at_division",
        192.0,
        "molecules",
        Method::Inherited {
            source: "4DWCM".into(),
            note: "predicted count at division".into(),
        },
        Uncertainty::Relative(0.15),
    )
    .unwrap());

    add(Param::new(
        "trna_total",
        5800.0,
        "molecules",
        Method::Inherited {
            source: "4DWCM".into(),
            note: "200 per isoform x 29 isoforms".into(),
        },
        Uncertainty::Relative(0.30),
    )
    .unwrap());

    add(Param::new(
        "protein_molecules_total",
        1.0e5,
        "molecules",
        Method::Estimated {
            basis: "volume scaling from E. coli (~3e6 proteins at ~1 fL) to a 200 nm-radius \
                    Syn3A cell (~0.034 fL), cross-checked against the ribosome count: \
                    E. coli ~20000 ribosomes/fL scales to ~670, and the 4DWCM starts at 500"
                .into(),
        },
        Uncertainty::OrderOfMagnitude(3.0),
    )
    .unwrap());

    add(Param::new(
        "mrna_standing_population",
        250.0,
        "molecules",
        Method::Estimated {
            basis: "4DWCM samples initial mRNA as Poisson at 2x the Thornburg 2022 means \
                    across 493 genes; standing population is order 1e2, not 1e3"
                .into(),
        },
        Uncertainty::OrderOfMagnitude(3.0),
    )
    .unwrap());

    add(Param::new(
        "mrna_half_life_s",
        120.0,
        "s",
        Method::Estimated {
            basis: "bacterial mRNA half-lives are 1-10 min; the 4DWCM's degradosome kinetics \
                    put Syn3A in the middle of that range"
                .into(),
        },
        Uncertainty::OrderOfMagnitude(3.0),
    )
    .unwrap());

    add(Param::new(
        "hop_probability_per_particle_per_step",
        0.5,
        "dimensionless",
        Method::Estimated {
            basis: "D*dt/l^2 for a cytoplasmic protein: D ~ 1 um^2/s, dt = 50 us, \
                    l = 10 nm lattice spacing. Order unity, which is the point — a \
                    mobile particle hops most steps"
                .into(),
        },
        Uncertainty::OrderOfMagnitude(3.0),
    )
    .unwrap());

    add(Param::new(
        "frame_dump_estimate_bytes",
        80.0e12,
        "bytes",
        Method::Inherited {
            source: "4DWCM Limitations".into(),
            note: "their own estimate of the cost of recording every RDME frame in order \
                   to follow one mRNA: >80 TB per trajectory"
                .into(),
        },
        Uncertainty::OrderOfMagnitude(2.0),
    )
    .unwrap());

    t
}

#[derive(Debug, Clone)]
pub struct MemoryBudget {
    pub live_entities: f64,
    pub bytes_per_entity: usize,
    pub resident_bytes: f64,
    /// What it would cost if dead entities were never evicted — the naive design.
    pub cumulative_entities: f64,
    pub cumulative_bytes: f64,
}

/// **Finding 4** — resident memory for individuation, computed rather than feared.
///
/// The scaffold's HPC role vetoed anything that "does not fit in memory at 1e7
/// entities". That figure is not a Syn3A figure. The T2 population — proteins,
/// ribosomes, RNAP, degradosomes, tRNA, mRNA — is order 1e5.
pub fn memory_budget(t: &ParamTable, bytes_per_entity: usize) -> MemoryBudget {
    let live = t.value("protein_molecules_total")
        + t.value("ribosomes_at_division")
        + t.value("rnap_at_division")
        + t.value("degradosomes_at_division")
        + t.value("trna_total")
        + t.value("mrna_standing_population");

    // Cumulative births over one cycle: the standing population doubles, and mRNA turns
    // over on its half-life throughout.
    let cycle = t.value("cell_cycle_s");
    let mrna_turnovers = cycle / t.value("mrna_half_life_s");
    let cumulative = live + t.value("mrna_standing_population") * mrna_turnovers;

    MemoryBudget {
        live_entities: live,
        bytes_per_entity,
        resident_bytes: live * bytes_per_entity as f64,
        cumulative_entities: cumulative,
        cumulative_bytes: cumulative * bytes_per_entity as f64,
    }
}

#[derive(Debug, Clone)]
pub struct LogBudget {
    pub steps_per_cycle: f64,
    pub bytes_per_event: usize,
    /// If diffusion hops were logged — the design that inverts the cost argument.
    pub naive_hops_per_cycle: f64,
    pub naive_bytes: f64,
    /// Logging only non-reconstructible events.
    pub events_per_cycle: f64,
    pub log_bytes: f64,
    /// Ratio against the 4DWCM's own frame-dump estimate.
    pub fraction_of_frame_dump: f64,
}

/// **Finding 3** — event-log volume per cell cycle, and the constraint that makes it work.
///
/// Two numbers, and the gap between them is the entire architecture:
///
/// - Log every particle motion: ~7e12 hops per cycle, a few hundred terabytes. Worse
///   than the 80 TB dense dump it was supposed to replace, not better.
/// - Log only what cannot be recomputed from the RNG address — births, deaths,
///   reactions, binds: a rounding error against the same 80 TB.
///
/// The second number is only reachable because the PRNG is counter-based. That is not a
/// performance detail, it is the load-bearing assumption, and `core::rng` states the two
/// rules that keep it true.
pub fn log_budget(t: &ParamTable, bytes_per_event: usize) -> LogBudget {
    let cycle = t.value("cell_cycle_s");
    let steps = cycle / t.value("rdme_timestep_s");

    let mobile = t.value("protein_molecules_total")
        + t.value("ribosomes_at_division")
        + t.value("trna_total")
        + t.value("mrna_standing_population");
    let naive_hops = steps * mobile * t.value("hop_probability_per_particle_per_step");

    // Non-reconstructible events over a cycle:
    //   - one birth + one death per protein produced (the population doubles)
    //   - mRNA birth and death on every turnover
    //   - translation initiation/termination and transcription events
    let proteins = t.value("protein_molecules_total");
    let mrna_births = t.value("mrna_standing_population") * cycle / t.value("mrna_half_life_s");
    let protein_events = proteins * 2.0; // one synthesis, one binding/assembly event each
    let mrna_events = mrna_births * 4.0; // birth, death, ribosome bind, ribosome release
    let events = protein_events + mrna_events;

    let log_bytes = events * bytes_per_event as f64;

    LogBudget {
        steps_per_cycle: steps,
        bytes_per_event,
        naive_hops_per_cycle: naive_hops,
        naive_bytes: naive_hops * bytes_per_event as f64,
        events_per_cycle: events,
        log_bytes,
        fraction_of_frame_dump: log_bytes / t.value("frame_dump_estimate_bytes"),
    }
}

/// Human-readable byte count.
pub fn human_bytes(b: f64) -> String {
    const UNITS: [&str; 7] = ["B", "kB", "MB", "GB", "TB", "PB", "EB"];
    let mut v = b;
    let mut i = 0;
    while v >= 1000.0 && i < UNITS.len() - 1 {
        v /= 1000.0;
        i += 1;
    }
    format!("{v:.2} {}", UNITS[i])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_budget_input_carries_provenance() {
        let t = syn3a_budget_inputs();
        assert!(t.len() >= 10);
        for p in t.iter() {
            p.validate().unwrap_or_else(|e| panic!("{e}"));
        }
    }

    /// None of the budget inputs may be tuned — they are the thing being predicted.
    #[test]
    fn no_budget_input_was_fitted_to_produce_a_favourable_answer() {
        assert!(syn3a_budget_inputs().fitted_parameters().is_empty());
    }

    #[test]
    fn resident_memory_for_individuation_is_small() {
        let t = syn3a_budget_inputs();
        let b = memory_budget(&t, 64);
        assert!(
            b.live_entities < 2.0e5,
            "Syn3A T2 population should be ~1e5, got {:.3e}",
            b.live_entities
        );
        assert!(
            b.resident_bytes < 100e6,
            "individuation should cost well under 100 MB, got {}",
            human_bytes(b.resident_bytes)
        );
    }

    /// The whole point of doing the arithmetic: the naive design is worse than the
    /// thing it replaces, and the actual design is a rounding error against it.
    #[test]
    fn logging_hops_would_be_catastrophic_and_logging_events_is_free() {
        let t = syn3a_budget_inputs();
        let b = log_budget(&t, 32);

        // The naive design must be shown to be *worse* than the thing it replaces.
        // It is ~2.7x worse, not the 50x a careless estimate suggests — the conclusion
        // survives, the inflated version of it does not, and only one of those is
        // worth writing down.
        assert!(
            b.naive_bytes > t.value("frame_dump_estimate_bytes"),
            "logging every hop must be worse than the 80 TB dump it replaces: {}",
            human_bytes(b.naive_bytes)
        );
        assert!(
            b.fraction_of_frame_dump < 1e-3,
            "the event log must be under 0.1% of the frame dump, got {:.3e}",
            b.fraction_of_frame_dump
        );
    }

    #[test]
    fn human_bytes_reads_correctly() {
        assert_eq!(human_bytes(1.0), "1.00 B");
        assert_eq!(human_bytes(80.0e12), "80.00 TB");
        assert_eq!(human_bytes(1.5e6), "1.50 MB");
    }
}
