//! Thermodynamic consistency (invariant 3), with the unit bug fixed.
//!
//! # The correction
//!
//! The scaffold stated invariant 3 as `k_f/k_r = exp(-ΔG°′/RT)`. That is only true for
//! unimolecular reactions. In general:
//!
//! ```text
//!     K_thermo = exp(-ΔG°'/RT)                 (dimensionless)
//!     K_c      = k_f / k_r = K_thermo · (c°)^Δn   (units of concentration^Δn)
//! ```
//!
//! where `Δn = Σν_products − Σν_reactants`. In molar units `c° = 1 M`, so the factor is
//! numerically invisible — which is exactly why this bug survives review. It stops being
//! invisible the moment anyone works in molecules-per-cell, where
//! `c° = N_A · V ≈ 2.0e7` molecules for a 200 nm-radius Syn3A cell. Every bimolecular
//! reverse rate is then wrong by seven orders of magnitude, silently, and detailed
//! balance still *looks* satisfied within the buggy unit system.
//!
//! [`ConcentrationUnit`] makes `c°` explicit so the factor cannot be dropped.
//!
//! # Michaelis–Menten needs the Haldane relation, not this identity
//!
//! Phase P1 is a metabolic ODE model, which is written in MM form. MM rate laws have no
//! `k_f`/`k_r` pair, so invariant 3 as originally stated could not be satisfied by P1 —
//! the phase plan contradicted its own invariant. The correct constraint for reversible
//! MM is the Haldane relation, [`haldane_keq`].

use protocell_core::consts::{N_A, R};

/// The concentration unit a rate constant is expressed in, and therefore the value of
/// the standard-state concentration `c°` in those units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConcentrationUnit {
    /// mol L^-1. `c° = 1`.
    Molar,
    /// Molecule counts in a compartment of the given volume in litres.
    /// `c° = N_A · V` molecules.
    MoleculesIn { volume_litres: f64 },
}

impl ConcentrationUnit {
    /// Standard-state concentration expressed in this unit.
    pub fn c_standard(&self) -> f64 {
        match *self {
            ConcentrationUnit::Molar => 1.0,
            ConcentrationUnit::MoleculesIn { volume_litres } => N_A * volume_litres,
        }
    }

    /// Volume of a sphere of the given radius, in litres. Convenience for Syn3A
    /// geometry (r = 200 nm at birth, 250 nm before division).
    pub fn sphere_nm(radius_nm: f64) -> Self {
        let r_cm = radius_nm * 1e-7;
        let v_cm3 = 4.0 / 3.0 * std::f64::consts::PI * r_cm * r_cm * r_cm;
        ConcentrationUnit::MoleculesIn {
            volume_litres: v_cm3 * 1e-3,
        }
    }
}

/// Dimensionless thermodynamic equilibrium constant from the standard free energy.
///
/// `dg_standard` in kJ mol^-1 (biochemical standard state, pH 7 — the prime), `t` in K.
pub fn k_eq_thermo(dg_standard_kj_per_mol: f64, temperature_k: f64) -> f64 {
    assert!(
        temperature_k > 0.0,
        "temperature must be positive, got {temperature_k}"
    );
    (-dg_standard_kj_per_mol * 1000.0 / (R * temperature_k)).exp()
}

/// Concentration-basis equilibrium constant `K_c = k_f/k_r`, carrying the `(c°)^Δn`
/// factor that the scalar form drops.
pub fn k_eq_concentration(
    dg_standard_kj_per_mol: f64,
    temperature_k: f64,
    delta_n: i32,
    unit: ConcentrationUnit,
) -> f64 {
    k_eq_thermo(dg_standard_kj_per_mol, temperature_k) * unit.c_standard().powi(delta_n)
}

/// Derive the reverse rate constant. **The only sanctioned way to obtain `k_r`.**
///
/// There is deliberately no function that accepts both `k_f` and `k_r`: setting them
/// independently breaks detailed balance silently, which `PROMPT.md` §7 names as the
/// worst class of bug available in this project. Make it unrepresentable rather than
/// forbidden.
pub fn reverse_rate(
    k_forward: f64,
    dg_standard_kj_per_mol: f64,
    temperature_k: f64,
    delta_n: i32,
    unit: ConcentrationUnit,
) -> f64 {
    assert!(
        k_forward > 0.0 && k_forward.is_finite(),
        "forward rate must be positive and finite, got {k_forward}"
    );
    let k_c = k_eq_concentration(dg_standard_kj_per_mol, temperature_k, delta_n, unit);
    assert!(
        k_c > 0.0 && k_c.is_finite(),
        "equilibrium constant is not usable ({k_c}); ΔG°′ = {dg_standard_kj_per_mol} kJ/mol \
         is outside the representable range at T = {temperature_k} K"
    );
    k_forward / k_c
}

/// Haldane relation for reversible Michaelis–Menten kinetics.
///
/// For `v = (Vf·S/Km_S − Vr·P/Km_P) / (1 + S/Km_S + P/Km_P)`, setting `v = 0` gives
/// `K_eq = P/S = (Vf · Km_P) / (Vr · Km_S)`.
///
/// This is the form invariant 3 takes for every enzyme in the P1 metabolic model.
pub fn haldane_keq(v_forward: f64, km_product: f64, v_reverse: f64, km_substrate: f64) -> f64 {
    assert!(
        v_reverse > 0.0 && km_substrate > 0.0,
        "reverse Vmax and substrate Km must be positive; an irreversible MM law has no \
         Haldane relation and must be declared irreversible explicitly"
    );
    (v_forward * km_product) / (v_reverse * km_substrate)
}

/// Check a reversible MM parameter set against its ΔG°′, within `rel_tol`.
pub fn check_haldane(
    v_forward: f64,
    km_product: f64,
    v_reverse: f64,
    km_substrate: f64,
    dg_standard_kj_per_mol: f64,
    temperature_k: f64,
    rel_tol: f64,
) -> Result<(), String> {
    let from_kinetics = haldane_keq(v_forward, km_product, v_reverse, km_substrate);
    let from_thermo = k_eq_thermo(dg_standard_kj_per_mol, temperature_k);
    let rel = (from_kinetics / from_thermo - 1.0).abs();
    if rel > rel_tol {
        return Err(format!(
            "Haldane violation: kinetics give K_eq = {from_kinetics:.6e}, \
             ΔG°′ = {dg_standard_kj_per_mol} kJ/mol gives {from_thermo:.6e} \
             (relative error {rel:.3e} > {rel_tol:.3e})"
        ));
    }
    Ok(())
}

/// Wegscheider condition: free energies must sum to zero around any closed cycle,
/// equivalently the equilibrium constants must multiply to one.
///
/// Violating this is a perpetual motion machine built out of three reactions that each
/// look individually reasonable, which is why it has to be checked on cycles rather
/// than on reactions.
pub fn check_wegscheider(cycle_dg_kj_per_mol: &[f64], abs_tol_kj: f64) -> Result<(), String> {
    let sum: f64 = cycle_dg_kj_per_mol.iter().sum();
    if sum.abs() > abs_tol_kj {
        return Err(format!(
            "Wegscheider violation: ΔG°′ around the cycle sums to {sum:.6e} kJ/mol \
             (tolerance {abs_tol_kj:.1e}); the cycle does work for free"
        ));
    }
    Ok(())
}

/// Entropy production rate of one reversible reaction, in units of `R`.
///
/// `σ = (v_f − v_r) · ln(v_f / v_r) ≥ 0`, the local form of the second law. Used by the
/// kill test: with no nutrient input this must decay monotonically to zero, and a
/// system that keeps producing entropy forever is generating free energy from nothing.
pub fn entropy_production(v_forward: f64, v_reverse: f64) -> f64 {
    if v_forward <= 0.0 && v_reverse <= 0.0 {
        return 0.0;
    }
    // At equilibrium the flux difference and the log both vanish; the product is 0.
    if v_forward <= 0.0 || v_reverse <= 0.0 {
        return f64::INFINITY; // a strictly irreversible step never reaches equilibrium
    }
    (v_forward - v_reverse) * (v_forward / v_reverse).ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: f64 = 310.15; // 37 °C

    #[test]
    fn zero_free_energy_gives_unit_equilibrium_constant() {
        assert!((k_eq_thermo(0.0, T) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn reverse_rate_recovers_the_forward_rate_at_equal_free_energy() {
        let k_f = 3.7;
        let k_r = reverse_rate(k_f, 0.0, T, 0, ConcentrationUnit::Molar);
        assert!((k_f - k_r).abs() < 1e-12);
    }

    /// The bug this module exists to prevent. In molar units the standard-state factor
    /// is 1 and invisible; in molecule counts it is ~1e7 for a Syn3A-sized cell.
    #[test]
    fn standard_state_factor_is_invisible_in_molar_and_enormous_in_molecule_counts() {
        let dg = -20.0;
        let molar = reverse_rate(1.0, dg, T, -1, ConcentrationUnit::Molar);

        let cell = ConcentrationUnit::sphere_nm(200.0);
        let counts = reverse_rate(1.0, dg, T, -1, cell);

        let c0 = cell.c_standard();
        assert!(
            (1.9e7..2.2e7).contains(&c0),
            "Syn3A standard state should be ~2e7 molecules, got {c0:.3e}"
        );
        // Association (Δn = -1) in count units: k_r differs from the molar answer by c°.
        let ratio = counts / molar;
        assert!(
            (ratio / c0 - 1.0).abs() < 1e-9,
            "expected the reverse rate to differ by exactly c° = {c0:.3e}, got ratio {ratio:.3e}"
        );
        assert!(
            ratio > 1e7,
            "seven orders of magnitude is the size of the bug being prevented"
        );
    }

    #[test]
    fn detailed_balance_holds_by_construction() {
        // k_f/k_r must equal K_c for any ΔG we pick, at any molecularity.
        for &dg in &[-30.0, -5.0, 0.0, 5.0, 30.0] {
            for &dn in &[-1i32, 0, 1] {
                let unit = ConcentrationUnit::sphere_nm(200.0);
                let k_f = 2.5;
                let k_r = reverse_rate(k_f, dg, T, dn, unit);
                let k_c = k_eq_concentration(dg, T, dn, unit);
                assert!(
                    ((k_f / k_r) / k_c - 1.0).abs() < 1e-9,
                    "detailed balance broken at ΔG={dg}, Δn={dn}"
                );
            }
        }
    }

    #[test]
    fn haldane_relation_ties_mm_parameters_to_free_energy() {
        // Pick Vf, Km_S, Km_P and a target ΔG; solve for the Vr that satisfies Haldane.
        let (vf, km_s, km_p, dg) = (10.0, 0.5, 2.0, -8.0);
        let k_eq = k_eq_thermo(dg, T);
        let vr = vf * km_p / (k_eq * km_s);
        check_haldane(vf, km_p, vr, km_s, dg, T, 1e-9).expect("constructed to satisfy Haldane");

        // A 20% error in Vr must be caught, not absorbed.
        let err = check_haldane(vf, km_p, vr * 1.2, km_s, dg, T, 1e-3).unwrap_err();
        assert!(err.contains("Haldane violation"), "{err}");
    }

    #[test]
    fn wegscheider_catches_a_free_energy_cycle() {
        check_wegscheider(&[-10.0, 25.0, -15.0], 1e-9).expect("balanced cycle");
        let e = check_wegscheider(&[-10.0, 25.0, -14.0], 1e-6).unwrap_err();
        assert!(e.contains("Wegscheider violation"), "{e}");
    }

    #[test]
    fn entropy_production_is_nonnegative_and_vanishes_at_equilibrium() {
        assert_eq!(entropy_production(5.0, 5.0), 0.0);
        assert!(entropy_production(10.0, 1.0) > 0.0);
        assert!(entropy_production(1.0, 10.0) > 0.0, "sign of the flux must not matter");
        assert!(
            entropy_production(1.0, 0.0).is_infinite(),
            "an irreversible step can never relax; the kill test must be able to see that"
        );
    }
}
