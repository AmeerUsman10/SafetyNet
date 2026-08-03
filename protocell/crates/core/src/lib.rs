//! # protocell-core — the P0 kernel
//!
//! Three things live here, and they are the three that everything else assumes:
//!
//! - [`rng`] — counter-based PRNG. Every draw is a pure function of its address
//!   `(seed, stream, step, entity)`. No sequential state anywhere.
//! - [`entity`] — `EntityId` allocation that never reuses an id, and a
//!   structure-of-arrays table whose resident cost tracks the *live* population.
//! - [`event`] — append-only, index-addressable log holding only non-reconstructible
//!   facts.
//!
//! Together these are the mechanism behind the project's cost claim: you do not dump
//! frames, you dump events and replay the rest from the RNG address. If any one of the
//! three is compromised the claim collapses, so all three are asserted rather than
//! documented. See `PROMPT.md` §4 invariants 7 and 8.

pub mod entity;
pub mod event;
pub mod rng;

pub use entity::{EntityId, EntityTable, SpeciesId};
pub use event::{Event, EventKind, EventLog, EVENT_BYTES};
pub use rng::{Rng, Stream, ENTITY_GLOBAL};

/// Physical constants. Values are exact by SI definition or CODATA-2018; they are the
/// one class of number in this codebase that does not need a provenance record,
/// because they are definitional rather than measured.
pub mod consts {
    /// Molar gas constant, J mol^-1 K^-1. Exact by the 2019 SI redefinition
    /// (R = N_A k_B, both exact).
    pub const R: f64 = 8.314_462_618_153_24;

    /// Avogadro constant, mol^-1. Exact by SI definition.
    pub const N_A: f64 = 6.022_140_76e23;

    /// Standard-state concentration for aqueous biochemistry, mol L^-1.
    ///
    /// This is the reference that makes `exp(-dG/RT)` dimensionless for reactions with
    /// a change in molecularity. Forgetting it is the unit error corrected in
    /// `protocell_chem::thermo`.
    pub const C_STANDARD: f64 = 1.0;
}
