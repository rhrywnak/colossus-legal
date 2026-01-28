use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct EvidenceCreateRequest {
    pub id: Option<String>,
    pub claim_id: Option<String>,
    pub document_id: Option<String>,
    pub description: Option<String>,
    pub evidence_type: Option<String>,
    pub is_supporting: Option<bool>,
    pub collected_on: Option<NaiveDate>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EvidenceUpdateRequest {
    pub claim_id: Option<String>,
    pub document_id: Option<String>,
    pub description: Option<String>,
    pub evidence_type: Option<String>,
    pub is_supporting: Option<bool>,
    pub collected_on: Option<NaiveDate>,
}

/// Response DTO for a single evidence item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceDto {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exhibit_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub significance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_title: Option<String>,
}

/// Response for GET /evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceResponse {
    pub evidence: Vec<EvidenceDto>,
    pub total: usize,
    pub by_kind: HashMap<String, usize>,
}
