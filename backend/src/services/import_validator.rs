//! JSON schema validation for claims import (Stage 1 & 2).
//!
//! Stage 1: File-level validation (JSON syntax, schema version, structure)
//! Stage 2: Claim-level validation (required fields, enum values, ranges)

use crate::models::import::{
    ImportClaim, ImportRequest, ValidationError, ValidationErrorType, ValidationResult,
};

const SUPPORTED_SCHEMA_VERSION: &str = "2.1";

/// Valid ClaimCategory values (19 total).
const VALID_CATEGORIES: &[&str] = &[
    "conversion", "fraud", "breach_of_fiduciary_duty", "defamation", "bias",
    "discovery_obstruction", "perjury", "collusion", "financial_harm",
    "procedural_misconduct", "conflict_of_interest", "unauthorized_possession",
    "impartiality_violation", "negligence", "misrepresentation", "abuse_of_process",
    "unjust_enrichment", "breach_of_contract", "emotional_distress",
];

/// Valid ClaimType values (3 total).
const VALID_CLAIM_TYPES: &[&str] = &["factual_event", "legal_conclusion", "procedural"];

// =============================================================================
// PUBLIC API
// =============================================================================

/// Validate raw JSON string and parse into ImportRequest.
pub fn validate_json(json_str: &str) -> Result<ImportRequest, ValidationResult> {
    // Step 1: Parse JSON syntax
    let request: ImportRequest = match serde_json::from_str(json_str) {
        Ok(req) => req,
        Err(e) => {
            return Err(make_result("", vec![make_error(
                None, "json", ValidationErrorType::InvalidJson,
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

    // Step 4: Validate claims (only if structure is valid enough)
    if errors.is_empty() {
        errors.extend(validate_claims(&request.claims));
    }

    if !errors.is_empty() {
        return Err(make_result(&request.source_document.title, errors));
    }

    Ok(request)
}

/// Validate all claims in the import request.
pub fn validate_claims(claims: &[ImportClaim]) -> Vec<ValidationError> {
    claims.iter().flat_map(validate_claim).collect()
}

fn validate_schema_version(version: &str) -> Result<(), ValidationError> {
    if version != SUPPORTED_SCHEMA_VERSION {
        return Err(make_error(
            None, "schema_version", ValidationErrorType::SchemaVersionMismatch,
            &format!("Unsupported schema version: {version}, expected {SUPPORTED_SCHEMA_VERSION}"),
        ));
    }
    Ok(())
}

fn validate_structure(request: &ImportRequest) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if request.claims.is_empty() {
        errors.push(make_error(None, "claims", ValidationErrorType::InvalidValue, "claims array cannot be empty"));
    }
    if request.source_document.id.is_empty() {
        errors.push(make_error(None, "source_document.id", ValidationErrorType::MissingField, "source_document.id is required"));
    }
    if request.source_document.title.is_empty() {
        errors.push(make_error(None, "source_document.title", ValidationErrorType::MissingField, "source_document.title is required"));
    }
    if request.case.id.is_empty() {
        errors.push(make_error(None, "case.id", ValidationErrorType::MissingField, "case.id is required"));
    }
    if request.case.name.is_empty() {
        errors.push(make_error(None, "case.name", ValidationErrorType::MissingField, "case.name is required"));
    }
    errors
}

/// Validate a single claim's fields.
fn validate_claim(claim: &ImportClaim) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let id = &claim.id;

    // Required fields
    if claim.id.is_empty() {
        errors.push(make_error(Some(id), "id", ValidationErrorType::MissingField, "id is required"));
    }
    if claim.quote.is_empty() {
        errors.push(make_error(Some(id), "quote", ValidationErrorType::MissingField, "quote is required"));
    }
    if claim.made_by.is_empty() {
        errors.push(make_error(Some(id), "made_by", ValidationErrorType::MissingField, "made_by is required"));
    }
    if claim.against.is_empty() {
        errors.push(make_error(Some(id), "against", ValidationErrorType::InvalidValue, "against array cannot be empty"));
    }
    if claim.source.document_id.is_empty() {
        errors.push(make_error(Some(id), "source.document_id", ValidationErrorType::MissingField, "source.document_id is required"));
    }

    // Category validation
    if !is_valid_category(&claim.category) {
        errors.push(make_error(Some(id), "category", ValidationErrorType::InvalidValue,
            &format!("Invalid category: '{}'. Valid values: {}", claim.category, VALID_CATEGORIES.join(", "))));
    }

    // Optional claim_type validation
    if let Some(ref ct) = claim.claim_type {
        if !is_valid_claim_type(ct) {
            errors.push(make_error(Some(id), "claim_type", ValidationErrorType::InvalidValue,
                &format!("Invalid claim_type: '{}'. Valid values: {}", ct, VALID_CLAIM_TYPES.join(", "))));
        }
    }

    // Optional severity validation (must be 1-10)
    if let Some(severity) = claim.severity {
        if !(1..=10).contains(&severity) {
            errors.push(make_error(Some(id), "severity", ValidationErrorType::OutOfRange,
                &format!("severity must be between 1 and 10, got {severity}")));
        }
    }

    errors
}

fn is_valid_category(category: &str) -> bool {
    VALID_CATEGORIES.contains(&category)
}

fn is_valid_claim_type(claim_type: &str) -> bool {
    VALID_CLAIM_TYPES.contains(&claim_type)
}

fn make_error(claim_id: Option<&str>, field: &str, error_type: ValidationErrorType, message: &str) -> ValidationError {
    ValidationError {
        claim_id: claim_id.map(String::from),
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
    use crate::models::import::ClaimSource;

    fn valid_json() -> &'static str {
        r#"{"schema_version":"2.1","extraction_metadata":{"extracted_at":"2025-12-20","extraction_model":"claude"},"source_document":{"id":"d1","title":"Doc","doc_type":"motion"},"case":{"id":"c1","name":"Case"},"parties":{"plaintiffs":[{"id":"p1","name":"P","role":"plaintiff"}],"defendants":[{"id":"d1","name":"D","role":"defendant"}]},"claims":[{"id":"CLAIM-001","category":"fraud","quote":"Test quote.","source":{"document_id":"d1"},"made_by":"p1","against":["d1"]}]}"#
    }

    fn make_test_claim(id: &str, category: &str, quote: &str, made_by: &str, against: Vec<&str>, doc_id: &str) -> ImportClaim {
        ImportClaim {
            id: id.to_string(), category: category.to_string(), severity: None, claim_type: None,
            quote: quote.to_string(), source: ClaimSource { document_id: doc_id.to_string(), document_title: None, document_type: None, line_start: None, line_end: None, page_number: None },
            made_by: made_by.to_string(), against: against.into_iter().map(String::from).collect(),
            amount: None, date_reference: None, evidence_refs: None,
        }
    }

    // Stage 1 tests (file-level)
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

    // Stage 2 tests (claim-level)
    #[test]
    fn test_validate_claim_valid() {
        let claim = make_test_claim("C1", "fraud", "Quote text", "p1", vec!["d1"], "doc1");
        assert!(validate_claim(&claim).is_empty());
    }

    #[test]
    fn test_validate_claim_missing_id() {
        let claim = make_test_claim("", "fraud", "Quote", "p1", vec!["d1"], "doc1");
        let errors = validate_claim(&claim);
        assert!(errors.iter().any(|e| e.field == "id" && e.error_type == ValidationErrorType::MissingField));
    }

    #[test]
    fn test_validate_claim_missing_quote() {
        let claim = make_test_claim("C1", "fraud", "", "p1", vec!["d1"], "doc1");
        let errors = validate_claim(&claim);
        assert!(errors.iter().any(|e| e.field == "quote" && e.error_type == ValidationErrorType::MissingField));
    }

    #[test]
    fn test_validate_claim_invalid_category() {
        let claim = make_test_claim("C1", "invalid_cat", "Quote", "p1", vec!["d1"], "doc1");
        let errors = validate_claim(&claim);
        assert!(errors.iter().any(|e| e.field == "category" && e.error_type == ValidationErrorType::InvalidValue));
    }

    #[test]
    fn test_validate_claim_invalid_claim_type() {
        let mut claim = make_test_claim("C1", "fraud", "Quote", "p1", vec!["d1"], "doc1");
        claim.claim_type = Some("invalid_type".to_string());
        let errors = validate_claim(&claim);
        assert!(errors.iter().any(|e| e.field == "claim_type" && e.error_type == ValidationErrorType::InvalidValue));
    }

    #[test]
    fn test_validate_claim_severity_out_of_range() {
        let mut claim = make_test_claim("C1", "fraud", "Quote", "p1", vec!["d1"], "doc1");
        claim.severity = Some(0);
        assert!(validate_claim(&claim).iter().any(|e| e.field == "severity" && e.error_type == ValidationErrorType::OutOfRange));
        claim.severity = Some(11);
        assert!(validate_claim(&claim).iter().any(|e| e.field == "severity" && e.error_type == ValidationErrorType::OutOfRange));
    }

    #[test]
    fn test_validate_claim_empty_against() {
        let claim = make_test_claim("C1", "fraud", "Quote", "p1", vec![], "doc1");
        let errors = validate_claim(&claim);
        assert!(errors.iter().any(|e| e.field == "against" && e.error_type == ValidationErrorType::InvalidValue));
    }

    #[test]
    fn test_validate_claim_missing_source_document_id() {
        let claim = make_test_claim("C1", "fraud", "Quote", "p1", vec!["d1"], "");
        let errors = validate_claim(&claim);
        assert!(errors.iter().any(|e| e.field == "source.document_id" && e.error_type == ValidationErrorType::MissingField));
    }

    #[test]
    fn test_validate_claims_multiple_errors() {
        let claims = vec![
            make_test_claim("", "fraud", "Quote", "p1", vec!["d1"], "doc1"),  // missing id
            make_test_claim("C2", "invalid", "Quote", "p1", vec!["d1"], "doc1"),  // invalid category
        ];
        let errors = validate_claims(&claims);
        assert!(errors.len() >= 2);
    }
}
