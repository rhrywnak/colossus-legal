//! Import validation API endpoint.
//!
//! POST /import/validate - Validates claims import JSON without persisting.

use crate::auth::{require_edit, AuthError, AuthUser};
use crate::models::import::ValidationResult;
use crate::services::import_validator::validate_json;
use axum::Json;

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

