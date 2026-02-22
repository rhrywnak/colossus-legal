use serde::{Deserialize, Serialize};

/// Response for GET /persons/:id/detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonDetailResponse {
    pub person: PersonInfo,
    pub summary: PersonSummary,
    pub documents: Vec<DocumentGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonInfo {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonSummary {
    pub total_statements: i64,
    pub documents_count: i64,
    pub characterizations_count: i64,
    pub rebuttals_received_count: i64,
}

/// Statements grouped by source document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentGroup {
    pub document_id: String,
    pub document_title: String,
    pub statement_count: usize,
    pub statements: Vec<StatementDetail>,
}

/// A single evidence statement with optional characterization and rebuttal info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementDetail {
    pub evidence_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbatim_quote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub significance: Option<String>,
    pub characterizes: Vec<CharacterizesInfo>,
    pub rebutted_by: Vec<RebuttalInfo>,
}

/// Which allegation a statement characterizes and how
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterizesInfo {
    pub allegation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allegation_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub characterization_label: Option<String>,
}

/// Evidence that rebuts a statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuttalInfo {
    pub evidence_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbatim_quote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stated_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_title: Option<String>,
}
