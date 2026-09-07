//! The remap (P2): carry curated rows across a re-extraction, when ids genuinely
//! change.
//!
//! ## What this is for, and why it still exists after the id arm
//!
//! Measured before the stable-id arm was built: a real reprocess of one document
//! destroyed **0 of 131** Evidence ids, because the id was a hash over the whole
//! LLM blob. The arm fixes that class at the door — same document, same page,
//! same quote, same id — so after it deploys most reprocesses should move no id
//! at all.
//!
//! "Most" is not "all". A re-extraction that reads a quote differently (OCR
//! improves, a template changes what counts as one statement) produces a node
//! that is genuinely new, and its curated rows are genuinely orphaned. That is
//! what this tool is for, and it is the reason it ships now rather than after the
//! first time it is needed.
//!
//! ## Three steps, because approval is a step
//!
//! ```text
//! remap_evidence snapshot --document D --out before.json   # BEFORE the reprocess
//! remap_evidence propose  --snapshot before.json --document D \
//!                         --out proposal.txt --queue queue.txt
//! # ... a human reads the proposal, deletes any MAP line they reject,
//! #     and writes an APPROVED line at the top ...
//! remap_evidence apply    --proposal proposal.txt
//! ```
//!
//! `apply` refuses a proposal with no `APPROVED` line. That is what makes the
//! middle step real rather than decorative: a generated file cannot be executed
//! by accident, and a human who did not read it cannot have added the line.
//!
//! ## What is auto-applied and what is not
//!
//! Only an UNAMBIGUOUS 1:1 match — one old node, one new node, one key, nobody
//! else claiming either — becomes a `MAP` line. Ambiguity in either direction and
//! every unmatched old node go to the human queue, one entry each, carrying the
//! quote, the page, the question and the candidates, so the decision can be made
//! from the file.
//!
//! ## Three tiers of evidence (added 2026-09-05)
//!
//! The first real re-extraction matched **0 of 14** movable nodes, because the
//! only test was byte-for-byte agreement and the corrected page text changed
//! every quote's line breaks. Matching now runs in three tiers — exact, exact
//! after a loose normalization, and a measured near match — and every `MAP` line
//! carries the tier and the score that produced it, so approving a tier-3 line is
//! a different act from approving a tier-1 line and reads like one. See
//! [`tiers`] for the order and [`normalize`] for what the loose form drops.
//!
//! Measured yield across a real template change: 87.8% unambiguous, 4 ambiguous,
//! 12 unmatched of 131. That is the floor the Morris gate test checks against.
//!
//! ## It runs NOWHERE until the Morris gate test, under supervision
//!
//! Shipped and tested; not run. The task says so and the tool does not argue.

pub mod execute;
pub mod normalize;
pub mod plan;
pub mod proposal;
mod tiers;

pub use plan::{AutoMove, Match, MatchTier, RemapPlan, Snapshot, SnapshotNode};
