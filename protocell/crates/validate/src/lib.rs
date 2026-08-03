//! # protocell-validate — the ladder, and the arithmetic that de-risks it
//!
//! `AGENTS.md` gives the V&V role standing on everything and one permanent question:
//! *what test asserts this?* This crate is the answer for phase P0.
//!
//! - [`stats`] — dependency-free goodness-of-fit machinery, itself checked against
//!   published chi-square critical points.
//! - [`ladder`] — L0a/L0b/L0c and the kill test. Each has a **negative control**: a
//!   deliberately wrong hypothesis that the same test must reject.
//! - [`replay`] — what standalone replay actually recovers, stated precisely.
//! - [`budget`] — the two calculations that could have ended the project cheaply, run
//!   before the code that depends on them rather than at P3.

pub mod budget;
pub mod ladder;
pub mod replay;
pub mod stats;
