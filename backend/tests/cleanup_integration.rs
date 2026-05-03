//! backend/tests/cleanup_integration.rs
//!
//! Integration tests for `crate::pipeline::steps::cleanup`. Every test in
//! this file is marked `#[ignore]` because they require live DEV Neo4j,
//! Qdrant, and PostgreSQL. CI does NOT run them.
//!
//! To run manually against live DEV infra:
//!   `cargo test -p colossus-legal-backend --test cleanup_integration -- \
//!      --ignored --test-threads=1`
//!
//! Each test seeds its own scoped data and cleans up after itself so the
//! suite can be run repeatedly without drift.

use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use colossus_extract::{EmbeddingProvider, LlmProvider};
use colossus_legal_backend::{
    config::AppConfig,
    neo4j::create_neo4j_graph,
    pipeline::steps::cleanup::{
        cleanup_all, cleanup_neo4j, cleanup_postgres, cleanup_qdrant, CleanupError,
    },
    services::qdrant_service,
    state::AppState, // not used directly — kept so the import list mirrors the documents_list.rs pattern
};
use neo4rs::Graph;
use reqwest::Client;
use sqlx::PgPool;
use tokio::sync::{Mutex, Semaphore};

// Silence the `AppState` import; it is kept for parallelism with the other
// integration test files but not referenced in these tests.
#[allow(dead_code)]
fn _appstate_import_noop(_s: AppState) {}

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

// Serialize integration tests that share DEV infra; running them in parallel
// leads to flaky interference on seed data.
static LIVE_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Stub provider used only to satisfy `AppContext`'s `Arc<dyn LlmProvider>`
/// field. None of these tests exercise LLM calls — if any does, we want a
/// loud panic, not silent success.
struct PanicLlm;

#[async_trait::async_trait]
impl LlmProvider for PanicLlm {
    async fn invoke(
        &self,
        _prompt: &str,
        _max_tokens: u32,
    ) -> Result<colossus_extract::LlmResponse, colossus_extract::PipelineError> {
        panic!("PanicLlm::invoke called — cleanup tests must not hit the LLM");
    }
    fn model_name(&self) -> &str {
        "panic-llm"
    }
    fn provider_name(&self) -> &str {
        // Mirrors model_name; the cleanup tests don't hit the LLM
        // (invoke panics), so the provider-name string only matters
        // for any code path that logs the provider on its way to a
        // non-LLM operation. "panic-llm" is the unambiguous sentinel.
        "panic-llm"
    }
    fn cost_per_input_token(&self) -> Option<f64> {
        None
    }
    fn cost_per_output_token(&self) -> Option<f64> {
        None
    }
    fn supports_structured_output(&self) -> bool {
        false
    }
}

/// Stub embedding provider. Same reasoning as `PanicLlm`.
struct PanicEmbedding;

#[async_trait::async_trait]
impl EmbeddingProvider for PanicEmbedding {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, colossus_extract::PipelineError> {
        panic!("PanicEmbedding::embed called — cleanup tests must not embed");
    }
    fn dimensions(&self) -> u32 {
        768
    }
    fn model_name(&self) -> &str {
        "panic-embedding"
    }
}

/// Construct a cleanup-focused [`AppContext`] from live-DEV env vars. The
/// LLM and embedding providers are stubs because the cleanup path does not
/// touch them.
async fn live_context() -> TestResult<colossus_legal_backend::pipeline::context::AppContext> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;
    let graph = create_neo4j_graph(&config).await?;
    let pipeline_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&config.pipeline_database_url)
        .await?;
    let http_client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(5))
        .build()?;

    Ok(colossus_legal_backend::pipeline::context::AppContext {
        pipeline_pool,
        graph,
        qdrant_url: config.qdrant_url.clone(),
        http_client,
        schema_dir: config.extraction_schema_dir.clone(),
        template_dir: config.extraction_template_dir.clone(),
        // Both directories are real production paths from `AppConfig`
        // (see config.rs:39, 41). Cleanup tests only exercise on_cancel
        // / on_delete hooks that don't load profile or system-prompt
        // files, so these values are never read at test time — but the
        // `AppContext` struct now requires them, so we populate them
        // from the same source production does.
        profile_dir: config.processing_profile_dir.clone(),
        system_prompt_dir: config.system_prompt_dir.clone(),
        document_storage_path: config.document_storage_path.clone(),
        llm_provider: Arc::new(PanicLlm) as Arc<dyn LlmProvider>,
        embedding_provider: Arc::new(PanicEmbedding) as Arc<dyn EmbeddingProvider>,
        llm_semaphore: Arc::new(Semaphore::new(1)),
    })
}

fn unique_doc_id(scope: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    format!("cleanup-{scope}-{nanos}")
}

// ─────────────────────────────────────────────────────────────────────────
// Seed / verify helpers
// ─────────────────────────────────────────────────────────────────────────

async fn seed_neo4j(graph: &Graph, doc_id: &str) -> Result<(), neo4rs::Error> {
    // Two nodes keyed on source_document, one keyed on source_document_id.
    let mut s1 = graph
        .execute(
            neo4rs::query("CREATE (:TestCleanup {source_document: $id, kind: 'a'})")
                .param("id", doc_id),
        )
        .await?;
    while s1.next().await?.is_some() {}
    let mut s2 = graph
        .execute(
            neo4rs::query("CREATE (:TestCleanup {source_document: $id, kind: 'b'})")
                .param("id", doc_id),
        )
        .await?;
    while s2.next().await?.is_some() {}
    let mut s3 = graph
        .execute(
            neo4rs::query("CREATE (:TestCleanup {source_document_id: $id, kind: 'doc'})")
                .param("id", doc_id),
        )
        .await?;
    while s3.next().await?.is_some() {}
    Ok(())
}

async fn count_neo4j_for(graph: &Graph, doc_id: &str) -> Result<i64, neo4rs::Error> {
    let mut r = graph
        .execute(
            neo4rs::query(
                "MATCH (n) WHERE n.source_document = $id OR n.source_document_id = $id RETURN count(n) AS c",
            )
            .param("id", doc_id),
        )
        .await?;
    let count = match r.next().await? {
        Some(row) => row.get::<i64>("c").unwrap_or(0),
        None => 0,
    };
    Ok(count)
}

async fn seed_qdrant(client: &Client, qdrant_url: &str, doc_id: &str) -> TestResult<()> {
    // A single test point with a 768-dim zero vector.
    let dummy_vector = vec![0.0f32; 768];
    let id: u64 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
    let point = qdrant_service::QdrantPoint {
        id,
        vector: dummy_vector,
        payload: serde_json::json!({ "document_id": doc_id, "node_type": "test" }),
    };
    qdrant_service::upsert_points(client, qdrant_url, vec![point]).await?;
    Ok(())
}

async fn count_qdrant_for(client: &Client, qdrant_url: &str, doc_id: &str) -> TestResult<usize> {
    let n =
        qdrant_service::count_points_by_filter(client, qdrant_url, "document_id", doc_id).await?;
    Ok(n)
}

async fn seed_postgres(pool: &PgPool, doc_id: &str) -> TestResult<()> {
    // `documents` is the FK target for every row we seed; it must exist first.
    sqlx::query(
        "INSERT INTO documents (id, title, file_path, file_hash, document_type, status) \
         VALUES ($1, $2, $3, $4, 'pdf', 'UPLOADED') \
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(doc_id)
    .bind(format!("test-{doc_id}"))
    .bind(format!("/tmp/{doc_id}.pdf"))
    .bind(format!("hash-{doc_id}"))
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO document_text (document_id, page_number, text_content) \
         VALUES ($1, 1, 'test page')",
    )
    .bind(doc_id)
    .execute(pool)
    .await?;

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

    sqlx::query(
        "INSERT INTO extraction_relationships \
         (run_id, document_id, from_item_id, to_item_id, relationship_type) \
         VALUES ($1, $2, $3, $3, 'TEST_REL')",
    )
    .bind(run_id)
    .bind(doc_id)
    .bind(item_id)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO extraction_chunks (extraction_run_id, chunk_index, chunk_text) \
         VALUES ($1, 0, 'chunk text')",
    )
    .bind(run_id)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO pipeline_config \
         (document_id, schema_file, created_by) \
         VALUES ($1, 'test.yaml', 'test-user')",
    )
    .bind(doc_id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn count_postgres_for(pool: &PgPool, doc_id: &str) -> TestResult<i64> {
    let n: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM extraction_relationships WHERE document_id = $1) \
              + (SELECT COUNT(*) FROM extraction_items WHERE document_id = $1) \
              + (SELECT COUNT(*) FROM extraction_chunks \
                 WHERE extraction_run_id IN (SELECT id FROM extraction_runs WHERE document_id = $1)) \
              + (SELECT COUNT(*) FROM extraction_runs WHERE document_id = $1) \
              + (SELECT COUNT(*) FROM document_text WHERE document_id = $1) \
              + (SELECT COUNT(*) FROM pipeline_config WHERE document_id = $1)",
    )
    .bind(doc_id)
    .fetch_one(pool)
    .await?;
    Ok(n)
}

async fn drop_documents_row(pool: &PgPool, doc_id: &str) -> TestResult<()> {
    sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(doc_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Tests (all #[ignore] — require live DEV infra)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn cleanup_neo4j_removes_nodes_by_both_properties() -> TestResult<()> {
    let _g = LIVE_MUTEX.get_or_init(|| Mutex::new(())).lock().await;
    let ctx = live_context().await?;
    let doc_id = unique_doc_id("neo4j");

    seed_neo4j(&ctx.graph, &doc_id).await?;
    assert_eq!(count_neo4j_for(&ctx.graph, &doc_id).await?, 3);

    let report = cleanup_neo4j(&doc_id, &ctx.graph).await?;
    assert_eq!(report.nodes_by_source_document, 2);
    assert_eq!(report.nodes_by_source_document_id, 1);
    assert_eq!(count_neo4j_for(&ctx.graph, &doc_id).await?, 0);
    Ok(())
}

#[tokio::test]
#[ignore]
async fn cleanup_qdrant_removes_vectors_by_filter() -> TestResult<()> {
    let _g = LIVE_MUTEX.get_or_init(|| Mutex::new(())).lock().await;
    let ctx = live_context().await?;
    let doc_id = unique_doc_id("qdrant");

    seed_qdrant(&ctx.http_client, &ctx.qdrant_url, &doc_id).await?;
    assert_eq!(
        count_qdrant_for(&ctx.http_client, &ctx.qdrant_url, &doc_id).await?,
        1
    );

    let report = cleanup_qdrant(&doc_id, &ctx).await?;
    assert_eq!(report.vectors_deleted, 1);
    assert_eq!(
        count_qdrant_for(&ctx.http_client, &ctx.qdrant_url, &doc_id).await?,
        0
    );
    Ok(())
}

#[tokio::test]
#[ignore]
async fn cleanup_postgres_clears_all_step_tables() -> TestResult<()> {
    let _g = LIVE_MUTEX.get_or_init(|| Mutex::new(())).lock().await;
    let ctx = live_context().await?;
    let doc_id = unique_doc_id("pg");

    seed_postgres(&ctx.pipeline_pool, &doc_id).await?;
    assert_eq!(count_postgres_for(&ctx.pipeline_pool, &doc_id).await?, 6);

    let report = cleanup_postgres(&doc_id, &ctx.pipeline_pool).await?;
    let tables: Vec<&str> = report.tables_cleared.iter().map(|(t, _)| *t).collect();
    assert_eq!(
        tables,
        vec![
            "extraction_relationships",
            "extraction_items",
            "extraction_chunks",
            "extraction_runs",
            "document_text",
            "pipeline_config",
        ]
    );
    let total: u64 = report.tables_cleared.iter().map(|(_, n)| *n).sum();
    assert_eq!(total, 6);
    assert_eq!(count_postgres_for(&ctx.pipeline_pool, &doc_id).await?, 0);

    drop_documents_row(&ctx.pipeline_pool, &doc_id).await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn cleanup_all_success_path() -> TestResult<()> {
    let _g = LIVE_MUTEX.get_or_init(|| Mutex::new(())).lock().await;
    let ctx = live_context().await?;
    let doc_id = unique_doc_id("all");

    seed_neo4j(&ctx.graph, &doc_id).await?;
    seed_qdrant(&ctx.http_client, &ctx.qdrant_url, &doc_id).await?;
    seed_postgres(&ctx.pipeline_pool, &doc_id).await?;

    let report = cleanup_all(&doc_id, &ctx.pipeline_pool, &ctx).await?;
    assert_eq!(report.neo4j.nodes_by_source_document, 2);
    assert_eq!(report.neo4j.nodes_by_source_document_id, 1);
    assert_eq!(report.qdrant.vectors_deleted, 1);
    assert_eq!(report.postgres.tables_cleared.len(), 6);

    assert_eq!(count_neo4j_for(&ctx.graph, &doc_id).await?, 0);
    assert_eq!(
        count_qdrant_for(&ctx.http_client, &ctx.qdrant_url, &doc_id).await?,
        0
    );
    assert_eq!(count_postgres_for(&ctx.pipeline_pool, &doc_id).await?, 0);

    drop_documents_row(&ctx.pipeline_pool, &doc_id).await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn cleanup_all_partial_failure_returns_partial() -> TestResult<()> {
    let _g = LIVE_MUTEX.get_or_init(|| Mutex::new(())).lock().await;
    let mut ctx = live_context().await?;
    let doc_id = unique_doc_id("partial");

    seed_neo4j(&ctx.graph, &doc_id).await?;
    seed_postgres(&ctx.pipeline_pool, &doc_id).await?;

    // Force Qdrant to fail — unroutable address. Use a short-timeout client
    // so we don't wait 30 seconds for the default timeout.
    ctx.qdrant_url = "http://127.0.0.1:1".to_string();
    ctx.http_client = Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .connect_timeout(std::time::Duration::from_millis(500))
        .build()?;

    let result = cleanup_all(&doc_id, &ctx.pipeline_pool, &ctx).await;
    let partial = match result {
        Err(CleanupError::Partial { .. }) => result.unwrap_err(),
        other => panic!("expected CleanupError::Partial, got {:?}", other),
    };
    if let CleanupError::Partial {
        neo4j_error,
        qdrant_error,
        postgres_error,
        partial_report,
        ..
    } = partial
    {
        assert!(neo4j_error.is_none(), "neo4j should succeed");
        assert!(postgres_error.is_none(), "postgres should succeed");
        assert!(qdrant_error.is_some(), "qdrant should fail");
        assert!(
            partial_report.neo4j.nodes_by_source_document
                + partial_report.neo4j.nodes_by_source_document_id
                > 0
        );
        assert!(!partial_report.postgres.tables_cleared.is_empty());
    }

    drop_documents_row(&ctx.pipeline_pool, &doc_id).await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn cleanup_all_is_idempotent() -> TestResult<()> {
    let _g = LIVE_MUTEX.get_or_init(|| Mutex::new(())).lock().await;
    let ctx = live_context().await?;
    let doc_id = unique_doc_id("idem");

    seed_neo4j(&ctx.graph, &doc_id).await?;
    seed_qdrant(&ctx.http_client, &ctx.qdrant_url, &doc_id).await?;
    seed_postgres(&ctx.pipeline_pool, &doc_id).await?;

    let first = cleanup_all(&doc_id, &ctx.pipeline_pool, &ctx).await?;
    assert!(first.postgres.tables_cleared.iter().any(|(_, n)| *n > 0));

    let second = cleanup_all(&doc_id, &ctx.pipeline_pool, &ctx).await?;
    assert_eq!(second.neo4j.nodes_by_source_document, 0);
    assert_eq!(second.neo4j.nodes_by_source_document_id, 0);
    assert_eq!(second.qdrant.vectors_deleted, 0);
    let total: u64 = second.postgres.tables_cleared.iter().map(|(_, n)| *n).sum();
    assert_eq!(total, 0, "re-running cleanup_all must be a no-op");

    drop_documents_row(&ctx.pipeline_pool, &doc_id).await?;
    Ok(())
}
