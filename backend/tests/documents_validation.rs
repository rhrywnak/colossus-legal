use std::sync::OnceLock;

use axum::{body::to_bytes, extract::State, http::StatusCode, response::IntoResponse};
use chrono::Utc;
use colossus_auth::AuthUser;
use colossus_legal_backend::{
    api::documents::{create_document, get_document, update_document},
    config::AppConfig,
    dto::document::{DocumentCreateRequest, DocumentDto, DocumentUpdateRequest},
    neo4j::create_neo4j_graph,
    repositories::audit_repository::AuditRepository,
    state::{AppState, SchemaMetadata},
};
use neo4rs::{query, Graph};
use serde_json::Value;
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

fn dummy_pipeline_pool() -> sqlx::PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/dummy_pipeline")
        .expect("lazy pool")
}

fn dummy_audit_repo() -> AuditRepository {
    AuditRepository::new(
        sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/dummy_audit")
            .expect("lazy pool"),
    )
}

static GRAPH_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

fn test_editor() -> AuthUser {
    AuthUser {
        username: "test_editor".to_string(),
        email: "editor@test.com".to_string(),
        display_name: "Test Editor".to_string(),
        groups: vec!["legal_editor".to_string()],
    }
}

async fn setup() -> TestResult<(Graph, AppConfig)> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env().map_err(|e| format!("config error: {e}"))?;
    let graph = create_neo4j_graph(&config)
        .await
        .map_err(|e| format!("neo4j connect error: {e}"))?;
    Ok((graph, config))
}

async fn cleanup_document_by_id(graph: &Graph, id: &str) -> Result<(), neo4rs::Error> {
    let mut result = graph
        .execute(query("MATCH (d:Document {id: $id}) DETACH DELETE d").param("id", id))
        .await?;
    while result.next().await?.is_some() {}
    Ok(())
}

fn base_create_payload(title: &str, doc_type: &str) -> DocumentCreateRequest {
    DocumentCreateRequest {
        title: title.to_string(),
        doc_type: doc_type.to_string(),
        created_at: None,
        description: None,
        file_path: None,
        uploaded_at: None,
        related_claim_id: None,
        source_url: None,
    }
}

#[tokio::test]
async fn create_document_rejects_empty_title() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let (graph, config) = setup().await?;
    let state = AppState {
        graph: graph.clone(),
        config,
        rag_pipeline: None,
        http_client: reqwest::Client::new(),
        pg_pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/dummy")
            .expect("lazy pool"),
        pipeline_pool: dummy_pipeline_pool(),
        audit_repo: dummy_audit_repo(),
        embedding_provider: std::sync::Arc::new(TestEmbeddingProvider),
        schema_metadata: SchemaMetadata {
            document_type: String::new(),
            entity_types: vec![],
            relationship_types: vec![],
        },
    };

    let payload = base_create_payload("", "pdf");

    let response = create_document(test_editor(), State(state), axum::Json(payload))
        .await
        .into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), 1024 * 1024).await?;
    let json: Value = serde_json::from_slice(&body)?;
    assert_eq!(json["error"], "validation_error");

    Ok(())
}

#[tokio::test]
async fn create_document_rejects_invalid_doc_type() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let (graph, config) = setup().await?;
    let state = AppState {
        graph: graph.clone(),
        config,
        rag_pipeline: None,
        http_client: reqwest::Client::new(),
        pg_pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/dummy")
            .expect("lazy pool"),
        pipeline_pool: dummy_pipeline_pool(),
        audit_repo: dummy_audit_repo(),
        embedding_provider: std::sync::Arc::new(TestEmbeddingProvider),
        schema_metadata: SchemaMetadata {
            document_type: String::new(),
            entity_types: vec![],
            relationship_types: vec![],
        },
    };

    let payload = base_create_payload("Valid Title", "invalid-type");

    let response = create_document(test_editor(), State(state), axum::Json(payload))
        .await
        .into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), 1024 * 1024).await?;
    let json: Value = serde_json::from_slice(&body)?;
    assert_eq!(json["error"], "validation_error");

    Ok(())
}

#[tokio::test]
async fn create_document_rejects_invalid_created_at() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let (graph, config) = setup().await?;
    let state = AppState {
        graph: graph.clone(),
        config,
        rag_pipeline: None,
        http_client: reqwest::Client::new(),
        pg_pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/dummy")
            .expect("lazy pool"),
        pipeline_pool: dummy_pipeline_pool(),
        audit_repo: dummy_audit_repo(),
        embedding_provider: std::sync::Arc::new(TestEmbeddingProvider),
        schema_metadata: SchemaMetadata {
            document_type: String::new(),
            entity_types: vec![],
            relationship_types: vec![],
        },
    };

    let mut payload = base_create_payload("Valid Title", "pdf");
    payload.created_at = Some("not-a-date".to_string());

    let response = create_document(test_editor(), State(state), axum::Json(payload))
        .await
        .into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), 1024 * 1024).await?;
    let json: Value = serde_json::from_slice(&body)?;
    assert_eq!(json["error"], "validation_error");

    Ok(())
}

#[tokio::test]
async fn get_document_returns_404_when_missing() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let (graph, config) = setup().await?;
    let state = AppState {
        graph: graph.clone(),
        config,
        rag_pipeline: None,
        http_client: reqwest::Client::new(),
        pg_pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/dummy")
            .expect("lazy pool"),
        pipeline_pool: dummy_pipeline_pool(),
        audit_repo: dummy_audit_repo(),
        embedding_provider: std::sync::Arc::new(TestEmbeddingProvider),
        schema_metadata: SchemaMetadata {
            document_type: String::new(),
            entity_types: vec![],
            relationship_types: vec![],
        },
    };

    let response = get_document(
        None,
        State(state),
        axum::extract::Path("no-such".to_string()),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = to_bytes(response.into_body(), 1024 * 1024).await?;
    let json: Value = serde_json::from_slice(&body)?;
    assert_eq!(json["error"], "not_found");

    Ok(())
}

#[tokio::test]
async fn update_document_rejects_invalid_doc_type() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let (graph, config) = setup().await?;
    let state = AppState {
        graph: graph.clone(),
        config,
        rag_pipeline: None,
        http_client: reqwest::Client::new(),
        pg_pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/dummy")
            .expect("lazy pool"),
        pipeline_pool: dummy_pipeline_pool(),
        audit_repo: dummy_audit_repo(),
        embedding_provider: std::sync::Arc::new(TestEmbeddingProvider),
        schema_metadata: SchemaMetadata {
            document_type: String::new(),
            entity_types: vec![],
            relationship_types: vec![],
        },
    };

    let payload = base_create_payload("Title", "pdf");
    let created_response =
        create_document(test_editor(), State(state.clone()), axum::Json(payload))
            .await
            .into_response();
    assert_eq!(created_response.status(), StatusCode::CREATED);
    let body = to_bytes(created_response.into_body(), 1024 * 1024).await?;
    let created: DocumentDto = serde_json::from_slice(&body)?;

    let update_payload = DocumentUpdateRequest {
        title: None,
        doc_type: Some("bad-type".to_string()),
        created_at: None,
        description: None,
        file_path: None,
        uploaded_at: None,
        related_claim_id: None,
        source_url: None,
    };

    let response = update_document(
        test_editor(),
        State(state.clone()),
        axum::extract::Path(created.id.clone()),
        axum::Json(update_payload),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), 1024 * 1024).await?;
    let json: Value = serde_json::from_slice(&body)?;
    assert_eq!(json["error"], "validation_error");

    cleanup_document_by_id(&graph, &created.id).await?;
    Ok(())
}

#[tokio::test]
async fn update_document_returns_404_when_missing() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let (graph, config) = setup().await?;
    let state = AppState {
        graph: graph.clone(),
        config,
        rag_pipeline: None,
        http_client: reqwest::Client::new(),
        pg_pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/dummy")
            .expect("lazy pool"),
        pipeline_pool: dummy_pipeline_pool(),
        audit_repo: dummy_audit_repo(),
        embedding_provider: std::sync::Arc::new(TestEmbeddingProvider),
        schema_metadata: SchemaMetadata {
            document_type: String::new(),
            entity_types: vec![],
            relationship_types: vec![],
        },
    };

    let update_payload = DocumentUpdateRequest {
        title: Some("New".to_string()),
        doc_type: Some("pdf".to_string()),
        created_at: None,
        description: None,
        file_path: None,
        uploaded_at: None,
        related_claim_id: None,
        source_url: None,
    };

    let response = update_document(
        test_editor(),
        State(state),
        axum::extract::Path("missing-id".to_string()),
        axum::Json(update_payload),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = to_bytes(response.into_body(), 1024 * 1024).await?;
    let json: Value = serde_json::from_slice(&body)?;
    assert_eq!(json["error"], "not_found");

    Ok(())
}

#[tokio::test]
async fn happy_path_create_get_update_document() -> TestResult<()> {
    let _guard = GRAPH_MUTEX.get_or_init(|| Mutex::new(())).lock().await;

    let (graph, config) = setup().await?;
    let state = AppState {
        graph: graph.clone(),
        config,
        rag_pipeline: None,
        http_client: reqwest::Client::new(),
        pg_pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/dummy")
            .expect("lazy pool"),
        pipeline_pool: dummy_pipeline_pool(),
        audit_repo: dummy_audit_repo(),
        embedding_provider: std::sync::Arc::new(TestEmbeddingProvider),
        schema_metadata: SchemaMetadata {
            document_type: String::new(),
            entity_types: vec![],
            relationship_types: vec![],
        },
    };

    let mut payload = base_create_payload("Happy Title", "pdf");
    payload.created_at = Some(Utc::now().to_rfc3339());

    let create_response = create_document(test_editor(), State(state.clone()), axum::Json(payload))
        .await
        .into_response();

    assert_eq!(create_response.status(), StatusCode::CREATED);
    let body = to_bytes(create_response.into_body(), 1024 * 1024).await?;
    let created: DocumentDto = serde_json::from_slice(&body)?;

    let get_response = get_document(
        None,
        State(state.clone()),
        axum::extract::Path(created.id.clone()),
    )
    .await
    .into_response();

    assert_eq!(get_response.status(), StatusCode::OK);
    let get_body = to_bytes(get_response.into_body(), 1024 * 1024).await?;
    let fetched: DocumentDto = serde_json::from_slice(&get_body)?;
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.title, "Happy Title");

    let update_payload = DocumentUpdateRequest {
        title: Some("Updated Title".to_string()),
        doc_type: Some("motion".to_string()),
        created_at: None,
        description: None,
        file_path: None,
        uploaded_at: None,
        related_claim_id: None,
        source_url: None,
    };

    let update_response = update_document(
        test_editor(),
        State(state),
        axum::extract::Path(fetched.id.clone()),
        axum::Json(update_payload),
    )
    .await
    .into_response();

    assert_eq!(update_response.status(), StatusCode::OK);
    let update_body = to_bytes(update_response.into_body(), 1024 * 1024).await?;
    let updated: DocumentDto = serde_json::from_slice(&update_body)?;
    assert_eq!(updated.title, "Updated Title");
    assert_eq!(updated.doc_type, "motion");

    cleanup_document_by_id(&graph, &updated.id).await?;
    Ok(())
}
