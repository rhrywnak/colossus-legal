//! Document model v2 — Supports legal document metadata and classification.
//!
//! This module defines the Document struct and DocumentType enum.
//! Documents are legal filings, court records, evidence, and other materials.

use chrono::{DateTime, NaiveDate, Utc};
use neo4rs::{DeError, Node};
use serde::{Deserialize, Serialize};

// =============================================================================
// ENUMS
// =============================================================================

/// Types of legal documents in the system.
/// Uses snake_case serialization for JSON compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    // Court Filings
    Complaint,
    Answer,
    Motion,
    Brief,
    Petition,
    Response,
    Reply,

    // Court Records
    Order,
    Ruling,
    Judgment,
    Opinion,
    Transcript,

    // Evidence
    Exhibit,
    Affidavit,
    Declaration,
    Deposition,

    // Discovery
    Interrogatories,
    Admissions,
    ProductionRequest,
    ProductionResponse,

    // Communications
    Letter,
    Email,
    Memorandum,

    // External
    Statute,
    Regulation,
    Caselaw,

    // Financial/Other
    BillingStatement,
    BankRecord,
    Contract,
    Photo,
    Video,
    Audio,
    Other,
}

// =============================================================================
// DISPLAY IMPLEMENTATION
// =============================================================================

/// Display impl for DocumentType — returns snake_case string for v1 API compatibility.
impl std::fmt::Display for DocumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Use serde to get the snake_case representation
        let s = match self {
            Self::Complaint => "complaint",
            Self::Answer => "answer",
            Self::Motion => "motion",
            Self::Brief => "brief",
            Self::Petition => "petition",
            Self::Response => "response",
            Self::Reply => "reply",
            Self::Order => "order",
            Self::Ruling => "ruling",
            Self::Judgment => "judgment",
            Self::Opinion => "opinion",
            Self::Transcript => "transcript",
            Self::Exhibit => "exhibit",
            Self::Affidavit => "affidavit",
            Self::Declaration => "declaration",
            Self::Deposition => "deposition",
            Self::Interrogatories => "interrogatories",
            Self::Admissions => "admissions",
            Self::ProductionRequest => "production_request",
            Self::ProductionResponse => "production_response",
            Self::Letter => "letter",
            Self::Email => "email",
            Self::Memorandum => "memorandum",
            Self::Statute => "statute",
            Self::Regulation => "regulation",
            Self::Caselaw => "caselaw",
            Self::BillingStatement => "billing_statement",
            Self::BankRecord => "bank_record",
            Self::Contract => "contract",
            Self::Photo => "photo",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Other => "other",
        };
        write!(f, "{s}")
    }
}

// =============================================================================
// DOCUMENT STRUCT
// =============================================================================

/// A legal document in the system.
///
/// Documents can be court filings, evidence, communications, or other materials.
/// They are stored in Neo4j and may have claims extracted from them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    // --- Identity ---
    pub id: String,
    pub title: String,

    // --- Classification ---
    /// Type of document (motion, complaint, exhibit, etc.)
    pub doc_type: DocumentType,

    // --- File information ---
    /// Original filename
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,

    /// Storage path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,

    // --- Court information ---
    /// Court name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub court: Option<String>,

    /// When filed (ISO date string for flexibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filed_date: Option<String>,

    // --- Content ---
    /// Document description or summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Number of pages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_count: Option<i32>,

    // --- Relationships (preserved from v1) ---
    /// Related claim ID (for evidence linking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_claim_id: Option<String>,

    /// External source URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,

    // --- Timestamps ---
    /// When the document record was created
    pub created_at: DateTime<Utc>,

    /// When the file was uploaded (preserved from v1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uploaded_at: Option<NaiveDate>,

    /// When the document was ingested/processed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ingested_at: Option<DateTime<Utc>>,

    /// When claims were extracted from this document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extracted_at: Option<DateTime<Utc>>,
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// Error type for converting Neo4j Node to Document.
#[derive(Debug)]
pub enum DocumentConversionError {
    MissingField(&'static str),
    Value(DeError),
    Neo4j(neo4rs::Error),
}

impl From<neo4rs::Error> for DocumentConversionError {
    fn from(value: neo4rs::Error) -> Self {
        DocumentConversionError::Neo4j(value)
    }
}

impl From<DeError> for DocumentConversionError {
    fn from(value: DeError) -> Self {
        DocumentConversionError::Value(value)
    }
}

// =============================================================================
// NEO4J CONVERSION
// =============================================================================

/// Parse a doc_type string from Neo4j into DocumentType enum.
/// Returns DocumentType::Other for unknown values.
fn parse_doc_type(s: &str) -> DocumentType {
    // Try to deserialize the string as a DocumentType
    // serde expects a JSON string, so we wrap it in quotes
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .unwrap_or(DocumentType::Other)
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

impl TryFrom<Node> for Document {
    type Error = DocumentConversionError;

    fn try_from(node: Node) -> Result<Self, Self::Error> {
        // Required fields
        let id: String = node.get("id").map_err(DocumentConversionError::from)?;
        let title: String = node
            .get("title")
            .map_err(|_| DocumentConversionError::MissingField("title"))?;

        // doc_type: parse from string with fallback to Other
        let doc_type_str: String = node.get("doc_type").unwrap_or_else(|_| "other".to_string());
        let doc_type = parse_doc_type(&doc_type_str);

        // created_at: handle both DateTime and String formats
        let created_at = parse_created_at(&node);

        // Optional fields
        let file_name: Option<String> = node.get("file_name").ok();
        let file_path: Option<String> = node.get("file_path").ok();
        let court: Option<String> = node.get("court").ok();
        let filed_date: Option<String> = node.get("filed_date").ok();
        let description: Option<String> = node.get("description").ok();
        let page_count: Option<i32> = node.get("page_count").ok();
        let related_claim_id: Option<String> = node.get("related_claim_id").ok();
        let source_url: Option<String> = node.get("source_url").ok();
        let uploaded_at: Option<NaiveDate> = node.get("uploaded_at").ok();
        let ingested_at: Option<DateTime<Utc>> = node.get("ingested_at").ok();
        let extracted_at: Option<DateTime<Utc>> = node.get("extracted_at").ok();

        Ok(Self {
            id,
            title,
            doc_type,
            file_name,
            file_path,
            court,
            filed_date,
            description,
            page_count,
            related_claim_id,
            source_url,
            created_at,
            uploaded_at,
            ingested_at,
            extracted_at,
        })
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a minimal valid Document for testing.
    fn make_test_document() -> Document {
        Document {
            id: "doc-001".to_string(),
            title: "Motion for Summary Judgment".to_string(),
            doc_type: DocumentType::Motion,
            file_name: Some("motion_summary.pdf".to_string()),
            file_path: Some("/documents/case-123/motion_summary.pdf".to_string()),
            court: Some("Superior Court of California".to_string()),
            filed_date: Some("2024-01-15".to_string()),
            description: Some("Plaintiff's motion for summary judgment".to_string()),
            page_count: Some(25),
            related_claim_id: None,
            source_url: None,
            created_at: Utc::now(),
            uploaded_at: None,
            ingested_at: Some(Utc::now()),
            extracted_at: None,
        }
    }

    #[test]
    fn test_document_serialization_roundtrip() {
        let doc = make_test_document();

        // Serialize to JSON
        let json = serde_json::to_string(&doc).expect("Failed to serialize Document");

        // Deserialize back
        let restored: Document =
            serde_json::from_str(&json).expect("Failed to deserialize Document");

        // Verify key fields match
        assert_eq!(restored.id, doc.id);
        assert_eq!(restored.title, doc.title);
        assert_eq!(restored.doc_type, doc.doc_type);
    }

    #[test]
    fn test_document_type_serializes_snake_case() {
        // Test ProductionRequest becomes "production_request"
        let doc_type = DocumentType::ProductionRequest;
        let json = serde_json::to_string(&doc_type).expect("Failed to serialize");

        assert_eq!(json, "\"production_request\"");

        // Test deserialization
        let restored: DocumentType =
            serde_json::from_str("\"production_request\"").expect("Failed to deserialize");
        assert_eq!(restored, DocumentType::ProductionRequest);

        // Test BillingStatement
        let billing = DocumentType::BillingStatement;
        let json2 = serde_json::to_string(&billing).expect("Failed to serialize");
        assert_eq!(json2, "\"billing_statement\"");
    }

    #[test]
    fn test_document_optional_fields_skip_null() {
        // Create document with minimal fields
        let doc = Document {
            id: "doc-002".to_string(),
            title: "Test Document".to_string(),
            doc_type: DocumentType::Other,
            file_name: None,
            file_path: None,
            court: None,
            filed_date: None,
            description: None,
            page_count: None,
            related_claim_id: None,
            source_url: None,
            created_at: Utc::now(),
            uploaded_at: None,
            ingested_at: None,
            extracted_at: None,
        };

        let json = serde_json::to_string(&doc).expect("Failed to serialize");

        // Verify None fields are not present
        assert!(!json.contains("\"file_name\""));
        assert!(!json.contains("\"court\""));
        assert!(!json.contains("\"page_count\""));
        assert!(!json.contains("\"ingested_at\""));

        // Required fields should be present
        assert!(json.contains("\"id\""));
        assert!(json.contains("\"title\""));
        assert!(json.contains("\"doc_type\""));
        assert!(json.contains("\"created_at\""));
    }

    #[test]
    fn test_document_type_display() {
        assert_eq!(DocumentType::Motion.to_string(), "motion");
        assert_eq!(DocumentType::ProductionRequest.to_string(), "production_request");
        assert_eq!(DocumentType::BillingStatement.to_string(), "billing_statement");
        assert_eq!(DocumentType::Other.to_string(), "other");
    }

    #[test]
    fn test_parse_doc_type_unknown_returns_other() {
        // Unknown values should return Other
        assert_eq!(parse_doc_type("unknown_type"), DocumentType::Other);
        assert_eq!(parse_doc_type("pdf"), DocumentType::Other);
        assert_eq!(parse_doc_type(""), DocumentType::Other);

        // Known values should parse correctly
        assert_eq!(parse_doc_type("motion"), DocumentType::Motion);
        assert_eq!(parse_doc_type("production_request"), DocumentType::ProductionRequest);
    }
}
