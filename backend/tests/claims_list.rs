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
    state::AppState,
};
use neo4rs::{query, Graph};
use tokio::sync::Mutex;

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

static GRAPH_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
const TEST_SOURCE: &str = "test";

fn unique_run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    format!("t2-1b-{nanos}")
}

async fn setup_graph() -> TestResult<Graph> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env().map_err(|e| format!("config error: {e}"))?;
    let graph = create_neo4j_graph(&config)
        .await
        .map_err(|e| format!("neo4j connect error: {e}"))?;
    Ok(graph)
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
    let _guard = GRAPH_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .await;

    let graph = setup_graph().await?;
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
    };
    let response = list_claims(State(state)).await.into_response();

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
    let _guard = GRAPH_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .await;

    let graph = setup_graph().await?;
    cleanup_claims(&graph).await?;

    let state = AppState {
        graph: graph.clone(),
    };
    let response = list_claims(State(state)).await.into_response();

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
