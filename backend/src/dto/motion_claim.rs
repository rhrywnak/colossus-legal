use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// DTO for MotionClaim nodes - claims made in legal motions
/// that link to evidence and allegations they prove
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionClaimDto {
    pub id: String,
    pub title: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub significance: Option<String>,

    /// IDs of allegations this claim proves (via PROVES relationship)
    pub proves_allegations: Vec<String>,

    /// IDs of evidence this claim relies on (via RELIES_ON relationship)
    pub relies_on_evidence: Vec<String>,

    /// Source document ID (via APPEARS_IN relationship)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_document_id: Option<String>,

    /// Source document title for display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_document_title: Option<String>,
}

/// Response for GET /motion-claims endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionClaimsResponse {
    pub motion_claims: Vec<MotionClaimDto>,
    pub total: usize,
    pub by_category: HashMap<String, usize>,
}
