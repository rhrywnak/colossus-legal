// =============================================================================
// backend/src/dto/decomposition.rs
// =============================================================================
//
// Data Transfer Objects for the Decomposition API (Phase F, Feature F.1)
//
// Three endpoints:
//   GET /decomposition             — Overview of all 18 allegations
//   GET /allegations/:id/detail    — Deep dive into one allegation
//   GET /rebuttals                 — All REBUTS grouped by George's claims
//
// RUST PATTERN: Nested response structs
// ─────────────────────────────────────
// Neo4j returns flat rows, but our API consumers need nested JSON.
// We define the nested shape here; the repository assembles flat rows
// into these structures using HashMap accumulators.
// =============================================================================

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Endpoint 1: GET /decomposition
// ─────────────────────────────────────────────────────────────────────────────

/// Top-level response for the decomposition overview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionResponse {
    pub allegations: Vec<AllegationOverview>,
    pub summary: DecompositionSummary,
}

/// One row in the decomposition table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllegationOverview {
    pub id: String,
    pub title: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    pub status: String,

    /// All labels George applied: ["frivolous", "false", "unfounded", ...]
    pub characterizations: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub characterized_by: Option<String>,

    pub proof_count: i64,
    pub rebuttal_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionSummary {
    pub total_allegations: i64,
    pub proven_count: i64,
    pub all_proven: bool,
    pub total_characterizations: i64,
    pub total_rebuttals: i64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Endpoint 2: GET /allegations/:id/detail
// ─────────────────────────────────────────────────────────────────────────────

/// Full detail view for a single allegation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllegationDetailResponse {
    pub allegation: AllegationInfo,
    pub characterizations: Vec<CharacterizationDetail>,
    pub proof_claims: Vec<ProofClaimSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllegationInfo {
    pub id: String,
    pub title: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    pub status: String,
    pub legal_counts: Vec<String>,
}

/// One characterization George made, with the rebuttal chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterizationDetail {
    pub label: String,
    pub evidence_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbatim_quote: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stated_by: Option<String>,

    pub rebuttals: Vec<RebuttalDetail>,
}

/// One piece of evidence that disproves a characterization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuttalDetail {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofClaimSummary {
    pub id: String,
    pub title: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    pub evidence_count: i64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Endpoint 3: GET /rebuttals
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuttalsResponse {
    pub george_claims: Vec<GeorgeClaimWithRebuttals>,
    pub summary: RebuttalsSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeorgeClaimWithRebuttals {
    pub claim_id: String,
    pub claim_title: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub george_quote: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,

    pub rebuttals: Vec<RebuttalDetail>,
    pub rebuttal_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuttalsSummary {
    pub total_george_claims_rebutted: i64,
    pub total_george_claims_unrebutted: i64,
    pub total_rebuttals: i64,
    pub unrebutted_reasons: Vec<UnrebuttedReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnrebuttedReason {
    pub claim: String,
    pub reason: String,
}
