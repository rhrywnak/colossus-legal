use serde::{Deserialize, Serialize};

/// Summary counts by evidence status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllegationSummary {
    pub proven: usize,
    pub partial: usize,
    pub unproven: usize,
}

/// Response DTO for a single allegation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllegationDto {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paragraph: Option<String>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allegation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<i64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub legal_count_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub legal_counts: Vec<String>,
}

/// Response for GET /allegations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllegationsResponse {
    pub allegations: Vec<AllegationDto>,
    pub total: usize,
    pub summary: AllegationSummary,
}
