//! `GET /api/admin/pipeline/document-types` — list document-type
//! entries from the pipeline registry.
//!
//! Backs the upload dialog's "Document Type" dropdown and any other UI
//! that needs to enumerate the document types the registry maps to
//! profile files. Excludes the registry's default entry — UIs render
//! the default implicitly via auto-detection or an "Other / Unknown"
//! pseudo-option, never as an explicit dropdown choice.
//!
//! ## Why not derive this from the profiles directory?
//!
//! Pre-registry, the frontend scanned `GET /profiles` and mapped
//! profile YAMLs to document types via the YAML's `document_type:`
//! field. That coupling broke whenever a profile YAML omitted or
//! mis-spelled `document_type:` — exactly the bug the registry exists
//! to fix. The new endpoint is authoritative: whatever the registry
//! lists is what the dropdown shows.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::state::AppState;

/// One element in the document-types list returned to the frontend.
///
/// Mirrors [`crate::pipeline::registry::DocumentTypeEntry`] minus the
/// `profile_file` and `is_default` fields — the frontend doesn't need
/// the on-disk filename (it's an implementation detail of the backend's
/// upload flow), and the default entry is filtered out before this DTO
/// is constructed.
#[derive(Debug, Serialize)]
pub struct DocumentTypeResponse {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub sort_order: i32,
}

/// Handler for `GET /api/admin/pipeline/document-types`.
///
/// Returns the registry's document types sorted by `sort_order`,
/// excluding the default entry. The response is intentionally
/// flat (no envelope) so the frontend can call
/// `await response.json()` and bind the array directly to its dropdown.
pub async fn list_document_types(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<DocumentTypeResponse>>, AppError> {
    require_admin(&user)?;

    let types: Vec<DocumentTypeResponse> = state
        .registry
        .document_types_sorted()
        .into_iter()
        .map(|dt| DocumentTypeResponse {
            name: dt.name.clone(),
            display_name: dt.display_name.clone(),
            description: dt.description.clone(),
            sort_order: dt.sort_order,
        })
        .collect();

    Ok(Json(types))
}

#[cfg(test)]
mod tests {
    //! Unit test for the document-types DTO construction.
    //!
    //! The handler itself needs an [`AppState`] (which requires DB
    //! pools) so we don't invoke it end-to-end here. The interesting
    //! behaviour — "sort, exclude default, build response" — lives in
    //! `PipelineRegistry::document_types_sorted` (registry-side test)
    //! and the mapping from registry entries to DTOs. This test pins
    //! the mapping.

    use super::*;
    use crate::pipeline::registry::{DocumentTypeEntry, PipelineDirectories, PipelineRegistry};

    fn registry_with_three_types() -> PipelineRegistry {
        PipelineRegistry {
            directories: PipelineDirectories {
                profiles: "/tmp".to_string(),
                schemas: "/tmp".to_string(),
                templates: "/tmp".to_string(),
                system_prompts: "/tmp".to_string(),
            },
            document_types: vec![
                DocumentTypeEntry {
                    name: "discovery_response".to_string(),
                    display_name: "Discovery Response".to_string(),
                    profile_file: "discovery.yaml".to_string(),
                    description: "Sworn discovery responses".to_string(),
                    is_default: false,
                    sort_order: 2,
                },
                DocumentTypeEntry {
                    name: "complaint".to_string(),
                    display_name: "Complaint".to_string(),
                    profile_file: "complaint.yaml".to_string(),
                    description: "Initiating pleading".to_string(),
                    is_default: false,
                    sort_order: 1,
                },
                DocumentTypeEntry {
                    name: "default".to_string(),
                    display_name: "Other".to_string(),
                    profile_file: "default.yaml".to_string(),
                    description: "Fallback".to_string(),
                    is_default: true,
                    sort_order: 99,
                },
            ],
        }
    }

    #[test]
    fn test_document_types_endpoint_returns_sorted_list() {
        let registry = registry_with_three_types();
        let dtos: Vec<DocumentTypeResponse> = registry
            .document_types_sorted()
            .into_iter()
            .map(|dt| DocumentTypeResponse {
                name: dt.name.clone(),
                display_name: dt.display_name.clone(),
                description: dt.description.clone(),
                sort_order: dt.sort_order,
            })
            .collect();

        let names: Vec<&str> = dtos.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["complaint", "discovery_response"],
            "sorted by sort_order; default entry excluded"
        );
        assert_eq!(dtos[0].display_name, "Complaint");
        assert_eq!(dtos[0].description, "Initiating pleading");
    }
}
