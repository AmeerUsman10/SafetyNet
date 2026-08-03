//! P1 — the first real Syn3A-relevant chemistry: the ATP-consuming half of glycolysis.
//!
//! # What this is, and what it deliberately is not
//!
//! Five reactions, canonical Embden–Meyerhof–Parnas pathway (KEGG map00010), from
//! glucose to the two triose phosphates: hexokinase, phosphoglucose isomerase,
//! phosphofructokinase, aldolase, triose phosphate isomerase.
//!
//! Every formula and charge state below is the standard BiGG/COBRA metabolic-model
//! convention (fully deprotonated at physiological pH — the same convention used by
//! `e_coli_core` and `iJO1366`), and **every reaction is balanced by the actual checker
//! in `network::Network::validate`, not asserted from memory.** The stoichiometric
//! proton (`H+`) in the two phosphoryl-transfer steps was derived by running the atom
//! count, not looked up — it falls out of the arithmetic once glucose, ATP and ADP are
//! fixed to consistent charge states, and getting it wrong is exactly the kind of error
//! invariant 1 exists to catch before it reaches a simulation.
//!
//! **The ΔG°′ values are `Method::Estimated`, not `Method::Measured`.** They are
//! standard textbook figures (the type of values found in comparative compilations
//! such as Bennett et al. 2009, *Nat Chem Biol*, or Alberty's thermodynamic tables),
//! good to the sign and rough magnitude, but I do not have a verified DOI in hand for
//! the exact decimal figure used here — see `docs/STATE.md` BLOCK-01. Marking them
//! `Measured` would be exactly the "fabricated citation" failure mode invariant 9
//! exists to prevent. They are legal to use under `Method::Estimated` precisely because
//! the uncertainty is disclosed rather than hidden, and they are upgraded to `Measured`
//! the moment a verified source lands.
//!
//! Whether Syn3A runs this specific pathway, at what flux, and under what regulation is
//! a P1 question this module does not answer — Syn3A's genome is famously stripped down
//! and its central carbon metabolism differs from *E. coli*'s. This is the pathway
//! skeleton, atom-balanced and thermodynamically self-consistent, that the real Syn3A
//! parameterization will be built on top of.
//!
//! # Independent confirmation, and why the ΔG values are still `Estimated`
//!
//! BLOCK-01 (`docs/STATE.md`) turned up the real thing: `Luthey-Schulten-Lab/Minimal_Cell`
//! on GitHub, the actual code release for Thornburg et al. 2022, containing
//! `iMB155_NoH2O.xml` — an SBML metabolic model sourced from Breuer et al. 2019
//! (*eLife* 8:e36842, doi:10.7554/eLife.36842), the real Syn3A metabolic reconstruction.
//!
//! Its species declarations are a **bit-for-bit match** to the formulas used here:
//! `M_g6p_c` / `M_f6p_c` are `C6H11O9P`, charge −2; `M_fdp_c` is `C6H10O12P2`, charge −4;
//! `M_atp_c` is `C10H12N5O13P3`, charge −4; `M_h_c` is `H`, charge +1 — independently
//! confirming the atom-balance work above rather than just this checker agreeing with
//! itself. Its reversible rate laws are also a convenience-kinetics / Haldane-relation
//! form (`kcrg`, `keq`, `kmc` per reaction) — the same structure as `thermo::haldane_keq`,
//! for the same reason: it is the physically correct way to parameterize reversible MM
//! kinetics.
//!
//! **The ΔG values here remain `Estimated` rather than being upgraded to the real
//! model's numbers.** That repository carries no LICENSE file, so its compiled
//! parameter tables are not established as reusable, and copying its fitted values
//! into this codebase would trade an honestly-disclosed textbook estimate for an
//! unlicensed copy — worse, not better. The correct upgrade path is independent: go to
//! the primary source (Breuer et al. 2019, or eQuilibrator directly) rather than the
//! downstream compilation. That is future P1 work, not done here.

pub mod source_note {
    //! Where a real Syn3A parameter set was located, for whoever does that P1 work.
    pub const REPO: &str = "https://github.com/Luthey-Schulten-Lab/Minimal_Cell";
    pub const PRIMARY_SOURCE: &str =
        "Breuer et al. 2019, eLife 8:e36842, doi:10.7554/eLife.36842 (Syn3A metabolic reconstruction)";
    pub const LICENSE_STATUS: &str =
        "No LICENSE file found at repo root as of the 2026-08-03 check. Treat as \
         all-rights-reserved for the compiled tables; re-derive from the primary \
         source or request permission before reuse.";
}

use crate::formula::Formula;
use crate::network::{Network, Tier};
use crate::provenance::{Method, Param, ParamTable, Uncertainty};
use protocell_core::SpeciesId;

/// Physiological-pH (BiGG/COBRA-convention) formulas for the five glycolytic
/// intermediates plus the cofactors they turn over. Fully deprotonated forms, matching
/// the convention already used for ATP/ADP/Pi elsewhere in this crate.
pub struct GlycolysisSpecies {
    pub glucose: SpeciesId,
    pub atp: SpeciesId,
    pub adp: SpeciesId,
    pub g6p: SpeciesId,
    pub f6p: SpeciesId,
    pub fbp: SpeciesId,
    pub dhap: SpeciesId,
    pub g3p: SpeciesId,
    pub h_plus: SpeciesId,
}

/// Reaction indices, forward half of each reversible pair, in pathway order.
pub struct GlycolysisReactions {
    pub hexokinase: usize,
    pub pgi: usize,
    pub pfk: usize,
    pub aldolase: usize,
    pub tpi: usize,
}

/// Provenance for the five ΔG°′ values used to derive reverse rates below.
///
/// All `Estimated`. See the module doc for why, and `docs/STATE.md` BLOCK-01 for the
/// path to `Measured`.
pub fn glycolysis_thermo_params() -> ParamTable {
    let mut t = ParamTable::new();
    let basis = "standard textbook compilation of glycolytic reaction free energies \
                 (Bennett et al. 2009-type comparative measurement, or Lehninger/Voet \
                 tabulated values); sign and order of magnitude are well established, \
                 exact decimal pending a verified primary source (docs/STATE.md BLOCK-01)";
    let mut add = |name: &str, dg: f64| {
        t.insert(
            Param::new(
                name,
                dg,
                "kJ/mol",
                Method::Estimated { basis: basis.into() },
                Uncertainty::Relative(0.3),
            )
            .unwrap(),
        )
        .unwrap();
    };
    add("dG_hexokinase", -16.7);
    add("dG_pgi", 1.7);
    add("dG_pfk", -14.2);
    add("dG_aldolase", 23.8);
    add("dG_tpi", 7.5);
    t
}

/// Build the five-reaction network. `unit` should match whatever concentration basis
/// the caller intends to run the SSA in (see `thermo::ConcentrationUnit`) — the reverse
/// rates carry the `(c°)^Δn` factor derived from it, per invariant 3.
pub fn build(
    net: &mut Network,
    thermo: &ParamTable,
    k_hexokinase: f64,
    k_pgi: f64,
    k_pfk: f64,
    k_aldolase: f64,
    k_tpi: f64,
) -> (GlycolysisSpecies, GlycolysisReactions) {
    let f = |s: &str, c: i64| Formula::parse(s, c).unwrap();

    let sp = GlycolysisSpecies {
        glucose: net.add_species("glucose", f("C6H12O6", 0), Tier::Counted),
        atp: net.add_species("ATP", f("C10H12N5O13P3", -4), Tier::Counted),
        adp: net.add_species("ADP", f("C10H12N5O10P2", -3), Tier::Counted),
        g6p: net.add_species("G6P", f("C6H11O9P", -2), Tier::Counted),
        f6p: net.add_species("F6P", f("C6H11O9P", -2), Tier::Counted),
        fbp: net.add_species("FBP", f("C6H10O12P2", -4), Tier::Counted),
        dhap: net.add_species("DHAP", f("C3H5O6P", -2), Tier::Counted),
        g3p: net.add_species("G3P", f("C3H5O6P", -2), Tier::Counted),
        h_plus: net.add_species("H+", f("H", 1), Tier::Counted),
    };

    // Hexokinase: glucose + ATP -> G6P + ADP + H+.
    // The H+ is not a textbook flourish, it is what atom-balancing this reaction under
    // BiGG-convention charge states (ATP4-, ADP3-, G6P2-) actually requires: without it
    // the reaction is short one H and one unit of negative charge on the product side.
    let (hexokinase, _) = net.add_reversible(
        "hexokinase",
        vec![(sp.glucose, 1), (sp.atp, 1)],
        vec![(sp.g6p, 1), (sp.adp, 1), (sp.h_plus, 1)],
        k_hexokinase,
        thermo.value("dG_hexokinase"),
    );

    // PGI: G6P <-> F6P. Isomerization — same formula both sides, no cofactors.
    let (pgi, _) = net.add_reversible(
        "pgi",
        vec![(sp.g6p, 1)],
        vec![(sp.f6p, 1)],
        k_pgi,
        thermo.value("dG_pgi"),
    );

    // PFK: F6P + ATP -> FBP + ADP + H+. Same phosphoryl-transfer pattern as hexokinase.
    let (pfk, _) = net.add_reversible(
        "pfk",
        vec![(sp.f6p, 1), (sp.atp, 1)],
        vec![(sp.fbp, 1), (sp.adp, 1), (sp.h_plus, 1)],
        k_pfk,
        thermo.value("dG_pfk"),
    );

    // Aldolase: FBP <-> DHAP + G3P. Aldol cleavage — balances with no extra proton.
    let (aldolase, _) = net.add_reversible(
        "aldolase",
        vec![(sp.fbp, 1)],
        vec![(sp.dhap, 1), (sp.g3p, 1)],
        k_aldolase,
        thermo.value("dG_aldolase"),
    );

    // TPI: DHAP <-> G3P. Isomerization, same formula.
    let (tpi, _) = net.add_reversible(
        "tpi",
        vec![(sp.dhap, 1)],
        vec![(sp.g3p, 1)],
        k_tpi,
        thermo.value("dG_tpi"),
    );

    (
        sp,
        GlycolysisReactions {
            hexokinase,
            pgi,
            pfk,
            aldolase,
            tpi,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thermo::ConcentrationUnit;

    const T: f64 = 310.15;

    fn cell_unit() -> ConcentrationUnit {
        ConcentrationUnit::sphere_nm(200.0)
    }

    /// The point of this module: every reaction balances atoms and charge under
    /// invariant 1, checked by the same validator every other network in this project
    /// goes through — not asserted from biochemistry recall.
    #[test]
    fn the_atp_consuming_half_of_glycolysis_balances() {
        let thermo = glycolysis_thermo_params();
        let mut net = Network::new(T, cell_unit());
        let (_sp, _rx) = build(&mut net, &thermo, 100.0, 50.0, 80.0, 10.0, 60.0);
        net.validate().expect(
            "every glycolytic reaction must balance atoms and charge; if this fails, \
             the formula or charge state of a species above is wrong, not the checker",
        );
        assert!(!net.is_abstract(), "this must exercise real chemistry, not placeholders");
    }

    /// Confirms the stoichiometric proton was derived correctly for both
    /// phosphoryl-transfer steps, and that the two isomerizations and the aldol
    /// cleavage need no proton at all — the asymmetry is a property of the chemistry,
    /// not an inconsistency.
    #[test]
    fn phosphoryl_transfers_release_exactly_one_proton() {
        let thermo = glycolysis_thermo_params();
        let mut net = Network::new(T, cell_unit());
        let (sp, rx) = build(&mut net, &thermo, 1.0, 1.0, 1.0, 1.0, 1.0);

        let released_h = |idx: usize| {
            net.reactions()[idx]
                .products
                .iter()
                .any(|&(s, n)| s == sp.h_plus && n == 1)
        };
        assert!(released_h(rx.hexokinase), "hexokinase must release H+ to balance");
        assert!(released_h(rx.pfk), "PFK must release H+ to balance");

        let no_h_either_side = |idx: usize| {
            let rxn = &net.reactions()[idx];
            !rxn.reactants.iter().any(|&(s, _)| s == sp.h_plus)
                && !rxn.products.iter().any(|&(s, _)| s == sp.h_plus)
        };
        assert!(no_h_either_side(rx.pgi), "isomerization must not involve H+");
        assert!(no_h_either_side(rx.aldolase), "aldol cleavage must not involve H+");
        assert!(no_h_either_side(rx.tpi), "isomerization must not involve H+");
    }

    /// No ΔG°′ input here is `Fitted`. They are honestly `Estimated`, with disclosed
    /// uncertainty, pending the verified source BLOCK-01 is chasing.
    #[test]
    fn no_thermo_parameter_was_fitted() {
        let t = glycolysis_thermo_params();
        assert_eq!(t.len(), 5);
        assert!(t.fitted_parameters().is_empty());
        for p in t.iter() {
            assert_eq!(p.method.label(), "estimated");
        }
    }

    /// Mass conservation end to end: run the pathway forward and back and confirm total
    /// atoms are unchanged, the same invariant-2 check every other network gets.
    #[test]
    fn mass_is_conserved_running_the_pathway() {
        let thermo = glycolysis_thermo_params();
        let mut net = Network::new(T, cell_unit());
        let (sp, _rx) = build(&mut net, &thermo, 50.0, 20.0, 40.0, 5.0, 30.0);

        let n = net.species().len();
        let mut counts = vec![0u64; n];
        counts[sp.glucose.0 as usize] = 500;
        counts[sp.atp.0 as usize] = 2000;
        counts[sp.adp.0 as usize] = 500;

        let mut sim = crate::ssa::Simulation::new(net, counts, 0xC0FFEE);
        for _ in 0..20_000 {
            sim.advance_one();
            sim.assert_mass_conservation();
        }
    }
}
