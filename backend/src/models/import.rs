//! Import DTOs for the claims import validation pipeline.
//! Wire-format structures for JSON serialization/deserialization.

use serde::{Deserialize, Serialize};

// =============================================================================
// TOP-LEVEL IMPORT REQUEST
// =============================================================================

/// Top-level structure for a claims import JSON file from Claude's extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRequest {
    pub schema_version: String,
    pub extraction_metadata: ExtractionMetadata,
    pub source_document: SourceDocument,
    pub case: CaseInfo,
    pub parties: PartiesInfo,
    pub claims: Vec<ImportClaim>,
}

/// Metadata about the extraction process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionMetadata {
    pub extracted_at: String,
    pub extraction_model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_hash: Option<String>,
}

/// Reference to the source document being imported.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDocument {
    pub id: String,
    pub title: String,
    pub doc_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub court: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filed_date: Option<String>,
}

/// Information about the case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseInfo {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub court: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case_number: Option<String>,
}

/// Information about the parties involved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartiesInfo {
    pub plaintiffs: Vec<PartyInfo>,
    pub defendants: Vec<PartyInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_parties: Option<Vec<PartyInfo>>,
}

/// Information about a single party.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyInfo {
    pub id: String,
    pub name: String,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
}

// =============================================================================
// CLAIM STRUCTURES (WIRE FORMAT)
// =============================================================================

/// A single claim in wire format. Field validation happens in T5.2.3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportClaim {
    pub id: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_type: Option<String>,
    pub quote: String,
    pub source: ClaimSource,
    pub made_by: String,
    pub against: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_refs: Option<Vec<String>>,
}

/// Source location within a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimSource {
    pub document_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_start: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_end: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<i32>,
}

// =============================================================================
// VALIDATION RESULTS
// =============================================================================

/// Result of validating an import request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub claim_count: i32,
    pub document_title: String,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

/// A validation error for a specific field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_id: Option<String>,
    pub field: String,
    pub error_type: ValidationErrorType,
    pub message: String,
}

/// Types of validation errors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationErrorType {
    MissingField,
    InvalidValue,
    DuplicateId,
    OutOfRange,
    InvalidJson,
    SchemaVersionMismatch,
}

/// A validation warning (non-fatal issue).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_id: Option<String>,
    pub field: String,
    pub message: String,
}

// =============================================================================
// IMPORT REPORT
// =============================================================================

/// Final report after an import operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReport {
    pub import_id: String,
    pub status: ImportStatus,
    pub duration_ms: i64,
    pub results: ImportResults,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

/// Status of an import operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportStatus {
    Pending,
    Validating,
    Validated,
    Importing,
    Completed,
    Failed,
    PartialSuccess,
}

/// Counts of entities created/modified during import. All fields default to 0.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ImportResults {
    #[serde(default)]
    pub cases_created: i32,
    #[serde(default)]
    pub documents_created: i32,
    #[serde(default)]
    pub persons_created: i32,
    #[serde(default)]
    pub persons_existing: i32,
    #[serde(default)]
    pub claims_created: i32,
    #[serde(default)]
    pub claims_skipped: i32,
    #[serde(default)]
    pub evidence_created: i32,
    #[serde(default)]
    pub relationships_created: i32,
}

// =============================================================================
// UNIT TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_request_deserialize_valid() {
        let json = r#"{"schema_version":"2.1","extraction_metadata":{"extracted_at":"2025-12-20T10:00:00Z","extraction_model":"claude-3-opus"},"source_document":{"id":"doc-001","title":"Motion for Default","doc_type":"motion"},"case":{"id":"case-001","name":"Awad v. CFS"},"parties":{"plaintiffs":[{"id":"p1","name":"Marie Awad","role":"plaintiff"}],"defendants":[{"id":"d1","name":"CFS","role":"defendant"}]},"claims":[]}"#;
        let request: ImportRequest = serde_json::from_str(json).expect("parse failed");
        assert_eq!(request.schema_version, "2.1");
        assert_eq!(request.source_document.title, "Motion for Default");
        assert_eq!(request.parties.plaintiffs.len(), 1);
    }

    #[test]
    fn test_import_claim_deserialize_with_optionals() {
        let json = r#"{"id":"CLAIM-001","category":"fraud","severity":8,"claim_type":"factual","quote":"The defendant misrepresented facts.","source":{"document_id":"doc-001","document_title":"Motion","line_start":100,"line_end":105,"page_number":5},"made_by":"plaintiff","against":["defendant-1","defendant-2"],"amount":"$50,000","date_reference":"2024-06-15","evidence_refs":["Exhibit 1","Exhibit 5"]}"#;
        let claim: ImportClaim = serde_json::from_str(json).expect("parse failed");
        assert_eq!(claim.id, "CLAIM-001");
        assert_eq!(claim.severity, Some(8));
        assert_eq!(claim.against.len(), 2);
        assert_eq!(claim.evidence_refs, Some(vec!["Exhibit 1".to_string(), "Exhibit 5".to_string()]));
    }

    #[test]
    fn test_import_claim_deserialize_minimal() {
        let json = r#"{"id":"CLAIM-002","category":"conversion","quote":"Property taken.","source":{"document_id":"doc-001"},"made_by":"plaintiff","against":["defendant"]}"#;
        let claim: ImportClaim = serde_json::from_str(json).expect("parse failed");
        assert_eq!(claim.id, "CLAIM-002");
        assert_eq!(claim.severity, None);
        assert_eq!(claim.evidence_refs, None);
    }

    #[test]
    fn test_validation_error_serialize() {
        let error = ValidationError {
            claim_id: Some("CLAIM-001".to_string()),
            field: "severity".to_string(),
            error_type: ValidationErrorType::OutOfRange,
            message: "Severity must be 1-10".to_string(),
        };
        let json = serde_json::to_string(&error).expect("serialize failed");
        assert!(json.contains("\"error_type\":\"out_of_range\""));
        assert!(json.contains("\"claim_id\":\"CLAIM-001\""));
    }

    #[test]
    fn test_import_results_default() {
        let results = ImportResults::default();
        assert_eq!(results.cases_created, 0);
        assert_eq!(results.documents_created, 0);
        assert_eq!(results.persons_created, 0);
        assert_eq!(results.persons_existing, 0);
        assert_eq!(results.claims_created, 0);
        assert_eq!(results.claims_skipped, 0);
        assert_eq!(results.evidence_created, 0);
        assert_eq!(results.relationships_created, 0);
    }
}
