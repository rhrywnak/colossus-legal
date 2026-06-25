//! DTOs for the scenario query library (`ScenarioRepository`).
//!
//! A "scenario" is a slice of the trial-prep picture assembled live from the
//! knowledge graph: the facts that rebut a wielder's claims, the contradiction
//! edges that impeach them, and the allegations a scenario's evidence points
//! at. These types are the wire surface those reads return.
//!
//! ## Why scenario-owned DTOs (not reuse of the existing feature DTOs)
//!
//! The shapes here deliberately mirror the field discipline of
//! `dto::decomposition::RebuttalDetail` and `dto::contradiction::*`, but they
//! are SEPARATE types. The rebuttals/contradictions features and the scenario
//! page are different audiences that will evolve independently; coupling them
//! to one struct would mean a change for one feature silently reshapes the
//! other's API. Keeping the scenario surface's DTOs its own is the cheaper
//! long-run choice even though the structs look similar today.

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Method 1 — rebuttal facts
// ─────────────────────────────────────────────────────────────────────────────

/// One piece of evidence that rebuts a wielder's claim.
///
/// Mirrors the field discipline of `decomposition::RebuttalDetail`:
/// `evidence_id` is always present (a fact with no id is meaningless), every
/// other field is optional because the source node may legitimately omit it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioRebuttalFact {
    pub evidence_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbatim_quote: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stated_by: Option<String>,
}

/// Response for `ScenarioRepository::rebuttal_facts`.
///
/// `wielder_id` echoes back the anchor the caller asked about, so a consumer
/// holding several scenario fragments can tell which wielder each belongs to
/// without threading that context separately.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioRebuttalFactsResponse {
    pub wielder_id: String,
    pub facts: Vec<ScenarioRebuttalFact>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Method 2 — contradictions against a wielder
// ─────────────────────────────────────────────────────────────────────────────

/// One side of a contradiction edge.
///
/// Mirrors `contradiction::ContradictionEvidence`. `id` is required; the
/// display fields are optional.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioContradictionEvidence {
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_title: Option<String>,
}

/// A contradiction between two pieces of evidence, where at least one side is
/// anchored to the queried wielder.
///
/// Domain note: a CONTRADICTS edge is directional in the graph (`a` made the
/// earlier claim, `b` the later admission), but for the scenario surface both
/// sides are carried so the reader sees the full impeachment, not just one end.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioContradiction {
    pub evidence_a: ScenarioContradictionEvidence,
    pub evidence_b: ScenarioContradictionEvidence,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub impeachment_value: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub earlier_claim: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub later_admission: Option<String>,
}

/// Response for `ScenarioRepository::contradictions_against_wielder`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioContradictionsResponse {
    pub anchor_id: String,
    pub contradictions: Vec<ScenarioContradiction>,
    pub total: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// Method 3 — related allegations
// ─────────────────────────────────────────────────────────────────────────────

/// An allegation that a scenario's anchoring evidence points at.
///
/// Domain note: the text of an Allegation lives in the `summary` property (NOT
/// `allegation_text`); `id` is the stable identifier. `title` and
/// `paragraph_number` are carried for display. The text is `Option` because a
/// node could exist without a `summary` — and per the tightened decode
/// discipline, an absent text degrades to `None` cleanly while a malformed one
/// surfaces as an error.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioRelatedAllegation {
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub paragraph_number: Option<String>,
}

/// Response for `ScenarioRepository::related_allegations`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioRelatedAllegationsResponse {
    pub anchor_id: String,
    pub allegations: Vec<ScenarioRelatedAllegation>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Page composition (task 0.4)
// ─────────────────────────────────────────────────────────────────────────────

/// The identity parameters that select a scenario, bound and passed to the
/// page-assembly.
///
/// These are the same bound composite ids the 0.3 methods take (the
/// `doc-...:evidence:<hash>` form) — `wielder_id` is the person whose claims the
/// scenario is built around; `anchor_id` is the evidence the scenario's
/// allegations hang off. For this task they arrive as function arguments; in a
/// later phase they will be read from the scenario `definition` row. The struct
/// is the seam for that change: adding a `definition_id` later is additive.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioPageParams {
    pub wielder_id: String,
    pub anchor_id: String,
}

/// One assembled scenario page — the graph-only sections, for now.
///
/// ## Why hold the three `*Response` types directly
///
/// Each section is exactly the return type of its 0.3 method, so assembly is a
/// lossless move with no remapping (the per-section `total` / echoed id survive).
/// The FULL scenario page (design v2 §2) will also carry Postgres-backed
/// sections — the scenario `definition`, the confirmed-reference-set, and
/// authored `responses` — which DO NOT EXIST YET (Phase 1). This struct is
/// shaped so those land as ADDITIONAL fields without reshaping the three that
/// exist today.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioPage {
    pub rebuttal_facts: ScenarioRebuttalFactsResponse,
    pub contradictions: ScenarioContradictionsResponse,
    pub related_allegations: ScenarioRelatedAllegationsResponse,
}
