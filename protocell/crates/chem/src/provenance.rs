//! Parameter provenance — every number carries where it came from, or the model
//! refuses to load.
//!
//! # Why "cite everything" as literally written cannot work
//!
//! The scaffold's rule was: no parameter without `{value, units, source_DOI, method,
//! uncertainty}`, missing citation ⇒ refuse to load. Applied literally that makes the
//! project's own first milestone impossible. Reproducing the 4DWCM (L2) requires
//! adopting parameters the 4DWCM itself adjusted — ribosome binding +30%, degradosome
//! binding −70% — and those adjusted values have no primary DOI. Their provenance is
//! "this paper, tuned to reach protein doubling", which is a real and *reportable*
//! provenance, just not a citation.
//!
//! So the rule here is stronger, not weaker: **missing provenance record ⇒ refuse to
//! load**, where `Estimated`, `Fitted` and `Inherited` are legal methods that must each
//! carry a non-empty justification. A fitted parameter you can *see* is a scientific
//! statement. A fitted parameter disguised as a measured one is the failure mode
//! `PROMPT.md` §7 warns about, and this type makes it unrepresentable.
//!
//! [`ParamTable::fitted_parameters`] exists so the report required by
//! `AGENTS.md` — "anything tuned to achieve agreement is named in STATE.md" — can be
//! generated mechanically instead of remembered.

use std::collections::BTreeMap;

/// How a parameter's value was arrived at. Every variant carries its evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Method {
    /// Measured and published. Requires a DOI or accession.
    Measured { doi: String, technique: String },
    /// Derived from other parameters by an identity (e.g. `k_r` from `k_f` and ΔG°′).
    /// Never independently assigned — that is the thermodynamics violation.
    Derived { from: String, relation: String },
    /// No measurement exists; value reasoned from related systems. Requires the basis.
    Estimated { basis: String },
    /// Adjusted to reach agreement with an observable. Requires naming the target.
    /// These are the parameters that must appear in `STATE.md`.
    Fitted { target: String, note: String },
    /// Adopted wholesale from a prior model, including any adjustment it made.
    Inherited { source: String, note: String },
}

impl Method {
    /// Whether this value was chosen to make something else come out right.
    pub fn is_tuned(&self) -> bool {
        matches!(self, Method::Fitted { .. })
    }

    pub fn label(&self) -> &'static str {
        match self {
            Method::Measured { .. } => "measured",
            Method::Derived { .. } => "derived",
            Method::Estimated { .. } => "estimated",
            Method::Fitted { .. } => "fitted",
            Method::Inherited { .. } => "inherited",
        }
    }

    /// The evidence string. Empty is a load failure — that is the whole point.
    fn evidence(&self) -> &str {
        match self {
            Method::Measured { doi, .. } => doi,
            Method::Derived { relation, .. } => relation,
            Method::Estimated { basis } => basis,
            Method::Fitted { target, .. } => target,
            Method::Inherited { source, .. } => source,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Uncertainty {
    /// Symmetric relative uncertainty, e.g. 0.2 for ±20%.
    Relative(f64),
    /// Known only to within a factor, e.g. 10.0 for "within an order of magnitude".
    OrderOfMagnitude(f64),
    /// Explicitly unquantified. Legal, but it is a statement, and it is reported.
    Unquantified,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub value: f64,
    pub units: String,
    pub method: Method,
    pub uncertainty: Uncertainty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenanceError {
    MissingName,
    MissingUnits(String),
    MissingEvidence { name: String, method: &'static str },
    NonFiniteValue(String),
    Duplicate(String),
}

impl std::fmt::Display for ProvenanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProvenanceError::MissingName => write!(f, "parameter has no name"),
            ProvenanceError::MissingUnits(n) => {
                write!(f, "parameter `{n}` has no units; use \"dimensionless\" if it truly has none")
            }
            ProvenanceError::MissingEvidence { name, method } => write!(
                f,
                "parameter `{name}` is {method} but carries no evidence string — \
                 an unjustified {method} value is exactly what the model refuses to load"
            ),
            ProvenanceError::NonFiniteValue(n) => write!(f, "parameter `{n}` is not finite"),
            ProvenanceError::Duplicate(n) => write!(f, "parameter `{n}` defined twice"),
        }
    }
}

impl std::error::Error for ProvenanceError {}

impl Param {
    pub fn new(
        name: impl Into<String>,
        value: f64,
        units: impl Into<String>,
        method: Method,
        uncertainty: Uncertainty,
    ) -> Result<Self, ProvenanceError> {
        let p = Param {
            name: name.into(),
            value,
            units: units.into(),
            method,
            uncertainty,
        };
        p.validate()?;
        Ok(p)
    }

    pub fn validate(&self) -> Result<(), ProvenanceError> {
        if self.name.trim().is_empty() {
            return Err(ProvenanceError::MissingName);
        }
        if self.units.trim().is_empty() {
            return Err(ProvenanceError::MissingUnits(self.name.clone()));
        }
        if !self.value.is_finite() {
            return Err(ProvenanceError::NonFiniteValue(self.name.clone()));
        }
        if self.method.evidence().trim().is_empty() {
            return Err(ProvenanceError::MissingEvidence {
                name: self.name.clone(),
                method: self.method.label(),
            });
        }
        Ok(())
    }
}

/// A validated parameter set. Construction is the only way in, and it validates.
#[derive(Debug, Clone, Default)]
pub struct ParamTable {
    params: BTreeMap<String, Param>,
}

impl ParamTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, p: Param) -> Result<(), ProvenanceError> {
        p.validate()?;
        if self.params.contains_key(&p.name) {
            return Err(ProvenanceError::Duplicate(p.name));
        }
        self.params.insert(p.name.clone(), p);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&Param> {
        self.params.get(name)
    }

    /// Value lookup that panics on a missing parameter rather than defaulting.
    ///
    /// A silent default is an invented rate constant with extra steps.
    pub fn value(&self, name: &str) -> f64 {
        self.params
            .get(name)
            .unwrap_or_else(|| panic!("parameter `{name}` is not in the table; it has no default"))
            .value
    }

    pub fn len(&self) -> usize {
        self.params.len()
    }

    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Param> {
        self.params.values()
    }

    /// Every parameter that was tuned to make an observable come out right.
    ///
    /// `AGENTS.md` requires these be named in `STATE.md`. Generating the list
    /// mechanically means it cannot quietly go stale.
    pub fn fitted_parameters(&self) -> Vec<&Param> {
        self.params.values().filter(|p| p.method.is_tuned()).collect()
    }

    /// Parameters with no quantified uncertainty. Not an error; a disclosure.
    pub fn unquantified_parameters(&self) -> Vec<&Param> {
        self.params
            .values()
            .filter(|p| matches!(p.uncertainty, Uncertainty::Unquantified))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measured(name: &str, v: f64) -> Param {
        Param::new(
            name,
            v,
            "s^-1",
            Method::Measured {
                doi: "10.1016/j.cell.2021.12.025".into(),
                technique: "single-molecule FRET".into(),
            },
            Uncertainty::Relative(0.2),
        )
        .unwrap()
    }

    #[test]
    fn a_parameter_without_evidence_refuses_to_load() {
        let e = Param::new(
            "k_on",
            1.0,
            "s^-1",
            Method::Estimated { basis: "   ".into() },
            Uncertainty::Unquantified,
        )
        .unwrap_err();
        assert_eq!(
            e,
            ProvenanceError::MissingEvidence {
                name: "k_on".into(),
                method: "estimated"
            }
        );
    }

    #[test]
    fn estimated_and_fitted_values_are_legal_when_justified() {
        // The 4DWCM's own adjustments have to be representable, or L2 is unreachable.
        let p = Param::new(
            "k_bind_ribosome_mrna",
            1.3,
            "nM^-1 s^-1",
            Method::Fitted {
                target: "protein doubling over one cell cycle".into(),
                note: "+30% vs. Thornburg 2022, per 4DWCM".into(),
            },
            Uncertainty::OrderOfMagnitude(3.0),
        )
        .unwrap();
        assert!(p.method.is_tuned());
    }

    #[test]
    fn tuned_parameters_are_reported_mechanically() {
        let mut t = ParamTable::new();
        t.insert(measured("k_f", 1.0)).unwrap();
        t.insert(
            Param::new(
                "k_degradosome",
                0.3,
                "nM^-1 s^-1",
                Method::Fitted {
                    target: "protein doubling".into(),
                    note: "-70% vs. Thornburg 2022".into(),
                },
                Uncertainty::OrderOfMagnitude(3.0),
            )
            .unwrap(),
        )
        .unwrap();

        let tuned = t.fitted_parameters();
        assert_eq!(tuned.len(), 1);
        assert_eq!(tuned[0].name, "k_degradosome");
    }

    #[test]
    fn missing_units_and_duplicates_are_rejected() {
        assert!(matches!(
            Param::new("x", 1.0, "", Method::Estimated { basis: "b".into() }, Uncertainty::Unquantified),
            Err(ProvenanceError::MissingUnits(_))
        ));
        let mut t = ParamTable::new();
        t.insert(measured("k", 1.0)).unwrap();
        assert!(matches!(
            t.insert(measured("k", 2.0)),
            Err(ProvenanceError::Duplicate(_))
        ));
    }

    #[test]
    #[should_panic(expected = "has no default")]
    fn a_missing_parameter_panics_rather_than_defaulting() {
        ParamTable::new().value("k_invented");
    }
}
