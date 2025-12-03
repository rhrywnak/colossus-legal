use std::{
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::to_bytes,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{Duration, Utc};
use colossus_legal_backend::{
    api::documents::{list_recent_documents, RecentDocumentsQuery},
    config::AppConfig,
    dto::DocumentDto,
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
    format!("t3-1e-{nanos}")
}

async fn setup_graph() -> TestResult<Graph> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env().map_err(|e| format!("config error: {e}"))?;
    let graph = create_neo4j_graph(&config)
        .await
        .map_err(|e| format!("neo4j connect error: {e}"))?;
    Ok(graph)
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
    ingested_at: Option<String>,
) -> Result<(), neo4rs::Error> {
    let created_at = Utc::now().to_rfc3339();
    let mut result = graph
        .execute(
            query(
                "CREATE (d:Document {
                    id: $id,
                    title: $title,
                    doc_type: $doc_type,
                    created_at: $created_at,
                    ingested_at: $ingested_at,
                    source: $source,
                    test_run_id: $run_id
                })",
            )
            .param("id", id)
            .param("title", title)
            .param("doc_type", doc_type)
            .param("created_at", created_at)
            .param("ingested_at", ingested_at)
            .param("source", TEST_SOURCE)
            .param("run_id", run_id),
        )
        .await?;

    while result.next().await?.is_some() {}
    Ok(())
}

#[tokio::test]
async fn list_recent_documents_returns_sorted_and_excludes_missing_ingested_at() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let graph = setup_graph().await?;
    cleanup_documents(&graph).await?;

    let run_id = unique_run_id();
    let newest_time = Utc::now();
    let middle_time = newest_time - Duration::seconds(30);
    let oldest_time = newest_time - Duration::seconds(60);

    insert_document(
        &graph,
        &run_id,
        "doc-oldest",
        "Oldest",
        "pdf",
        Some(oldest_time.to_rfc3339()),
    )
    .await?;
    insert_document(
        &graph,
        &run_id,
        "doc-middle",
        "Middle",
        "motion",
        Some(middle_time.to_rfc3339()),
    )
    .await?;
    insert_document(
        &graph,
        &run_id,
        "doc-newest",
        "Newest",
        "ruling",
        Some(newest_time.to_rfc3339()),
    )
    .await?;
    insert_document(
        &graph,
        &run_id,
        "doc-without-ingested",
        "No Ingested",
        "pdf",
        None,
    )
    .await?;

    let state = AppState {
        graph: graph.clone(),
    };
    let response = list_recent_documents(
        State(state),
        Query(RecentDocumentsQuery { limit: None }),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = to_bytes(response.into_body(), 1024 * 1024).await?;
    let documents: Vec<DocumentDto> = serde_json::from_slice(&body_bytes)?;

    assert_eq!(documents.len(), 3, "should only include ingested documents");
    let ids: Vec<_> = documents.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids, ["doc-newest", "doc-middle", "doc-oldest"]);

    cleanup_documents(&graph).await?;
    Ok(())
}

#[tokio::test]
async fn list_recent_documents_honors_limit() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let graph = setup_graph().await?;
    cleanup_documents(&graph).await?;

    let run_id = unique_run_id();
    let now = Utc::now();
    insert_document(
        &graph,
        &run_id,
        "doc-one",
        "One",
        "pdf",
        Some((now - Duration::seconds(10)).to_rfc3339()),
    )
    .await?;
    insert_document(
        &graph,
        &run_id,
        "doc-two",
        "Two",
        "pdf",
        Some(now.to_rfc3339()),
    )
    .await?;
    insert_document(
        &graph,
        &run_id,
        "doc-three",
        "Three",
        "pdf",
        Some((now - Duration::seconds(20)).to_rfc3339()),
    )
    .await?;

    let state = AppState {
        graph: graph.clone(),
    };
    let response = list_recent_documents(
        State(state),
        Query(RecentDocumentsQuery { limit: Some(2) }),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = to_bytes(response.into_body(), 1024 * 1024).await?;
    let documents: Vec<DocumentDto> = serde_json::from_slice(&body_bytes)?;

    assert_eq!(documents.len(), 2, "limit should cap results");
    let ids: Vec<_> = documents.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids, ["doc-two", "doc-one"], "sorted by ingested_at desc");

    cleanup_documents(&graph).await?;
    Ok(())
}
