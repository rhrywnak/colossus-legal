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

/// Legal count with its ID, name, allegation count, and constituent Elements.
///
/// `allegation_count` is the DISTINCT count of allegations supporting any
/// element of this LegalCount. `elements` carries the per-Element detail
/// rendered on the Home page Count card.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegalCountInfo {
    pub id: String,
    pub name: String,
    pub count_number: i64,
    pub allegation_count: i64,
    /// Elements of proof attached to this count, ordered by `order_in_count`
    /// ascending (then `element_name` ascending for elements with no order).
    /// Empty when the count has no Element children in the graph yet.
    pub elements: Vec<ElementInfo>,
}

/// One Element-of-proof attached to a LegalCount.
///
/// Sourced from the `Element` node label in Neo4j. The `controlling_authority`
/// field carries the case citation or jury-instruction reference that anchors
/// the element legally; it is populated lazily after the canonical Element
/// library is approved, so existing nodes return `None`. The frontend renders
/// a placeholder string when `None` so the popover icon is always present —
/// "missing" and "pending" must be distinguishable in the UI per Standing
/// Rule 1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ElementInfo {
    pub id: String,
    pub element_name: String,
    pub title: String,
    /// Ordering hint emitted by the extraction pipeline. May be missing on
    /// older extractions — the repository sort closure falls back to
    /// alphabetical-on-`element_name` when this is `None`.
    pub order_in_count: Option<i64>,
    /// DISTINCT count of allegations that prove this specific element.
    pub allegation_count: i64,
    /// Case citation or jury-instruction reference. `None` until the
    /// canonical Element library lands; never collapsed to empty string,
    /// so the popover icon is always present and the UI can show a
    /// "pending review" placeholder instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controlling_authority: Option<String>,
}

/// How many characterizations a specific person made
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonCharacterizationCount {
    pub person: String,
    pub count: i64,
}
