//! Evidence model v2 — Represents exhibits, documents, and other evidence.
//!
//! This module defines the Evidence struct and EvidenceKind enum.
//! Evidence can be testimonial, documentary, physical, digital, etc.

use chrono::{DateTime, NaiveDate, Utc};
use neo4rs::{DeError, Node};
use serde::{Deserialize, Serialize};

// =============================================================================
// ENUMS
// =============================================================================

/// Types of evidence in legal proceedings.
/// Uses snake_case serialization for JSON compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Testimonial,    // Witness statements, depositions
    Documentary,    // Documents, records, contracts
    Physical,       // Tangible objects, photos
    Demonstrative,  // Charts, diagrams, models
    Digital,        // Electronic records, emails
    Expert,         // Expert opinions, reports
    Circumstantial, // Indirect evidence
    Direct,         // Direct evidence of fact
    Other,
}

// =============================================================================
// DISPLAY IMPLEMENTATION
// =============================================================================

/// Display impl for EvidenceKind — returns human-readable string.
impl std::fmt::Display for EvidenceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Testimonial => "Testimonial",
            Self::Documentary => "Documentary",
            Self::Physical => "Physical",
            Self::Demonstrative => "Demonstrative",
            Self::Digital => "Digital",
            Self::Expert => "Expert",
            Self::Circumstantial => "Circumstantial",
            Self::Direct => "Direct",
            Self::Other => "Other",
        };
        write!(f, "{s}")
    }
}

// =============================================================================
// EVIDENCE STRUCT
// =============================================================================

/// A piece of evidence in a legal case.
///
/// Evidence can be exhibits, documents, statements, physical items, or other
/// materials used to prove or disprove facts in legal proceedings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    // --- Identity ---
    pub id: String,

    /// Exhibit number (e.g., "Exhibit 1", "Exhibit 36")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exhibit_number: Option<String>,

    /// Short title for the evidence
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    // --- Content ---
    /// Description of the evidence (required)
    pub description: String,

    /// Detailed summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    // --- Classification ---
    /// Type of evidence
    pub kind: EvidenceKind,

    // --- Weight/Importance ---
    /// Subjective importance rating (1-10)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<i32>,

    // --- Relationships ---
    /// Link to associated Claim (preserved from v1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_id: Option<String>,

    /// Legacy document link (preserved from v1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,

    /// Document containing this evidence (v2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_document: Option<String>,

    // --- Flags (preserved from v1) ---
    /// Is this supporting evidence for the claim?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_supporting: Option<bool>,

    // --- Dates (preserved from v1) ---
    /// When the evidence was collected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collected_on: Option<NaiveDate>,

    // --- Timestamps ---
    pub created_at: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// Error type for converting Neo4j Node to Evidence.
#[derive(Debug)]
pub enum EvidenceConversionError {
    MissingField(&'static str),
    Value(DeError),
    Neo4j(neo4rs::Error),
}

impl From<neo4rs::Error> for EvidenceConversionError {
    fn from(value: neo4rs::Error) -> Self {
        EvidenceConversionError::Neo4j(value)
    }
}

impl From<DeError> for EvidenceConversionError {
    fn from(value: DeError) -> Self {
        EvidenceConversionError::Value(value)
    }
}

// =============================================================================
// NEO4J CONVERSION
// =============================================================================

/// Parse a kind string from Neo4j into EvidenceKind enum.
/// Returns EvidenceKind::Other for unknown values.
fn parse_kind(s: &str) -> EvidenceKind {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .unwrap_or(EvidenceKind::Other)
}

/// Parse created_at which may be stored as String or DateTime in Neo4j.
/// Returns current time if parsing fails.
fn parse_created_at(node: &Node) -> DateTime<Utc> {
    // Try as DateTime first
    if let Ok(dt) = node.get::<DateTime<Utc>>("created_at") {
        return dt;
    }

    // Try as String (ISO-8601 / RFC-3339)
    if let Ok(s) = node.get::<String>("created_at") {
        if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
            return dt.with_timezone(&Utc);
        }
    }

    // Fallback to now
    Utc::now()
}

impl TryFrom<Node> for Evidence {
    type Error = EvidenceConversionError;

    fn try_from(node: Node) -> Result<Self, Self::Error> {
        // Required fields
        let id: String = node.get("id").map_err(EvidenceConversionError::from)?;

        // description: was Option in v1, now required — use empty string as fallback
        let description: String = node
            .get("description")
            .unwrap_or_else(|_| String::new());

        // kind: check both "kind" (v2) and "evidence_type" (v1) for backward compatibility
        let kind_str: String = node
            .get("kind")
            .or_else(|_| node.get("evidence_type"))
            .unwrap_or_else(|_| "other".to_string());
        let kind = parse_kind(&kind_str);

        // created_at: handle both DateTime and String formats
        let created_at = parse_created_at(&node);

        // Optional fields
        let exhibit_number: Option<String> = node.get("exhibit_number").ok();
        let title: Option<String> = node.get("title").ok();
        let summary: Option<String> = node.get("summary").ok();
        let weight: Option<i32> = node.get("weight").ok();
        let claim_id: Option<String> = node.get("claim_id").ok();
        let document_id: Option<String> = node.get("document_id").ok();
        let source_document: Option<String> = node.get("source_document").ok();
        let is_supporting: Option<bool> = node.get("is_supporting").ok();
        let collected_on: Option<NaiveDate> = node.get("collected_on").ok();
        let updated_at: Option<DateTime<Utc>> = node.get("updated_at").ok();

        Ok(Self {
            id,
            exhibit_number,
            title,
            description,
            summary,
            kind,
            weight,
            claim_id,
            document_id,
            source_document,
            is_supporting,
            collected_on,
            created_at,
            updated_at,
        })
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a minimal valid Evidence for testing.
    fn make_test_evidence() -> Evidence {
        Evidence {
            id: "evidence-001".to_string(),
            exhibit_number: Some("Exhibit 36".to_string()),
            title: Some("Bank Statement".to_string()),
            description: "Monthly bank statement showing unauthorized withdrawals".to_string(),
            summary: Some("Statement from First National Bank for account ending 1234".to_string()),
            kind: EvidenceKind::Documentary,
            weight: Some(8),
            claim_id: Some("claim-001".to_string()),
            document_id: None,
            source_document: Some("doc-bank-stmt-001".to_string()),
            is_supporting: Some(true),
            collected_on: None,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    #[test]
    fn test_evidence_serialization_roundtrip() {
        let evidence = make_test_evidence();

        // Serialize to JSON
        let json = serde_json::to_string(&evidence).expect("Failed to serialize Evidence");

        // Deserialize back
        let restored: Evidence =
            serde_json::from_str(&json).expect("Failed to deserialize Evidence");

        // Verify key fields match
        assert_eq!(restored.id, evidence.id);
        assert_eq!(restored.description, evidence.description);
        assert_eq!(restored.kind, evidence.kind);
    }

    #[test]
    fn test_evidence_kind_serializes_snake_case() {
        // Test Circumstantial becomes "circumstantial"
        let kind = EvidenceKind::Circumstantial;
        let json = serde_json::to_string(&kind).expect("Failed to serialize");

        assert_eq!(json, "\"circumstantial\"");

        // Test deserialization
        let restored: EvidenceKind =
            serde_json::from_str("\"circumstantial\"").expect("Failed to deserialize");
        assert_eq!(restored, EvidenceKind::Circumstantial);

        // Test Demonstrative
        let demo = EvidenceKind::Demonstrative;
        let json2 = serde_json::to_string(&demo).expect("Failed to serialize");
        assert_eq!(json2, "\"demonstrative\"");
    }

    #[test]
    fn test_evidence_optional_fields_skip_null() {
        // Create evidence with minimal fields
        let evidence = Evidence {
            id: "evidence-002".to_string(),
            exhibit_number: None,
            title: None,
            description: "Minimal evidence".to_string(),
            summary: None,
            kind: EvidenceKind::Other,
            weight: None,
            claim_id: None,
            document_id: None,
            source_document: None,
            is_supporting: None,
            collected_on: None,
            created_at: Utc::now(),
            updated_at: None,
        };

        let json = serde_json::to_string(&evidence).expect("Failed to serialize");

        // Verify None fields are not present
        assert!(!json.contains("\"exhibit_number\""));
        assert!(!json.contains("\"title\""));
        assert!(!json.contains("\"summary\""));
        assert!(!json.contains("\"weight\""));
        assert!(!json.contains("\"updated_at\""));

        // Required fields should be present
        assert!(json.contains("\"id\""));
        assert!(json.contains("\"description\""));
        assert!(json.contains("\"kind\""));
        assert!(json.contains("\"created_at\""));
    }

    #[test]
    fn test_evidence_kind_display() {
        assert_eq!(EvidenceKind::Testimonial.to_string(), "Testimonial");
        assert_eq!(EvidenceKind::Documentary.to_string(), "Documentary");
        assert_eq!(EvidenceKind::Circumstantial.to_string(), "Circumstantial");
        assert_eq!(EvidenceKind::Other.to_string(), "Other");
    }

    #[test]
    fn test_parse_kind_unknown_returns_other() {
        // Unknown values should return Other
        assert_eq!(parse_kind("unknown_kind"), EvidenceKind::Other);
        assert_eq!(parse_kind("proof"), EvidenceKind::Other);
        assert_eq!(parse_kind(""), EvidenceKind::Other);

        // Known values should parse correctly
        assert_eq!(parse_kind("testimonial"), EvidenceKind::Testimonial);
        assert_eq!(parse_kind("documentary"), EvidenceKind::Documentary);
        assert_eq!(parse_kind("circumstantial"), EvidenceKind::Circumstantial);
    }
}
