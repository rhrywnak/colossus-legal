use serde::{Deserialize, Serialize};

/// Response for GET /case-summary — analytical dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseSummaryResponse {
    // Case identity
    pub case_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub court: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case_number: Option<String>,

    // Proof strength
    pub allegations_total: i64,
    pub allegations_proven: i64,
    pub legal_counts: i64,
    pub legal_count_details: Vec<LegalCountInfo>,

    // Damages
    pub damages_total: f64,
    pub damages_financial: f64,
    pub damages_reputational_count: i64,
    pub harms_total: i64,

    // Decomposition intelligence
    pub characterizations_total: i64,
    pub characterizations_by_person: Vec<PersonCharacterizationCount>,
    pub rebuttals_total: i64,
    pub unique_characterization_labels: Vec<String>,

    // Evidence strength
    pub evidence_total: i64,
    pub evidence_grounded: i64,
    pub documents_total: i64,

    // Parties
    pub plaintiffs: Vec<String>,
    pub defendants: Vec<String>,
}

/// Legal count with its ID, name, and how many allegations support it
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalCountInfo {
    pub id: String,
    pub name: String,
    pub count_number: i64,
    pub allegation_count: i64,
}

/// How many characterizations a specific person made
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonCharacterizationCount {
    pub person: String,
    pub count: i64,
}
