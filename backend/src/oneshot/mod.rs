//! The one-shot maintenance-tool family: shared law, shared plumbing.
//!
//! Four binaries now live in this family — `rekey_evidence` (shipped
//! 2026-08-14), `merge_evidence_twins`, `remap_evidence` and `merge_parties`.
//! They do very different work, but the ruling of 2026-08-14 gave them one
//! identical discipline, and this module is where that discipline lives exactly
//! once:
//!
//! - **dry run is the default**; `--apply` is the only writing path;
//! - **the unit of work is small and verified** — count before, write, count
//!   after, commit or roll that unit back;
//! - **the count proof is real output**, to `tracing` and to a report file, both
//!   rendered from the same structures so the file cannot say something the log
//!   did not;
//! - **idempotent**, so a run interrupted half-way is safe to repeat;
//! - **honest exit codes** on one scheme (see [`exit`]).
//!
//! ## Why this module exists rather than four copies
//!
//! The first tool wrote its exit codes, its two connection helpers and its
//! report-file write inline. Three more tools would have been three more copies
//! of each — and the failure mode of copied exit codes is the ugly one: they
//! drift, and a runbook step that branches on `3` starts meaning two different
//! things depending on which binary produced it. The codes are a CONTRACT with
//! the runbook, so they get one definition.
//!
//! ## What is deliberately NOT shared
//!
//! Each tool's *plan* — the decisions it makes — stays in its own crate module,
//! pure and unit-tested without a database. There is no shared `Plan` trait and
//! there should not be one: a re-key decides per node, a merge decides per
//! cluster, a remap decides per candidate pair, and forcing those three into one
//! shape would buy nothing but a layer of indirection between a reader and the
//! decision they came to read.

pub mod cli;
pub mod exit;
pub mod refs;
