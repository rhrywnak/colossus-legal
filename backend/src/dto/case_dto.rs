use serde::{Deserialize, Serialize};

/// Core case metadata from the Case node
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartiesGroup {
    pub plaintiffs: Vec<PartyDto>,
    pub defendants: Vec<PartyDto>,
    pub other: Vec<PartyDto>,
}

/// Summary info for a legal count (cause of action)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalCountSummary {
    pub id: String,
    pub name: String,
    pub count_number: i64,
}

/// Aggregated statistics for the case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseStats {
    pub allegations_total: i64,
    pub allegations_proven: i64,
    pub evidence_count: i64,
    pub document_count: i64,
    pub damages_total: f64,
    pub legal_counts: i64,
    pub legal_count_details: Vec<LegalCountSummary>,
}

impl Default for CaseStats {
    fn default() -> Self {
        Self {
            allegations_total: 0,
            allegations_proven: 0,
            evidence_count: 0,
            document_count: 0,
            damages_total: 0.0,
            legal_counts: 0,
            legal_count_details: Vec::new(),
        }
    }
}

/// Full response for GET /case
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
