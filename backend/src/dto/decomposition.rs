//!
//! ## Why no `deny_unknown_fields` in this module
//!
//! Response shapes only; the reasoning is in `dto::case_dto`.

// =============================================================================
// backend/src/dto/decomposition.rs
// =============================================================================
//
// Data Transfer Objects for the Decomposition API (Phase F, Feature F.1)
//
// Two endpoints:
//   GET /allegations/:id/detail    — Deep dive into one allegation
//   GET /rebuttals                 — All REBUTS grouped by George's claims
//
// The former `GET /decomposition` overview was retired in the 2026-07-27
// honesty batch: its `proven_count` / `all_proven` summary tested an allegation
// `status` property the v5.1 migration had already dropped (the query returned a
// literal NULL), so both figures were permanently zero/false by construction,
// and no other surface consumed the endpoint.
//
// RUST PATTERN: Nested response structs
// ─────────────────────────────────────
// Neo4j returns flat rows, but our API consumers need nested JSON.
// We define the nested shape here; the repository assembles flat rows
// into these structures using HashMap accumulators.
// =============================================================================

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Endpoint 1: GET /allegations/:id/detail
// ─────────────────────────────────────────────────────────────────────────────

/// Full detail view for a single allegation.
// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllegationDetailResponse {
    pub allegation: AllegationInfo,
    pub characterizations: Vec<CharacterizationDetail>,
    pub proof_claims: Vec<ProofClaimSummary>,
}

// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllegationInfo {
    pub id: String,
    pub title: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    pub legal_counts: Vec<String>,
}

/// One characterization George made, with the rebuttal chain.
// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
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
// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
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

// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofClaimSummary {
    pub id: String,
    pub title: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    pub evidence_count: i64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Endpoint 2: GET /rebuttals
// ─────────────────────────────────────────────────────────────────────────────

// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuttalsResponse {
    pub george_claims: Vec<GeorgeClaimWithRebuttals>,
    pub summary: RebuttalsSummary,
}

// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
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

// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuttalsSummary {
    pub total_george_claims_rebutted: i64,
    pub total_george_claims_unrebutted: i64,
    pub total_rebuttals: i64,
    pub unrebutted_reasons: Vec<UnrebuttedReason>,
}

// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnrebuttedReason {
    pub claim: String,
    pub reason: String,
}
