//! Claim-level validation for claims import (Stage 2).
//! Validates individual claim fields, enum values, ranges, and detects duplicates.

use crate::models::import::{ImportClaim, ValidationError, ValidationErrorType};
use std::collections::HashSet;

const VALID_CATEGORIES: &[&str] = &[
    "conversion",
    "fraud",
    "breach_of_fiduciary_duty",
    "defamation",
    "bias",
    "discovery_obstruction",
    "perjury",
    "collusion",
    "financial_harm",
    "procedural_misconduct",
    "conflict_of_interest",
    "unauthorized_possession",
    "impartiality_violation",
    "negligence",
    "misrepresentation",
    "abuse_of_process",
    "unjust_enrichment",
    "breach_of_contract",
    "emotional_distress",
];
const VALID_CLAIM_TYPES: &[&str] = &["factual_event", "legal_conclusion", "procedural"];

/// Validate all claims: duplicates first, then field validation.
pub fn validate_claims(claims: &[ImportClaim]) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    errors.extend(detect_duplicate_ids(claims));
    errors.extend(claims.iter().flat_map(validate_claim));
    errors
}

/// Detect duplicate claim IDs. Returns error for each duplicate (not the first).
pub fn detect_duplicate_ids(claims: &[ImportClaim]) -> Vec<ValidationError> {
    let mut seen = HashSet::new();
    let mut errors = Vec::new();
    for claim in claims {
        if !seen.insert(&claim.id) {
            errors.push(make_error(
                Some(&claim.id),
                "id",
                ValidationErrorType::DuplicateId,
                &format!("Duplicate claim ID: '{}'", claim.id),
            ));
        }
    }
    errors
}

fn validate_claim(claim: &ImportClaim) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let id = &claim.id;
    if claim.id.is_empty() {
        errors.push(make_error(
            Some(id),
            "id",
            ValidationErrorType::MissingField,
            "id is required",
        ));
    }
    if claim.quote.is_empty() {
        errors.push(make_error(
            Some(id),
            "quote",
            ValidationErrorType::MissingField,
            "quote is required",
        ));
    }
    if claim.made_by.is_empty() {
        errors.push(make_error(
            Some(id),
            "made_by",
            ValidationErrorType::MissingField,
            "made_by is required",
        ));
    }
    if claim.against.is_empty() {
        errors.push(make_error(
            Some(id),
            "against",
            ValidationErrorType::InvalidValue,
            "against array cannot be empty",
        ));
    }
    if claim.source.document_id.is_empty() {
        errors.push(make_error(
            Some(id),
            "source.document_id",
            ValidationErrorType::MissingField,
            "source.document_id is required",
        ));
    }
    if !VALID_CATEGORIES.contains(&claim.category.as_str()) {
        errors.push(make_error(
            Some(id),
            "category",
            ValidationErrorType::InvalidValue,
            &format!(
                "Invalid category: '{}'. Valid: {}",
                claim.category,
                VALID_CATEGORIES.join(", ")
            ),
        ));
    }
    if let Some(ref ct) = claim.claim_type {
        if !VALID_CLAIM_TYPES.contains(&ct.as_str()) {
            errors.push(make_error(
                Some(id),
                "claim_type",
                ValidationErrorType::InvalidValue,
                &format!(
                    "Invalid claim_type: '{}'. Valid: {}",
                    ct,
                    VALID_CLAIM_TYPES.join(", ")
                ),
            ));
        }
    }
    if let Some(sev) = claim.severity {
        if !(1..=10).contains(&sev) {
            errors.push(make_error(
                Some(id),
                "severity",
                ValidationErrorType::OutOfRange,
                &format!("severity must be 1-10, got {sev}"),
            ));
        }
    }
    errors
}

fn make_error(
    claim_id: Option<&str>,
    field: &str,
    error_type: ValidationErrorType,
    msg: &str,
) -> ValidationError {
    ValidationError {
        claim_id: claim_id.map(String::from),
        field: field.to_string(),
        error_type,
        message: msg.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::import::ClaimSource;

    fn claim(
        id: &str,
        cat: &str,
        quote: &str,
        by: &str,
        against: Vec<&str>,
        doc: &str,
    ) -> ImportClaim {
        ImportClaim {
            id: id.into(),
            category: cat.into(),
            severity: None,
            claim_type: None,
            quote: quote.into(),
            source: ClaimSource {
                document_id: doc.into(),
                document_title: None,
                document_type: None,
                line_start: None,
                line_end: None,
                page_number: None,
            },
            made_by: by.into(),
            against: against.into_iter().map(String::from).collect(),
            amount: None,
            date_reference: None,
            evidence_refs: None,
        }
    }

    #[test]
    fn test_validate_claim_valid() {
        assert!(
            validate_claim(&claim("C1", "fraud", "Quote", "p1", vec!["d1"], "doc1")).is_empty()
        );
    }
    #[test]
    fn test_validate_claim_missing_id() {
        assert!(
            validate_claim(&claim("", "fraud", "Q", "p1", vec!["d1"], "d1"))
                .iter()
                .any(|e| e.field == "id")
        );
    }
    #[test]
    fn test_validate_claim_missing_quote() {
        assert!(
            validate_claim(&claim("C1", "fraud", "", "p1", vec!["d1"], "d1"))
                .iter()
                .any(|e| e.field == "quote")
        );
    }
    #[test]
    fn test_validate_claim_invalid_category() {
        assert!(
            validate_claim(&claim("C1", "bad", "Q", "p1", vec!["d1"], "d1"))
                .iter()
                .any(|e| e.field == "category")
        );
    }
    #[test]
    fn test_validate_claim_invalid_claim_type() {
        let mut c = claim("C1", "fraud", "Q", "p1", vec!["d1"], "d1");
        c.claim_type = Some("bad".into());
        assert!(validate_claim(&c).iter().any(|e| e.field == "claim_type"));
    }
    #[test]
    fn test_validate_claim_severity_out_of_range() {
        let mut c = claim("C1", "fraud", "Q", "p1", vec!["d1"], "d1");
        c.severity = Some(0);
        assert!(validate_claim(&c).iter().any(|e| e.field == "severity"));
        c.severity = Some(11);
        assert!(validate_claim(&c).iter().any(|e| e.field == "severity"));
    }
    #[test]
    fn test_validate_claim_empty_against() {
        assert!(
            validate_claim(&claim("C1", "fraud", "Q", "p1", vec![], "d1"))
                .iter()
                .any(|e| e.field == "against")
        );
    }
    #[test]
    fn test_validate_claim_missing_source_document_id() {
        assert!(
            validate_claim(&claim("C1", "fraud", "Q", "p1", vec!["d1"], ""))
                .iter()
                .any(|e| e.field == "source.document_id")
        );
    }
    #[test]
    fn test_validate_claims_multiple_errors() {
        let claims = vec![
            claim("", "fraud", "Q", "p1", vec!["d1"], "d1"),
            claim("C2", "bad", "Q", "p1", vec!["d1"], "d1"),
        ];
        assert!(validate_claims(&claims).len() >= 2);
    }
    #[test]
    fn test_detect_duplicate_ids_no_duplicates() {
        let claims = vec![
            claim("C1", "fraud", "Q", "p1", vec!["d1"], "d1"),
            claim("C2", "fraud", "Q", "p1", vec!["d1"], "d1"),
        ];
        assert!(detect_duplicate_ids(&claims).is_empty());
    }
    #[test]
    fn test_detect_duplicate_ids_one_duplicate() {
        let claims = vec![
            claim("C1", "fraud", "Q", "p1", vec!["d1"], "d1"),
            claim("C1", "fraud", "Q", "p1", vec!["d1"], "d1"),
        ];
        let errs = detect_duplicate_ids(&claims);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_type, ValidationErrorType::DuplicateId);
    }
    #[test]
    fn test_detect_duplicate_ids_multiple_duplicates() {
        let claims = vec![
            claim("C1", "fraud", "Q", "p1", vec!["d1"], "d1"),
            claim("C2", "fraud", "Q", "p1", vec!["d1"], "d1"),
            claim("C1", "fraud", "Q", "p1", vec!["d1"], "d1"),
            claim("C2", "fraud", "Q", "p1", vec!["d1"], "d1"),
        ];
        assert_eq!(detect_duplicate_ids(&claims).len(), 2);
    }
    #[test]
    fn test_detect_duplicate_ids_same_id_three_times() {
        let claims = vec![
            claim("C1", "fraud", "Q", "p1", vec!["d1"], "d1"),
            claim("C1", "fraud", "Q", "p1", vec!["d1"], "d1"),
            claim("C1", "fraud", "Q", "p1", vec!["d1"], "d1"),
        ];
        assert_eq!(detect_duplicate_ids(&claims).len(), 2);
    }
}
