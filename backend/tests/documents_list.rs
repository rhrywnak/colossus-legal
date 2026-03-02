use std::{
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{body::to_bytes, extract::State, http::StatusCode, response::IntoResponse};
use chrono::Utc;
use colossus_legal_backend::{
    api::documents::list_documents, config::AppConfig, dto::DocumentDto, neo4j::create_neo4j_graph,
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
    format!("t3-1b-{nanos}")
}

async fn setup() -> TestResult<(Graph, AppConfig)> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env().map_err(|e| format!("config error: {e}"))?;
    let graph = create_neo4j_graph(&config)
        .await
        .map_err(|e| format!("neo4j connect error: {e}"))?;
    Ok((graph, config))
}

async fn cleanup_documents(graph: &Graph) -> Result<(), neo4rs::Error> {
    let mut result = graph
        .execute(
            query("MATCH (d:Document {source: $source}) DETACH DELETE d")
                .param("source", TEST_SOURCE),
        )
        .await?;
    while result.next().await?.is_some() {}
    Ok(())
}

async fn insert_document(
    graph: &Graph,
    run_id: &str,
    id: &str,
    title: &str,
    doc_type: &str,
) -> Result<(), neo4rs::Error> {
    let created_at = Utc::now().to_rfc3339();
    let mut result = graph
        .execute(
            query(
                "CREATE (d:Document {id: $id, title: $title, doc_type: $doc_type, created_at: $created_at, test_run_id: $run_id, source: $source})",
            )
            .param("id", id)
            .param("title", title)
            .param("doc_type", doc_type)
            .param("created_at", created_at)
            .param("run_id", run_id)
            .param("source", TEST_SOURCE),
        )
        .await?;

    while result.next().await?.is_some() {}
    Ok(())
}

#[tokio::test]
async fn get_documents_returns_non_empty_when_data_exists() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let (graph, config) = setup().await?;
    cleanup_documents(&graph).await?;

    let run_id = unique_run_id();
    let doc_id = format!("doc-{run_id}");
    insert_document(&graph, &run_id, &doc_id, "Test Document", "pdf").await?;

    let state = AppState {
        graph: graph.clone(),
        config,
    };
    let response = list_documents(None, State(state)).await.into_response();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = to_bytes(response.into_body(), 1024 * 1024).await?;
    let documents: Vec<DocumentDto> = serde_json::from_slice(&body_bytes)?;

    assert!(
        !documents.is_empty(),
        "Expected at least one document in response"
    );

    let inserted = documents
        .iter()
        .find(|d| d.id == doc_id)
        .expect("inserted document should be present");

    assert_eq!(inserted.title, "Test Document");
    assert_eq!(inserted.doc_type, "pdf");

    cleanup_documents(&graph).await?;
    Ok(())
}

#[tokio::test]
async fn get_documents_returns_empty_when_no_data() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let (graph, config) = setup().await?;
    cleanup_documents(&graph).await?;

    let state = AppState {
        graph: graph.clone(),
        config,
    };
    let response = list_documents(None, State(state)).await.into_response();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = to_bytes(response.into_body(), 1024 * 1024).await?;
    let documents: Vec<DocumentDto> = serde_json::from_slice(&body_bytes)?;
    assert!(
        documents.is_empty(),
        "Expected empty documents list when no data"
    );

    cleanup_documents(&graph).await?;
    Ok(())
}
