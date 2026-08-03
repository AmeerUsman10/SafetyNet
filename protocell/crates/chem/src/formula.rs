//! Molecular formulas and the atom/charge balance check (invariant 1).
//!
//! Balance is checked when the network is built, and failure is a hard error rather
//! than a warning. An unbalanced reaction is not a modelling approximation, it is a
//! statement that matter appears from nowhere, and invariant 2 (mass conservation)
//! cannot hold downstream of one.

use std::collections::BTreeMap;

/// Element counts plus formal charge.
///
/// `BTreeMap` rather than `HashMap`: formula comparison and error messages are part of
/// the model-load output, and unordered output makes diffs unreadable.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Formula {
    atoms: BTreeMap<String, i64>,
    charge: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormulaError {
    UnexpectedChar { pos: usize, ch: char },
    EmptyFormula,
    CountOverflow,
}

impl std::fmt::Display for FormulaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormulaError::UnexpectedChar { pos, ch } => {
                write!(f, "unexpected character {ch:?} at position {pos}")
            }
            FormulaError::EmptyFormula => write!(f, "empty formula"),
            FormulaError::CountOverflow => write!(f, "atom count overflow"),
        }
    }
}

impl std::error::Error for FormulaError {}

impl Formula {
    /// Parse a Hill-ish formula such as `C6H12O6` or `HPO4`, with charge supplied
    /// separately.
    ///
    /// Charge is a separate argument rather than a suffix on purpose: `"CO3-2"` is
    /// ambiguous between "charge −2" and "two fewer somethings", and a silent
    /// misparse here would propagate into every ΔG in the model.
    pub fn parse(s: &str, charge: i64) -> Result<Self, FormulaError> {
        let bytes: Vec<char> = s.trim().chars().collect();
        if bytes.is_empty() {
            return Err(FormulaError::EmptyFormula);
        }
        let mut atoms: BTreeMap<String, i64> = BTreeMap::new();
        let mut i = 0usize;
        while i < bytes.len() {
            let c = bytes[i];
            if !c.is_ascii_uppercase() {
                return Err(FormulaError::UnexpectedChar { pos: i, ch: c });
            }
            let mut sym = String::new();
            sym.push(c);
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_lowercase() {
                sym.push(bytes[i]);
                i += 1;
            }
            let mut n: i64 = 0;
            let mut have_digits = false;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                have_digits = true;
                n = n
                    .checked_mul(10)
                    .and_then(|v| v.checked_add(bytes[i] as i64 - '0' as i64))
                    .ok_or(FormulaError::CountOverflow)?;
                i += 1;
            }
            let n = if have_digits { n } else { 1 };
            *atoms.entry(sym).or_insert(0) += n;
        }
        atoms.retain(|_, v| *v != 0);
        Ok(Formula { atoms, charge })
    }

    /// A formula with no atoms and no charge — for abstract species in solver tests.
    ///
    /// Abstract species make invariant 1 vacuously true, which is fine for exercising
    /// the *machinery* but proves nothing about chemistry. Networks built from these
    /// are marked abstract so `Network::validate` can say so out loud.
    pub fn abstract_species() -> Self {
        Formula::default()
    }

    pub fn is_abstract(&self) -> bool {
        self.atoms.is_empty() && self.charge == 0
    }

    pub fn charge(&self) -> i64 {
        self.charge
    }

    pub fn atoms(&self) -> &BTreeMap<String, i64> {
        &self.atoms
    }

    /// Scale by a stoichiometric coefficient.
    pub fn scaled(&self, n: i64) -> Formula {
        Formula {
            atoms: self.atoms.iter().map(|(k, v)| (k.clone(), v * n)).collect(),
            charge: self.charge * n,
        }
    }

    pub fn add_assign(&mut self, other: &Formula) {
        for (k, v) in &other.atoms {
            *self.atoms.entry(k.clone()).or_insert(0) += v;
        }
        self.charge += other.charge;
        self.atoms.retain(|_, v| *v != 0);
    }

    /// Per-element difference `self - other`, plus the charge difference. Empty when
    /// balanced.
    pub fn difference(&self, other: &Formula) -> (BTreeMap<String, i64>, i64) {
        let mut d = self.atoms.clone();
        for (k, v) in &other.atoms {
            *d.entry(k.clone()).or_insert(0) -= v;
        }
        d.retain(|_, v| *v != 0);
        (d, self.charge - other.charge)
    }

    /// Total atom count, for the mass-conservation ledger (invariant 2).
    pub fn total_atoms(&self) -> i64 {
        self.atoms.values().sum()
    }
}

impl std::fmt::Display for Formula {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_abstract() {
            return write!(f, "<abstract>");
        }
        for (sym, n) in &self.atoms {
            if *n == 1 {
                write!(f, "{sym}")?;
            } else {
                write!(f, "{sym}{n}")?;
            }
        }
        match self.charge.cmp(&0) {
            std::cmp::Ordering::Greater => write!(f, "^{}+", self.charge)?,
            std::cmp::Ordering::Less => write!(f, "^{}-", -self.charge)?,
            std::cmp::Ordering::Equal => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multi_letter_elements_and_counts() {
        let f = Formula::parse("C6H12O6", 0).unwrap();
        assert_eq!(f.atoms()["C"], 6);
        assert_eq!(f.atoms()["H"], 12);
        assert_eq!(f.atoms()["O"], 6);
        assert_eq!(f.total_atoms(), 24);

        let mg = Formula::parse("MgATP", 0);
        // "Mg" then "A","T","P" as elements — the parser is deliberately literal.
        assert!(mg.is_ok());
        assert_eq!(mg.unwrap().atoms()["Mg"], 1);
    }

    #[test]
    fn atp_hydrolysis_balances() {
        // ATP^4- + H2O -> ADP^3- + HPO4^2- + H+
        let atp = Formula::parse("C10H12N5O13P3", -4).unwrap();
        let water = Formula::parse("H2O", 0).unwrap();
        let adp = Formula::parse("C10H12N5O10P2", -3).unwrap();
        let pi = Formula::parse("HO4P", -2).unwrap();
        let h = Formula::parse("H", 1).unwrap();

        let mut lhs = Formula::default();
        lhs.add_assign(&atp);
        lhs.add_assign(&water);

        let mut rhs = Formula::default();
        rhs.add_assign(&adp);
        rhs.add_assign(&pi);
        rhs.add_assign(&h);

        let (datoms, dcharge) = lhs.difference(&rhs);
        assert!(datoms.is_empty(), "atoms unbalanced: {datoms:?}");
        assert_eq!(dcharge, 0, "charge unbalanced");
    }

    #[test]
    fn detects_the_off_by_one_hydrogen_that_a_warning_would_hide() {
        let lhs = Formula::parse("H2O", 0).unwrap();
        let rhs = Formula::parse("HO", 0).unwrap();
        let (d, _) = lhs.difference(&rhs);
        assert_eq!(d.get("H"), Some(&1));
    }

    #[test]
    fn rejects_malformed_input_rather_than_guessing() {
        assert!(Formula::parse("6C", 0).is_err());
        assert!(Formula::parse("", 0).is_err());
        assert!(Formula::parse("c6", 0).is_err());
    }

    #[test]
    fn charge_only_imbalance_is_caught() {
        let a = Formula::parse("Na", 1).unwrap();
        let b = Formula::parse("Na", 0).unwrap();
        let (atoms, charge) = a.difference(&b);
        assert!(atoms.is_empty());
        assert_eq!(charge, 1);
    }
}
