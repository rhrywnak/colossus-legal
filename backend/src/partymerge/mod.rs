//! The party merge (P7): collapse duplicate `Person` / `Organization` nodes onto
//! one canonical node — but ONLY where Roman has named the survivor by hand.
//!
//! ## Why this exists
//!
//! Measured: 54 `Person` + 8 `Organization` nodes, of which only 8 Persons carry
//! any statement at all. The worst cluster is the judge — `Karen A. Tighe` holds
//! 39 sworn statements from the hearing transcript and `Tighe` holds 62 from the
//! opinion, so 101 statements are split across two nodes that are one person.
//! Roman's ruled People-view totality is fragmented until that is fixed.
//!
//! It will not fix itself. Ingest resolution deliberately auto-merges only exact
//! and normalized name matches (`ingest_resolver.rs`, ruling 2026-07-20) —
//! everything fuzzier is DEMOTED to a new node and logged, on the correct
//! reasoning that a false merge silently welds two real people into one and
//! attributes one person's sworn statements to another. The consequence of that
//! good policy is that every re-extraction spelling a name differently mints
//! another node, which is why the merge pass must run BEFORE the wave.
//!
//! ## The tool takes no view on who is who
//!
//! **Input is a rulings file, never a guess.** The tool merges exactly what
//! Roman's file names: per cluster, one survivor id and the member ids to merge
//! into it, or `SKIP`. There is no fuzzy matching anywhere in the execution path,
//! no default survivor, and no cluster acts without a human having named it.
//!
//! `--emit-template` generates the file for him to fill in — every party in the
//! graph with its label, statement count, source documents and recorded aliases,
//! ordered so probable duplicates sit next to each other. That ordering is a
//! READING AID and is labelled as one in the file it writes: the generated blocks
//! all say `SKIP`, so a template returned unedited merges nothing.
//!
//! ## Acceptance, per the addendum
//!
//! Statement totals conserved per cluster (Tighe 39 + 62 → one node with 101) ·
//! zero dangling references · the People page count drops by exactly the merged
//! member count · the report proves each. The tool measures the statement count
//! before and after every cluster and ABORTS that cluster if the total moved.
//!
//! ## Discipline
//!
//! Same family as `rekey_evidence` and `merge_evidence_twins`: dry run by
//! default, `--apply` the only write path, one cluster is one verified unit of
//! work, count proofs to `tracing` and to a file, idempotent, exit codes on the
//! scheme in [`crate::oneshot::exit`].

pub mod census;
pub mod execute;
pub mod graph_ops;
pub mod plan;
pub mod report;
pub mod rulings;

pub use plan::{ClusterPlan, MergePlan};
pub use rulings::{Ruling, RulingsFile};
