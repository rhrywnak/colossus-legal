//! Pipeline state machine — single source of truth for document status
//! transitions and available actions.
//!
//! ## 5-Status Model
//!
//! NEW → PROCESSING → COMPLETED
//!                  → FAILED
//!                  → CANCELLED
//!
//! Every non-PROCESSING status has at least one action (no dead ends).

use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::models::document_status::{
    STATUS_CANCELLED, STATUS_COMPLETED, STATUS_FAILED, STATUS_NEW, STATUS_PROCESSING,
};
use crate::repositories::pipeline_repository::{self, steps};
use crate::state::AppState;

use axum::{extract::Path, extract::State, Json};

// ── Pipeline step definitions ──────────────────────────────────

/// Internal pipeline steps (for execution history display, not user-facing stages).
/// These are the step_names recorded in the pipeline_steps table.
pub const PIPELINE_STEPS: &[(&str, &str)] = &[
    ("extract_text", "Read Document"),
    ("extract", "Analyze Content"),
    ("verify", "Verify Quotes"),
    ("ingest", "Write to Graph"),
    ("index", "Enable Search"),
    ("completeness", "Validate"),
];

// ── Response types ──────────────────────────────────────────────

/// Full response for GET /documents/:id/actions.
#[derive(Debug, Serialize)]
pub struct DocumentActions {
    pub document_id: String,
    pub current_status: String,
    pub available_actions: Vec<AvailableAction>,
    pub execution_history: Vec<ExecutionHistoryEntry>,
    pub delete_confirmation_level: String,
}

/// An action the user can take on a document in its current state.
#[derive(Debug, Clone, Serialize)]
pub struct AvailableAction {
    pub action: String,
    pub label: String,
    pub method: String,
    pub requires_confirmation: bool,
    pub description: String,
    pub is_navigation: bool,
    /// Relative URL path, e.g. "/documents/{id}/process"
    pub endpoint: String,
}

/// A raw execution history entry from the pipeline_steps table.
#[derive(Debug, Serialize)]
pub struct ExecutionHistoryEntry {
    pub step_name: String,
    pub label: String,
    pub status: String,
    pub started_at: String,
    pub duration_secs: Option<f64>,
    pub triggered_by: Option<String>,
    pub summary: Option<serde_json::Value>,
    pub error_message: Option<String>,
}

// ── State machine core ──────────────────────────────────────────

/// Available actions per document status.
///
/// 5 statuses, no dead ends. Every non-PROCESSING status has at least one action.
/// The `_pending_review` and `_total_items` params are unused but kept in the
/// signature for backward compatibility with callers.
fn get_available_actions(
    document_status: &str,
    _pending_review: i64,
    _total_items: i64,
) -> Vec<AvailableAction> {
    match document_status {
        STATUS_NEW => vec![
            make_action(
                "process",
                "Process Document",
                "POST",
                true,
                false,
                "Run the full extraction pipeline",
                "/documents/{id}/process",
            ),
            make_action(
                "delete",
                "Delete",
                "DELETE",
                true,
                false,
                "Delete this document",
                "/documents/{id}",
            ),
        ],
        STATUS_PROCESSING => vec![make_action(
            "cancel",
            "Cancel",
            "POST",
            true,
            false,
            "Cancel processing",
            "/documents/{id}/cancel",
        )],
        STATUS_COMPLETED => vec![
            make_action(
                "reprocess",
                "Re-process",
                "POST",
                true,
                false,
                "Re-run extraction with same or different settings",
                "/documents/{id}/process",
            ),
            make_action(
                "delete",
                "Delete",
                "DELETE",
                true,
                false,
                "Delete this document and its graph data",
                "/documents/{id}",
            ),
        ],
        STATUS_FAILED => vec![
            make_action(
                "reprocess",
                "Re-process",
                "POST",
                true,
                false,
                "Re-run extraction",
                "/documents/{id}/process",
            ),
            make_action(
                "delete",
                "Delete",
                "DELETE",
                true,
                false,
                "Delete this document",
                "/documents/{id}",
            ),
        ],
        STATUS_CANCELLED => vec![
            make_action(
                "reprocess",
                "Re-process",
                "POST",
                true,
                false,
                "Re-run extraction",
                "/documents/{id}/process",
            ),
            make_action(
                "delete",
                "Delete",
                "DELETE",
                true,
                false,
                "Delete this document",
                "/documents/{id}",
            ),
        ],
        _ => vec![],
    }
}

fn delete_confirmation_level(status: &str) -> &'static str {
    match status {
        STATUS_NEW => "simple",
        STATUS_FAILED | STATUS_CANCELLED => "moderate",
        STATUS_COMPLETED | STATUS_PROCESSING => "strict",
        _ => "strict",
    }
}

fn make_action(
    name: &str,
    label: &str,
    method: &str,
    confirm: bool,
    is_nav: bool,
    desc: &str,
    endpoint: &str,
) -> AvailableAction {
    AvailableAction {
        action: name.to_string(),
        label: label.to_string(),
        method: method.to_string(),
        requires_confirmation: confirm,
        description: desc.to_string(),
        is_navigation: is_nav,
        endpoint: endpoint.to_string(),
    }
}

// ── Execution history ──────────────────────────────────────────

/// Build execution history from pipeline_steps records.
/// Used by the Processing tab to show what happened during processing.
fn build_execution_history(
    step_records: &[steps::PipelineStepRecord],
) -> Vec<ExecutionHistoryEntry> {
    step_records
        .iter()
        .map(|s| {
            let label = PIPELINE_STEPS
                .iter()
                .find(|(name, _)| *name == s.step_name.as_str())
                .map(|(_, l)| l.to_string())
                .unwrap_or_else(|| titleize_step(&s.step_name));

            let summary = if s.result_summary.is_null() {
                None
            } else {
                Some(s.result_summary.clone())
            };

            ExecutionHistoryEntry {
                step_name: s.step_name.clone(),
                label,
                status: s.status.clone(),
                started_at: s.started_at.to_rfc3339(),
                duration_secs: s.duration_secs,
                triggered_by: s.triggered_by.clone(),
                summary,
                error_message: s.error_message.clone(),
            }
        })
        .collect()
}

// ── Handler ─────────────────────────────────────────────────────

/// GET /api/admin/pipeline/documents/:id/actions
pub async fn get_document_actions(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<DocumentActions>, AppError> {
    require_admin(&user)?;

    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    // Get all step records (sorted most recent first)
    let step_records = steps::get_steps_for_document(&state.pipeline_pool, &doc_id)
        .await
        .unwrap_or_default();

    let available_actions = get_available_actions(&document.status, 0, 0);
    let execution_history = build_execution_history(&step_records);

    let confirm_level = delete_confirmation_level(&document.status).to_string();

    Ok(Json(DocumentActions {
        document_id: doc_id,
        current_status: document.status,
        available_actions,
        execution_history,
        delete_confirmation_level: confirm_level,
    }))
}

/// Titleize a step name for display (e.g., "bulk_approve" → "Bulk Approve").
fn titleize_step(name: &str) -> String {
    name.split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // --- 5-status action tests ---

    #[test]
    fn test_new_has_process_and_delete() {
        let actions = get_available_actions("NEW", 0, 0);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].action, "process");
        assert_eq!(actions[1].action, "delete");
    }

    #[test]
    fn test_processing_has_cancel() {
        let actions = get_available_actions("PROCESSING", 0, 0);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action, "cancel");
        assert!(actions[0].requires_confirmation);
    }

    #[test]
    fn test_completed_has_reprocess_and_delete() {
        let actions = get_available_actions("COMPLETED", 0, 0);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].action, "reprocess");
        assert_eq!(actions[1].action, "delete");
    }

    #[test]
    fn test_failed_has_reprocess_and_delete() {
        let actions = get_available_actions("FAILED", 0, 0);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].action, "reprocess");
        assert_eq!(actions[1].action, "delete");
    }

    #[test]
    fn test_cancelled_has_reprocess_and_delete() {
        let actions = get_available_actions("CANCELLED", 0, 0);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].action, "reprocess");
        assert_eq!(actions[1].action, "delete");
    }

    #[test]
    fn test_unknown_status_has_no_actions() {
        let actions = get_available_actions("GARBAGE", 0, 0);
        assert!(actions.is_empty());
    }

    // --- No dead ends ---

    #[test]
    fn test_no_status_is_dead_end() {
        for status in &["NEW", "PROCESSING", "COMPLETED", "FAILED", "CANCELLED"] {
            let actions = get_available_actions(status, 0, 0);
            assert!(
                !actions.is_empty(),
                "Status '{status}' has no actions — this is a dead end"
            );
        }
    }

    // --- delete_confirmation_level tests ---

    #[test]
    fn test_delete_confirmation_simple_for_new() {
        assert_eq!(delete_confirmation_level("NEW"), "simple");
    }

    #[test]
    fn test_delete_confirmation_moderate_for_failed() {
        assert_eq!(delete_confirmation_level("FAILED"), "moderate");
        assert_eq!(delete_confirmation_level("CANCELLED"), "moderate");
    }

    #[test]
    fn test_delete_confirmation_strict_for_completed() {
        assert_eq!(delete_confirmation_level("COMPLETED"), "strict");
        assert_eq!(delete_confirmation_level("PROCESSING"), "strict");
    }

    // --- pipeline steps ---

    #[test]
    fn test_pipeline_steps_has_6_entries() {
        assert_eq!(PIPELINE_STEPS.len(), 6);
    }

    #[test]
    fn test_pipeline_steps_names() {
        assert_eq!(PIPELINE_STEPS[0].0, "extract_text");
        assert_eq!(PIPELINE_STEPS[1].0, "extract");
        assert_eq!(PIPELINE_STEPS[2].0, "verify");
        assert_eq!(PIPELINE_STEPS[3].0, "ingest");
        assert_eq!(PIPELINE_STEPS[4].0, "index");
        assert_eq!(PIPELINE_STEPS[5].0, "completeness");
    }

    // --- endpoint tests ---

    #[test]
    fn test_all_actions_have_nonempty_endpoint() {
        for status in &["NEW", "PROCESSING", "COMPLETED", "FAILED", "CANCELLED"] {
            let actions = get_available_actions(status, 0, 0);
            for action in &actions {
                assert!(
                    !action.endpoint.is_empty(),
                    "Action '{}' for status '{}' has empty endpoint",
                    action.action,
                    status
                );
                assert!(
                    action.endpoint.starts_with("/documents/{id}"),
                    "Action '{}' endpoint '{}' should start with /documents/{{id}}",
                    action.action,
                    action.endpoint
                );
            }
        }
    }

    // --- titleize_step ---

    #[test]
    fn test_titleize_step() {
        assert_eq!(titleize_step("bulk_approve"), "Bulk Approve");
        assert_eq!(titleize_step("extract_text"), "Extract Text");
        assert_eq!(titleize_step("ingest"), "Ingest");
    }
}
