//! Response DTOs for `GET /api/case`.
//!
//! ## Why no `deny_unknown_fields` in this module
//!
//! Every struct here is a RESPONSE shape — the backend produces them and the
//! client renders them; nothing deserializes them but a test. `deny_unknown_fields`
//! belongs on request bodies, where a client typo must fail loudly. Same posture
//! as `dto::case_health`.

use serde::{Deserialize, Serialize};

/// Core case metadata from the Case node
// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseInfo {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub court: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub court_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filing_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// A party involved in the case (Person or Organization)
// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyDto {
    pub id: String,
    pub name: String,
    /// "person" or "organization"
    #[serde(rename = "type")]
    pub party_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Parties grouped by their role in the case
// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartiesGroup {
    pub plaintiffs: Vec<PartyDto>,
    pub defendants: Vec<PartyDto>,
    pub other: Vec<PartyDto>,
}

/// Summary info for a legal count (cause of action)
// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalCountSummary {
    pub id: String,
    pub name: String,
    pub count_number: i64,
}

/// Aggregated statistics for the case.
///
/// ## Two fields were deleted here on 2026-07-27, not merely zeroed
///
/// `allegations_proven` counted Allegations whose own `grounding_status` was
/// `exact` or `normalized` — i.e. whether an Allegation's **own quote could be
/// located in its own source PDF** — and published that as "proven". That is
/// quote-verifiability, not evidentiary support; the label inverted the meaning
/// of the word this product exists to measure. Element-level coverage is served
/// by `causes_of_action_builder::derive_proof_status`, which is the codebase's
/// single element-verdict vocabulary.
///
/// `evidence_count` was the literal `0`, carrying the comment "Evidence nodes
/// don't exist in v2". They do — the graph holds hundreds. Corpus Evidence
/// totals are served by the Case Health inventory, which measures them.
///
/// Both were computed on every page load and rendered nowhere (only
/// `legal_count_details` is consumed, via `CaseContext`), so removing them cost
/// no display. They are gone rather than corrected because a correct version of
/// either already exists on a surface that owns it.
// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseStats {
    pub allegations_total: i64,
    pub document_count: i64,
    pub damages_total: f64,
    pub legal_counts: i64,
    pub legal_count_details: Vec<LegalCountSummary>,
}

impl Default for CaseStats {
    fn default() -> Self {
        Self {
            allegations_total: 0,
            document_count: 0,
            damages_total: 0.0,
            legal_counts: 0,
            legal_count_details: Vec::new(),
        }
    }
}

/// Full response for GET /case
// serde: allows unknown fields because this is a response shape whose only deserializer is a test — see the module note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case: Option<CaseInfo>,
    pub parties: PartiesGroup,
    pub stats: CaseStats,
}

impl CaseResponse {
    /// Return an empty response for when no Case node exists in Neo4j.
    pub fn empty() -> Self {
        Self {
            case: None,
            parties: PartiesGroup {
                plaintiffs: vec![],
                defendants: vec![],
                other: vec![],
            },
            stats: CaseStats::default(),
        }
    }
}
