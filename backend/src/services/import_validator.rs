//! JSON schema validation for claims import (Stage 1).
//!
//! This module handles file-level validation:
//! - Valid JSON syntax
//! - Required top-level fields present
//! - Schema version compatible
//!
//! Individual claim field validation is handled separately in T5.2.3.

use crate::models::import::{
    ImportRequest, ValidationError, ValidationErrorType, ValidationResult,
};

/// The currently supported schema version.
const SUPPORTED_SCHEMA_VERSION: &str = "2.1";

/// Validate raw JSON string and parse into ImportRequest.
///
/// # Arguments
/// * `json_str` - Raw JSON string from file upload
///
/// # Returns
/// * `Ok(ImportRequest)` - Successfully parsed and validated request
/// * `Err(ValidationResult)` - Validation failed with error details
pub fn validate_json(json_str: &str) -> Result<ImportRequest, ValidationResult> {
    // Step 1: Parse JSON syntax
    let request: ImportRequest = match serde_json::from_str(json_str) {
        Ok(req) => req,
        Err(e) => {
            // Return a ValidationResult with the parse error
            return Err(make_validation_result(
                "",
                vec![make_error(
                    "json",
                    ValidationErrorType::InvalidJson,
                    &format!("Invalid JSON: {} at line {}, column {}", e, e.line(), e.column()),
                )],
            ));
        }
    };

    // Step 2: Validate schema version
    if let Err(error) = validate_schema_version(&request.schema_version) {
        return Err(make_validation_result(
            &request.source_document.title,
            vec![error],
        ));
    }

    // Step 3: Validate structure (collect all errors)
    let errors = validate_structure(&request);
    if !errors.is_empty() {
        return Err(make_validation_result(&request.source_document.title, errors));
    }

    Ok(request)
}

/// Check schema version is compatible.
///
/// Currently only "2.1" is supported.
fn validate_schema_version(version: &str) -> Result<(), ValidationError> {
    if version != SUPPORTED_SCHEMA_VERSION {
        return Err(make_error(
            "schema_version",
            ValidationErrorType::SchemaVersionMismatch,
            &format!("Unsupported schema version: {version}, expected {SUPPORTED_SCHEMA_VERSION}"),
        ));
    }
    Ok(())
}

/// Check required top-level sections exist and are valid.
///
/// Returns a vector of all validation errors found.
fn validate_structure(request: &ImportRequest) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Check claims array is not empty
    if request.claims.is_empty() {
        errors.push(make_error(
            "claims",
            ValidationErrorType::InvalidValue,
            "claims array cannot be empty",
        ));
    }

    // Check source_document has required fields
    if request.source_document.id.is_empty() {
        errors.push(make_error(
            "source_document.id",
            ValidationErrorType::MissingField,
            "source_document.id is required",
        ));
    }
    if request.source_document.title.is_empty() {
        errors.push(make_error(
            "source_document.title",
            ValidationErrorType::MissingField,
            "source_document.title is required",
        ));
    }

    // Check case has required fields
    if request.case.id.is_empty() {
        errors.push(make_error(
            "case.id",
            ValidationErrorType::MissingField,
            "case.id is required",
        ));
    }
    if request.case.name.is_empty() {
        errors.push(make_error(
            "case.name",
            ValidationErrorType::MissingField,
            "case.name is required",
        ));
    }

    errors
}

/// Helper to create a ValidationError.
fn make_error(field: &str, error_type: ValidationErrorType, message: &str) -> ValidationError {
    ValidationError {
        claim_id: None,
        field: field.to_string(),
        error_type,
        message: message.to_string(),
    }
}

/// Helper to create a ValidationResult for errors.
fn make_validation_result(document_title: &str, errors: Vec<ValidationError>) -> ValidationResult {
    ValidationResult {
        valid: false,
        claim_count: 0,
        document_title: document_title.to_string(),
        errors,
        warnings: Vec::new(),
    }
}

// =============================================================================
// UNIT TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a valid JSON string for testing.
    fn valid_json() -> String {
        r#"{
            "schema_version": "2.1",
            "extraction_metadata": {
                "extracted_at": "2025-12-20T10:00:00Z",
                "extraction_model": "claude-3-opus"
            },
            "source_document": {
                "id": "doc-001",
                "title": "Motion for Default",
                "doc_type": "motion"
            },
            "case": {
                "id": "case-001",
                "name": "Awad v. CFS"
            },
            "parties": {
                "plaintiffs": [{"id": "p1", "name": "Marie Awad", "role": "plaintiff"}],
                "defendants": [{"id": "d1", "name": "CFS", "role": "defendant"}]
            },
            "claims": [{
                "id": "CLAIM-001",
                "category": "fraud",
                "quote": "The defendant misrepresented facts.",
                "source": {"document_id": "doc-001"},
                "made_by": "plaintiff",
                "against": ["defendant"]
            }]
        }"#.to_string()
    }

    #[test]
    fn test_validate_json_valid_input() {
        let json = valid_json();
        let result = validate_json(&json);
        assert!(result.is_ok(), "Expected Ok, got {:?}", result.err());
        let request = result.unwrap();
        assert_eq!(request.schema_version, "2.1");
        assert_eq!(request.claims.len(), 1);
    }

    #[test]
    fn test_validate_json_invalid_syntax() {
        let json = r#"{ "schema_version": "2.1", invalid }"#;
        let result = validate_json(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(!err.valid);
        assert_eq!(err.errors.len(), 1);
        assert_eq!(err.errors[0].error_type, ValidationErrorType::InvalidJson);
        assert!(err.errors[0].message.contains("Invalid JSON"));
    }

    #[test]
    fn test_validate_json_missing_schema_version() {
        // serde will fail to parse if schema_version is missing (it's required in the struct)
        let json = r#"{
            "extraction_metadata": {"extracted_at": "2025-12-20", "extraction_model": "claude"},
            "source_document": {"id": "d1", "title": "Doc", "doc_type": "motion"},
            "case": {"id": "c1", "name": "Case"},
            "parties": {"plaintiffs": [], "defendants": []},
            "claims": []
        }"#;
        let result = validate_json(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.errors[0].error_type, ValidationErrorType::InvalidJson);
        assert!(err.errors[0].message.contains("schema_version"));
    }

    #[test]
    fn test_validate_json_unsupported_version() {
        let json = r#"{
            "schema_version": "1.0",
            "extraction_metadata": {"extracted_at": "2025-12-20", "extraction_model": "claude"},
            "source_document": {"id": "d1", "title": "Doc", "doc_type": "motion"},
            "case": {"id": "c1", "name": "Case"},
            "parties": {"plaintiffs": [], "defendants": []},
            "claims": [{"id": "c1", "category": "fraud", "quote": "x", "source": {"document_id": "d1"}, "made_by": "p", "against": ["d"]}]
        }"#;
        let result = validate_json(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.errors[0].error_type, ValidationErrorType::SchemaVersionMismatch);
        assert!(err.errors[0].message.contains("1.0"));
        assert!(err.errors[0].message.contains("2.1"));
    }

    #[test]
    fn test_validate_json_empty_claims() {
        let json = r#"{
            "schema_version": "2.1",
            "extraction_metadata": {"extracted_at": "2025-12-20", "extraction_model": "claude"},
            "source_document": {"id": "d1", "title": "Doc", "doc_type": "motion"},
            "case": {"id": "c1", "name": "Case"},
            "parties": {"plaintiffs": [], "defendants": []},
            "claims": []
        }"#;
        let result = validate_json(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.errors[0].error_type, ValidationErrorType::InvalidValue);
        assert!(err.errors[0].message.contains("empty"));
    }

    #[test]
    fn test_validate_json_missing_source_document() {
        // source_document with empty id/title
        let json = r#"{
            "schema_version": "2.1",
            "extraction_metadata": {"extracted_at": "2025-12-20", "extraction_model": "claude"},
            "source_document": {"id": "", "title": "", "doc_type": "motion"},
            "case": {"id": "c1", "name": "Case"},
            "parties": {"plaintiffs": [], "defendants": []},
            "claims": [{"id": "c1", "category": "fraud", "quote": "x", "source": {"document_id": "d1"}, "made_by": "p", "against": ["d"]}]
        }"#;
        let result = validate_json(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Should have errors for both source_document.id and source_document.title
        let fields: Vec<&str> = err.errors.iter().map(|e| e.field.as_str()).collect();
        assert!(fields.contains(&"source_document.id"));
        assert!(fields.contains(&"source_document.title"));
    }

    #[test]
    fn test_validate_json_missing_case() {
        // case with empty id/name
        let json = r#"{
            "schema_version": "2.1",
            "extraction_metadata": {"extracted_at": "2025-12-20", "extraction_model": "claude"},
            "source_document": {"id": "d1", "title": "Doc", "doc_type": "motion"},
            "case": {"id": "", "name": ""},
            "parties": {"plaintiffs": [], "defendants": []},
            "claims": [{"id": "c1", "category": "fraud", "quote": "x", "source": {"document_id": "d1"}, "made_by": "p", "against": ["d"]}]
        }"#;
        let result = validate_json(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let fields: Vec<&str> = err.errors.iter().map(|e| e.field.as_str()).collect();
        assert!(fields.contains(&"case.id"));
        assert!(fields.contains(&"case.name"));
    }
}
