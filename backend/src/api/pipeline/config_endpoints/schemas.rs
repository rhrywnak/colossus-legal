//! Admin CRUD endpoints for extraction schema YAML files.
//!
//! Schemas are YAML files under `extraction_schema_dir` that define the
//! entity / relationship types the pipeline extracts per document type.
//!
//! Design: DOC_PROCESSING_CONFIG_DESIGN_v2.md Section 3.4.4.

use axum::{
    extract::{Path as AxumPath, State},
    Json,
};
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::state::AppState;

use super::shared::{
    profiles_referencing, require_extension, validate_filename, CreateFileInput,
    FileContentResponse, UpdateFileInput,
};

/// Required extension for schema files.
const SCHEMA_EXT: &str = ".yaml";

#[derive(Debug, Serialize)]
pub struct SchemasResponse {
    pub schemas: Vec<SchemaInfo>,
}

#[derive(Debug, Serialize)]
pub struct SchemaInfo {
    pub filename: String,
    pub document_type: String,
    pub version: String,
    pub description: String,
    pub entity_type_count: usize,
    pub entity_types: Vec<String>,
}

/// GET /api/admin/pipeline/schemas — list available extraction schemas.
pub async fn list_schemas(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<SchemasResponse>, AppError> {
    require_admin(&user)?;

    let schema_dir = &state.config.extraction_schema_dir;
    let mut schemas = Vec::new();

    let mut entries = tokio::fs::read_dir(schema_dir)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to read schema directory: {e}"),
        })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| AppError::Internal {
        message: format!("Failed to read directory entry: {e}"),
    })? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let filename = entry.file_name().to_string_lossy().to_string();
        match colossus_extract::ExtractionSchema::from_file(&path) {
            Ok(schema) => {
                let entity_types: Vec<String> =
                    schema.entity_types.iter().map(|et| et.name.clone()).collect();
                schemas.push(SchemaInfo {
                    filename,
                    document_type: schema.document_type,
                    version: schema.version,
                    description: schema.description,
                    entity_type_count: entity_types.len(),
                    entity_types,
                });
            }
            Err(e) => {
                tracing::warn!(file = %filename, error = %e, "Skipping invalid schema file");
            }
        }
    }

    schemas.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(Json(SchemasResponse { schemas }))
}

/// GET /api/admin/pipeline/schemas/:filename — read a single schema file.
pub async fn get_schema(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
) -> Result<Json<FileContentResponse>, AppError> {
    require_admin(&user)?;
    validate_filename(&filename)?;

    let path = std::path::Path::new(&state.config.extraction_schema_dir).join(&filename);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("Schema '{filename}' not found"),
        });
    }

    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to read schema '{filename}': {e}"),
        })?;
    let size_bytes = content.len() as u64;

    Ok(Json(FileContentResponse {
        filename,
        content,
        size_bytes,
    }))
}

/// POST /api/admin/pipeline/schemas — create a new schema file.
pub async fn create_schema(
    user: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CreateFileInput>,
) -> Result<Json<FileContentResponse>, AppError> {
    require_admin(&user)?;
    validate_filename(&input.filename)?;
    require_extension(&input.filename, SCHEMA_EXT)?;

    let path = std::path::Path::new(&state.config.extraction_schema_dir).join(&input.filename);
    if tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::Conflict {
            message: format!("Schema '{}' already exists", input.filename),
            details: serde_json::json!({"filename": input.filename}),
        });
    }

    tokio::fs::write(&path, &input.content)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to write schema '{}': {e}", input.filename),
        })?;

    let size_bytes = input.content.len() as u64;
    Ok(Json(FileContentResponse {
        filename: input.filename,
        content: input.content,
        size_bytes,
    }))
}

/// PUT /api/admin/pipeline/schemas/:filename — overwrite an existing schema.
pub async fn update_schema(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
    Json(input): Json<UpdateFileInput>,
) -> Result<Json<FileContentResponse>, AppError> {
    require_admin(&user)?;
    validate_filename(&filename)?;

    let path = std::path::Path::new(&state.config.extraction_schema_dir).join(&filename);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("Schema '{filename}' not found"),
        });
    }

    tokio::fs::write(&path, &input.content)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to write schema '{filename}': {e}"),
        })?;

    let size_bytes = input.content.len() as u64;
    Ok(Json(FileContentResponse {
        filename,
        content: input.content,
        size_bytes,
    }))
}

/// DELETE /api/admin/pipeline/schemas/:filename — delete a schema file.
///
/// Refuses the delete (`409 Conflict`) if any profile YAML references
/// this filename as `schema_file`.
pub async fn delete_schema(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&user)?;
    validate_filename(&filename)?;

    let path = std::path::Path::new(&state.config.extraction_schema_dir).join(&filename);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("Schema '{filename}' not found"),
        });
    }

    let referencing = profiles_referencing(&state.config.processing_profile_dir, &filename)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to scan profile directory: {e}"),
        })?;

    if !referencing.is_empty() {
        return Err(AppError::Conflict {
            message: format!(
                "Schema '{filename}' is referenced by {} profile(s)",
                referencing.len()
            ),
            details: serde_json::json!({"referenced_by": referencing}),
        });
    }

    tokio::fs::remove_file(&path)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to delete schema '{filename}': {e}"),
        })?;

    Ok(Json(serde_json::json!({"deleted": filename})))
}
