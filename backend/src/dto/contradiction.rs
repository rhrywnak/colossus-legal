use serde::{Deserialize, Serialize};

/// One side of a contradiction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionEvidence {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_title: Option<String>,
}

/// A contradiction between two pieces of evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionDto {
    pub evidence_a: ContradictionEvidence,
    pub evidence_b: ContradictionEvidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Response for GET /contradictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionsResponse {
    pub contradictions: Vec<ContradictionDto>,
    pub total: usize,
}
