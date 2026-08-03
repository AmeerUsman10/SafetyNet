//! # protocell-chem — reactions, thermodynamics, provenance
//!
//! Three constraints from `PROMPT.md` §4 are enforced structurally here rather than by
//! discipline, because discipline is what fails at 3 a.m. in month six:
//!
//! - **Invariant 1** (atom and charge balance) is checked in [`network::Network::validate`],
//!   which [`ssa::Simulation::new`] calls unconditionally. There is no path to a
//!   simulation that skipped it.
//! - **Invariant 3** (thermodynamic consistency) is enforced by there being no API that
//!   accepts an independently chosen reverse rate. See [`network::Network::add_reversible`].
//! - **Invariant 9** (provenance) is enforced by [`provenance::Param`], whose only
//!   constructor validates, and which cannot represent an unjustified value.

pub mod formula;
pub mod network;
pub mod provenance;
pub mod ssa;
pub mod thermo;

pub use formula::Formula;
pub use network::{Network, Reaction, Species, Tier};
pub use provenance::{Method, Param, ParamTable, Uncertainty};
pub use ssa::{Simulation, StopReason};
pub use thermo::ConcentrationUnit;
