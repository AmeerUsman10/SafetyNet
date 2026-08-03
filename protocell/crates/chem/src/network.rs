//! Reaction networks, built so that the thermodynamically unsound thing is not
//! expressible.
//!
//! Two design choices carry most of the weight:
//!
//! 1. **There is no constructor that takes both `k_f` and `k_r`.** Reversible reactions
//!    are added as a pair from `k_f` and ΔG°′, and the reverse rate is derived. The
//!    scaffold listed "setting forward and reverse rates independently" as the worst
//!    class of bug available here; a rule you can follow is weaker than an API you
//!    cannot break.
//!
//! 2. **Irreversibility must be declared, with a justification.** A reaction with no
//!    reverse is a claim that ΔG°′ = −∞, and it means the system can never reach
//!    equilibrium. The kill test (invariant 4) cannot pass in the presence of one, so
//!    the network keeps a register of these declarations and the kill test reports them
//!    rather than silently failing. See `docs/RELAXATIONS.md`.

use crate::formula::Formula;
use crate::thermo::{self, ConcentrationUnit};
use protocell_core::SpeciesId;

/// How a species is represented. This is the tier assignment from `PROMPT.md` §2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// T1 — count matters, individuals do not. No `EntityId`, no per-entity storage.
    Counted,
    /// T2 — individuated. Every molecule gets an `EntityId`, a birth and a death.
    Individuated,
}

#[derive(Debug, Clone)]
pub struct Species {
    pub name: String,
    pub formula: Formula,
    pub tier: Tier,
}

#[derive(Debug, Clone)]
pub struct Reaction {
    pub name: String,
    /// `(species, stoichiometric coefficient)`, ascending species index.
    pub reactants: Vec<(SpeciesId, u32)>,
    pub products: Vec<(SpeciesId, u32)>,
    /// Stochastic rate constant, molecule-count basis, s^-1.
    pub k: f64,
    /// Index of the reverse reaction, if this reaction is one half of a reversible pair.
    pub reverse: Option<usize>,
}

impl Reaction {
    /// Δn = Σν_products − Σν_reactants. Sets the standard-state exponent.
    pub fn delta_n(&self) -> i32 {
        let p: i64 = self.products.iter().map(|(_, n)| *n as i64).sum();
        let r: i64 = self.reactants.iter().map(|(_, n)| *n as i64).sum();
        (p - r) as i32
    }

    /// Reaction order — the number of colliding molecules.
    pub fn order(&self) -> u32 {
        self.reactants.iter().map(|(_, n)| *n).sum()
    }
}

/// A reaction declared irreversible, and why.
#[derive(Debug, Clone)]
pub struct IrreversibleDeclaration {
    pub reaction: usize,
    pub justification: String,
}

#[derive(Debug, Clone)]
pub enum NetworkError {
    UnbalancedAtoms {
        reaction: String,
        difference: String,
    },
    UnbalancedCharge {
        reaction: String,
        difference: i64,
    },
    UnknownSpecies(u32),
    EmptyJustification(String),
    NonPositiveRate {
        reaction: String,
        k: f64,
    },
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkError::UnbalancedAtoms { reaction, difference } => write!(
                f,
                "invariant 1: reaction `{reaction}` does not balance atoms ({difference})"
            ),
            NetworkError::UnbalancedCharge { reaction, difference } => write!(
                f,
                "invariant 1: reaction `{reaction}` does not balance charge (net {difference:+})"
            ),
            NetworkError::UnknownSpecies(i) => write!(f, "reaction refers to unknown species {i}"),
            NetworkError::EmptyJustification(r) => write!(
                f,
                "reaction `{r}` is declared irreversible with no justification; \
                 irreversibility is a physical claim and must be argued for"
            ),
            NetworkError::NonPositiveRate { reaction, k } => {
                write!(f, "reaction `{reaction}` has non-positive rate constant {k}")
            }
        }
    }
}

impl std::error::Error for NetworkError {}

#[derive(Debug, Clone)]
pub struct Network {
    species: Vec<Species>,
    reactions: Vec<Reaction>,
    irreversible: Vec<IrreversibleDeclaration>,
    temperature_k: f64,
    unit: ConcentrationUnit,
}

impl Network {
    pub fn new(temperature_k: f64, unit: ConcentrationUnit) -> Self {
        Network {
            species: Vec::new(),
            reactions: Vec::new(),
            irreversible: Vec::new(),
            temperature_k,
            unit,
        }
    }

    pub fn add_species(&mut self, name: impl Into<String>, formula: Formula, tier: Tier) -> SpeciesId {
        let id = SpeciesId(self.species.len() as u32);
        self.species.push(Species {
            name: name.into(),
            formula,
            tier,
        });
        id
    }

    /// Add a reversible reaction from `k_f` and ΔG°′. Returns `(forward, reverse)`.
    ///
    /// The reverse rate is *derived*, including the `(c°)^Δn` standard-state factor.
    /// There is no way to override it, which is the point.
    pub fn add_reversible(
        &mut self,
        name: impl Into<String>,
        reactants: Vec<(SpeciesId, u32)>,
        products: Vec<(SpeciesId, u32)>,
        k_forward: f64,
        dg_standard_kj_per_mol: f64,
    ) -> (usize, usize) {
        let name = name.into();
        let fwd_idx = self.reactions.len();
        let rev_idx = fwd_idx + 1;

        let delta_n = {
            let p: i64 = products.iter().map(|(_, n)| *n as i64).sum();
            let r: i64 = reactants.iter().map(|(_, n)| *n as i64).sum();
            (p - r) as i32
        };
        let k_reverse = thermo::reverse_rate(
            k_forward,
            dg_standard_kj_per_mol,
            self.temperature_k,
            delta_n,
            self.unit,
        );

        self.reactions.push(Reaction {
            name: format!("{name}[f]"),
            reactants: reactants.clone(),
            products: products.clone(),
            k: k_forward,
            reverse: Some(rev_idx),
        });
        self.reactions.push(Reaction {
            name: format!("{name}[r]"),
            reactants: products,
            products: reactants,
            k: k_reverse,
            reverse: Some(fwd_idx),
        });
        (fwd_idx, rev_idx)
    }

    /// Add a reaction with no reverse. Requires a justification, which is recorded and
    /// surfaced by the kill test.
    pub fn add_irreversible(
        &mut self,
        name: impl Into<String>,
        reactants: Vec<(SpeciesId, u32)>,
        products: Vec<(SpeciesId, u32)>,
        k: f64,
        justification: impl Into<String>,
    ) -> Result<usize, NetworkError> {
        let name = name.into();
        let justification = justification.into();
        if justification.trim().is_empty() {
            return Err(NetworkError::EmptyJustification(name));
        }
        let idx = self.reactions.len();
        self.reactions.push(Reaction {
            name,
            reactants,
            products,
            k,
            reverse: None,
        });
        self.irreversible.push(IrreversibleDeclaration {
            reaction: idx,
            justification,
        });
        Ok(idx)
    }

    pub fn species(&self) -> &[Species] {
        &self.species
    }

    pub fn reactions(&self) -> &[Reaction] {
        &self.reactions
    }

    pub fn irreversible_declarations(&self) -> &[IrreversibleDeclaration] {
        &self.irreversible
    }

    pub fn temperature_k(&self) -> f64 {
        self.temperature_k
    }

    pub fn unit(&self) -> ConcentrationUnit {
        self.unit
    }

    pub fn tier_of(&self, s: SpeciesId) -> Tier {
        self.species[s.0 as usize].tier
    }

    /// True when every species is abstract, i.e. invariant 1 is vacuous here.
    ///
    /// Reported rather than hidden: a P0 solver test built on `A ⇌ B` proves the
    /// machinery works and proves nothing whatever about chemistry, and the Definition
    /// of Done should not be allowed to claim otherwise.
    pub fn is_abstract(&self) -> bool {
        !self.species.is_empty() && self.species.iter().all(|s| s.formula.is_abstract())
    }

    /// Invariant 1, checked at load. Hard failure, not a warning.
    pub fn validate(&self) -> Result<(), NetworkError> {
        for r in &self.reactions {
            if !r.k.is_finite() || r.k <= 0.0 {
                return Err(NetworkError::NonPositiveRate {
                    reaction: r.name.clone(),
                    k: r.k,
                });
            }
            let mut lhs = Formula::default();
            for (s, n) in &r.reactants {
                let sp = self
                    .species
                    .get(s.0 as usize)
                    .ok_or(NetworkError::UnknownSpecies(s.0))?;
                lhs.add_assign(&sp.formula.scaled(*n as i64));
            }
            let mut rhs = Formula::default();
            for (s, n) in &r.products {
                let sp = self
                    .species
                    .get(s.0 as usize)
                    .ok_or(NetworkError::UnknownSpecies(s.0))?;
                rhs.add_assign(&sp.formula.scaled(*n as i64));
            }
            let (datoms, dcharge) = lhs.difference(&rhs);
            if !datoms.is_empty() {
                let diff = datoms
                    .iter()
                    .map(|(k, v)| format!("{k}{v:+}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                return Err(NetworkError::UnbalancedAtoms {
                    reaction: r.name.clone(),
                    difference: diff,
                });
            }
            if dcharge != 0 {
                return Err(NetworkError::UnbalancedCharge {
                    reaction: r.name.clone(),
                    difference: dcharge,
                });
            }
        }
        Ok(())
    }

    /// Total atoms present, given a count vector. The ledger behind invariant 2.
    pub fn total_atoms(&self, counts: &[u64]) -> i64 {
        self.species
            .iter()
            .zip(counts)
            .map(|(s, n)| s.formula.total_atoms() * (*n as i64))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: f64 = 310.15;

    fn cell_unit() -> ConcentrationUnit {
        ConcentrationUnit::sphere_nm(200.0)
    }

    #[test]
    fn unbalanced_reactions_fail_to_load() {
        let mut n = Network::new(T, cell_unit());
        let a = n.add_species("A", Formula::parse("CH4", 0).unwrap(), Tier::Counted);
        let b = n.add_species("B", Formula::parse("CH3", 0).unwrap(), Tier::Counted);
        n.add_reversible("bad", vec![(a, 1)], vec![(b, 1)], 1.0, 0.0);
        let e = n.validate().unwrap_err();
        assert!(matches!(e, NetworkError::UnbalancedAtoms { .. }), "{e}");
        assert!(e.to_string().contains("H+1"), "{e}");
    }

    #[test]
    fn charge_imbalance_fails_even_when_atoms_balance() {
        let mut n = Network::new(T, cell_unit());
        let a = n.add_species("Na", Formula::parse("Na", 0).unwrap(), Tier::Counted);
        let b = n.add_species("Na+", Formula::parse("Na", 1).unwrap(), Tier::Counted);
        n.add_reversible("ionise", vec![(a, 1)], vec![(b, 1)], 1.0, 0.0);
        assert!(matches!(
            n.validate().unwrap_err(),
            NetworkError::UnbalancedCharge { .. }
        ));
    }

    #[test]
    fn a_balanced_isomerisation_loads() {
        let mut n = Network::new(T, cell_unit());
        let g6p = n.add_species("G6P", Formula::parse("C6H11O9P", -2).unwrap(), Tier::Counted);
        let f6p = n.add_species("F6P", Formula::parse("C6H11O9P", -2).unwrap(), Tier::Counted);
        n.add_reversible("pgi", vec![(g6p, 1)], vec![(f6p, 1)], 100.0, 2.0);
        n.validate().expect("balanced network must load");
        assert!(!n.is_abstract());
    }

    #[test]
    fn reverse_rate_is_derived_and_carries_the_standard_state_factor() {
        let mut n = Network::new(T, cell_unit());
        let a = n.add_species("A", Formula::parse("H2", 0).unwrap(), Tier::Counted);
        let b = n.add_species("B", Formula::parse("H4", 0).unwrap(), Tier::Counted);
        // 2A <=> B, association: delta_n = -1.
        let (f, r) = n.add_reversible("assoc", vec![(a, 2)], vec![(b, 1)], 1e-3, -20.0);
        n.validate().unwrap();
        assert_eq!(n.reactions()[f].delta_n(), -1);
        assert_eq!(n.reactions()[r].delta_n(), 1);

        let expected = thermo::reverse_rate(1e-3, -20.0, T, -1, cell_unit());
        assert_eq!(n.reactions()[r].k, expected);
        assert_eq!(n.reactions()[f].reverse, Some(r));
        assert_eq!(n.reactions()[r].reverse, Some(f));
    }

    #[test]
    fn irreversibility_requires_an_argument() {
        let mut n = Network::new(T, cell_unit());
        let a = n.add_species("A", Formula::abstract_species(), Tier::Counted);
        let e = n
            .add_irreversible("decay", vec![(a, 1)], vec![], 1.0, "  ")
            .unwrap_err();
        assert!(matches!(e, NetworkError::EmptyJustification(_)), "{e}");

        n.add_irreversible(
            "decay",
            vec![(a, 1)],
            vec![],
            1.0,
            "hydrolysis under saturating water; reverse flux negligible at cell pH",
        )
        .unwrap();
        assert_eq!(n.irreversible_declarations().len(), 1);
    }

    #[test]
    fn abstract_networks_admit_they_are_abstract() {
        let mut n = Network::new(T, cell_unit());
        let a = n.add_species("A", Formula::abstract_species(), Tier::Counted);
        let b = n.add_species("B", Formula::abstract_species(), Tier::Counted);
        n.add_reversible("iso", vec![(a, 1)], vec![(b, 1)], 1.0, 0.0);
        n.validate().unwrap();
        assert!(
            n.is_abstract(),
            "a network of abstract species must report that its balance check is vacuous"
        );
    }
}
