//! The Evidence twin merge: collapse the duplicate-extraction class at the data
//! layer, one cluster at a time.
//!
//! ## What a "twin" is, measured
//!
//! 21 pairs of `Evidence` nodes in the live graph produce the SAME key under the
//! stable-id arm — the same statement extracted twice, identical on document,
//! page, Q-number, verbatim quote, question and (20 of 21) statement type,
//! differing only in LLM-mooded prose (`significance` phrased two ways). No
//! triples: the ×2 class is the whole of it.
//!
//! Because the twins are not distinguishable by anything stable, `rekey_evidence`
//! REFUSES them — 42 of 525 nodes keep their old blob-hash ids on every run. That
//! refusal is correct and it is also permanent until something collapses the
//! duplicates. This tool is that something. Ruled option (C) 2026-08-14: treat
//! the ×2 class as the data defect it is.
//!
//! ## The one decision this tool will not make
//!
//! Measured on DEV 2026-08-15: **7 of the 21 pairs carry curated rows on BOTH
//! twins** (112 rows between them), and three of those pairs carry CONFLICTING
//! weights — one twin `carries` a scenario, the other is `backup`, same scenario.
//! Roman ruled both, on two cards, because the duplicate reached the ruling queue
//! twice.
//!
//! There is no honest way for a program to decide which of those two rulings
//! survives. So it does not: **any cluster with curated rows on more than one
//! member is refused and written to the human queue**, with every ruling
//! enumerated, for Roman's merge session. The other 14 pairs carry no curated
//! rows at all and merge mechanically in the same run.
//!
//! Expected on the first live dry run, from today's measurement:
//! **21 clusters seen · 14 merged · 7 refused (curated on both) · 0 refused for
//! any other reason.** A dry run that prints different numbers means the corpus
//! moved, and the runbook step says to stop and find out why.
//!
//! ## Why relationships are compared rather than repointed
//!
//! A general node merge has to move the loser's edges onto the survivor, and
//! creating an edge whose TYPE is only known at run time is not something plain
//! Cypher can do. This tool avoids the problem instead of reaching for a plugin:
//! twins are the same statement in the same document, so their edge sets are the
//! same edge set. Measured: **20 of 21 pairs have byte-identical
//! `(type, other node)` sets**; the 21st differs, and it is one of the 7 already
//! refused for curation.
//!
//! So the rule is: the loser's edges must be a SUBSET of the survivor's. If they
//! are, deleting the loser loses nothing and the merge is provably lossless. If
//! they are not, the loser knows something the survivor does not — which is a
//! difference, and differences go to the human queue like every other one.
//!
//! ## Discipline
//!
//! Dry run by default; `--apply` is the only writing path; one cluster is one
//! unit of work, counted before and after and rolled back whole on any mismatch;
//! idempotent (a merged cluster is a single node next run, and single nodes are
//! not clusters). Exit codes on the family scheme in [`crate::oneshot::exit`].

pub mod execute;
pub mod graph;
pub mod plan;
pub mod report;

pub use plan::{ClusterPlan, Disposition, TwinNode, TwinPlan};
