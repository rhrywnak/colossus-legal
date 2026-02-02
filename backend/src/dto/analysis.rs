use serde::{Deserialize, Serialize};

// ============================================================================
// Gap Analysis DTOs
// ============================================================================

/// Strength information for a single allegation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllegationStrength {
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allegation: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub paragraph: Option<String>,

    /// Calculated strength percentage (0-100)
    pub strength_percent: i32,

    /// Category: "strong", "moderate", "weak", "gap"
    pub strength_category: String,

    /// Number of evidence items supporting this allegation
    pub supporting_evidence_count: i32,

    /// Brief descriptions of supporting evidence
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub supporting_evidence: Vec<String>,

    /// Notes about what evidence is missing (for gaps)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap_notes: Option<String>,
}

/// Summary of gap analysis across all allegations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapAnalysis {
    pub total_allegations: i32,
    pub strong_evidence: i32,   // 90%+ strength
    pub moderate_evidence: i32, // 70-89% strength
    pub weak_evidence: i32,     // 50-69% strength
    pub gaps: i32,              // <50% strength
    pub allegations: Vec<AllegationStrength>,
}

// ============================================================================
// Contradictions Summary DTOs
// ============================================================================

/// Brief summary of a contradiction for the dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionBrief {
    pub evidence_a_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_a_title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_a_answer: Option<String>,

    pub evidence_b_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_b_title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_b_answer: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Summary of all contradictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionsSummary {
    pub total: i32,
    pub contradictions: Vec<ContradictionBrief>,
}

// ============================================================================
// Evidence Coverage DTOs
// ============================================================================

/// Coverage statistics for a single document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentCoverage {
    pub document_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_title: Option<String>,

    /// Total evidence items extracted from this document
    pub evidence_count: i32,

    /// Evidence items linked to allegations (via MotionClaims)
    pub linked_count: i32,
}

/// Overall evidence coverage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceCoverage {
    pub total_evidence_nodes: i32,
    pub linked_to_allegations: i32,
    pub unlinked: i32,
    pub by_document: Vec<DocumentCoverage>,
}

// ============================================================================
// Main Response DTO
// ============================================================================

/// Complete analysis response for GET /analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResponse {
    pub gap_analysis: GapAnalysis,
    pub contradictions_summary: ContradictionsSummary,
    pub evidence_coverage: EvidenceCoverage,
}
