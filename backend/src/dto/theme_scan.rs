//! Theme Scan response DTOs (D2b).
//!
//! The wire shape returned by
//! `POST /cases/:slug/scenarios/:scenario_id/theme-scan`. A scan reads every
//! candidate quote about the scenario's subject, asks the LLM judge to rate each
//! one against the scenario's `attack_meaning`, writes the RELEVANT verdicts to
//! `scenario_fact_refs` (as `confirmed=false` suggestions), and returns this
//! summary so D3 can render the suggestions and the honesty-check sample.
//!
//! ## Why the rejected sample rides in the response, not a second endpoint
//!
//! Amendment 1 wants an honesty check: after a scan, show a sample of the quotes
//! the judge REJECTED, so a human can confirm the judge is not silently dropping
//! relevant evidence. We do NOT persist irrelevant verdicts (only relevant ones
//! become `scenario_fact_refs` rows), so the rejected quotes exist only in the
//! scan's own memory. Returning a bounded sample INLINE here is therefore the
//! cheapest honest design — the scan already judged them; a separate "sample
//! rejects" endpoint would have to re-run the whole scan to reconstruct the set.
//!
//! ## Why `content` is a `BiasInstance`
//!
//! A `scenario_fact_refs` row stores only a `graph_node_id`; the human-readable
//! content (quote, speaker, document, pattern tags) lives in the graph. The Bias
//! Explorer already assembles exactly that into `BiasInstance`, and the fact
//! curation DTOs (`ScenarioFactDto`) already reuse it — so one frontend card
//! renders a bias candidate, a saved fact, AND a scan suggestion, with one
//! graph→content mapping rather than three that can drift.

use serde::Serialize;

use crate::bias::dto::BiasInstance;

/// Result of one Theme Scan run.
///
/// The four counts are exhaustive and non-overlapping:
/// `candidates_read == relevant_written + irrelevant + failed`. This identity is
/// the recall guarantee made observable (Standing Rule 1) — every candidate the
/// scan read lands in exactly one bucket, so a dropped quote would show up as a
/// count that does not add up rather than as a silent absence.
#[derive(Debug, Clone, Serialize)]
pub struct ThemeScanSummary {
    /// Total candidate quotes read for the subject (the ungated
    /// `all_evidence_about_subject` count — every Evidence ABOUT the subject).
    pub candidates_read: usize,
    /// Verdicts judged RELEVANT and successfully written to
    /// `scenario_fact_refs` as `confirmed=false` suggestions.
    pub relevant_written: usize,
    /// Verdicts judged NOT relevant to the accusation. Not written; a sample is
    /// surfaced in [`Self::rejected_sample`] for the honesty check.
    pub irrelevant: usize,
    /// Candidates whose verdict could not be produced: an LLM call that
    /// exhausted retries, a reply that failed the strict parse, an out-of-set
    /// role, or a write that failed. Counted, never silently dropped — each is
    /// logged with its `evidence_id` and cause (Standing Rule 1).
    pub failed: usize,
    /// The written suggestions, so the client can render them without a second
    /// round-trip. One entry per `relevant_written`.
    pub suggestions: Vec<ThemeScanSuggestion>,
    /// A bounded, spread-out sample of the rejected quotes for the Amendment-1
    /// honesty check. Empty when nothing was rejected; at most
    /// `THEME_SCAN_REJECTED_SAMPLE_SIZE` entries otherwise.
    pub rejected_sample: Vec<ThemeScanRejected>,
}

/// One RELEVANT verdict written to `scenario_fact_refs`.
#[derive(Debug, Clone, Serialize)]
pub struct ThemeScanSuggestion {
    /// The Evidence node id this suggestion references (the `graph_node_id`
    /// column, equal to the candidate's `evidence_id`).
    pub graph_node_id: String,
    /// The role the judge assigned — one of the four `FactRole` tokens
    /// (`supports` / `corroborates` / `contradicts` / `rebuts`).
    pub proposed_role: String,
    /// The judge's one-to-two-sentence justification (stored in the row's
    /// `note` column).
    pub reason: String,
    /// The judge's self-reported confidence in `[0.0, 1.0]` (stored in the
    /// row's `confidence` column).
    pub confidence: f32,
    /// Live graph card content for the referenced node, so the client renders
    /// the suggestion as a normal fact card.
    pub content: BiasInstance,
}

/// One REJECTED quote, surfaced for the honesty check (never persisted).
#[derive(Debug, Clone, Serialize)]
pub struct ThemeScanRejected {
    /// The Evidence node id the judge rejected.
    pub graph_node_id: String,
    /// Why the judge deemed it not relevant — lets a human sanity-check that the
    /// rejection was sound rather than a missed connection.
    pub reason: String,
    /// The judge's confidence in the (irrelevant) verdict, `[0.0, 1.0]`.
    pub confidence: f32,
    /// Live graph card content for the rejected node.
    pub content: BiasInstance,
}
