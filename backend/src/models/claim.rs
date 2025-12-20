//! Claim model v2 — Supports Claude-extracted claims with source grounding.
//!
//! This module defines the Claim struct and its associated enums for categorization.
//! Claims are extracted from legal documents by Claude and stored in Neo4j.

use chrono::{DateTime, Utc};
use neo4rs::{DeError, Node};
use serde::{Deserialize, Serialize};

// =============================================================================
// ENUMS
// =============================================================================

/// Categories of legal claims extracted from documents.
/// Each variant represents a type of allegation or legal issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimCategory {
    Conversion,
    Fraud,
    BreachOfFiduciaryDuty,
    Defamation,
    Bias,
    DiscoveryObstruction,
    Perjury,
    Collusion,
    FinancialHarm,
    ProceduralMisconduct,
    ConflictOfInterest,
    UnauthorizedPossession,
    ImpartialityViolation,
    WitnessStatement,
    EvidenceContradiction,
    JudicialError,
    ContradictoryStatements,
    ImproperAppointment,
    Other,
}

/// Type of claim — whether it's a factual event, legal conclusion, or procedural matter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimType {
    FactualEvent,
    LegalConclusion,
    Procedural,
}

/// Status of a claim in the analysis workflow.
/// Defaults to `Open` for newly imported claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    #[default]
    Open,
    Closed,
    Refuted,
    Pending,
}

// =============================================================================
// DISPLAY IMPLEMENTATIONS
// =============================================================================

/// Display impl for ClaimCategory — used for v1 API bridge (fallback title).
impl std::fmt::Display for ClaimCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conversion => write!(f, "Conversion"),
            Self::Fraud => write!(f, "Fraud"),
            Self::BreachOfFiduciaryDuty => write!(f, "Breach of Fiduciary Duty"),
            Self::Defamation => write!(f, "Defamation"),
            Self::Bias => write!(f, "Bias"),
            Self::DiscoveryObstruction => write!(f, "Discovery Obstruction"),
            Self::Perjury => write!(f, "Perjury"),
            Self::Collusion => write!(f, "Collusion"),
            Self::FinancialHarm => write!(f, "Financial Harm"),
            Self::ProceduralMisconduct => write!(f, "Procedural Misconduct"),
            Self::ConflictOfInterest => write!(f, "Conflict of Interest"),
            Self::UnauthorizedPossession => write!(f, "Unauthorized Possession"),
            Self::ImpartialityViolation => write!(f, "Impartiality Violation"),
            Self::WitnessStatement => write!(f, "Witness Statement"),
            Self::EvidenceContradiction => write!(f, "Evidence Contradiction"),
            Self::JudicialError => write!(f, "Judicial Error"),
            Self::ContradictoryStatements => write!(f, "Contradictory Statements"),
            Self::ImproperAppointment => write!(f, "Improper Appointment"),
            Self::Other => write!(f, "Other"),
        }
    }
}

/// Display impl for ClaimStatus — used for v1 API bridge.
impl std::fmt::Display for ClaimStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::Closed => write!(f, "closed"),
            Self::Refuted => write!(f, "refuted"),
            Self::Pending => write!(f, "pending"),
        }
    }
}

// =============================================================================
// CLAIM STRUCT
// =============================================================================

/// A claim extracted from a legal document.
///
/// Claims are grounded in source documents via verbatim quotes and location references.
/// They are categorized and tracked through the analysis workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    // --- Identity ---
    pub id: String,

    // --- Content (grounding) ---
    /// Verbatim text from the source document
    pub quote: String,

    /// Optional short title for the claim
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Additional context or summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    // --- Source location ---
    /// Foreign key to Document node
    pub source_document_id: String,

    /// Denormalized document title for display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_document_title: Option<String>,

    /// Document type: motion, complaint, order, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_document_type: Option<String>,

    /// Starting line number in source document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_start: Option<i32>,

    /// Ending line number in source document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_end: Option<i32>,

    /// Page number if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<i32>,

    // --- Classification ---
    /// Primary category of the claim
    pub category: ClaimCategory,

    /// Type: factual event, legal conclusion, or procedural
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_type: Option<ClaimType>,

    /// Severity rating 1-10
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<i32>,

    /// Workflow status
    #[serde(default)]
    pub status: ClaimStatus,

    // --- Financial ---
    /// Dollar amount if claim involves money
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,

    // --- Temporal ---
    /// When the alleged event occurred (ISO date string)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_date: Option<String>,

    // --- Metadata ---
    pub created_at: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extracted_at: Option<DateTime<Utc>>,

    /// Model that extracted this claim, e.g., "claude-opus-4.5"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_model: Option<String>,
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// Error type for converting Neo4j Node to Claim.
#[derive(Debug)]
pub enum ClaimConversionError {
    MissingField(&'static str),
    Value(DeError),
    Neo4j(neo4rs::Error),
}

impl From<neo4rs::Error> for ClaimConversionError {
    fn from(value: neo4rs::Error) -> Self {
        ClaimConversionError::Neo4j(value)
    }
}

impl From<DeError> for ClaimConversionError {
    fn from(value: DeError) -> Self {
        ClaimConversionError::Value(value)
    }
}

// =============================================================================
// NEO4J CONVERSION
// =============================================================================

impl TryFrom<Node> for Claim {
    type Error = ClaimConversionError;

    fn try_from(node: Node) -> Result<Self, Self::Error> {
        // Required fields — error if missing
        let id: String = node.get("id").map_err(ClaimConversionError::from)?;
        let quote: String = node.get("quote").map_err(ClaimConversionError::from)?;
        let source_document_id: String = node
            .get("source_document_id")
            .map_err(ClaimConversionError::from)?;
        let category: ClaimCategory =
            node.get("category").map_err(ClaimConversionError::from)?;
        let created_at: DateTime<Utc> =
            node.get("created_at").map_err(ClaimConversionError::from)?;

        // Optional fields — use .ok() to convert errors to None
        let title: Option<String> = node.get("title").ok();
        let description: Option<String> = node.get("description").ok();
        let source_document_title: Option<String> = node.get("source_document_title").ok();
        let source_document_type: Option<String> = node.get("source_document_type").ok();
        let line_start: Option<i32> = node.get("line_start").ok();
        let line_end: Option<i32> = node.get("line_end").ok();
        let page_number: Option<i32> = node.get("page_number").ok();
        let claim_type: Option<ClaimType> = node.get("claim_type").ok();
        let severity: Option<i32> = node.get("severity").ok();
        let status: ClaimStatus = node.get("status").unwrap_or_default();
        let amount: Option<String> = node.get("amount").ok();
        let event_date: Option<String> = node.get("event_date").ok();
        let updated_at: Option<DateTime<Utc>> = node.get("updated_at").ok();
        let extracted_at: Option<DateTime<Utc>> = node.get("extracted_at").ok();
        let extraction_model: Option<String> = node.get("extraction_model").ok();

        Ok(Self {
            id,
            quote,
            title,
            description,
            source_document_id,
            source_document_title,
            source_document_type,
            line_start,
            line_end,
            page_number,
            category,
            claim_type,
            severity,
            status,
            amount,
            event_date,
            created_at,
            updated_at,
            extracted_at,
            extraction_model,
        })
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a minimal valid Claim for testing.
    fn make_test_claim() -> Claim {
        Claim {
            id: "claim-001".to_string(),
            quote: "The defendant failed to disclose material information.".to_string(),
            title: Some("Failure to Disclose".to_string()),
            description: None,
            source_document_id: "doc-123".to_string(),
            source_document_title: Some("Motion for Default".to_string()),
            source_document_type: Some("motion".to_string()),
            line_start: Some(42),
            line_end: Some(45),
            page_number: Some(3),
            category: ClaimCategory::Fraud,
            claim_type: Some(ClaimType::FactualEvent),
            severity: Some(8),
            status: ClaimStatus::Open,
            amount: Some("$50,000".to_string()),
            event_date: Some("2023-05-15".to_string()),
            created_at: Utc::now(),
            updated_at: None,
            extracted_at: Some(Utc::now()),
            extraction_model: Some("claude-opus-4.5".to_string()),
        }
    }

    #[test]
    fn test_claim_serialization_roundtrip() {
        let claim = make_test_claim();

        // Serialize to JSON
        let json = serde_json::to_string(&claim).expect("Failed to serialize Claim");

        // Deserialize back
        let restored: Claim =
            serde_json::from_str(&json).expect("Failed to deserialize Claim");

        // Verify key fields match
        assert_eq!(restored.id, claim.id);
        assert_eq!(restored.quote, claim.quote);
        assert_eq!(restored.category, claim.category);
        assert_eq!(restored.status, claim.status);
    }

    #[test]
    fn test_claim_category_serializes_snake_case() {
        // Test that BreachOfFiduciaryDuty becomes "breach_of_fiduciary_duty"
        let category = ClaimCategory::BreachOfFiduciaryDuty;
        let json = serde_json::to_string(&category).expect("Failed to serialize");

        assert_eq!(json, "\"breach_of_fiduciary_duty\"");

        // Test deserialization
        let restored: ClaimCategory =
            serde_json::from_str("\"breach_of_fiduciary_duty\"").expect("Failed to deserialize");
        assert_eq!(restored, ClaimCategory::BreachOfFiduciaryDuty);
    }

    #[test]
    fn test_claim_status_default_is_open() {
        let status = ClaimStatus::default();
        assert_eq!(status, ClaimStatus::Open);
    }

    #[test]
    fn test_claim_optional_fields_skip_null() {
        // Create claim with None fields
        let claim = Claim {
            id: "claim-002".to_string(),
            quote: "Test quote".to_string(),
            title: None, // Should be skipped
            description: None, // Should be skipped
            source_document_id: "doc-456".to_string(),
            source_document_title: None,
            source_document_type: None,
            line_start: None,
            line_end: None,
            page_number: None,
            category: ClaimCategory::Other,
            claim_type: None,
            severity: None,
            status: ClaimStatus::Open,
            amount: None,
            event_date: None,
            created_at: Utc::now(),
            updated_at: None,
            extracted_at: None,
            extraction_model: None,
        };

        let json = serde_json::to_string(&claim).expect("Failed to serialize");

        // Verify None fields are not present in JSON
        assert!(!json.contains("\"title\""));
        assert!(!json.contains("\"description\""));
        assert!(!json.contains("\"severity\""));
        assert!(!json.contains("\"amount\""));

        // Required fields should be present
        assert!(json.contains("\"id\""));
        assert!(json.contains("\"quote\""));
        assert!(json.contains("\"category\""));
    }

    #[test]
    fn test_claim_deserialize_from_extraction_json() {
        // Sample JSON matching extraction output format
        let json = r#"{
            "id": "claim-ext-001",
            "quote": "Defendant converted plaintiff's property for personal use.",
            "title": "Property Conversion",
            "source_document_id": "doc-motion-001",
            "source_document_title": "Motion for Default Judgment",
            "source_document_type": "motion",
            "line_start": 15,
            "line_end": 18,
            "page_number": 2,
            "category": "conversion",
            "claim_type": "factual_event",
            "severity": 7,
            "status": "open",
            "amount": "$25,000",
            "event_date": "2022-11-20",
            "created_at": "2024-01-15T10:30:00Z",
            "extracted_at": "2024-01-15T10:30:00Z",
            "extraction_model": "claude-opus-4.5"
        }"#;

        let claim: Claim = serde_json::from_str(json).expect("Failed to parse extraction JSON");

        assert_eq!(claim.id, "claim-ext-001");
        assert_eq!(claim.category, ClaimCategory::Conversion);
        assert_eq!(claim.claim_type, Some(ClaimType::FactualEvent));
        assert_eq!(claim.severity, Some(7));
        assert_eq!(claim.status, ClaimStatus::Open);
        assert_eq!(claim.line_start, Some(15));
    }
}
