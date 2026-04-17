use std::{
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{body::to_bytes, extract::State, http::StatusCode, response::IntoResponse};
use colossus_legal_backend::{
    api::claims::list_claims,
    config::AppConfig,
    dto::claim::ClaimDto,
    neo4j::create_neo4j_graph,
    state::{AppState, SchemaMetadata},
};
use neo4rs::{query, Graph};
use tokio::sync::Mutex;

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Minimal `EmbeddingProvider` stub for tests.
///
/// Tests that construct `AppState` need a concrete provider to satisfy the
/// trait object. This stub compiles and satisfies the trait without doing
/// any real work — `embed()` panics if called, because no test in this
/// file should actually trigger embedding. `dimensions()` returns the
/// historical Nomic default of 768 so any code that reads it during a test
/// sees a sensible value.
struct TestEmbeddingProvider;

#[async_trait::async_trait]
impl colossus_extract::EmbeddingProvider for TestEmbeddingProvider {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, colossus_extract::PipelineError> {
        panic!("TestEmbeddingProvider::embed called in a test — tests should not exercise real embedding")
    }
    fn dimensions(&self) -> u32 {
        768
    }
    fn model_name(&self) -> &str {
        "test-embedding-provider"
    }
}

static GRAPH_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
const TEST_SOURCE: &str = "test";

fn unique_run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    format!("t2-1b-{nanos}")
}

async fn setup() -> TestResult<(Graph, AppConfig)> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env().map_err(|e| format!("config error: {e}"))?;
    let graph = create_neo4j_graph(&config)
        .await
        .map_err(|e| format!("neo4j connect error: {e}"))?;
    Ok((graph, config))
}

async fn cleanup_claims(graph: &Graph) -> Result<(), neo4rs::Error> {
    let mut result = graph
        .execute(
            query("MATCH (c:Claim {source: $source}) DETACH DELETE c").param("source", TEST_SOURCE),
        )
        .await?;
    while result.next().await?.is_some() {}
    Ok(())
}

async fn insert_claim(
    graph: &Graph,
    run_id: &str,
    id: &str,
    title: &str,
    description: Option<&str>,
    status: &str,
) -> Result<(), neo4rs::Error> {
    let mut result = graph
        .execute(
            query(
                "CREATE (c:Claim {id: $id, title: $title, description: $description, status: $status, test_run_id: $run_id, source: $source})",
            )
            .param("id", id)
            .param("title", title)
            .param("description", description)
            .param("status", status)
            .param("run_id", run_id)
            .param("source", TEST_SOURCE),
        )
        .await?;

    while result.next().await?.is_some() {}
    Ok(())
}

#[tokio::test]
async fn get_claims_returns_non_empty_when_data_exists() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let (graph, config) = setup().await?;
    cleanup_claims(&graph).await?;

    let run_id = unique_run_id();
    let claim_id = format!("claim-{run_id}");
    insert_claim(
        &graph,
        &run_id,
        &claim_id,
        "Test Claim Title",
        Some("Test claim description"),
        "open",
    )
    .await?;

    let state = AppState {
        graph: graph.clone(),
        config,
        rag_pipeline: None,
        http_client: reqwest::Client::new(),
        pg_pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/dummy")
            .expect("lazy pool"),
        pipeline_pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/dummy_pipeline")
            .expect("lazy pool"),
        audit_repo: colossus_legal_backend::repositories::audit_repository::AuditRepository::new(
            sqlx::postgres::PgPoolOptions::new()
                .connect_lazy("postgres://localhost/dummy_audit")
                .expect("lazy pool"),
        ),
        embedding_provider: std::sync::Arc::new(TestEmbeddingProvider),
        schema_metadata: SchemaMetadata {
            document_type: String::new(),
            entity_types: vec![],
            relationship_types: vec![],
        },
    };
    let response = list_claims(None, State(state)).await.into_response();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = to_bytes(response.into_body(), 1024 * 1024).await?;
    let claims: Vec<ClaimDto> = serde_json::from_slice(&body_bytes)?;

    assert!(
        !claims.is_empty(),
        "Expected at least one claim in response"
    );

    let inserted = claims
        .iter()
        .find(|c| c.id == claim_id)
        .expect("inserted claim should be present");

    assert_eq!(inserted.title, "Test Claim Title");
    assert_eq!(inserted.status, "open");

    cleanup_claims(&graph).await?;
    Ok(())
}

#[tokio::test]
async fn get_claims_returns_empty_when_no_data() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let (graph, config) = setup().await?;
    cleanup_claims(&graph).await?;

    let state = AppState {
        graph: graph.clone(),
        config,
        rag_pipeline: None,
        http_client: reqwest::Client::new(),
        pg_pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/dummy")
            .expect("lazy pool"),
        pipeline_pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/dummy_pipeline")
            .expect("lazy pool"),
        audit_repo: colossus_legal_backend::repositories::audit_repository::AuditRepository::new(
            sqlx::postgres::PgPoolOptions::new()
                .connect_lazy("postgres://localhost/dummy_audit")
                .expect("lazy pool"),
        ),
        embedding_provider: std::sync::Arc::new(TestEmbeddingProvider),
        schema_metadata: SchemaMetadata {
            document_type: String::new(),
            entity_types: vec![],
            relationship_types: vec![],
        },
    };
    let response = list_claims(None, State(state)).await.into_response();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = to_bytes(response.into_body(), 1024 * 1024).await?;
    let claims: Vec<ClaimDto> = serde_json::from_slice(&body_bytes)?;
    assert!(
        claims.is_empty(),
        "Expected empty claims list when graph has no Claim nodes"
    );

    cleanup_claims(&graph).await?;
    Ok(())
}
