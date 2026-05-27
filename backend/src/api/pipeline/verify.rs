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

/// Per-entity-type verification config drawn from the schema.
///
/// ## Rust Learning: composite config struct
///
/// Replaces the bare `HashMap<String, GroundingMode>` returned by the
/// pre-v5.1 `load_grounding_modes` with a struct that also carries
/// `provenance_required`. That flag is parsed by `colossus_extract` from
/// the schema YAML but was previously read by no one — the runtime side
/// of the contract was missing, which is exactly the kind of silent
/// schema/code drift v5.1 closes for derived entities.
///
/// We could have plumbed two parallel maps (`HashMap<_, GroundingMode>`
/// and `HashMap<_, bool>`) but a struct keeps the two fields paired at
/// the type level — there's no risk of one map having a key the other
/// is missing.
#[derive(Debug, Clone)]
pub(crate) struct EntityVerificationConfig {
    pub mode: GroundingMode,
    pub provenance_required: bool,
}

use super::canonical_verifier::{find_in_canonical_text, CanonicalMatchType};
use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::models::document_status::{STATUS_EXTRACTED, STATUS_VERIFIED};
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
    /// Derived-mode items whose provenance validated successfully.
    pub derived: usize,
    /// Derived-mode items that failed v5.1 §5.4 provenance validation
    /// (missing provenance array, empty array, dangling paragraph
    /// reference, or null `item_data`). Each carries a diagnostic
    /// `verification_reason` in `extraction_items.verification_reason`.
    pub derived_invalid: usize,
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

    // 2. Load extraction schema for grounding mode + provenance_required
    //    lookup. Failure is fatal: without the schema we'd default every
    //    entity to Verbatim, silently corrupting Party / LegalCount / Harm
    //    grounding. (v5.1 also reads provenance_required from this config
    //    so the derived-validation step can apply the schema's rule.)
    let grounding_config =
        load_grounding_config(&state.pipeline_pool, state.registry.schema_dir(), doc_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    document_id = %doc_id, error = %e,
                    "Verify cannot proceed without grounding config"
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
            message: format!(
                "No canonical text found for document '{doc_id}'. Was ExtractText run?"
            ),
        });
    }

    // Convert to (page_number, text_content) tuples for the verifier
    let document_pages: Vec<(u32, String)> = document_text_rows
        .into_iter()
        .map(|row| (row.page_number as u32, row.text_content))
        .collect();

    // 6. Categorize items by grounding mode using the extracted pure function.
    //    Then build combined snippets for PageGrounder.
    let categorization = categorize_items_for_grounding(&items, &grounding_config)
        .map_err(|message| AppError::Internal { message })?;

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
            None,
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update item {}: {e}", meta.item_id),
        })?;
    }

    // 9. Validate derived-mode items per v5.1 §5.4. Pre-v5.1, every
    //    item routed to the derived bucket got blanket-stamped
    //    'derived' regardless of whether its provenance array was
    //    present, non-empty, or referenced real Allegations. That
    //    silently passed malformed entities through to the green-flag
    //    state — exactly the no-silent-failures violation v5.1 closes.
    //
    //    Now: build a paragraph_number → item_id lookup from the
    //    Allegations in the same document, then validate each derived
    //    item against it. Valid → 'derived' (NULL reason); invalid →
    //    'derived_invalid' with a durable diagnostic reason.
    let para_to_item_id = build_para_to_item_id(&items);
    let mut derived_invalid_count = 0usize;
    let mut derived_valid_count = 0usize;
    for item_id in &derived_items {
        // `unwrap` here is safe: derived_items came from items, and
        // items is the source vector — the id MUST exist. The
        // alternative (filter_map / log-and-skip) would mask a real
        // invariant violation; per Rule 1 we panic rather than write
        // a malformed status.
        let item = items
            .iter()
            .find(|i| i.id == *item_id)
            .expect("derived_item_ids only contains ids drawn from items");
        let provenance_required = grounding_config
            .get(&item.entity_type)
            .map(|c| c.provenance_required)
            .unwrap_or(false);
        let validation = validate_derived_provenance(item, &para_to_item_id, provenance_required);
        let (status_str, reason) = match validation {
            DerivedValidation::Valid => {
                derived_valid_count += 1;
                ("derived", None)
            }
            DerivedValidation::Invalid(r) => {
                derived_invalid_count += 1;
                ("derived_invalid", Some(r))
            }
        };
        pipeline_repository::update_item_grounding(
            &state.pipeline_pool,
            *item_id,
            status_str,
            None,
            reason.as_deref(),
        )
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

    // `derived_count` here means VALID derived items only — invalid
    // derived items are tallied separately in `derived_invalid_count`
    // (written to grounding_status='derived_invalid' above) and excluded
    // from the auto-approve `GROUNDED_STATUSES` list per Roman's Q1A.
    let derived_count = derived_valid_count;
    let unverified_count = none_items.len();
    let skipped_count = missing_quote_items.len();

    tracing::info!(
        doc_id = %doc_id, exact, normalized, not_found,
        derived = derived_count, derived_invalid = derived_invalid_count,
        unverified = unverified_count,
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
            "derived": derived_count, "derived_invalid": derived_invalid_count,
            "unverified": unverified_count,
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
            "derived": derived_count, "derived_invalid": derived_invalid_count,
            "unverified": unverified_count,
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
        derived_invalid: derived_invalid_count,
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
#[derive(Debug)]
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
/// Pure function — no IO, no database. Takes items and the schema's
/// per-entity verification config, returns categorized lists ready for
/// the grounding step.
///
/// ## Provenance validation is NOT done here
///
/// `derived_item_ids` only carries the bucket assignment. The actual
/// validation (does the item have a non-empty provenance array? do the
/// refs resolve to real Allegations?) lives in
/// `validate_derived_provenance` and is called by `run_verify` after
/// this categorization completes. Splitting the two keeps each function
/// pure with one job: this one routes by mode, the other validates.
pub(crate) fn categorize_items_for_grounding(
    items: &[ExtractionItemRecord],
    grounding_config: &HashMap<String, EntityVerificationConfig>,
) -> Result<GroundingCategorization, String> {
    let mut verbatim_items = Vec::new();
    let mut name_match_items = Vec::new();
    let mut heading_match_items = Vec::new();
    let mut derived_item_ids = Vec::new();
    let mut none_item_ids = Vec::new();
    let mut missing_quote_item_ids = Vec::new();

    for item in items {
        // Hard-error on schema lookup miss. The pre-fix code silently
        // defaulted unmapped entity_types to Verbatim, which routed
        // name_match entities (Party, etc.) through the empty-quote
        // branch and stamped them `grounding_status = "missing_quote"`
        // without surfacing the underlying mismatch. If we hit this,
        // either `pipeline_config.schema_file` points at the wrong
        // schema or the LLM emitted an entity_type the schema doesn't
        // declare — both are operator-actionable bugs, not conditions
        // to mask.
        let mode = grounding_config
            .get(&item.entity_type)
            .map(|c| &c.mode)
            .ok_or_else(|| {
                format!(
                    "Entity type '{}' (item id {}) is not declared in the loaded schema. \
                     Schema declares: [{}]. Check that pipeline_config.schema_file matches \
                     the entity types the LLM emitted for this document.",
                    item.entity_type,
                    item.id,
                    {
                        let mut keys: Vec<&str> =
                            grounding_config.keys().map(String::as_str).collect();
                        keys.sort();
                        keys.join(", ")
                    },
                )
            })?;

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

    Ok(GroundingCategorization {
        verbatim_items,
        name_match_items,
        heading_match_items,
        derived_item_ids,
        none_item_ids,
        missing_quote_item_ids,
    })
}

/// Load per-entity verification config (grounding mode + provenance_required)
/// from the extraction schema for a document.
///
/// Returns a map of `entity_type → EntityVerificationConfig`. Fails if
/// the schema cannot be loaded — callers must handle the error rather
/// than silently degrading to Verbatim for all entities.
///
/// ## Why Result instead of an empty-HashMap fallback
///
/// Returning Result forces every caller to decide what to do when the
/// schema is missing. The prior code returned an empty HashMap, which
/// silently changed behavior: every entity defaulted to Verbatim mode,
/// and Party entities (with no `verbatim_quote`) got stuck at
/// `missing_quote` with no error visible to the user.
///
/// ## v5.1 change
///
/// Previously `load_grounding_modes` returned only `GroundingMode`. The
/// schema's `provenance_required: bool` (set on every Derived entity in
/// `complaint_v5.yaml`) was parsed by `colossus_extract` but read
/// nowhere in the verifier. The renamed function returns both fields so
/// `validate_derived_provenance` can apply the schema's rule.
pub(crate) async fn load_grounding_config(
    pool: &sqlx::PgPool,
    extraction_schema_dir: &str,
    doc_id: &str,
) -> Result<HashMap<String, EntityVerificationConfig>, String> {
    let pipe_config = match pipeline_repository::get_pipeline_config(pool, doc_id).await {
        Ok(Some(cfg)) => cfg,
        Ok(None) => {
            return Err(format!(
                "No pipeline_config found for document '{doc_id}' — cannot determine grounding config"
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
        .map(|et| {
            (
                et.name.clone(),
                EntityVerificationConfig {
                    mode: et.grounding_mode.clone(),
                    provenance_required: et.provenance_required,
                },
            )
        })
        .collect())
}

/// Outcome of `validate_derived_provenance` for a single derived-mode item.
///
/// ## Rust Learning: custom enum vs `Result<(), String>`
///
/// We could have used `Result<(), String>` here (Ok = valid, Err = the
/// reason). The custom enum names the success/failure cases at the call
/// site (`DerivedValidation::Valid` / `DerivedValidation::Invalid`)
/// instead of relying on the reader to know that "Err" means "invalid"
/// in this domain. `Result` is for fallible operations; this is a pure
/// classification — the function never *errors*, it always returns one
/// of two well-defined classifications.
pub(crate) enum DerivedValidation {
    Valid,
    /// Carries the diagnostic `verification_reason` to persist on the
    /// extraction_items row. Roman's Q2 specifies the exact strings —
    /// they surface in the Review tab UI as forensic notes, so wording
    /// is contractual, not stylistic.
    Invalid(String),
}

/// Build a `paragraph_number → item_id` lookup from the document's
/// Allegations.
///
/// ## Why filter to Allegation only
///
/// The instruction wording in CC Instruction 2 §2B is "at least one
/// entry in `provenance` references an Allegation whose
/// `paragraph_number` exists in the document." LegalCount carries
/// `paragraph_range`, Element carries `anchor_paragraph_numbers`.
/// Neither should match a Harm's `provenance.ref` lookup — only paragraph
/// numbers from genuine Allegations should resolve. Filtering at map
/// build time enforces that without polluting the validation function.
///
/// ## Polymorphism (Q5)
///
/// Reads both `paragraph_number` (v2/v3/v5 schema) and `paragraph_ref`
/// (v4 alias), and accepts either string or integer JSON shape. Same
/// pattern as `ingest_helpers::create_provenance_relationships` so the
/// verifier and ingest agree on what counts as a paragraph reference.
pub(crate) fn build_para_to_item_id(items: &[ExtractionItemRecord]) -> HashMap<String, i32> {
    let mut map: HashMap<String, i32> = HashMap::new();
    for item in items.iter().filter(|i| i.entity_type == "Allegation") {
        let props = &item.item_data["properties"];
        let para = props["paragraph_number"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| props["paragraph_number"].as_i64().map(|n| n.to_string()))
            .or_else(|| props["paragraph_ref"].as_str().map(|s| s.to_string()))
            .or_else(|| props["paragraph_ref"].as_i64().map(|n| n.to_string()));
        if let Some(para) = para {
            map.insert(para, item.id);
        }
    }
    map
}

/// Validate provenance for a single derived-mode item per v5.1 §5.4.
///
/// Caller has already established that this item is in the derived
/// bucket; this function decides Valid vs Invalid(reason) based on
/// the item's `item_data.provenance` array and the document's
/// Allegation paragraph map.
///
/// ## At-least-one-resolves semantics
///
/// If the provenance array has at least one entry whose `ref` field
/// resolves to an extracted Allegation paragraph in the same document,
/// the item is Valid. Entries with null/missing refs and entries
/// whose refs don't match any extracted Allegation are logged with
/// `tracing::warn!` and skipped — they are LLM noise, not fatal.
///
/// Only truly empty/missing provenance (no array, empty array, or
/// every entry unresolvable) results in Invalid.
///
/// ## Strict-mode reading (Roman's Q1A)
///
/// Reads ONLY `item_data.provenance` (top-level on item_data, alongside
/// `properties`). Does NOT fall back to `item_data.properties.provenance`
/// or to entity-type-specific alternatives like
/// `properties.paragraph_numbers`. Schema/template/data disagreements
/// (a derived entity whose template does not request a provenance array)
/// surface loudly here as `Invalid` — the fix lives upstream in the
/// template/schema, not in a permissive verifier.
pub(crate) fn validate_derived_provenance(
    item: &ExtractionItemRecord,
    para_to_item_id: &HashMap<String, i32>,
    _provenance_required: bool,
) -> DerivedValidation {
    // Step 1 — NULL item_data sentinel (matches the Harm id 5106
    // anomaly Roman flagged in the Q6 results).
    if item.item_data.is_null() {
        return DerivedValidation::Invalid("item_data is null".to_string());
    }

    // Step 2 — read top-level provenance array (Q1A strict). Missing
    // provenance is Invalid for every derived type — the fix lives
    // upstream in the template/schema, not in a permissive verifier.
    let provenance = match item.item_data.get("provenance").and_then(|p| p.as_array()) {
        Some(arr) => arr,
        None => return DerivedValidation::Invalid("no provenance array".to_string()),
    };

    // Step 3 — empty array.
    if provenance.is_empty() {
        return DerivedValidation::Invalid("empty provenance array".to_string());
    }

    // Step 4 — at-least-one-resolves with tolerance for null refs
    // and dangling refs. Each entry is inspected; null/missing refs
    // and unresolved refs are logged as warnings and skipped. If at
    // least one entry resolves, the item is Valid.
    //
    // ## Rust Learning: `entry.get("ref")` on serde_json::Value
    //
    // `serde_json::Value::get` returns `Option<&Value>`. When the
    // JSON has `"ref": null`, `.get("ref")` returns `Some(Value::Null)`.
    // The `.and_then(|v| v.as_str())` chain then returns `None` for
    // null — same as a missing key. Both cases land in the null-ref
    // warning branch.
    let entity_label = item.item_data["label"].as_str().unwrap_or("unknown");
    let mut resolved_count = 0usize;

    for (idx, entry) in provenance.iter().enumerate() {
        let ref_val = entry.get("ref").and_then(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.as_i64().map(|n| n.to_string()))
        });

        let ref_val = match ref_val {
            Some(v) => v,
            None => {
                tracing::warn!(
                    entity_type = %item.entity_type,
                    item_id = item.id,
                    document_id = %item.document_id,
                    label = %entity_label,
                    provenance_index = idx,
                    "provenance entry has null/missing ref — skipping"
                );
                continue;
            }
        };

        if para_to_item_id.contains_key(&ref_val) {
            resolved_count += 1;
        } else {
            tracing::warn!(
                entity_type = %item.entity_type,
                item_id = item.id,
                document_id = %item.document_id,
                label = %entity_label,
                provenance_index = idx,
                paragraph_ref = %ref_val,
                "provenance references paragraph {} which is not an extracted Allegation — skipping",
                ref_val
            );
        }
    }

    if resolved_count > 0 {
        DerivedValidation::Valid
    } else {
        DerivedValidation::Invalid(
            "no provenance entries resolved to extracted Allegations".to_string(),
        )
    }
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

    fn cfg(mode: GroundingMode, provenance_required: bool) -> EntityVerificationConfig {
        EntityVerificationConfig {
            mode,
            provenance_required,
        }
    }

    fn complaint_grounding_modes() -> HashMap<String, EntityVerificationConfig> {
        // Mirrors the complaint_v2.yaml grounding modes (provenance_required
        // tracks the schema: Harm Derived → true, others false).
        let mut modes = HashMap::new();
        modes.insert("Party".to_string(), cfg(GroundingMode::NameMatch, false));
        modes.insert(
            "LegalCount".to_string(),
            cfg(GroundingMode::HeadingMatch, false),
        );
        modes.insert(
            "ComplaintAllegation".to_string(),
            cfg(GroundingMode::Verbatim, false),
        );
        modes.insert("Harm".to_string(), cfg(GroundingMode::Derived, true));
        modes
    }

    #[test]
    fn test_categorize_items_for_grounding_routing() {
        // Routing table: (entity_type, has_quote) → which categorization
        // bucket the item lands in. Each row preserves a documented
        // contract from the source tests:
        //
        // - ComplaintAllegation + quote → verbatim_items (canonical happy path)
        // - ComplaintAllegation - quote → missing_quote_item_ids
        //   (the bug that stranded 211 items: LLM produced no quotes)
        // - Party (with name) → name_match_items
        // - Harm → derived_item_ids
        // - UnknownType → missing_quote (silent default to Verbatim;
        //   FOLLOWUP-verify-silent-default tracks the rule-1 violation)
        let modes = complaint_grounding_modes();

        // Case 1: ComplaintAllegation with quote → verbatim
        {
            let items = vec![make_item(
                1,
                "ComplaintAllegation",
                Some("Defendant fired plaintiff."),
            )];
            let cat = categorize_items_for_grounding(&items, &modes).unwrap();
            assert_eq!(
                cat.verbatim_items.len(),
                1,
                "ComplaintAllegation+quote → verbatim"
            );
            assert_eq!(cat.verbatim_items[0].0, 1);
            assert_eq!(cat.verbatim_items[0].1, "Defendant fired plaintiff.");
            assert!(cat.missing_quote_item_ids.is_empty());
        }
        // Case 2: ComplaintAllegation without quote → missing_quote
        {
            let items = vec![make_item(2, "ComplaintAllegation", None)];
            let cat = categorize_items_for_grounding(&items, &modes).unwrap();
            assert!(
                cat.verbatim_items.is_empty(),
                "ComplaintAllegation-quote → not verbatim"
            );
            assert_eq!(cat.missing_quote_item_ids, vec![2]);
        }
        // Case 3: Party with name → name_match
        {
            let mut item = make_item(3, "Party", None);
            item.item_data = serde_json::json!({"properties": {"full_name": "Marie Awad"}});
            let cat = categorize_items_for_grounding(&[item], &modes).unwrap();
            assert_eq!(cat.name_match_items.len(), 1, "Party+name → name_match");
            assert!(cat.missing_quote_item_ids.is_empty());
        }
        // Case 4: Harm → derived
        {
            let items = vec![make_item(4, "Harm", None)];
            let cat = categorize_items_for_grounding(&items, &modes).unwrap();
            assert_eq!(cat.derived_item_ids, vec![4], "Harm → derived");
        }
        // Case 5: UnknownType → hard error. The pre-fix code silently
        // defaulted to Verbatim and routed the item to missing_quote,
        // masking the real bug (schema/LLM mismatch). The fix raises
        // an Err naming the unmapped entity_type and the schema keys.
        {
            let items = vec![make_item(5, "UnknownType", None)];
            let err = categorize_items_for_grounding(&items, &modes)
                .expect_err("UnknownType not in modes → Err");
            assert!(
                err.contains("UnknownType"),
                "error must name the unmapped entity_type, got: {err}"
            );
            assert!(
                err.contains("item id 5"),
                "error must name the offending item id, got: {err}"
            );
        }
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
        modes.insert("Statement".to_string(), cfg(GroundingMode::Verbatim, false));
        modes.insert("Party".to_string(), cfg(GroundingMode::NameMatch, false));

        let cat = categorize_items_for_grounding(&items, &modes).unwrap();
        // Statement without quote → missing_quote (verbatim mode, no quote)
        // Party without name label → missing_quote (name_match mode, empty item_data)
        // Both end up in missing_quote because neither has the data its mode requires.
        assert_eq!(cat.missing_quote_item_ids, vec![1, 2]);
        assert!(cat.verbatim_items.is_empty());
        assert!(cat.name_match_items.is_empty());
    }

    // ── v5.1 derived-provenance validation ────────────────────────
    //
    // Per LEGAL_DATA_MODEL_v5_1 §5.4 and CC Instruction 2 the verifier
    // must validate that derived-mode items actually carry a non-empty
    // `provenance` array whose entries reference real Allegations in
    // the same document. The four-string vocabulary below is contractual
    // — the strings surface in the Review tab UI as forensic notes, so
    // the assertions hold the wording rather than just shape.

    fn make_item_with_data(
        id: i32,
        entity_type: &str,
        item_data: serde_json::Value,
    ) -> ExtractionItemRecord {
        let mut rec = make_item(id, entity_type, None);
        rec.item_data = item_data;
        rec
    }

    fn allegation_with_paragraph(id: i32, paragraph: &str) -> ExtractionItemRecord {
        make_item_with_data(
            id,
            "Allegation",
            serde_json::json!({
                "label": format!("Para {paragraph}"),
                "properties": { "paragraph_number": paragraph, "summary": "x" },
            }),
        )
    }

    #[test]
    fn test_validate_derived_returns_invalid_with_exact_reason() {
        // Routing table: (item fixture, expected reason string).
        // Pins the canonical diagnostic strings v5.1 §5.4 emits.
        // Each row's docstring documents what regression it catches.

        // Build a paragraph map pre-populated with paragraph 8 so the
        // dangling-ref case can fire by passing ref "99".
        let allegation = allegation_with_paragraph(101, "8");
        let para_map = build_para_to_item_id(&[allegation]);

        // Case 1: NULL item_data — May-5 Awad Harm id 5106 anomaly.
        // Catches a dropped null-check that would crash on indexing or
        // silently treat null as "no provenance" with the wrong reason.
        let null_harm = ExtractionItemRecord {
            item_data: serde_json::Value::Null,
            ..make_item(201, "Harm", None)
        };

        // Case 2: derived type (Harm) with no provenance key — the generic
        // "no provenance array" message. Missing provenance is Invalid for
        // every derived type; the fix lives upstream in the template/schema.
        let harm_no_prov = make_item_with_data(
            401,
            "Harm",
            serde_json::json!({ "properties": { "kind": "economic" } }),
        );

        // Case 3: provenance: [] — distinguishing empty-array from
        // missing-key is what `verification_reason` is for. The empty
        // array means the LLM emitted the field but found nothing to
        // put in it; missing means the template never asked.
        let harm_empty_prov = make_item_with_data(
            501,
            "Harm",
            serde_json::json!({ "properties": { "kind": "economic" }, "provenance": [] }),
        );

        // Case 4: provenance entry refs a paragraph that's not in the map
        // and it's the ONLY entry. With at-least-one-resolves semantics,
        // a sole dangling ref still produces Invalid because nothing
        // resolved. The per-entry detail goes to tracing::warn!; the
        // DB reason is the summary.
        let harm_dangling_ref = make_item_with_data(
            601,
            "Harm",
            serde_json::json!({
                "properties": { "kind": "economic" },
                "provenance": [{ "ref": "99" }]
            }),
        );

        // Case 5: provenance entry missing 'ref' field (sole entry).
        // Null ref is logged and skipped; with no entries resolving,
        // the summary reason fires.
        let harm_missing_ref = make_item_with_data(
            701,
            "Harm",
            serde_json::json!({
                "properties": { "kind": "economic" },
                "provenance": [{ "quote_snippet": "no ref here" }]
            }),
        );

        let cases = [
            (&null_harm, "item_data is null"),
            (&harm_no_prov, "no provenance array"),
            (&harm_empty_prov, "empty provenance array"),
            (
                &harm_dangling_ref,
                "no provenance entries resolved to extracted Allegations",
            ),
            (
                &harm_missing_ref,
                "no provenance entries resolved to extracted Allegations",
            ),
        ];

        for (item, expected_reason) in cases {
            match validate_derived_provenance(item, &para_map, true) {
                DerivedValidation::Invalid(r) => assert_eq!(
                    r, *expected_reason,
                    "item id {} should produce reason {expected_reason:?}; got: {r:?}",
                    item.id
                ),
                DerivedValidation::Valid => panic!(
                    "item id {} expected Invalid({expected_reason:?}); got Valid",
                    item.id
                ),
            }
        }
    }

    #[test]
    fn validate_derived_polymorphic_paragraph_ref_string_and_integer() {
        // Catches: dropped polymorphism on either side. The map MUST
        // accept paragraph_number as integer (some extractors emit
        // numeric); the validator MUST accept ref as integer too.
        // Q5 of the PCA — same rule as ingest_helpers.
        let allegation = make_item_with_data(
            101,
            "Allegation",
            serde_json::json!({
                "label": "Para 8",
                "properties": { "paragraph_number": 8 }, // integer
            }),
        );
        let harm = make_item_with_data(
            801,
            "Harm",
            serde_json::json!({
                "properties": { "kind": "economic" },
                "provenance": [{ "ref": 8 }] // integer
            }),
        );
        // build_para_to_item_id filters to entity_type=="Allegation",
        // so only Allegations need to be in the slice. Avoiding the clone
        // also keeps `harm` movable into validate_derived_provenance.
        let map = build_para_to_item_id(&[allegation]);
        assert!(
            map.contains_key("8"),
            "build_para_to_item_id must coerce integer paragraph_number to string"
        );
        match validate_derived_provenance(&harm, &map, true) {
            DerivedValidation::Valid => {}
            DerivedValidation::Invalid(r) => panic!(
                "polymorphic integer ref must validate against string-keyed map; got Invalid({r})"
            ),
        }
    }

    #[test]
    fn validate_derived_tolerates_one_dangling_among_valid() {
        // At-least-one-resolves semantics: if ref "8" resolves, the
        // dangling ref "99" is logged as a warning and skipped. The
        // overall item is Valid because at least one entry resolved.
        let allegation = allegation_with_paragraph(101, "8");
        let harm = make_item_with_data(
            901,
            "Harm",
            serde_json::json!({
                "properties": { "kind": "economic" },
                "provenance": [
                    { "ref": "8" },   // resolves
                    { "ref": "99" }   // dangles — logged, skipped
                ]
            }),
        );
        let map = build_para_to_item_id(&[allegation]);
        match validate_derived_provenance(&harm, &map, true) {
            DerivedValidation::Valid => {}
            DerivedValidation::Invalid(r) => {
                panic!("at-least-one-resolves: ref 8 resolves, so item should be Valid; got Invalid({r})")
            }
        }
    }

    #[test]
    fn build_para_to_item_id_only_indexes_allegations() {
        // Catches: a refactor that drops the entity_type filter and
        // pollutes the map with LegalCount paragraph_range or Element
        // anchor_paragraph_numbers values. Instruction wording is
        // "references an Allegation" — strict.
        let allegation = allegation_with_paragraph(101, "8");
        let count_with_para = make_item_with_data(
            201,
            "LegalCount",
            serde_json::json!({
                "properties": { "paragraph_number": "999" }, // would pollute if filter dropped
            }),
        );
        let element_with_para = make_item_with_data(
            301,
            "Element",
            serde_json::json!({
                "properties": { "paragraph_number": "888" },
            }),
        );
        let map = build_para_to_item_id(&[allegation, count_with_para, element_with_para]);
        assert_eq!(map.get("8"), Some(&101));
        assert!(
            !map.contains_key("999"),
            "LegalCount must not appear in the para→item map"
        );
        assert!(
            !map.contains_key("888"),
            "Element must not appear in the para→item map"
        );
    }

    #[test]
    fn validate_derived_partial_null_provenance_tolerant() {
        // Awad Harm id 5777 scenario: first entry resolves, second
        // has null ref (LLM garbage). At-least-one resolves → Valid.
        let allegation = allegation_with_paragraph(101, "68");
        let harm = make_item_with_data(
            5777,
            "Harm",
            serde_json::json!({
                "label": "Economic harm — guardianship funds",
                "properties": { "kind": "economic" },
                "provenance": [
                    { "ref": "68", "ref_type": "paragraph", "quote_snippet": "additional funds existed..." },
                    { "ref": null, "ref_type": "paragraph" }
                ]
            }),
        );
        let map = build_para_to_item_id(&[allegation]);
        match validate_derived_provenance(&harm, &map, true) {
            DerivedValidation::Valid => {}
            DerivedValidation::Invalid(r) => {
                panic!("partial null provenance: ref 68 resolves, null ref skipped; expected Valid, got Invalid({r})")
            }
        }
    }

    #[test]
    fn validate_derived_all_null_refs_invalid() {
        // Every provenance entry has a null/missing ref — nothing
        // resolves, so the item is Invalid.
        let allegation = allegation_with_paragraph(101, "8");
        let harm = make_item_with_data(
            802,
            "Harm",
            serde_json::json!({
                "properties": { "kind": "economic" },
                "provenance": [
                    { "ref": null, "ref_type": "paragraph" },
                    { "quote_snippet": "no ref field at all" }
                ]
            }),
        );
        let map = build_para_to_item_id(&[allegation]);
        match validate_derived_provenance(&harm, &map, true) {
            DerivedValidation::Invalid(r) => assert_eq!(
                r, "no provenance entries resolved to extracted Allegations",
                "all-null-refs should produce the no-resolved summary reason; got: {r}"
            ),
            DerivedValidation::Valid => {
                panic!("all null refs: nothing resolved, must be Invalid")
            }
        }
    }

    #[test]
    fn validate_derived_null_and_dangling_mix_invalid() {
        // Mix of null ref and non-null dangling ref — neither resolves.
        let allegation = allegation_with_paragraph(101, "8");
        let harm = make_item_with_data(
            803,
            "Harm",
            serde_json::json!({
                "properties": { "kind": "economic" },
                "provenance": [
                    { "ref": null },
                    { "ref": "99" }
                ]
            }),
        );
        let map = build_para_to_item_id(&[allegation]);
        match validate_derived_provenance(&harm, &map, true) {
            DerivedValidation::Invalid(r) => assert_eq!(
                r, "no provenance entries resolved to extracted Allegations",
                "null + dangling: nothing resolved; got: {r}"
            ),
            DerivedValidation::Valid => {
                panic!("null + dangling (no resolves): must be Invalid")
            }
        }
    }

    #[test]
    fn build_para_to_item_id_handles_paragraph_ref_v4_alias() {
        // Catches: the v4-compatibility chain getting trimmed to v5
        // canonical (paragraph_number) only. CC Instruction 2 Q5
        // explicitly mandated polymorphism.
        let allegation_v4 = make_item_with_data(
            101,
            "Allegation",
            serde_json::json!({
                "properties": { "paragraph_ref": "42", "allegation_text": "x" },
            }),
        );
        let map = build_para_to_item_id(&[allegation_v4]);
        assert_eq!(
            map.get("42"),
            Some(&101),
            "v4 paragraph_ref alias must be indexed under its string value"
        );
    }
}
