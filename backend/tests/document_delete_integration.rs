//! backend/tests/document_delete_integration.rs
//!
//! Integration tests for the full document-delete path
//! (`repositories::pipeline_repository::documents::delete_all_document_data`)
//! and its companion Neo4j `source_documents` array sweep
//! (`pipeline::steps::cleanup::strip_source_document_from_arrays`, reached
//! here through the public `cleanup_neo4j`).
//!
//! Every test is `#[ignore]` because it requires a live `colossus_legal`
//! PostgreSQL database (and, for the array-sweep test, live DEV Neo4j) — the
//! project has no `#[sqlx::test]` fixture infra (see `cleanup_integration.rs`
//! and `authored_entities_integration.rs` for the same convention). CI does
//! NOT run them.
//!
//! Run manually against live DEV infra:
//!   `cargo test -p colossus-legal-backend --test document_delete_integration -- \
//!      --ignored --test-threads=1`
//!
//! Each test seeds its own uniquely-keyed data and cleans up after itself so
//! the suite is re-runnable without drift.

use neo4rs::Graph;
use sqlx::PgPool;
use std::time::{SystemTime, UNIX_EPOCH};

use colossus_legal_backend::config::AppConfig;
use colossus_legal_backend::neo4j::create_neo4j_graph;
use colossus_legal_backend::pipeline::steps::cleanup;
use colossus_legal_backend::repositories::pipeline_repository::documents::delete_all_document_data;

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Provenance value used by Pass-2 extracted cross-tier edges — the only
/// rows the delete path may remove. Mirrors `PROVENANCE_EXTRACTED` in
/// `authored_entities.rs` (kept as a literal here because the test asserts
/// the exact stored value).
const PROVENANCE_EXTRACTED: &str = "extracted";

/// Provenance value the delete path must PRESERVE (human-authored rows).
const PROVENANCE_AUTHORED: &str = "authored";

/// Documented base case slug (the real matter). Tests suffix it so any
/// destructive cleanup only ever touches the test's own rows.
const CASE_SLUG_BASE: &str = "awad_v_catholic_family_service";

/// Neo4j label used only by the array-sweep test, so teardown can target the
/// seeded node precisely without risking real Party nodes.
const TEST_PARTY_LABEL: &str = "Ep3PreTestParty";

/// Connect to the live pipeline database from env (`.env` honored).
async fn pipeline_pool() -> TestResult<PgPool> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&config.pipeline_database_url)
        .await?;
    Ok(pool)
}

/// Connect to the live DEV Neo4j graph from env.
async fn neo4j_graph() -> TestResult<Graph> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;
    let graph = create_neo4j_graph(&config).await?;
    Ok(graph)
}

/// Nanosecond-unique document id so concurrent or repeated runs never collide.
fn unique_doc_id(scope: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    format!("ep3pre3-{scope}-{nanos}")
}

/// Insert the `documents` parent row every other table FKs to.
async fn seed_document(pool: &PgPool, doc_id: &str) -> TestResult<()> {
    sqlx::query(
        "INSERT INTO documents (id, title, file_path, file_hash, document_type, status) \
         VALUES ($1, $2, $3, $4, 'pdf', 'INGESTED') \
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(doc_id)
    .bind(format!("test-{doc_id}"))
    .bind(format!("/tmp/{doc_id}.pdf"))
    .bind(format!("hash-{doc_id}"))
    .execute(pool)
    .await?;
    Ok(())
}

/// Insert one extraction_run + one extraction_item for the document and
/// return the item's id (so a caller can attach review_edit_history to it).
async fn seed_run_and_item(pool: &PgPool, doc_id: &str) -> TestResult<i32> {
    let run_id: i32 = sqlx::query_scalar(
        "INSERT INTO extraction_runs \
         (document_id, pass_number, model_name, raw_output, schema_version, started_at, status) \
         VALUES ($1, 1, 'test-model', '{}'::jsonb, 'v0', NOW(), 'RUNNING') \
         RETURNING id",
    )
    .bind(doc_id)
    .fetch_one(pool)
    .await?;

    let item_id: i32 = sqlx::query_scalar(
        "INSERT INTO extraction_items \
         (run_id, document_id, entity_type, item_data) \
         VALUES ($1, $2, 'TestEntity', '{}'::jsonb) \
         RETURNING id",
    )
    .bind(run_id)
    .bind(doc_id)
    .fetch_one(pool)
    .await?;
    Ok(item_id)
}

/// Insert a raw `authored_relationships` row with an explicit provenance and
/// document_id. Used to seed both the deletable extracted edge and the
/// must-survive authored edge.
async fn seed_authored_relationship(
    pool: &PgPool,
    case_slug: &str,
    doc_id: &str,
    provenance: &str,
    from_id: &str,
    to_id: &str,
    rel_type: &str,
) -> TestResult<()> {
    sqlx::query(
        "INSERT INTO authored_relationships \
         (case_slug, from_entity_id, to_entity_id, relationship_type, provenance, document_id) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(case_slug)
    .bind(from_id)
    .bind(to_id)
    .bind(rel_type)
    .bind(provenance)
    .bind(doc_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Count `authored_relationships` rows for a document at a given provenance.
async fn count_authored_for(pool: &PgPool, doc_id: &str, provenance: &str) -> TestResult<i64> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM authored_relationships \
         WHERE document_id = $1 AND provenance = $2",
    )
    .bind(doc_id)
    .bind(provenance)
    .fetch_one(pool)
    .await?;
    Ok(n)
}

// ─────────────────────────────────────────────────────────────────────────
// Tests (all #[ignore] — require live DEV infra)
// ─────────────────────────────────────────────────────────────────────────

/// Fix 1 regression: a document carrying `review_edit_history` rows must
/// delete cleanly. Before the fix, `delete_all_document_data` deleted
/// `extraction_items` without first clearing the history rows whose
/// `item_id` FK is RESTRICT — aborting the whole transaction. This test
/// FAILS against the pre-fix code (the delete returns a FK violation).
#[tokio::test]
#[ignore]
async fn delete_all_succeeds_with_edit_history() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let doc_id = unique_doc_id("edithist");

    seed_document(&pool, &doc_id).await?;
    let item_id = seed_run_and_item(&pool, &doc_id).await?;

    // The row that triggers the RESTRICT FK abort on the items delete.
    sqlx::query(
        "INSERT INTO review_edit_history \
         (item_id, field_changed, old_value, new_value, changed_by) \
         VALUES ($1, 'review_status', 'PENDING', 'APPROVED', 'test-user')",
    )
    .bind(item_id)
    .execute(&pool)
    .await?;

    // The operation under test — must NOT error on the FK.
    delete_all_document_data(&pool, &doc_id).await?;

    // Document row is gone…
    let doc_remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents WHERE id = $1")
        .bind(&doc_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(doc_remaining, 0, "documents row must be deleted");

    // …and so are the history rows for its items.
    let history_remaining: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM review_edit_history WHERE item_id = $1")
            .bind(item_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(
        history_remaining, 0,
        "review_edit_history rows must be cleared by the delete"
    );

    Ok(())
}

/// Fix 2: the delete removes the document's extracted (Pass-2) authored
/// relationships, while authored/canonical rows for the SAME document
/// survive. This test FAILS against the pre-fix code (no authored-rel
/// delete existed, so the extracted row would still be present).
#[tokio::test]
#[ignore]
async fn delete_all_removes_extracted_authored_only() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let doc_id = unique_doc_id("authored");
    let case_slug = format!("{CASE_SLUG_BASE}__test_ep3pre3");

    seed_document(&pool, &doc_id).await?;

    // Deletable: provenance = 'extracted', owned by this document.
    seed_authored_relationship(
        &pool,
        &case_slug,
        &doc_id,
        PROVENANCE_EXTRACTED,
        &format!("{doc_id}-from-x"),
        &format!("{doc_id}-to-x"),
        "PROVES_ELEMENT",
    )
    .await?;

    // Must survive: provenance = 'authored', same document_id. This is the
    // provenance guard — only 'extracted' rows may be removed.
    seed_authored_relationship(
        &pool,
        &case_slug,
        &doc_id,
        PROVENANCE_AUTHORED,
        &format!("{doc_id}-from-y"),
        &format!("{doc_id}-to-y"),
        "BEARS_ON",
    )
    .await?;

    assert_eq!(
        count_authored_for(&pool, &doc_id, PROVENANCE_EXTRACTED).await?,
        1,
        "seed: one extracted row should exist before delete"
    );
    assert_eq!(
        count_authored_for(&pool, &doc_id, PROVENANCE_AUTHORED).await?,
        1,
        "seed: one authored row should exist before delete"
    );

    delete_all_document_data(&pool, &doc_id).await?;

    assert_eq!(
        count_authored_for(&pool, &doc_id, PROVENANCE_EXTRACTED).await?,
        0,
        "extracted authored relationship must be deleted"
    );
    assert_eq!(
        count_authored_for(&pool, &doc_id, PROVENANCE_AUTHORED).await?,
        1,
        "authored relationship must be preserved (provenance guard)"
    );

    // Clean up the surviving authored guard row (delete does not touch it).
    sqlx::query("DELETE FROM authored_relationships WHERE document_id = $1")
        .bind(&doc_id)
        .execute(&pool)
        .await?;

    Ok(())
}

/// Fix 3: after deleting one document, a Party node shared with a second
/// document survives but no longer lists the deleted doc id in its
/// `source_documents` array. The delete endpoint's `cleanup_neo4j` now
/// delegates to this exact `cleanup::cleanup_neo4j` (see the
/// `delete_path_cleanup_neo4j_delegates_to_canonical` consistency test that
/// pins that wiring), so exercising it here covers the delete path's strip
/// behavior. FAILS against the pre-fix path (the strip was absent, so the
/// stale doc id lingered).
#[tokio::test]
#[ignore]
async fn cleanup_neo4j_strips_doc_from_shared_array() -> TestResult<()> {
    let graph = neo4j_graph().await?;
    let owner_doc = unique_doc_id("owner");
    let deleted_doc = unique_doc_id("deleted");

    // Shared Party node: owned (scalar source_document) by `owner_doc`, but
    // its source_documents array lists BOTH documents. Deleting `deleted_doc`
    // must keep the node and strip only the deleted id from the array.
    let seed = format!(
        "CREATE (:{TEST_PARTY_LABEL} {{name: 'shared', source_document: $owner, \
         source_documents: [$owner, $deleted]}})"
    );
    let mut s = graph
        .execute(
            neo4rs::query(&seed)
                .param("owner", owner_doc.as_str())
                .param("deleted", deleted_doc.as_str()),
        )
        .await?;
    while s.next().await?.is_some() {}

    // Run the delete-path Neo4j cleanup for the deleted document.
    cleanup::cleanup_neo4j(&deleted_doc, &graph).await?;

    // Node still exists; array now holds only the owner doc id.
    let mut r = graph
        .execute(neo4rs::query(&format!(
            "MATCH (n:{TEST_PARTY_LABEL} {{name: 'shared'}}) \
                 RETURN n.source_documents AS sd, n.source_document AS owner"
        )))
        .await?;
    let row = r
        .next()
        .await?
        .ok_or("shared Party node was deleted but should survive")?;
    let sd: Vec<String> = row.get("sd")?;
    let owner: String = row.get("owner")?;

    assert_eq!(
        owner, owner_doc,
        "scalar source_document owner must be unchanged"
    );
    assert_eq!(
        sd,
        vec![owner_doc.clone()],
        "deleted doc id must be stripped from source_documents, leaving only the owner"
    );

    // Teardown: remove the seeded test node.
    let mut d = graph
        .execute(neo4rs::query(&format!(
            "MATCH (n:{TEST_PARTY_LABEL} {{name: 'shared'}}) DETACH DELETE n"
        )))
        .await?;
    while d.next().await?.is_some() {}

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Disk/code consistency test (Rule 21) — runs WITHOUT live infra
// ─────────────────────────────────────────────────────────────────────────

/// Pin the Fix 3 wiring: the delete endpoint's `cleanup_neo4j` must delegate
/// to the canonical `cleanup::cleanup_neo4j` (which performs the DETACH
/// DELETEs *and* the `source_documents` array strip) rather than re-implement
/// its own Cypher.
///
/// ## Why a source-scan test (CLAUDE.md Rule 21)
///
/// The live `cleanup_neo4j_strips_doc_from_shared_array` test proves the
/// canonical helper strips arrays, but it calls `cleanup::cleanup_neo4j`
/// directly — it cannot reach `delete.rs::cleanup_neo4j` without a live
/// `AppState`. So if the delegation were reverted (the delete path going back
/// to its own duplicated DETACH DELETE Cypher with no strip), no behavioral
/// test would catch it. This scan asserts the wiring on disk: the delegation
/// call is present, and the old duplicated Cypher signature is gone. It needs
/// no database, so it runs in the normal `cargo test` pass.
#[test]
fn delete_path_cleanup_neo4j_delegates_to_canonical() -> TestResult<()> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/api/pipeline/delete.rs");
    let src = std::fs::read_to_string(path)?;

    assert!(
        src.contains("cleanup::cleanup_neo4j(document_id, &state.graph)"),
        "delete.rs::cleanup_neo4j must delegate to the canonical \
         cleanup::cleanup_neo4j (Fix 3 wiring); found neither the call"
    );
    assert!(
        !src.contains("DETACH DELETE n RETURN count(n) AS removed"),
        "delete.rs must NOT re-implement the node-delete Cypher — that \
         duplication was removed in favor of delegating to cleanup::cleanup_neo4j"
    );
    Ok(())
}
