//! The Evidence re-key: a one-shot maintenance tool, not a migration.
//!
//! ## Why this is a binary and not a `migrations/` file
//!
//! Ruled 2026-08-14, correcting the task file. The re-key has to move an id in
//! Postgres AND on the Neo4j node in step, and the new id is computed by
//! `api::pipeline::evidence_key` — NFC normalization and a SHA-256 over
//! normalized text. sqlx's runner can do neither: it speaks only SQL, only to
//! Postgres. `migrations_neo4j/` exists in this repo but nothing runs it.
//!
//! So the re-key is a one-shot binary Roman runs from a runbook step after the
//! batch deploys and before the wave — deliberately nothing clickable and
//! nothing an accidental redeploy can re-trigger.
//!
//! ## The honest safety story
//!
//! There is no cross-store transaction here and this module does not pretend
//! otherwise. What it does instead:
//!
//! - the unit of work is ONE DOCUMENT, and the Postgres side of that unit is one
//!   transaction that either lands whole or rolls back;
//! - every table's row count is measured BEFORE the write and verified AFTER it;
//!   a mismatch aborts that document and leaves it untouched;
//! - the runbook step takes database backups immediately before `--apply`. That
//!   is the real net under the two-store seam, stated plainly rather than
//!   implied by a word like "atomic".
//!
//! ## Dry-run is the default
//!
//! Running the binary with no flags plans everything, proves the counts it would
//! write, and writes nothing. `--apply` is the only path that mutates, and what
//! it executes is the same plan the dry-run printed.

pub mod execute;
pub mod plan;
pub mod report;

pub use plan::{Disposition, EvidenceRow, PlanTotals, PlannedNode, RekeyPlan};
