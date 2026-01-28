use serde::{Deserialize, Serialize};

/// Document linked to evidence in the chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainDocument {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<i64>,
}

/// Evidence item with optional linked document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceWithDocument {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<ChainDocument>,
}

/// Motion claim with its supporting evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionClaimWithEvidence {
    pub id: String,
    pub title: String,
    pub evidence: Vec<EvidenceWithDocument>,
}

/// Allegation at the root of the chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainAllegation {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paragraph: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_status: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub legal_counts: Vec<String>,
}

/// Summary counts for the evidence chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainSummary {
    pub motion_claim_count: usize,
    pub evidence_count: usize,
    pub document_count: usize,
}

/// Complete evidence chain response for an allegation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceChainResponse {
    pub allegation: ChainAllegation,
    pub motion_claims: Vec<MotionClaimWithEvidence>,
    pub summary: ChainSummary,
}
