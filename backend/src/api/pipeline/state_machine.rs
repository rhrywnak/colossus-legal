//! Pipeline state machine — single source of truth for document status
//! transitions and available actions.
//!
//! The frontend calls GET /api/admin/pipeline/documents/:id/actions to
//! determine what buttons to show. It never checks status strings itself.
//!
//! ## Rust Learning: Centralizing business logic
//!
//! Previously, both the frontend and backend checked status strings to decide
//! what was allowed. This module is the SINGLE source of truth. The frontend
//! renders what this module returns — no status comparisons for decisions.

use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::pipeline_repository::{self, steps};
use crate::state::AppState;

use axum::{extract::Path, extract::State, Json};

/// An action the user can take on a document in its current state.
#[derive(Debug, Serialize)]
pub struct AvailableAction {
    /// Action identifier (matches the API endpoint name)
    pub action: String,
    /// Human-readable label for the button
    pub label: String,
    /// HTTP method to call (POST or GET)
    pub method: String,
    /// Whether this action requires confirmation
    pub requires_confirmation: bool,
    /// What this action does
    pub description: String,
    /// Whether this action navigates to a tab instead of calling an API
    pub is_navigation: bool,
}

/// Full response for GET /documents/:id/actions.
#[derive(Debug, Serialize)]
pub struct DocumentActions {
    pub document_id: String,
    pub current_status: String,
    pub available_actions: Vec<AvailableAction>,
    pub completed_steps: Vec<CompletedStep>,
    /// Confirmation level for delete: "simple", "moderate", or "strict"
    pub delete_confirmation_level: String,
}

/// A completed pipeline step with summary data.
#[derive(Debug, Serialize)]
pub struct CompletedStep {
    pub step_name: String,
    pub label: String,
    pub status: String,
    pub duration_secs: Option<f64>,
    pub result_summary: Option<serde_json::Value>,
    pub error_message: Option<String>,
}

// ── State machine core ──────────────────────────────────────────

/// Determine available actions based on document status and review state.
///
/// ## Rust Learning: Match as state machine
///
/// Each arm of the match is a state in the pipeline. The function returns
/// the actions available in that state. The frontend just renders buttons
/// for whatever this returns — zero decision-making on the client side.
fn get_available_actions(
    document_status: &str,
    pending_review_count: i64,
    total_item_count: i64,
) -> Vec<AvailableAction> {
    match document_status {
        "UPLOADED" => vec![
            action("extract_text", "Extract Text", "POST", false, false,
                   "Extract text from the PDF document"),
        ],
        "TEXT_EXTRACTED" => vec![
            action("extract", "LLM Extract", "POST", false, false,
                   "Run LLM extraction to identify entities and relationships"),
        ],
        "EXTRACTED" => vec![
            action("verify", "Verify / Ground", "POST", false, false,
                   "Verify extracted quotes against document text"),
        ],
        "VERIFIED" => {
            let mut actions = vec![
                action("review", "Review Items", "GET", false, true,
                       "Review and approve extracted items"),
            ];
            // Show ingest button when all items are reviewed
            if pending_review_count == 0 && total_item_count > 0 {
                actions.push(
                    action("ingest", "Ingest to Graph", "POST", true, false,
                           "Write approved items to the knowledge graph"),
                );
            }
            actions
        },
        "INGESTED" => vec![
            action("index", "Index Embeddings", "POST", false, false,
                   "Generate vector embeddings for search"),
        ],
        "INDEXED" => vec![
            action("completeness", "Check Completeness", "POST", false, false,
                   "Verify all items are in the graph and indexed"),
        ],
        "PUBLISHED" => vec![], // Terminal state
        _ => vec![],
    }
}

/// Determine delete confirmation level based on document status.
fn delete_confirmation_level(status: &str) -> &'static str {
    match status {
        "UPLOADED" | "TEXT_EXTRACTED" => "simple",
        "EXTRACTED" | "VERIFIED" => "moderate",
        _ => "strict", // INGESTED, INDEXED, PUBLISHED
    }
}

fn action(
    name: &str, label: &str, method: &str, confirm: bool, is_nav: bool, desc: &str,
) -> AvailableAction {
    AvailableAction {
        action: name.to_string(),
        label: label.to_string(),
        method: method.to_string(),
        requires_confirmation: confirm,
        description: desc.to_string(),
        is_navigation: is_nav,
    }
}

fn step_label(name: &str) -> String {
    match name {
        "upload" => "Upload",
        "extract_text" => "Extract Text",
        "extract" => "LLM Extract",
        "verify" => "Verify / Ground",
        "review" => "Review",
        "ingest" => "Ingest to Graph",
        "index" => "Index Embeddings",
        "completeness" => "Completeness Check",
        _ => name,
    }.to_string()
}

// ── Handler ─────────────────────────────────────────────────────

/// GET /api/admin/pipeline/documents/:id/actions
///
/// Returns the document's current status, available actions, and completed
/// steps. The frontend uses this to render buttons — no client-side
/// status checks needed.
pub async fn get_document_actions(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<DocumentActions>, AppError> {
    require_admin(&user)?;

    // Get document
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    // Get review counts for the document
    let pending = pipeline_repository::review::count_pending(&state.pipeline_pool, &doc_id)
        .await
        .unwrap_or(0);
    let total = count_total_items(&state.pipeline_pool, &doc_id)
        .await
        .unwrap_or(0);

    // Get completed steps
    let step_records = steps::get_steps_for_document(&state.pipeline_pool, &doc_id)
        .await
        .unwrap_or_default();

    let completed_steps: Vec<CompletedStep> = step_records.iter().map(|s| {
        // result_summary is serde_json::Value — wrap in Option (null → None)
        let summary = if s.result_summary.is_null() {
            None
        } else {
            Some(s.result_summary.clone())
        };

        CompletedStep {
            step_name: s.step_name.clone(),
            label: step_label(&s.step_name),
            status: s.status.clone(),
            duration_secs: s.duration_secs,
            result_summary: summary,
            error_message: s.error_message.clone(),
        }
    }).collect();

    let available_actions = get_available_actions(&document.status, pending, total);
    let confirm_level = delete_confirmation_level(&document.status).to_string();

    Ok(Json(DocumentActions {
        document_id: doc_id,
        current_status: document.status,
        available_actions,
        completed_steps,
        delete_confirmation_level: confirm_level,
    }))
}

/// Count total extraction items for a document (across all runs).
async fn count_total_items(pool: &sqlx::PgPool, document_id: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM extraction_items WHERE document_id = $1",
    )
    .bind(document_id)
    .fetch_one(pool)
    .await
}
