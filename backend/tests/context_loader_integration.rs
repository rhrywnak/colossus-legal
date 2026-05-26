//! backend/tests/context_loader_integration.rs
//!
//! Integration tests for `load_authored_entities_for_context` and its
//! merge with `load_cross_document_context` (Option A Step 2). Every test
//! is `#[ignore]` — they require a live `colossus_legal_v2` PostgreSQL
//! database (no `#[sqlx::test]` fixture infra in this repo). CI does NOT
//! run them.
//!
//! Run manually:
//!   `cargo test -p colossus-legal-backend --test context_loader_integration -- \
//!      --ignored --test-threads=1`
//!
//! ## Case-slug safety
//!
//! Tests DELETE authored rows by `case_slug`, so they use a per-test
//! `awad_v_catholic_family_service__test_ctx_<tag>` slug (never the bare
//! production slug) to avoid wiping real authored data. The pure-data
//! conversion test (instruction #4 — entity_id-as-id injection) lives in
//! `extraction_context.rs`'s unit-test module, not here.

use sqlx::PgPool;

use colossus_legal_backend::config::AppConfig;
use colossus_legal_backend::repositories::pipeline_repository::{
    delete_authored_entities_for_case, load_authored_entities_for_context,
    load_cross_document_context, upsert_authored_entity, CROSS_DOC_ENTITY_TYPES,
};

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const CASE_SLUG_BASE: &str = "awad_v_catholic_family_service";

fn test_slug(tag: &str) -> String {
    format!("{CASE_SLUG_BASE}__test_ctx_{tag}")
}

async fn pipeline_pool() -> TestResult<PgPool> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&config.pipeline_database_url)
        .await?;
    Ok(pool)
}

// ── (1) empty table → empty context ──────────────────────────────

#[tokio::test]
#[ignore]
async fn authored_context_empty_returns_empty_vec() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("empty");
    delete_authored_entities_for_case(&pool, &slug).await?;

    let out = load_authored_entities_for_context(&pool, &slug, CROSS_DOC_ENTITY_TYPES).await?;
    assert!(
        out.is_empty(),
        "no authored entities for the case → empty context"
    );
    Ok(())
}

// ── (2) whitelisted types returned as CrossDocEntity ─────────────

#[tokio::test]
#[ignore]
async fn authored_context_returns_whitelisted_types() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("whitelist");
    delete_authored_entities_for_case(&pool, &slug).await?;

    let el_id = format!("{slug}-element-1");
    let count_id = format!("{slug}-count-1");
    upsert_authored_entity(
        &pool,
        &slug,
        "Element",
        &el_id,
        &serde_json::json!({ "id": el_id, "label": "Duty" }),
        "canonical",
        None,
    )
    .await?;
    upsert_authored_entity(
        &pool,
        &slug,
        "LegalCount",
        &count_id,
        &serde_json::json!({ "id": count_id, "label": "Count I" }),
        "canonical",
        None,
    )
    .await?;

    let out = load_authored_entities_for_context(&pool, &slug, CROSS_DOC_ENTITY_TYPES).await?;
    assert_eq!(out.len(), 2, "both whitelisted authored entities returned");
    assert!(
        out.iter().all(|e| e.source_document_id == "canonical"),
        "authored entities carry the canonical source sentinel"
    );
    assert!(out.iter().all(|e| e.prefixed_id.starts_with("ctx:")));
    assert!(out.iter().any(|e| e.entity_type == "Element"));
    assert!(out.iter().any(|e| e.entity_type == "LegalCount"));

    delete_authored_entities_for_case(&pool, &slug).await?;
    Ok(())
}

// ── (3) non-whitelisted types excluded ───────────────────────────

#[tokio::test]
#[ignore]
async fn authored_context_excludes_non_whitelisted_types() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("nonwhitelist");
    delete_authored_entities_for_case(&pool, &slug).await?;

    let el_id = format!("{slug}-el");
    let scenario_id = format!("{slug}-sc");
    upsert_authored_entity(
        &pool,
        &slug,
        "Element",
        &el_id,
        &serde_json::json!({ "id": el_id }),
        "canonical",
        None,
    )
    .await?;
    // "Scenario" is deliberately NOT in CROSS_DOC_ENTITY_TYPES.
    upsert_authored_entity(
        &pool,
        &slug,
        "Scenario",
        &scenario_id,
        &serde_json::json!({ "id": scenario_id }),
        "canonical",
        None,
    )
    .await?;

    let out = load_authored_entities_for_context(&pool, &slug, CROSS_DOC_ENTITY_TYPES).await?;
    assert_eq!(out.len(), 1, "only the whitelisted Element is returned");
    assert_eq!(out[0].entity_type, "Element");

    delete_authored_entities_for_case(&pool, &slug).await?;
    Ok(())
}

// ── (5) extracted + authored coexist without error ───────────────

/// Seed a PUBLISHED document with a COMPLETED pass-1 run and one APPROVED
/// `Element` extraction item whose `item_data.id` is `element_id` — the
/// shape `load_cross_document_context` selects. Idempotent on re-run.
async fn seed_published_extracted_element(
    pool: &PgPool,
    doc_id: &str,
    element_id: &str,
) -> TestResult<()> {
    sqlx::query(
        "INSERT INTO documents (id, title, file_path, file_hash, document_type, status) \
         VALUES ($1, 'ctx-loader-test', '/tmp/ctx-test', 'ctx-test-hash', 'complaint', 'PUBLISHED') \
         ON CONFLICT (id) DO UPDATE SET status = 'PUBLISHED'",
    )
    .bind(doc_id)
    .execute(pool)
    .await?;

    let run_id: i32 = sqlx::query_scalar(
        "INSERT INTO extraction_runs \
             (document_id, pass_number, model_name, schema_version, started_at, raw_output, status) \
         VALUES ($1, 1, 'test-model', 'v5.1', NOW(), '{}'::jsonb, 'COMPLETED') RETURNING id",
    )
    .bind(doc_id)
    .fetch_one(pool)
    .await?;

    sqlx::query(
        "INSERT INTO extraction_items (run_id, document_id, entity_type, item_data, review_status) \
         VALUES ($1, $2, 'Element', $3, 'approved')",
    )
    .bind(run_id)
    .bind(doc_id)
    .bind(serde_json::json!({ "id": element_id, "label": "Shared Element", "properties": {} }))
    .execute(pool)
    .await?;
    Ok(())
}

async fn cleanup_extracted(pool: &PgPool, doc_id: &str) -> TestResult<()> {
    sqlx::query("DELETE FROM extraction_items WHERE document_id = $1")
        .bind(doc_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM extraction_runs WHERE document_id = $1")
        .bind(doc_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(doc_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// An Element that exists in BOTH `extraction_items` (a prior published
/// extraction) and `authored_entities`. Option B does not deduplicate;
/// the contract is "both present without errors" — and crucially the two
/// `item_id`s don't collide (extracted is a real positive id, authored is
/// negated), so the merged context is well-formed.
#[tokio::test]
#[ignore]
async fn extracted_and_authored_element_both_present_when_merged() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("merge");
    let src_doc = format!("{slug}-srcdoc");
    let current_doc = format!("{slug}-currentdoc");
    let shared_element_id = format!("{slug}-shared-element");

    // Clean slate.
    delete_authored_entities_for_case(&pool, &slug).await?;
    cleanup_extracted(&pool, &src_doc).await?;

    // Same Element in both tiers.
    seed_published_extracted_element(&pool, &src_doc, &shared_element_id).await?;
    upsert_authored_entity(
        &pool,
        &slug,
        "Element",
        &shared_element_id,
        &serde_json::json!({ "id": shared_element_id, "label": "Shared Element" }),
        "canonical",
        None,
    )
    .await?;

    // Extracted side: cross-doc loader sees the PUBLISHED src doc's Element
    // (current_doc is a different, unseeded id so the src is "other").
    let extracted = load_cross_document_context(&pool, &current_doc).await?;
    let extracted_match: Vec<_> = extracted
        .iter()
        .filter(|e| e.prefixed_id == format!("ctx:{shared_element_id}"))
        .collect();
    assert_eq!(
        extracted_match.len(),
        1,
        "extracted Element present via cross-doc loader"
    );
    assert!(
        extracted_match[0].item_id > 0,
        "extracted item_id is a real positive id"
    );

    // Authored side.
    let authored = load_authored_entities_for_context(&pool, &slug, CROSS_DOC_ENTITY_TYPES).await?;
    assert_eq!(authored.len(), 1, "authored Element present");
    assert!(
        authored[0].item_id < 0,
        "authored item_id is negated (no FK collision)"
    );

    // Merge (as the Pass-2 call site does) — both present, no error, no
    // id collision between the two sources.
    let mut merged = extracted;
    merged.extend(authored);
    let shared: Vec<_> = merged
        .iter()
        .filter(|e| e.prefixed_id == format!("ctx:{shared_element_id}"))
        .collect();
    assert_eq!(
        shared.len(),
        2,
        "both the extracted and authored Element survive the merge"
    );

    cleanup_extracted(&pool, &src_doc).await?;
    delete_authored_entities_for_case(&pool, &slug).await?;
    Ok(())
}
