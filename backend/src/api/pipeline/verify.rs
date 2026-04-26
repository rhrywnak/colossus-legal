//! POST /api/admin/pipeline/documents/:id/verify — Canonical text verification.
//!
//! Searches the document's canonical stored text (`document_text` table) for
//! each extraction item's grounding snippet. This is format-agnostic: text PDFs,
//! scanned PDFs (OCR), and future formats all verify against the same canonical
//! text the LLM saw during extraction.
//!
//! Replaces the previous PageGrounder approach which opened the original PDF —
//! that failed for scanned documents because the PDF has no native text layer.
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

use super::canonical_verifier::{find_in_canonical_text, CanonicalMatchType};
use crate::auth::{require_admin, AuthUser};
use crate::models::document_status::{STATUS_EXTRACTED, STATUS_VERIFIED};
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

/// Core logic for verification — callable from handler AND process endpoint.
///
/// Runs PDF grounding for all extraction items, updates grounding status.
/// Does NOT check document status — caller is responsible for validation.
pub(crate) async fn run_verify(
    state: &AppState,
    doc_id: &str,
    username: &str,
) -> Result<VerifyResponse, AppError> {
    let start = std::time::Instant::now();

    let step_id = steps::record_step_start(
        &state.pipeline_pool,
        doc_id,
        "verify",
        username,
        &serde_json::json!({}),
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Step logging: {e}"),
    })?;

    // 1. Fetch document (existence guard — content is no longer read here;
    //    canonical text comes from document_text in step 5).
    let _document = pipeline_repository::get_document(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    // 2. Load extraction schema for grounding mode lookup. Failure is
    //    fatal: without the schema we'd default every entity to Verbatim,
    //    silently corrupting Party / LegalCount / Harm grounding.
    let grounding_modes = load_grounding_modes(
        &state.pipeline_pool,
        &state.config.extraction_schema_dir,
        doc_id,
    )
    .await
    .map_err(|e| {
        tracing::error!(
            document_id = %doc_id, error = %e,
            "Verify cannot proceed without grounding modes"
        );
        AppError::Internal { message: e }
    })?;

    // 4. Fetch ALL items (not just those with quotes)
    let items = pipeline_repository::get_all_items(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?;

    // 5. Load canonical text from document_text table
    let document_text_rows = pipeline_repository::get_document_text(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to load document text: {e}"),
        })?;

    if document_text_rows.is_empty() {
        return Err(AppError::Internal {
            message: format!("No canonical text found for document '{doc_id}'. Was ExtractText run?"),
        });
    }

    // Convert to (page_number, text_content) tuples for the verifier
    let document_pages: Vec<(u32, String)> = document_text_rows
        .into_iter()
        .map(|row| (row.page_number as u32, row.text_content))
        .collect();

    // 6. Categorize items by grounding mode using the extracted pure function.
    //    Then build combined snippets for PageGrounder.
    let categorization = categorize_items_for_grounding(&items, &grounding_modes);

    let mut snippets: Vec<String> = Vec::new();
    let mut snippet_items: Vec<SnippetMeta> = Vec::new();
    let derived_items = categorization.derived_item_ids;
    let none_items = categorization.none_item_ids;
    let missing_quote_items = categorization.missing_quote_item_ids;

    for (item_id, quote) in &categorization.verbatim_items {
        snippets.push(quote.clone());
        snippet_items.push(SnippetMeta {
            item_id: *item_id,
            kind: SnippetKind::Verbatim,
        });
    }
    for (item_id, name) in &categorization.name_match_items {
        snippets.push(name.clone());
        snippet_items.push(SnippetMeta {
            item_id: *item_id,
            kind: SnippetKind::NameMatch,
        });
    }
    for (item_id, heading) in &categorization.heading_match_items {
        snippets.push(heading.clone());
        snippet_items.push(SnippetMeta {
            item_id: *item_id,
            kind: SnippetKind::HeadingMatch,
        });
    }

    // 7. Search each snippet against canonical text and update DB
    let (mut exact, mut normalized, mut not_found) = (0usize, 0usize, 0usize);
    let (mut name_matched, mut heading_matched) = (0usize, 0usize);

    for (i, snippet) in snippets.iter().enumerate() {
        let meta = &snippet_items[i];
        let result = find_in_canonical_text(snippet, &document_pages);

        let (status_str, page) = match result.match_type {
            CanonicalMatchType::Exact => {
                exact += 1;
                if matches!(meta.kind, SnippetKind::NameMatch) {
                    name_matched += 1;
                }
                if matches!(meta.kind, SnippetKind::HeadingMatch) {
                    heading_matched += 1;
                }
                ("exact", result.page_number.map(|p| p as i32))
            }
            CanonicalMatchType::Normalized => {
                normalized += 1;
                if matches!(meta.kind, SnippetKind::NameMatch) {
                    name_matched += 1;
                }
                if matches!(meta.kind, SnippetKind::HeadingMatch) {
                    heading_matched += 1;
                }
                ("normalized", result.page_number.map(|p| p as i32))
            }
            CanonicalMatchType::NotFound => {
                not_found += 1;
                ("not_found", None)
            }
        };

        pipeline_repository::update_item_grounding(
            &state.pipeline_pool,
            meta.item_id,
            status_str,
            page,
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update item {}: {e}", meta.item_id),
        })?;
    }

    // 9. Update derived items
    for item_id in &derived_items {
        pipeline_repository::update_item_grounding(&state.pipeline_pool, *item_id, "derived", None)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to update derived item {item_id}: {e}"),
            })?;
    }

    // 10. Update none-mode items
    for item_id in &none_items {
        pipeline_repository::update_item_grounding(
            &state.pipeline_pool,
            *item_id,
            "unverified",
            None,
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
            &state.pipeline_pool,
            *item_id,
            "missing_quote",
            None,
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update missing-quote item {item_id}: {e}"),
        })?;
    }

    // 12. Update document status
    pipeline_repository::update_document_status(&state.pipeline_pool, doc_id, STATUS_VERIFIED)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update status: {e}"),
        })?;

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
        &state.audit_repo,
        username,
        "pipeline.document.verify",
        Some("document"),
        Some(doc_id),
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
    if let Err(e) = steps::record_step_complete(
        &state.pipeline_pool,
        step_id,
        start.elapsed().as_secs_f64(),
        &serde_json::json!({
            "grounding_rate": grounding_rate,
            "exact": exact, "normalized": normalized, "not_found": not_found,
            "derived": derived_count, "unverified": unverified_count,
            "name_matched": name_matched, "heading_matched": heading_matched,
        }),
    )
    .await
    {
        tracing::error!(
            document_id = %doc_id,
            step_id = step_id,
            error = %e,
            "Failed to record verify step completion — audit trail gap"
        );
    }

    Ok(VerifyResponse {
        document_id: doc_id.to_string(),
        status: STATUS_VERIFIED.to_string(),
        total_items: total,
        grounded_exact: exact,
        grounded_normalized: normalized,
        not_found,
        skipped_no_quote: skipped_count,
        derived: derived_count,
        unverified: unverified_count,
        name_matched,
        heading_matched,
    })
}

/// POST /api/admin/pipeline/documents/:id/verify
///
/// HTTP handler — thin wrapper around `run_verify`.
/// Checks admin auth and status guard, then delegates to core logic.
pub async fn verify_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<Json<VerifyResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST verify");

    // Status guard
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    if document.status != STATUS_EXTRACTED && document.status != STATUS_VERIFIED {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot verify: status is '{}', expected '{STATUS_EXTRACTED}'",
                document.status
            ),
            details: serde_json::json!({ "status": document.status }),
        });
    }

    let result = run_verify(&state, &doc_id, &user.username).await?;
    Ok(Json(result))
}

// ── Helpers ──────────────────────────────────────────────────────

/// Tracks which item a snippet belongs to and what kind of grounding it is.
pub(crate) struct SnippetMeta {
    pub(crate) item_id: i32,
    pub(crate) kind: SnippetKind,
}

/// The grounding mode category for a snippet in the combined batch.
pub(crate) enum SnippetKind {
    Verbatim,
    NameMatch,
    HeadingMatch,
}

/// Result of categorizing extraction items by grounding mode.
///
/// ## Why this struct is extracted from run_verify
///
/// The categorization logic — which items need verbatim quote matching,
/// which need name matching, which are derived — is pure business logic
/// with no IO dependencies. Extracting it as a pure function allows it
/// to be tested without a database connection.
pub(crate) struct GroundingCategorization {
    /// Items that need verbatim quote search (have a non-empty quote)
    pub verbatim_items: Vec<(i32, String)>, // (item_id, quote)
    /// Items that need name-based search
    pub name_match_items: Vec<(i32, String)>, // (item_id, name)
    /// Items that need heading-based search
    pub heading_match_items: Vec<(i32, String)>, // (item_id, heading)
    /// Items marked as derived (no PDF search needed)
    pub derived_item_ids: Vec<i32>,
    /// Items marked as unverified (grounding_mode = None)
    pub none_item_ids: Vec<i32>,
    /// Items that should have a quote but don't (will get missing_quote status)
    pub missing_quote_item_ids: Vec<i32>,
}

/// Categorize extraction items by grounding mode.
///
/// Pure function — no IO, no database. Takes items and their grounding modes
/// from the schema, returns categorized lists ready for the grounding step.
pub(crate) fn categorize_items_for_grounding(
    items: &[ExtractionItemRecord],
    grounding_modes: &HashMap<String, GroundingMode>,
) -> GroundingCategorization {
    let mut verbatim_items = Vec::new();
    let mut name_match_items = Vec::new();
    let mut heading_match_items = Vec::new();
    let mut derived_item_ids = Vec::new();
    let mut none_item_ids = Vec::new();
    let mut missing_quote_item_ids = Vec::new();

    for item in items {
        let mode = grounding_modes
            .get(&item.entity_type)
            .unwrap_or(&GroundingMode::Verbatim);

        match mode {
            GroundingMode::Derived => {
                derived_item_ids.push(item.id);
            }
            GroundingMode::None => {
                none_item_ids.push(item.id);
            }
            GroundingMode::Verbatim => {
                if let Some(quote) = item.verbatim_quote.as_deref().filter(|q| !q.is_empty()) {
                    verbatim_items.push((item.id, quote.to_string()));
                } else {
                    missing_quote_item_ids.push(item.id);
                }
            }
            GroundingMode::NameMatch => {
                let label = extract_name_label(item);
                if !label.is_empty() {
                    name_match_items.push((item.id, label));
                } else {
                    missing_quote_item_ids.push(item.id);
                }
            }
            GroundingMode::HeadingMatch => {
                let label = extract_heading_label(item);
                if !label.is_empty() {
                    heading_match_items.push((item.id, label));
                } else {
                    missing_quote_item_ids.push(item.id);
                }
            }
        }
    }

    GroundingCategorization {
        verbatim_items,
        name_match_items,
        heading_match_items,
        derived_item_ids,
        none_item_ids,
        missing_quote_item_ids,
    }
}

/// Load grounding modes from the extraction schema for a document.
///
/// Returns a map of entity_type → GroundingMode. Fails if the schema
/// cannot be loaded — callers must handle the error rather than
/// silently degrading to Verbatim for all entities.
///
/// ## Why Result instead of an empty-HashMap fallback
///
/// Returning Result forces every caller to decide what to do when the
/// schema is missing. The prior code returned an empty HashMap, which
/// silently changed behavior: every entity defaulted to Verbatim mode,
/// and Party entities (with no `verbatim_quote`) got stuck at
/// `missing_quote` with no error visible to the user.
pub(crate) async fn load_grounding_modes(
    pool: &sqlx::PgPool,
    extraction_schema_dir: &str,
    doc_id: &str,
) -> Result<HashMap<String, GroundingMode>, String> {
    let pipe_config = match pipeline_repository::get_pipeline_config(pool, doc_id).await {
        Ok(Some(cfg)) => cfg,
        Ok(None) => {
            return Err(format!(
                "No pipeline_config found for document '{doc_id}' — cannot determine grounding modes"
            ));
        }
        Err(e) => {
            return Err(format!(
                "Failed to load pipeline_config for '{doc_id}': {e}"
            ));
        }
    };

    let schema_path = format!("{}/{}", extraction_schema_dir, pipe_config.schema_file);
    let schema = match colossus_extract::ExtractionSchema::from_file(Path::new(&schema_path)) {
        Ok(s) => s,
        Err(e) => {
            return Err(format!(
                "Failed to load schema '{}' for document '{doc_id}': {e}",
                pipe_config.schema_file
            ));
        }
    };

    Ok(schema
        .entity_types
        .iter()
        .map(|et| (et.name.clone(), et.grounding_mode.clone()))
        .collect())
}

/// Extract a name label from an item for NameMatch grounding.
///
/// Tries `label`, then common name properties.
pub(crate) fn extract_name_label(item: &ExtractionItemRecord) -> String {
    item.item_data["label"]
        .as_str()
        .or_else(|| item.item_data["properties"]["full_name"].as_str())
        .or_else(|| item.item_data["properties"]["party_name"].as_str())
        .unwrap_or("")
        .to_string()
}

/// Extract a heading label from an item for HeadingMatch grounding.
///
/// Tries `label`, then heading-specific properties.
pub(crate) fn extract_heading_label(item: &ExtractionItemRecord) -> String {
    item.item_data["label"]
        .as_str()
        .or_else(|| item.item_data["properties"]["legal_basis"].as_str())
        .or_else(|| item.item_data["properties"]["count_name"].as_str())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(id: i32, entity_type: &str, verbatim_quote: Option<&str>) -> ExtractionItemRecord {
        ExtractionItemRecord {
            id,
            run_id: 1,
            document_id: "test-doc".to_string(),
            entity_type: entity_type.to_string(),
            item_data: serde_json::json!({}),
            verbatim_quote: verbatim_quote.map(|s| s.to_string()),
            grounding_status: None,
            grounded_page: None,
            review_status: "pending".to_string(),
            reviewed_by: None,
            reviewed_at: None,
            review_notes: None,
            neo4j_node_id: None,
            resolved_entity_type: None,
            graph_status: "pending".to_string(),
        }
    }

    fn complaint_grounding_modes() -> HashMap<String, GroundingMode> {
        // Mirrors the complaint_v2.yaml grounding modes
        let mut modes = HashMap::new();
        modes.insert("Party".to_string(), GroundingMode::NameMatch);
        modes.insert("LegalCount".to_string(), GroundingMode::HeadingMatch);
        modes.insert("ComplaintAllegation".to_string(), GroundingMode::Verbatim);
        modes.insert("Harm".to_string(), GroundingMode::Derived);
        modes
    }

    #[test]
    fn test_complaint_allegation_with_quote_goes_to_verbatim() {
        let items = vec![make_item(
            1,
            "ComplaintAllegation",
            Some("Defendant fired plaintiff."),
        )];
        let modes = complaint_grounding_modes();
        let cat = categorize_items_for_grounding(&items, &modes);
        assert_eq!(cat.verbatim_items.len(), 1);
        assert_eq!(cat.verbatim_items[0].0, 1);
        assert_eq!(cat.verbatim_items[0].1, "Defendant fired plaintiff.");
        assert!(cat.missing_quote_item_ids.is_empty());
    }

    #[test]
    fn test_complaint_allegation_without_quote_goes_to_missing() {
        // This is the bug: 211 items went to missing_quote because LLM
        // did not produce verbatim quotes. Test documents the expectation.
        let items = vec![make_item(1, "ComplaintAllegation", None)];
        let modes = complaint_grounding_modes();
        let cat = categorize_items_for_grounding(&items, &modes);
        assert!(cat.verbatim_items.is_empty());
        assert_eq!(cat.missing_quote_item_ids, vec![1]);
    }

    #[test]
    fn test_party_goes_to_name_match() {
        let mut item = make_item(2, "Party", None);
        item.item_data = serde_json::json!({"properties": {"full_name": "Marie Awad"}});
        let items = vec![item];
        let modes = complaint_grounding_modes();
        let cat = categorize_items_for_grounding(&items, &modes);
        assert_eq!(cat.name_match_items.len(), 1);
        assert!(cat.missing_quote_item_ids.is_empty());
    }

    #[test]
    fn test_harm_goes_to_derived() {
        let items = vec![make_item(3, "Harm", None)];
        let modes = complaint_grounding_modes();
        let cat = categorize_items_for_grounding(&items, &modes);
        assert_eq!(cat.derived_item_ids, vec![3]);
    }

    #[test]
    fn test_unknown_entity_type_defaults_to_verbatim() {
        // Items with entity_type not in the schema default to Verbatim.
        // Without a quote they go to missing_quote.
        let items = vec![make_item(4, "UnknownType", None)];
        let modes = complaint_grounding_modes();
        let cat = categorize_items_for_grounding(&items, &modes);
        assert_eq!(cat.missing_quote_item_ids, vec![4]);
    }

    #[test]
    fn test_general_legal_schema_gives_all_missing_quote() {
        // This documents WHY general_legal.yaml produces all missing_quote:
        // Statement entities have grounding_mode=verbatim but general_legal
        // extracts no verbatim_quote field → all 211 items go to missing_quote.
        // Fixed by using complaint_v2.yaml which has proper verbatim_quote fields.
        let items = vec![
            make_item(1, "Statement", None), // no quote
            make_item(2, "Party", None),
        ];
        // general_legal modes — Statement is Verbatim, Party is NameMatch
        let mut modes = HashMap::new();
        modes.insert("Statement".to_string(), GroundingMode::Verbatim);
        modes.insert("Party".to_string(), GroundingMode::NameMatch);

        let cat = categorize_items_for_grounding(&items, &modes);
        // Statement without quote → missing_quote (verbatim mode, no quote)
        // Party without name label → missing_quote (name_match mode, empty item_data)
        // Both end up in missing_quote because neither has the data its mode requires.
        assert_eq!(cat.missing_quote_item_ids, vec![1, 2]);
        assert!(cat.verbatim_items.is_empty());
        assert!(cat.name_match_items.is_empty());
    }
}
