//! Import validation API endpoint.
//!
//! POST /import/validate - Validates claims import JSON without persisting.

use axum::Json;
use crate::auth::{AuthError, AuthUser, require_edit};
use crate::models::import::ValidationResult;
use crate::services::import_validator::validate_json;

/// Validate import JSON and return validation result.
///
/// Always returns 200 OK — validation errors are data, not HTTP errors.
/// The `valid` field in the response indicates success/failure.
pub async fn validate_import(
    user: AuthUser,
    body: String,
) -> Result<Json<ValidationResult>, AuthError> {
    require_edit(&user)?;
    tracing::info!("{} POST /import/validate", user.username);

    match validate_json(&body) {
        Ok(request) => {
            // Valid JSON with no errors
            Ok(Json(ValidationResult {
                valid: true,
                claim_count: request.claims.len() as i32,
                document_title: request.source_document.title,
                errors: Vec::new(),
                warnings: Vec::new(),
            }))
        }
        Err(result) => {
            // Validation failed — return the error result
            Ok(Json(result))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use colossus_auth::AuthUser;

    fn test_editor() -> AuthUser {
        AuthUser {
            username: "test_editor".to_string(),
            email: "editor@test.com".to_string(),
            display_name: "Test Editor".to_string(),
            groups: vec!["legal_editor".to_string()],
        }
    }

    fn valid_json() -> String {
        r#"{"schema_version":"2.1","extraction_metadata":{"extracted_at":"2025-12-20","extraction_model":"claude"},"source_document":{"id":"d1","title":"Test Doc","doc_type":"motion"},"case":{"id":"c1","name":"Test Case"},"parties":{"plaintiffs":[{"id":"p1","name":"P","role":"plaintiff"}],"defendants":[{"id":"d1","name":"D","role":"defendant"}]},"claims":[{"id":"CLAIM-001","category":"fraud","quote":"Test quote.","source":{"document_id":"d1"},"made_by":"p1","against":["d1"]}]}"#.to_string()
    }

    #[tokio::test]
    async fn test_validate_import_valid_json() {
        let result = validate_import(test_editor(), valid_json()).await.unwrap();
        assert!(result.valid);
        assert_eq!(result.claim_count, 1);
        assert_eq!(result.document_title, "Test Doc");
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_validate_import_invalid_json_syntax() {
        let result = validate_import(test_editor(), "{ invalid }".to_string()).await.unwrap();
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
        assert_eq!(result.errors[0].field, "json");
    }

    #[tokio::test]
    async fn test_validate_import_validation_errors() {
        // Missing required fields in claim
        let json = r#"{"schema_version":"2.1","extraction_metadata":{"extracted_at":"x","extraction_model":"x"},"source_document":{"id":"d","title":"D","doc_type":"m"},"case":{"id":"c","name":"C"},"parties":{"plaintiffs":[],"defendants":[]},"claims":[{"id":"","category":"bad","quote":"","source":{"document_id":""},"made_by":"","against":[]}]}"#;
        let result = validate_import(test_editor(), json.to_string()).await.unwrap();
        assert!(!result.valid);
        assert!(result.errors.len() >= 2); // Multiple validation errors
    }
}
