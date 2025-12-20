use std::{
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{body::to_bytes, extract::State, http::StatusCode, response::IntoResponse};
use colossus_legal_backend::{
    api::claims::{create_claim, get_claim, update_claim},
    config::AppConfig,
    dto::claim::{ClaimCreateRequest, ClaimDto, ClaimUpdateRequest},
    neo4j::create_neo4j_graph,
    state::AppState,
};
use neo4rs::{query, Graph};
use serde_json::Value;
use tokio::sync::Mutex;

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

static GRAPH_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
const TEST_SOURCE: &str = "test";

fn unique_run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    format!("t2-1c-{nanos}")
}

async fn setup_graph() -> TestResult<Graph> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env().map_err(|e| format!("config error: {e}"))?;
    let graph = create_neo4j_graph(&config)
        .await
        .map_err(|e| format!("neo4j connect error: {e}"))?;
    Ok(graph)
}

async fn cleanup_claim_by_id(graph: &Graph, id: &str) -> Result<(), neo4rs::Error> {
    let mut result = graph
        .execute(query("MATCH (c:Claim {id: $id}) DETACH DELETE c").param("id", id))
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
async fn create_claim_rejects_empty_title() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let graph = setup_graph().await?;
    let state = AppState {
        graph: graph.clone(),
    };

    let payload = ClaimCreateRequest {
        title: "".to_string(),
        description: None,
        status: "open".to_string(),
    };

    let response = create_claim(State(state), axum::Json(payload))
        .await
        .into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), 1024 * 1024).await?;
    let json: Value = serde_json::from_slice(&body)?;
    assert_eq!(json["error"], "validation_error");

    Ok(())
}

#[tokio::test]
async fn create_claim_rejects_invalid_status() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let graph = setup_graph().await?;
    let state = AppState {
        graph: graph.clone(),
    };

    let payload = ClaimCreateRequest {
        title: "Valid Title".to_string(),
        description: None,
        status: "invalid".to_string(),
    };

    let response = create_claim(State(state), axum::Json(payload))
        .await
        .into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), 1024 * 1024).await?;
    let json: Value = serde_json::from_slice(&body)?;
    assert_eq!(json["error"], "validation_error");

    Ok(())
}

#[tokio::test]
async fn get_claim_returns_404_when_missing() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let graph = setup_graph().await?;
    let state = AppState {
        graph: graph.clone(),
    };

    let response = get_claim(State(state), axum::extract::Path("no-such".to_string()))
        .await
        .into_response();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = to_bytes(response.into_body(), 1024 * 1024).await?;
    let json: Value = serde_json::from_slice(&body)?;
    assert_eq!(json["error"], "not_found");

    Ok(())
}

#[tokio::test]
async fn update_claim_rejects_invalid_status() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let graph = setup_graph().await?;
    let run_id = unique_run_id();
    let claim_id = format!("claim-{run_id}");
    insert_claim(
        &graph,
        &run_id,
        &claim_id,
        "Original Title",
        Some("desc"),
        "open",
    )
    .await?;

    let state = AppState {
        graph: graph.clone(),
    };

    let payload = ClaimUpdateRequest {
        title: None,
        description: None,
        status: Some("bad-status".to_string()),
    };

    let response = update_claim(
        State(state),
        axum::extract::Path(claim_id.clone()),
        axum::Json(payload),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), 1024 * 1024).await?;
    let json: Value = serde_json::from_slice(&body)?;
    assert_eq!(json["error"], "validation_error");

    cleanup_claim_by_id(&graph, &claim_id).await?;
    Ok(())
}

#[ignore] // TODO: Re-enable after T5.4.x updates ClaimRepository to v2 schema
#[tokio::test]
async fn happy_path_create_and_get_claim() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let graph = setup_graph().await?;
    let state = AppState {
        graph: graph.clone(),
    };

    let payload = ClaimCreateRequest {
        title: "Happy Title".to_string(),
        description: Some("Happy description".to_string()),
        status: "open".to_string(),
    };

    let response = create_claim(State(state.clone()), axum::Json(payload))
        .await
        .into_response();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = to_bytes(response.into_body(), 1024 * 1024).await?;
    let created: ClaimDto = serde_json::from_slice(&body)?;

    let get_response = get_claim(State(state), axum::extract::Path(created.id.clone()))
        .await
        .into_response();

    assert_eq!(get_response.status(), StatusCode::OK);
    let get_body = to_bytes(get_response.into_body(), 1024 * 1024).await?;
    let fetched: ClaimDto = serde_json::from_slice(&get_body)?;
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.title, "Happy Title");

    cleanup_claim_by_id(&graph, &created.id).await?;
    Ok(())
}
