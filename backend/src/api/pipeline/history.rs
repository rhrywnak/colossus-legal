//! GET /api/admin/pipeline/documents/:id/history — execution history.

use axum::{extract::Path, extract::State, Json};
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::pipeline::registry::PipelineRegistry;
use crate::repositories::pipeline_repository::steps::{self, PipelineStepRecord};
use crate::state::AppState;

/// Single row of execution history with the user-facing step label
/// resolved from the registry.
///
/// `#[serde(flatten)]` keeps the wire shape backwards-compatible:
/// every existing `PipelineStepRecord` field appears at the top level
/// of the JSON object, with the new `step_label` field alongside.
/// Legacy callers that ignored unknown fields are unaffected; the
/// frontend reads `step_label` for the Execution History display and
/// falls back to `step_name` when it's null.
#[derive(Debug, Serialize)]
pub struct PipelineStepHistoryEntry {
    #[serde(flatten)]
    pub record: PipelineStepRecord,
    /// Operator-facing label from `pipeline_registry.yaml`'s
    /// `step_labels:` section. `None` when the row's `step_name`
    /// has no registry entry (legacy rows, ad-hoc steps).
    pub step_label: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub document_id: String,
    pub steps: Vec<PipelineStepHistoryEntry>,
}

/// GET /api/admin/pipeline/documents/:id/history
pub async fn history_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(document_id): Path<String>,
) -> Result<Json<HistoryResponse>, AppError> {
    require_admin(&user)?;

    let records = steps::get_steps_for_document(&state.pipeline_pool, &document_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("History query failed: {e}"),
        })?;

    let steps = records
        .into_iter()
        .map(|record| {
            let step_label = resolve_step_label(&state.registry, &record);
            PipelineStepHistoryEntry { record, step_label }
        })
        .collect();

    Ok(Json(HistoryResponse { document_id, steps }))
}

/// Resolve a row's user-facing label from the registry, substituting
/// `{grounding_pct}` from `result_summary` when the verify step has
/// completed.
///
/// The verify entry's label carries a `{grounding_pct}` placeholder
/// that's normally substituted at write time into
/// `documents.processing_step_label`. For Execution History we don't
/// have that pre-substituted string — the rows store raw `step_name`
/// — so we redo the substitution here when the data is available.
/// Pre-completion rows (`status='running'`) keep the literal
/// placeholder, which is rare in practice (verify is fast and
/// historical rows are almost always completed).
fn resolve_step_label(registry: &PipelineRegistry, record: &PipelineStepRecord) -> Option<String> {
    let entry = registry.step_label(&record.step_name)?;
    let template = &entry.label;

    if !template.contains("{grounding_pct}") {
        return Some(template.clone());
    }

    if record.status == "completed" {
        if let Some(pct) = record
            .result_summary
            .get("grounding_pct")
            .and_then(|v| v.as_f64())
        {
            return Some(template.replace("{grounding_pct}", &format!("{pct:.0}")));
        }
    }

    // status='running' or completed-without-grounding_pct → leave the
    // placeholder literal so the operator sees the same string the
    // backend would have written and can spot the missing data.
    Some(template.clone())
}
