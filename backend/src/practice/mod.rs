//! The practice drill's non-HTTP half: the seeded deck, and the tool that writes it.
//!
//! ## What lives here and what does not
//!
//! This module is the SEED's world — reading a deck file a human wrote, proving
//! it against the scenario it claims to be about, and writing it once. The
//! running drill (the payload, the read, the sheet) lives under `services` and
//! `api` beside every other surface.
//!
//! The seam is the one the one-shot family already draws (`oneshot`): a tool's
//! PLAN is pure and unit-testable without a database, and only its execution
//! touches one. [`deck_file`] is that plan; [`seed`] is that execution.

pub mod deck_file;
pub mod seed;
pub mod sources;
