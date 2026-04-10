//! POST /api/admin/pipeline/documents/:id/verify — PageGrounder verification.
//!
//! Searches the document's PDF for each extraction item's grounding snippet,
//! using per-entity-type grounding modes from the extraction schema.
//!
//! ## Grounding Modes
//!
//! - **Verbatim** — search for the item's `verbatim_quote` in the PDF
//! - **NameMatch** — search for the entity label/name in the PDF
//! - **HeadingMatch** — search for the entity label or legal_basis in the PDF
//! - **Derived** — no PDF search; mark as "derived" (provenance-based)
//! - **None** — no PDF search; mark as "unverified"
//!
//! If the schema cannot be loaded, all items fall back to Verbatim behavior
//! for backward compatibility.

use std::collections::HashMap;
use std::path::Path;

use axum::{extract::Path as AxumPath, extract::State, Json};
use colossus_extract::GroundingMode;
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
    pub derived: usize,
    pub unverified: usize,
    pub name_matched: usize,
    pub heading_matched: usize,
}

/// POST /api/admin/pipeline/documents/:id/verify
pub async fn verify_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
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

    // 3. Load extraction schema for grounding mode lookup.
    //    If schema loading fails, fall back to treating everything as Verbatim.
    let grounding_modes = load_grounding_modes(&state, &doc_id).await;

    // 4. Fetch ALL items (not just those with quotes)
    let items = pipeline_repository::get_all_items(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;

    // 5. Build full path and verify PDF exists
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

    // 6. Categorize items by grounding mode and build combined snippets.
    //    Each snippet-based item gets an entry in `snippets` and a back-reference
    //    in `snippet_items` so we can distribute PageGrounder results.
    let mut snippets: Vec<String> = Vec::new();
    let mut snippet_items: Vec<SnippetMeta> = Vec::new();
    let mut derived_items: Vec<i32> = Vec::new();
    let mut none_items: Vec<i32> = Vec::new();
    let mut missing_quote_items: Vec<i32> = Vec::new();

    for item in &items {
        let mode = grounding_modes
            .get(&item.entity_type)
            .unwrap_or(&GroundingMode::Verbatim);

        match mode {
            GroundingMode::Derived => {
                derived_items.push(item.id);
            }
            GroundingMode::None => {
                none_items.push(item.id);
            }
            GroundingMode::Verbatim => {
                if let Some(quote) = item.verbatim_quote.as_deref().filter(|q| !q.is_empty()) {
                    snippets.push(quote.to_string());
                    snippet_items.push(SnippetMeta { item_id: item.id, kind: SnippetKind::Verbatim });
                } else {
                    missing_quote_items.push(item.id);
                }
            }
            GroundingMode::NameMatch => {
                let label = extract_name_label(item);
                if !label.is_empty() {
                    snippets.push(label);
                    snippet_items.push(SnippetMeta { item_id: item.id, kind: SnippetKind::NameMatch });
                } else {
                    missing_quote_items.push(item.id);
                }
            }
            GroundingMode::HeadingMatch => {
                let label = extract_heading_label(item);
                if !label.is_empty() {
                    snippets.push(label);
                    snippet_items.push(SnippetMeta { item_id: item.id, kind: SnippetKind::HeadingMatch });
                } else {
                    missing_quote_items.push(item.id);
                }
            }
        }
    }

    // 7. Run PageGrounder in blocking thread for all snippet-based items
    let pdf_path = full_path.clone();
    let grounding_results = tokio::task::spawn_blocking(move || {
        run_grounding(&pdf_path, &snippets)
    })
    .await
    .map_err(|e| AppError::Internal { message: format!("Grounding task panicked: {e}") })??;

    // 8. Distribute grounding results and update DB
    let (mut exact, mut normalized, mut not_found) = (0usize, 0usize, 0usize);
    let (mut name_matched, mut heading_matched) = (0usize, 0usize);

    for (i, result) in grounding_results.iter().enumerate() {
        let meta = &snippet_items[i];
        let (status_str, page) = match result.match_type {
            colossus_pdf::MatchType::Exact => {
                exact += 1;
                if matches!(meta.kind, SnippetKind::NameMatch) { name_matched += 1; }
                if matches!(meta.kind, SnippetKind::HeadingMatch) { heading_matched += 1; }
                ("exact", result.page_number.map(|p| p as i32))
            }
            colossus_pdf::MatchType::Normalized => {
                normalized += 1;
                if matches!(meta.kind, SnippetKind::NameMatch) { name_matched += 1; }
                if matches!(meta.kind, SnippetKind::HeadingMatch) { heading_matched += 1; }
                ("normalized", result.page_number.map(|p| p as i32))
            }
            colossus_pdf::MatchType::NotFound => {
                not_found += 1;
                ("not_found", None)
            }
        };

        pipeline_repository::update_item_grounding(
            &state.pipeline_pool, meta.item_id, status_str, page,
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update item {}: {e}", meta.item_id),
        })?;
    }

    // 9. Update derived items
    for item_id in &derived_items {
        pipeline_repository::update_item_grounding(
            &state.pipeline_pool, *item_id, "derived", None,
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update derived item {item_id}: {e}"),
        })?;
    }

    // 10. Update none-mode items
    for item_id in &none_items {
        pipeline_repository::update_item_grounding(
            &state.pipeline_pool, *item_id, "unverified", None,
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update unverified item {item_id}: {e}"),
        })?;
    }

    // 11. Update missing-quote items
    if !missing_quote_items.is_empty() {
        tracing::warn!(
            doc_id = %doc_id,
            count = missing_quote_items.len(),
            "Items missing required grounding snippet"
        );
    }
    for item_id in &missing_quote_items {
        pipeline_repository::update_item_grounding(
            &state.pipeline_pool, *item_id, "missing_quote", None,
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update missing-quote item {item_id}: {e}"),
        })?;
    }

    // 12. Update document status
    pipeline_repository::update_document_status(&state.pipeline_pool, &doc_id, "VERIFIED")
        .await
        .map_err(|e| AppError::Internal { message: format!("Failed to update status: {e}") })?;

    let derived_count = derived_items.len();
    let unverified_count = none_items.len();
    let skipped_count = missing_quote_items.len();

    tracing::info!(
        doc_id = %doc_id, exact, normalized, not_found,
        derived = derived_count, unverified = unverified_count,
        name_matched, heading_matched, skipped = skipped_count,
        "Verification complete"
    );

    log_admin_action(
        &state.audit_repo, &user.username, "pipeline.document.verify",
        Some("document"), Some(&doc_id),
        Some(serde_json::json!({
            "exact": exact, "normalized": normalized, "not_found": not_found,
            "derived": derived_count, "unverified": unverified_count,
            "name_matched": name_matched, "heading_matched": heading_matched,
            "skipped_no_quote": skipped_count,
        })),
    )
    .await;

    let total = items.len();
    let grounding_rate = if total > 0 {
        ((exact + normalized) as f64 / total as f64 * 100.0).round()
    } else {
        0.0
    };
    steps::record_step_complete(
        &state.pipeline_pool, step_id, start.elapsed().as_secs_f64(),
        &serde_json::json!({
            "grounding_rate": grounding_rate,
            "exact": exact, "normalized": normalized, "not_found": not_found,
            "derived": derived_count, "unverified": unverified_count,
            "name_matched": name_matched, "heading_matched": heading_matched,
        }),
    ).await.ok();

    Ok(Json(VerifyResponse {
        document_id: doc_id,
        status: "VERIFIED".to_string(),
        total_items: total,
        grounded_exact: exact,
        grounded_normalized: normalized,
        not_found,
        skipped_no_quote: skipped_count,
        derived: derived_count,
        unverified: unverified_count,
        name_matched,
        heading_matched,
    }))
}

// ── Helpers ──────────────────────────────────────────────────────

/// Tracks which item a snippet belongs to and what kind of grounding it is.
struct SnippetMeta {
    item_id: i32,
    kind: SnippetKind,
}

/// The grounding mode category for a snippet in the combined batch.
enum SnippetKind {
    Verbatim,
    NameMatch,
    HeadingMatch,
}

/// Load grounding modes from the extraction schema.
///
/// Returns an empty map on failure (all items default to Verbatim).
/// This ensures backward compatibility for documents uploaded before F2.
async fn load_grounding_modes(
    state: &AppState,
    doc_id: &str,
) -> HashMap<String, GroundingMode> {
    let pipe_config = match pipeline_repository::get_pipeline_config(&state.pipeline_pool, doc_id).await {
        Ok(Some(cfg)) => cfg,
        Ok(None) => {
            tracing::warn!(doc_id = %doc_id, "No pipeline config found — defaulting all items to Verbatim");
            return HashMap::new();
        }
        Err(e) => {
            tracing::warn!(doc_id = %doc_id, error = %e, "Failed to load pipeline config — defaulting all items to Verbatim");
            return HashMap::new();
        }
    };

    let schema_path = format!("{}/{}", state.config.extraction_schema_dir, pipe_config.schema_file);
    let schema = match colossus_extract::ExtractionSchema::from_file(Path::new(&schema_path)) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                doc_id = %doc_id, schema = %pipe_config.schema_file, error = %e,
                "Failed to load extraction schema — defaulting all items to Verbatim"
            );
            return HashMap::new();
        }
    };

    schema
        .entity_types
        .iter()
        .map(|et| (et.name.clone(), et.grounding_mode.clone()))
        .collect()
}

/// Extract a name label from an item for NameMatch grounding.
///
/// Tries `label`, then common name properties.
fn extract_name_label(item: &ExtractionItemRecord) -> String {
    item.item_data["label"].as_str()
        .or_else(|| item.item_data["properties"]["full_name"].as_str())
        .or_else(|| item.item_data["properties"]["party_name"].as_str())
        .unwrap_or("")
        .to_string()
}

/// Extract a heading label from an item for HeadingMatch grounding.
///
/// Tries `label`, then heading-specific properties.
fn extract_heading_label(item: &ExtractionItemRecord) -> String {
    item.item_data["label"].as_str()
        .or_else(|| item.item_data["properties"]["legal_basis"].as_str())
        .or_else(|| item.item_data["properties"]["count_name"].as_str())
        .unwrap_or("")
        .to_string()
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
