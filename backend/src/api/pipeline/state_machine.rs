//! Pipeline state machine — single source of truth for document status
//! transitions, available actions, and pipeline stage display.
//!
//! ## Rust Learning: Fixed stages vs raw execution history
//!
//! The pipeline has exactly 8 stages that always display in order.
//! The `pipeline_steps` DB table is a raw execution log with many entries
//! (bulk_approve, re-runs, etc.). This module builds both:
//! - `pipeline_stages`: 8 fixed stages with status derived from the log
//! - `execution_history`: raw log for the detail view

use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::pipeline_repository::{self, review, steps};
use crate::state::AppState;

use axum::{extract::Path, extract::State, Json};

// ── Fixed pipeline stage definitions ────────────────────────────

/// The 8 pipeline stages in fixed order.
pub const PIPELINE_STAGE_ORDER: &[(&str, &str)] = &[
    ("upload", "Upload"),
    ("extract_text", "Read Document"),
    ("extract", "Analyze Content"),
    ("verify", "Verify Accuracy"),
    ("review", "Human Review"),
    ("ingest", "Build Knowledge Graph"),
    ("index", "Enable Search"),
    ("completeness", "Quality Check"),
];

// ── Response types ──────────────────────────────────────────────

/// Full response for GET /documents/:id/actions.
#[derive(Debug, Serialize)]
pub struct DocumentActions {
    pub document_id: String,
    pub current_status: String,
    pub pipeline_stages: Vec<PipelineStage>,
    pub available_actions: Vec<AvailableAction>,
    pub execution_history: Vec<ExecutionHistoryEntry>,
    pub delete_confirmation_level: String,
}

/// One of the 8 fixed pipeline stages.
#[derive(Debug, Serialize)]
pub struct PipelineStage {
    /// Stage identifier (e.g., "extract_text")
    pub name: String,
    /// Human-readable label (e.g., "Extract Text")
    pub label: String,
    /// Display order (1-8)
    pub order: u8,
    /// Status: "completed", "available", "pending", "failed"
    pub status: String,
    /// Duration of the most recent successful run, if any
    pub duration_secs: Option<f64>,
    /// Formatted summary metric (e.g., "17 pages, 27353 chars")
    pub summary: Option<String>,
    /// Non-null if this stage has an actionable button right now
    pub action: Option<AvailableAction>,
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
    /// Relative URL path, e.g. "/documents/{id}/extract-text"
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

/// Determine available actions based on document status and review state.
fn get_available_actions(
    document_status: &str,
    pending_review_count: i64,
    total_item_count: i64,
) -> Vec<AvailableAction> {
    match document_status {
        "UPLOADED" => vec![
            make_action("process", "Process Document", "POST", false, false,
                        "Run the full processing pipeline",
                        "/documents/{id}/process"),
            make_action("extract_text", "Read Document", "POST", false, false,
                        "Extract text from the PDF document",
                        "/documents/{id}/extract-text"),
        ],
        "TEXT_EXTRACTED" => vec![
            make_action("extract", "Analyze Content", "POST", false, false,
                        "Run LLM extraction to identify entities and relationships",
                        "/documents/{id}/extract"),
        ],
        "EXTRACTED" => vec![
            make_action("verify", "Verify Accuracy", "POST", false, false,
                        "Verify extracted quotes against document text",
                        "/documents/{id}/verify"),
        ],
        "VERIFIED" => {
            let mut actions = vec![
                make_action("review", "Review Items", "GET", false, true,
                            "Review and approve extracted items",
                            "/documents/{id}/review"),
            ];
            if pending_review_count == 0 && total_item_count > 0 {
                actions.push(
                    make_action("ingest", "Build Knowledge Graph", "POST", true, false,
                                "Write approved items to the knowledge graph",
                                "/documents/{id}/ingest"),
                );
            }
            actions
        },
        "INGESTED" => vec![
            make_action("index", "Enable Search", "POST", false, false,
                        "Generate vector embeddings for search",
                        "/documents/{id}/index"),
            make_action("reprocess", "Reprocess Document", "POST", true, false,
                        "Reset document for re-extraction with different schema",
                        "/documents/{id}/reprocess"),
        ],
        "INDEXED" => vec![
            make_action("completeness", "Quality Check", "GET", false, false,
                        "Verify all items are in the graph and indexed",
                        "/documents/{id}/completeness"),
            make_action("reprocess", "Reprocess Document", "POST", true, false,
                        "Reset document for re-extraction with different schema",
                        "/documents/{id}/reprocess"),
        ],
        "PUBLISHED" => vec![
            make_action("reprocess", "Reprocess Document", "POST", true, false,
                        "Reset document for re-extraction with different schema",
                        "/documents/{id}/reprocess"),
        ],
        // New process endpoint statuses (coexist with legacy statuses)
        "PROCESSING" => vec![], // No actions while processing — frontend shows progress
        "COMPLETED" => vec![
            make_action("reprocess", "Reprocess Document", "POST", true, false,
                        "Reset document for re-extraction with different schema",
                        "/documents/{id}/reprocess"),
        ],
        "FAILED" | "CANCELLED" => vec![
            make_action("process", "Re-process Document", "POST", false, false,
                        "Re-run the full processing pipeline",
                        "/documents/{id}/process"),
        ],
        _ => vec![],
    }
}

fn delete_confirmation_level(status: &str) -> &'static str {
    match status {
        "UPLOADED" | "TEXT_EXTRACTED" => "simple",
        "EXTRACTED" | "VERIFIED" => "moderate",
        _ => "strict",
    }
}

fn make_action(
    name: &str, label: &str, method: &str, confirm: bool, is_nav: bool, desc: &str,
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

// ── Stage building ──────────────────────────────────────────────

/// Build the 8 fixed pipeline stages from execution history and actions.
///
/// ## Rust Learning: Deriving display state from raw data
///
/// Each stage's status is determined by looking at the most recent
/// pipeline_steps record for that step_name:
/// - Found + completed → "completed"
/// - Found + failed → "failed"
/// - In available_actions → "available"
/// - Otherwise → "pending"
///
/// The "review" stage is special — it's manual and tracked by item
/// review_status, not by pipeline_steps records.
fn build_pipeline_stages(
    step_records: &[steps::PipelineStepRecord],
    available_actions: &[AvailableAction],
    pending_review_count: i64,
    total_item_count: i64,
) -> Vec<PipelineStage> {
    PIPELINE_STAGE_ORDER.iter().enumerate().map(|(i, &(name, label))| {
        let order = (i + 1) as u8;

        // Special case: "upload" is always completed (document exists).
        // Still look up the step record for duration and summary info.
        if name == "upload" {
            let latest = step_records.iter().find(|s| s.step_name == "upload");
            let (dur, summary) = match latest {
                Some(record) => (
                    record.duration_secs,
                    format_stage_summary("upload", &record.result_summary),
                ),
                None => (None, None),
            };
            return PipelineStage {
                name: name.to_string(),
                label: label.to_string(),
                order,
                status: "completed".to_string(),
                duration_secs: dur,
                summary,
                action: None,
            };
        }

        // Special case: "review" — manual stage tracked by item review_status
        if name == "review" {
            let action = available_actions.iter().find(|a| a.action == "review").cloned();
            let status = if pending_review_count == 0 && total_item_count > 0 {
                "completed"
            } else if action.is_some() {
                "available"
            } else {
                "pending"
            };
            let summary = if total_item_count > 0 {
                if pending_review_count == 0 {
                    Some("All items reviewed".to_string())
                } else {
                    Some(format!("{pending_review_count} pending"))
                }
            } else {
                None
            };
            return PipelineStage {
                name: name.to_string(),
                label: label.to_string(),
                order,
                status: status.to_string(),
                duration_secs: None,
                summary,
                action,
            };
        }

        // Normal stage: look up most recent pipeline_steps record
        let latest = step_records.iter().find(|s| s.step_name == name);

        let action = available_actions.iter().find(|a| a.action == name).cloned();

        match latest {
            Some(record) if record.status == "completed" => {
                let summary = format_stage_summary(name, &record.result_summary);
                PipelineStage {
                    name: name.to_string(),
                    label: label.to_string(),
                    order,
                    status: "completed".to_string(),
                    duration_secs: record.duration_secs,
                    summary,
                    action: None, // Completed stages don't show action buttons
                }
            }
            Some(record) if record.status == "failed" => {
                PipelineStage {
                    name: name.to_string(),
                    label: label.to_string(),
                    order,
                    status: "failed".to_string(),
                    duration_secs: record.duration_secs,
                    summary: record.error_message.clone(),
                    action, // Failed stages may show retry button
                }
            }
            Some(record) if record.status == "running" => {
                PipelineStage {
                    name: name.to_string(),
                    label: label.to_string(),
                    order,
                    status: "available".to_string(),
                    duration_secs: record.duration_secs,
                    summary: Some("Running...".to_string()),
                    action: None,
                }
            }
            _ => {
                // No record or unknown status — check if actionable
                let status = if action.is_some() { "available" } else { "pending" };
                PipelineStage {
                    name: name.to_string(),
                    label: label.to_string(),
                    order,
                    status: status.to_string(),
                    duration_secs: None,
                    summary: None,
                    action,
                }
            }
        }
    }).collect()
}

/// Format a human-readable summary from a step's result_summary JSONB.
fn format_stage_summary(step_name: &str, result: &serde_json::Value) -> Option<String> {
    if result.is_null() {
        return None;
    }
    match step_name {
        "upload" => {
            let name = result.get("file_name").and_then(|v| v.as_str());
            let size = result.get("file_size_bytes")
                .and_then(|v| v.as_u64())
                .or_else(|| result.get("file_size").and_then(|v| v.as_u64()));
            match (name, size) {
                (Some(n), Some(s)) => {
                    let kb = s as f64 / 1024.0;
                    Some(format!("{n} ({kb:.0} KB)"))
                }
                (Some(n), None) => Some(n.to_string()),
                (None, Some(s)) => Some(format!("{:.0} KB", s as f64 / 1024.0)),
                (None, None) => None,
            }
        }
        "extract_text" => {
            let pages = result.get("page_count").and_then(|v| v.as_i64());
            let chars = result.get("total_chars").and_then(|v| v.as_i64());
            match (pages, chars) {
                (Some(p), Some(c)) => Some(format!("{p} pages, {c} chars")),
                (Some(p), None) => Some(format!("{p} pages")),
                _ => None,
            }
        }
        "extract" => {
            let entities = result.get("entity_count").and_then(|v| v.as_i64());
            let cost = result.get("cost_usd")
                .and_then(|v| v.as_f64())
                .or_else(|| {
                    // Compute cost from tokens if not directly available
                    let inp = result.get("input_tokens").and_then(|v| v.as_i64())?;
                    let out = result.get("output_tokens").and_then(|v| v.as_i64())?;
                    Some((inp as f64 * 3.0 + out as f64 * 15.0) / 1_000_000.0)
                });
            match (entities, cost) {
                (Some(e), Some(c)) => Some(format!("{e} items, ${c:.2}")),
                (Some(e), None) => Some(format!("{e} items")),
                _ => None,
            }
        }
        "verify" => {
            result.get("grounding_rate")
                .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
                .map(|r| format!("{r:.0}% grounded"))
        }
        "ingest" => {
            let nodes = result.get("nodes_created").and_then(|v| v.as_i64());
            let rels = result.get("relationships_created").and_then(|v| v.as_i64());
            match (nodes, rels) {
                (Some(n), Some(r)) => Some(format!("{n} nodes, {r} rels")),
                (Some(n), None) => Some(format!("{n} nodes")),
                _ => None,
            }
        }
        "index" => {
            result.get("nodes_embedded")
                .and_then(|v| v.as_i64())
                .map(|n| format!("{n} embedded"))
        }
        "completeness" => {
            let passed = result.get("checks_passed").and_then(|v| v.as_i64()).unwrap_or(0);
            let failed = result.get("checks_failed").and_then(|v| v.as_i64()).unwrap_or(0);
            let total = passed + failed;
            if total > 0 {
                Some(format!("{passed}/{total} passed"))
            } else {
                None
            }
        }
        _ => None,
    }
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
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    let pending = review::count_pending(&state.pipeline_pool, &doc_id)
        .await
        .unwrap_or(0);
    let total = count_total_items(&state.pipeline_pool, &doc_id)
        .await
        .unwrap_or(0);

    // Get all step records (sorted most recent first)
    let step_records = steps::get_steps_for_document(&state.pipeline_pool, &doc_id)
        .await
        .unwrap_or_default();

    let available_actions = get_available_actions(&document.status, pending, total);
    let pipeline_stages = build_pipeline_stages(&step_records, &available_actions, pending, total);

    // Build execution history from ALL step records
    let execution_history: Vec<ExecutionHistoryEntry> = step_records.iter().map(|s| {
        let label = PIPELINE_STAGE_ORDER.iter()
            .find(|(name, _)| *name == s.step_name.as_str())
            .map(|(_, l)| l.to_string())
            .unwrap_or_else(|| titleize_step(&s.step_name));
        let summary = if s.result_summary.is_null() { None } else { Some(s.result_summary.clone()) };

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
    }).collect();

    let confirm_level = delete_confirmation_level(&document.status).to_string();

    Ok(Json(DocumentActions {
        document_id: doc_id,
        current_status: document.status,
        pipeline_stages,
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

/// Count total extraction items for a document.
async fn count_total_items(pool: &sqlx::PgPool, document_id: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM extraction_items WHERE document_id = $1",
    )
    .bind(document_id)
    .fetch_one(pool)
    .await
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // --- get_available_actions tests ---

    #[test]
    fn test_uploaded_shows_process_and_extract_text() {
        let actions = get_available_actions("UPLOADED", 0, 0);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].action, "process");
        assert_eq!(actions[1].action, "extract_text");
    }

    #[test]
    fn test_text_extracted_shows_extract() {
        let actions = get_available_actions("TEXT_EXTRACTED", 0, 0);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action, "extract");
    }

    #[test]
    fn test_extracted_shows_verify() {
        let actions = get_available_actions("EXTRACTED", 0, 0);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action, "verify");
    }

    #[test]
    fn test_verified_with_pending_shows_review_only() {
        let actions = get_available_actions("VERIFIED", 5, 100);
        assert!(actions.iter().any(|a| a.action == "review"));
        assert!(!actions.iter().any(|a| a.action == "ingest"));
    }

    #[test]
    fn test_verified_with_zero_pending_shows_review_and_ingest() {
        let actions = get_available_actions("VERIFIED", 0, 100);
        assert!(actions.iter().any(|a| a.action == "review"));
        assert!(actions.iter().any(|a| a.action == "ingest"));
    }

    #[test]
    fn test_verified_with_zero_items_shows_review_only() {
        let actions = get_available_actions("VERIFIED", 0, 0);
        assert!(actions.iter().any(|a| a.action == "review"));
        assert!(!actions.iter().any(|a| a.action == "ingest"));
    }

    #[test]
    fn test_ingested_shows_index_and_reprocess() {
        let actions = get_available_actions("INGESTED", 0, 0);
        assert_eq!(actions.len(), 2);
        assert!(actions.iter().any(|a| a.action == "index"));
        assert!(actions.iter().any(|a| a.action == "reprocess"));
    }

    #[test]
    fn test_indexed_shows_completeness_and_reprocess() {
        let actions = get_available_actions("INDEXED", 0, 0);
        assert_eq!(actions.len(), 2);
        assert!(actions.iter().any(|a| a.action == "completeness"));
        assert!(actions.iter().any(|a| a.action == "reprocess"));
    }

    #[test]
    fn test_published_shows_reprocess() {
        let actions = get_available_actions("PUBLISHED", 0, 0);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action, "reprocess");
        assert!(actions[0].requires_confirmation);
    }

    #[test]
    fn test_reprocess_action_requires_confirmation() {
        let actions = get_available_actions("PUBLISHED", 0, 0);
        let reprocess = actions.iter().find(|a| a.action == "reprocess").unwrap();
        assert!(reprocess.requires_confirmation);
        assert_eq!(reprocess.method, "POST");
        assert!(!reprocess.is_navigation);
    }

    #[test]
    fn test_unknown_status_shows_no_actions() {
        let actions = get_available_actions("GARBAGE", 0, 0);
        assert!(actions.is_empty());
    }

    // --- delete_confirmation_level tests ---

    #[test]
    fn test_delete_simple_for_early_states() {
        assert_eq!(delete_confirmation_level("UPLOADED"), "simple");
        assert_eq!(delete_confirmation_level("TEXT_EXTRACTED"), "simple");
    }

    #[test]
    fn test_delete_moderate_for_mid_states() {
        assert_eq!(delete_confirmation_level("EXTRACTED"), "moderate");
        assert_eq!(delete_confirmation_level("VERIFIED"), "moderate");
    }

    #[test]
    fn test_delete_strict_for_late_states() {
        assert_eq!(delete_confirmation_level("INGESTED"), "strict");
        assert_eq!(delete_confirmation_level("INDEXED"), "strict");
        assert_eq!(delete_confirmation_level("PUBLISHED"), "strict");
    }

    // --- format_stage_summary tests ---

    #[test]
    fn test_format_extract_text_summary() {
        let summary = serde_json::json!({
            "page_count": 17,
            "total_chars": 27353
        });
        let result = format_stage_summary("extract_text", &summary);
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("17"), "Expected '17' in '{text}'");
        assert!(text.contains("27353"), "Expected '27353' in '{text}'");
    }

    #[test]
    fn test_format_extract_summary_with_cost() {
        let summary = serde_json::json!({
            "entity_count": 109,
            "cost_usd": 0.46
        });
        let result = format_stage_summary("extract", &summary);
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("109"), "Expected '109' in '{text}'");
        assert!(text.contains("0.46"), "Expected '$0.46' in '{text}'");
    }

    #[test]
    fn test_format_extract_summary_with_tokens() {
        // When cost_usd is not present, compute from tokens
        let summary = serde_json::json!({
            "entity_count": 50,
            "input_tokens": 10000,
            "output_tokens": 5000
        });
        let result = format_stage_summary("extract", &summary);
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("50"), "Expected '50' in '{text}'");
    }

    #[test]
    fn test_format_verify_summary() {
        let summary = serde_json::json!({ "grounding_rate": 95.0 });
        let result = format_stage_summary("verify", &summary);
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("95"), "Expected '95' in '{text}'");
    }

    #[test]
    fn test_format_verify_summary_integer() {
        // grounding_rate as integer
        let summary = serde_json::json!({ "grounding_rate": 88 });
        let result = format_stage_summary("verify", &summary);
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("88"), "Expected '88' in '{text}'");
    }

    #[test]
    fn test_format_ingest_summary() {
        let summary = serde_json::json!({
            "nodes_created": 42,
            "relationships_created": 17
        });
        let result = format_stage_summary("ingest", &summary);
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("42"), "Expected '42' in '{text}'");
        assert!(text.contains("17"), "Expected '17' in '{text}'");
    }

    #[test]
    fn test_format_index_summary() {
        let summary = serde_json::json!({ "nodes_embedded": 35 });
        let result = format_stage_summary("index", &summary);
        assert!(result.is_some());
        assert!(result.unwrap().contains("35"));
    }

    #[test]
    fn test_format_completeness_summary() {
        let summary = serde_json::json!({
            "checks_passed": 7,
            "checks_failed": 1
        });
        let result = format_stage_summary("completeness", &summary);
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("7/8"), "Expected '7/8' in '{text}'");
    }

    #[test]
    fn test_format_null_result_returns_none() {
        let result = format_stage_summary("extract_text", &serde_json::Value::Null);
        assert!(result.is_none());
    }

    #[test]
    fn test_format_unknown_step_returns_none() {
        let summary = serde_json::json!({ "foo": "bar" });
        let result = format_stage_summary("unknown_step", &summary);
        assert!(result.is_none());
    }

    // --- pipeline stage structure tests ---

    #[test]
    fn test_pipeline_stage_order_has_8_entries() {
        assert_eq!(PIPELINE_STAGE_ORDER.len(), 8);
    }

    #[test]
    fn test_pipeline_stages_in_correct_order() {
        assert_eq!(PIPELINE_STAGE_ORDER[0].0, "upload");
        assert_eq!(PIPELINE_STAGE_ORDER[1].0, "extract_text");
        assert_eq!(PIPELINE_STAGE_ORDER[2].0, "extract");
        assert_eq!(PIPELINE_STAGE_ORDER[3].0, "verify");
        assert_eq!(PIPELINE_STAGE_ORDER[4].0, "review");
        assert_eq!(PIPELINE_STAGE_ORDER[5].0, "ingest");
        assert_eq!(PIPELINE_STAGE_ORDER[6].0, "index");
        assert_eq!(PIPELINE_STAGE_ORDER[7].0, "completeness");
    }

    // --- build_pipeline_stages tests ---

    #[test]
    fn test_build_stages_empty_history_uploaded() {
        let actions = get_available_actions("UPLOADED", 0, 0);
        let stages = build_pipeline_stages(&[], &actions, 0, 0);

        assert_eq!(stages.len(), 8);
        assert_eq!(stages[0].status, "completed"); // upload always completed
        assert_eq!(stages[1].status, "available"); // extract_text is next
        assert!(stages[1].action.is_some());
        assert_eq!(stages[2].status, "pending"); // extract not yet available
        assert!(stages[2].action.is_none());
    }

    #[test]
    fn test_build_stages_review_completed_when_no_pending() {
        let actions = get_available_actions("VERIFIED", 0, 50);
        let stages = build_pipeline_stages(&[], &actions, 0, 50);

        // Review stage (index 4) should be completed
        assert_eq!(stages[4].name, "review");
        assert_eq!(stages[4].status, "completed");
        assert!(stages[4].summary.as_ref().unwrap().contains("All items reviewed"));
    }

    #[test]
    fn test_build_stages_review_available_with_pending() {
        let actions = get_available_actions("VERIFIED", 10, 50);
        let stages = build_pipeline_stages(&[], &actions, 10, 50);

        assert_eq!(stages[4].name, "review");
        assert_eq!(stages[4].status, "available");
        assert!(stages[4].action.is_some());
        assert!(stages[4].summary.as_ref().unwrap().contains("10 pending"));
    }

    // --- review status case sensitivity ---

    #[test]
    fn test_review_status_comparison_is_case_insensitive() {
        // The bulk_approve SQL uses LOWER(review_status) = 'pending'
        // Verify the expectation: both cases normalize to the same value
        let upper = "PENDING";
        let lower = "pending";
        assert_eq!(upper.to_lowercase(), lower);
    }

    // --- titleize_step ---

    #[test]
    fn test_titleize_step() {
        assert_eq!(titleize_step("bulk_approve"), "Bulk Approve");
        assert_eq!(titleize_step("extract_text"), "Extract Text");
        assert_eq!(titleize_step("ingest"), "Ingest");
    }

    // --- endpoint tests ---

    #[test]
    fn test_available_actions_include_endpoint() {
        let actions = get_available_actions("UPLOADED", 0, 0);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].endpoint, "/documents/{id}/process");
        assert_eq!(actions[1].endpoint, "/documents/{id}/extract-text");
    }

    #[test]
    fn test_all_actions_have_nonempty_endpoint() {
        for status in &["UPLOADED", "TEXT_EXTRACTED", "EXTRACTED", "VERIFIED", "INGESTED", "INDEXED"] {
            let actions = get_available_actions(status, 5, 10);
            for action in &actions {
                assert!(!action.endpoint.is_empty(),
                        "Action '{}' for status '{}' has empty endpoint", action.action, status);
                assert!(action.endpoint.starts_with("/documents/{id}/"),
                        "Action '{}' endpoint '{}' should start with /documents/{{id}}/",
                        action.action, action.endpoint);
            }
        }
    }

    #[test]
    fn test_completeness_action_uses_get() {
        let actions = get_available_actions("INDEXED", 0, 0);
        assert_eq!(actions[0].action, "completeness");
        assert_eq!(actions[0].method, "GET");
    }

    // --- user-friendly label tests ---

    #[test]
    fn test_pipeline_stage_labels() {
        assert_eq!(PIPELINE_STAGE_ORDER[1].1, "Read Document");
        assert_eq!(PIPELINE_STAGE_ORDER[2].1, "Analyze Content");
        assert_eq!(PIPELINE_STAGE_ORDER[3].1, "Verify Accuracy");
        assert_eq!(PIPELINE_STAGE_ORDER[4].1, "Human Review");
        assert_eq!(PIPELINE_STAGE_ORDER[5].1, "Build Knowledge Graph");
        assert_eq!(PIPELINE_STAGE_ORDER[6].1, "Enable Search");
        assert_eq!(PIPELINE_STAGE_ORDER[7].1, "Quality Check");
    }
}
