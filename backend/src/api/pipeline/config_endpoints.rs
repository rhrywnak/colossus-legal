//! Configuration discovery endpoints for the extraction pipeline.
//!
//! These endpoints let users see what models, schemas, and templates
//! are available before triggering extraction. No database access —
//! they read from the filesystem and config files.

use axum::{extract::State, Json};
use serde::Serialize;
use std::path::Path;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::state::AppState;

// ── Models endpoint ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ModelsResponse {
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub display_name: String,
    pub input_cost_per_mtok: f64,
    pub output_cost_per_mtok: f64,
    pub max_context: u64,
    pub max_output: u64,
    #[serde(default)]
    pub recommended_for: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct ModelsFile {
    models: Vec<ModelInfo>,
}

/// GET /api/admin/pipeline/models — list available LLM models.
pub async fn list_models(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ModelsResponse>, AppError> {
    require_admin(&user)?;

    let models_path = Path::new(&state.config.extraction_config_dir).join("models.yaml");
    let content =
        tokio::fs::read_to_string(&models_path)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to read models.yaml: {e}"),
            })?;
    let parsed: ModelsFile = serde_yaml::from_str(&content).map_err(|e| AppError::Internal {
        message: format!("Failed to parse models.yaml: {e}"),
    })?;

    Ok(Json(ModelsResponse {
        models: parsed.models,
    }))
}

// ── Schemas endpoint ────────────────────────────────────────────

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
        if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
            let filename = entry.file_name().to_string_lossy().to_string();
            match colossus_extract::ExtractionSchema::from_file(&path) {
                Ok(schema) => {
                    let entity_types: Vec<String> = schema
                        .entity_types
                        .iter()
                        .map(|et| et.name.clone())
                        .collect();
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
    }

    schemas.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(Json(SchemasResponse { schemas }))
}

// ── Templates endpoint ──────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct TemplatesResponse {
    pub templates: Vec<TemplateInfo>,
}

#[derive(Debug, Serialize)]
pub struct TemplateInfo {
    pub filename: String,
    pub preview: String,
    pub size_bytes: u64,
}

/// GET /api/admin/pipeline/templates — list available prompt templates.
pub async fn list_templates(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<TemplatesResponse>, AppError> {
    require_admin(&user)?;

    let template_dir = &state.config.extraction_template_dir;
    let mut templates = Vec::new();

    let mut entries = tokio::fs::read_dir(template_dir)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to read template directory: {e}"),
        })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| AppError::Internal {
        message: format!("Failed to read directory entry: {e}"),
    })? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let filename = entry.file_name().to_string_lossy().to_string();
            let metadata = entry.metadata().await.map_err(|e| AppError::Internal {
                message: format!("Failed to read file metadata: {e}"),
            })?;
            let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
            let preview: String = content.chars().take(500).collect();
            templates.push(TemplateInfo {
                filename,
                preview,
                size_bytes: metadata.len(),
            });
        }
    }

    templates.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(Json(TemplatesResponse { templates }))
}
