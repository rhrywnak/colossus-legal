//! POST /api/admin/pipeline/documents/:id/verify — PageGrounder verification.
//!
//! Searches the document's PDF for each extraction item's verbatim quote,
//! updating grounding_status and grounded_page in the database.

use axum::{extract::Path, extract::State, Json};
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository::{self, steps, ExtractionItemRecord};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub document_id: String,
    pub status: String,
    pub total_items: usize,
    pub grounded_exact: usize,
    pub grounded_normalized: usize,
    pub not_found: usize,
    pub skipped_no_quote: usize,
}

/// POST /api/admin/pipeline/documents/:id/verify
pub async fn verify_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<VerifyResponse>, AppError> {
    require_admin(&user)?;
    let start = std::time::Instant::now();
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST verify");

    let step_id = steps::record_step_start(
        &state.pipeline_pool, &doc_id, "verify", &user.username, &serde_json::json!({}),
    ).await.map_err(|e| AppError::Internal { message: format!("Step logging: {e}") })?;

    // 1. Fetch document
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound { message: format!("Document '{doc_id}' not found") })?;

    // 2. Check status
    if document.status != "EXTRACTED" && document.status != "VERIFIED" {
        return Err(AppError::Conflict {
            message: format!("Cannot verify: status is '{}', expected 'EXTRACTED'", document.status),
            details: serde_json::json!({ "status": document.status }),
        });
    }

    // 3. Fetch items with verbatim quotes
    let items = pipeline_repository::get_items_with_quotes(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;

    // 4. Build full path and verify PDF exists
    let full_path = format!(
        "{}/{}",
        state.config.document_storage_path.trim_end_matches('/'),
        document.file_path
    );
    if !tokio::fs::try_exists(&full_path).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("PDF file not found: {}", document.file_path),
        });
    }

    // 5. Collect snippets for grounding
    let snippets: Vec<String> = items
        .iter()
        .filter_map(|item| item.verbatim_quote.clone())
        .collect();

    // 6. Run PageGrounder in blocking thread
    let pdf_path = full_path.clone();
    let grounding_results = tokio::task::spawn_blocking(move || {
        run_grounding(&pdf_path, &snippets)
    })
    .await
    .map_err(|e| AppError::Internal { message: format!("Grounding task panicked: {e}") })??;

    // 7. Update each item's grounding status
    let (mut exact, mut normalized, mut not_found) = (0usize, 0usize, 0usize);
    let items_with_quotes: Vec<&ExtractionItemRecord> = items
        .iter()
        .filter(|i| i.verbatim_quote.is_some())
        .collect();

    for (i, result) in grounding_results.iter().enumerate() {
        let item = items_with_quotes[i];
        let (status_str, page) = match result.match_type {
            colossus_pdf::MatchType::Exact => {
                exact += 1;
                ("exact", result.page_number.map(|p| p as i32))
            }
            colossus_pdf::MatchType::Normalized => {
                normalized += 1;
                ("normalized", result.page_number.map(|p| p as i32))
            }
            colossus_pdf::MatchType::NotFound => {
                not_found += 1;
                ("not_found", None)
            }
        };

        pipeline_repository::update_item_grounding(
            &state.pipeline_pool, item.id, status_str, page,
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update item {}: {e}", item.id),
        })?;
    }

    // 8. Update document status
    pipeline_repository::update_document_status(&state.pipeline_pool, &doc_id, "VERIFIED")
        .await
        .map_err(|e| AppError::Internal { message: format!("Failed to update status: {e}") })?;

    let total_all = pipeline_repository::get_all_items(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .len();
    let skipped = total_all - items_with_quotes.len();

    tracing::info!(doc_id = %doc_id, exact, normalized, not_found, skipped, "Verification complete");

    log_admin_action(
        &state.audit_repo, &user.username, "pipeline.document.verify",
        Some("document"), Some(&doc_id),
        Some(serde_json::json!({ "exact": exact, "normalized": normalized, "not_found": not_found })),
    )
    .await;

    let grounding_rate = if total_all > 0 {
        ((exact + normalized) as f64 / total_all as f64 * 100.0).round()
    } else { 0.0 };
    steps::record_step_complete(
        &state.pipeline_pool, step_id, start.elapsed().as_secs_f64(),
        &serde_json::json!({"grounding_rate": grounding_rate, "exact": exact, "normalized": normalized, "not_found": not_found}),
    ).await.ok();

    Ok(Json(VerifyResponse {
        document_id: doc_id,
        status: "VERIFIED".to_string(),
        total_items: total_all,
        grounded_exact: exact,
        grounded_normalized: normalized,
        not_found,
        skipped_no_quote: skipped,
    }))
}

/// Run PDF grounding (sync — called from spawn_blocking).
fn run_grounding(
    pdf_path: &str,
    snippets: &[String],
) -> Result<Vec<colossus_pdf::GroundingResult>, AppError> {
    let mut extractor = colossus_pdf::PdfTextExtractor::open(pdf_path).map_err(|e| {
        AppError::Internal { message: format!("Failed to open PDF: {e}") }
    })?;
    // extract_all_pages() must be called before grounding to load page text
    extractor.extract_all_pages().map_err(|e| {
        AppError::Internal { message: format!("Failed to extract PDF pages: {e}") }
    })?;

    let snippet_refs: Vec<&str> = snippets.iter().map(|s| s.as_str()).collect();
    let mut grounder = colossus_pdf::PageGrounder::new(&mut extractor);
    let results = grounder.ground_snippets(&snippet_refs).map_err(|e| {
        AppError::Internal { message: format!("Grounding failed: {e}") }
    })?;
    Ok(results)
}
