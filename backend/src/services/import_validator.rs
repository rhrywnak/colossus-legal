//! File-level validation for claims import (Stage 1).
//!
//! Validates JSON syntax, schema version, and top-level structure.
//! Delegates claim-level validation to claim_validator module.

use crate::models::import::{ImportRequest, ValidationError, ValidationErrorType, ValidationResult};
use crate::services::claim_validator::validate_claims;

const SUPPORTED_SCHEMA_VERSION: &str = "2.1";

/// Validate raw JSON string and parse into ImportRequest.
///
/// Performs validation in order:
/// 1. JSON syntax parsing
/// 2. Schema version check
/// 3. Top-level structure validation
/// 4. Claim-level validation (via claim_validator)
pub fn validate_json(json_str: &str) -> Result<ImportRequest, ValidationResult> {
    // Step 1: Parse JSON syntax
    let request: ImportRequest = match serde_json::from_str(json_str) {
        Ok(req) => req,
        Err(e) => {
            return Err(make_result("", vec![make_error(
                "json", ValidationErrorType::InvalidJson,
                &format!("Invalid JSON: {} at line {}, column {}", e, e.line(), e.column()),
            )]));
        }
    };

    // Step 2: Validate schema version
    if let Err(error) = validate_schema_version(&request.schema_version) {
        return Err(make_result(&request.source_document.title, vec![error]));
    }

    // Step 3: Validate structure
    let mut errors = validate_structure(&request);

    // Step 4: Validate claims (only if structure is valid)
    if errors.is_empty() {
        errors.extend(validate_claims(&request.claims));
    }

    if !errors.is_empty() {
        return Err(make_result(&request.source_document.title, errors));
    }

    Ok(request)
}

fn validate_schema_version(version: &str) -> Result<(), ValidationError> {
    if version != SUPPORTED_SCHEMA_VERSION {
        return Err(make_error(
            "schema_version", ValidationErrorType::SchemaVersionMismatch,
            &format!("Unsupported schema version: {version}, expected {SUPPORTED_SCHEMA_VERSION}"),
        ));
    }
    Ok(())
}

fn validate_structure(request: &ImportRequest) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if request.claims.is_empty() {
        errors.push(make_error("claims", ValidationErrorType::InvalidValue, "claims array cannot be empty"));
    }
    if request.source_document.id.is_empty() {
        errors.push(make_error("source_document.id", ValidationErrorType::MissingField, "source_document.id is required"));
    }
    if request.source_document.title.is_empty() {
        errors.push(make_error("source_document.title", ValidationErrorType::MissingField, "source_document.title is required"));
    }
    if request.case.id.is_empty() {
        errors.push(make_error("case.id", ValidationErrorType::MissingField, "case.id is required"));
    }
    if request.case.name.is_empty() {
        errors.push(make_error("case.name", ValidationErrorType::MissingField, "case.name is required"));
    }
    errors
}

fn make_error(field: &str, error_type: ValidationErrorType, message: &str) -> ValidationError {
    ValidationError {
        claim_id: None,
        field: field.to_string(),
        error_type,
        message: message.to_string(),
    }
}

fn make_result(doc_title: &str, errors: Vec<ValidationError>) -> ValidationResult {
    ValidationResult { valid: false, claim_count: 0, document_title: doc_title.to_string(), errors, warnings: Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_json() -> &'static str {
        r#"{"schema_version":"2.1","extraction_metadata":{"extracted_at":"2025-12-20","extraction_model":"claude"},"source_document":{"id":"d1","title":"Doc","doc_type":"motion"},"case":{"id":"c1","name":"Case"},"parties":{"plaintiffs":[{"id":"p1","name":"P","role":"plaintiff"}],"defendants":[{"id":"d1","name":"D","role":"defendant"}]},"claims":[{"id":"CLAIM-001","category":"fraud","quote":"Test quote.","source":{"document_id":"d1"},"made_by":"p1","against":["d1"]}]}"#
    }

    #[test]
    fn test_validate_json_valid_input() {
        let result = validate_json(valid_json());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result.err());
    }

    #[test]
    fn test_validate_json_invalid_syntax() {
        let result = validate_json(r#"{ invalid }"#);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().errors[0].error_type, ValidationErrorType::InvalidJson);
    }

    #[test]
    fn test_validate_json_missing_schema_version() {
        let json = r#"{"extraction_metadata":{"extracted_at":"x","extraction_model":"x"},"source_document":{"id":"d","title":"D","doc_type":"m"},"case":{"id":"c","name":"C"},"parties":{"plaintiffs":[],"defendants":[]},"claims":[]}"#;
        let err = validate_json(json).unwrap_err();
        assert_eq!(err.errors[0].error_type, ValidationErrorType::InvalidJson);
    }

    #[test]
    fn test_validate_json_unsupported_version() {
        let json = r#"{"schema_version":"1.0","extraction_metadata":{"extracted_at":"x","extraction_model":"x"},"source_document":{"id":"d","title":"D","doc_type":"m"},"case":{"id":"c","name":"C"},"parties":{"plaintiffs":[],"defendants":[]},"claims":[{"id":"c1","category":"fraud","quote":"x","source":{"document_id":"d"},"made_by":"p","against":["d"]}]}"#;
        let err = validate_json(json).unwrap_err();
        assert_eq!(err.errors[0].error_type, ValidationErrorType::SchemaVersionMismatch);
    }

    #[test]
    fn test_validate_json_empty_claims() {
        let json = r#"{"schema_version":"2.1","extraction_metadata":{"extracted_at":"x","extraction_model":"x"},"source_document":{"id":"d","title":"D","doc_type":"m"},"case":{"id":"c","name":"C"},"parties":{"plaintiffs":[],"defendants":[]},"claims":[]}"#;
        let err = validate_json(json).unwrap_err();
        assert_eq!(err.errors[0].error_type, ValidationErrorType::InvalidValue);
    }

    #[test]
    fn test_validate_json_missing_source_document() {
        let json = r#"{"schema_version":"2.1","extraction_metadata":{"extracted_at":"x","extraction_model":"x"},"source_document":{"id":"","title":"","doc_type":"m"},"case":{"id":"c","name":"C"},"parties":{"plaintiffs":[],"defendants":[]},"claims":[{"id":"c1","category":"fraud","quote":"x","source":{"document_id":"d"},"made_by":"p","against":["d"]}]}"#;
        let err = validate_json(json).unwrap_err();
        let fields: Vec<_> = err.errors.iter().map(|e| e.field.as_str()).collect();
        assert!(fields.contains(&"source_document.id") && fields.contains(&"source_document.title"));
    }

    #[test]
    fn test_validate_json_missing_case() {
        let json = r#"{"schema_version":"2.1","extraction_metadata":{"extracted_at":"x","extraction_model":"x"},"source_document":{"id":"d","title":"D","doc_type":"m"},"case":{"id":"","name":""},"parties":{"plaintiffs":[],"defendants":[]},"claims":[{"id":"c1","category":"fraud","quote":"x","source":{"document_id":"d"},"made_by":"p","against":["d"]}]}"#;
        let err = validate_json(json).unwrap_err();
        let fields: Vec<_> = err.errors.iter().map(|e| e.field.as_str()).collect();
        assert!(fields.contains(&"case.id") && fields.contains(&"case.name"));
    }
}
