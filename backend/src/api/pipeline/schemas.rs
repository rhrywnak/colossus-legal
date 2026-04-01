//! GET /api/admin/pipeline/schemas — list available extraction schemas.
//! Reads .yaml files from the extraction_schemas directory at request time.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct SchemasResponse {
    pub schemas: Vec<SchemaInfo>,
}

#[derive(Debug, Serialize)]
pub struct SchemaInfo {
    pub name: String,
    pub label: String,
    pub description: String,
}

/// GET /schemas
pub async fn list_schemas_handler(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<SchemasResponse>, AppError> {
    require_admin(&user)?;

    let dir = &state.config.extraction_schema_dir;
    let mut schemas = Vec::new();

    let entries = match tokio::fs::read_dir(dir).await {
        Ok(e) => e,
        Err(_) => return Ok(Json(SchemasResponse { schemas })),
    };

    let mut entries = entries;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }

        let label = name
            .split('_')
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().to_string() + c.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        // Try to extract description from the YAML file
        let description = tokio::fs::read_to_string(&path)
            .await
            .ok()
            .and_then(|content| {
                content.lines().find_map(|line| {
                    line.strip_prefix("description:")
                        .or_else(|| line.strip_prefix("description :"))
                        .map(|v| v.trim().trim_matches('"').to_string())
                })
            })
            .unwrap_or_default();

        schemas.push(SchemaInfo { name, label, description });
    }

    schemas.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Json(SchemasResponse { schemas }))
}
