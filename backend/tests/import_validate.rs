//! Integration tests for POST /import/validate endpoint.
//!
//! Tests the full HTTP request/response cycle using tower::ServiceExt::oneshot.

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use colossus_legal_backend::{
    api::router,
    config::AppConfig,
    models::import::{ValidationErrorType, ValidationResult},
    neo4j::create_neo4j_graph,
    state::{AppState, SchemaMetadata},
};
use tower::ServiceExt;

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

/// Create the app router with Neo4j state.
/// Note: Validation endpoint doesn't query Neo4j, but router requires AppState.
async fn setup_app() -> TestResult<Router> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env().map_err(|e| format!("config error: {e}"))?;
    let graph = create_neo4j_graph(&config)
        .await
        .map_err(|e| format!("neo4j connect error: {e}"))?;
    let pg_pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/dummy")
        .expect("lazy pool");
    let pipeline_pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/dummy_pipeline")
        .expect("lazy pool");
    let audit_repo = colossus_legal_backend::repositories::audit_repository::AuditRepository::new(
        sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/dummy_audit")
            .expect("lazy pool"),
    );
    let state = AppState {
        graph,
        config,
        rag_pipeline: None,
        http_client: reqwest::Client::new(),
        pg_pool,
        pipeline_pool,
        audit_repo,
        embedding_provider: std::sync::Arc::new(TestEmbeddingProvider),
        schema_metadata: SchemaMetadata {
            document_type: String::new(),
            entity_types: vec![],
            relationship_types: vec![],
        },
        chat_providers: std::collections::HashMap::new(),
        default_chat_model: String::new(),
    };
    Ok(router().with_state(state))
}

/// Valid import JSON with one claim.
fn valid_import_json() -> String {
    r#"{
        "schema_version": "2.1",
        "extraction_metadata": {
            "extracted_at": "2025-12-23T00:00:00Z",
            "extraction_model": "claude-opus"
        },
        "source_document": {
            "id": "doc-001",
            "title": "Test Motion",
            "doc_type": "motion"
        },
        "case": {
            "id": "case-001",
            "name": "Test v. Test"
        },
        "parties": {
            "plaintiffs": [{"id": "p1", "name": "Plaintiff", "role": "plaintiff"}],
            "defendants": [{"id": "d1", "name": "Defendant", "role": "defendant"}]
        },
        "claims": [
            {
                "id": "CLAIM-001",
                "category": "fraud",
                "quote": "Test quote for claim",
                "made_by": "p1",
                "against": ["d1"],
                "source": {"document_id": "doc-001"}
            }
        ]
    }"#
    .to_string()
}

/// Helper to make POST request and parse response.
async fn post_validate(app: Router, body: &str) -> TestResult<(StatusCode, ValidationResult)> {
    let request = Request::builder()
        .method("POST")
        .uri("/import/validate")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))?;

    let response = app.oneshot(request).await?;
    let status = response.status();
    let body_bytes = to_bytes(response.into_body(), usize::MAX).await?;
    let result: ValidationResult = serde_json::from_slice(&body_bytes)?;
    Ok((status, result))
}

#[tokio::test]
async fn test_endpoint_valid_complete_request() -> TestResult<()> {
    let app = setup_app().await?;
    let (status, result) = post_validate(app, &valid_import_json()).await?;

    assert_eq!(status, StatusCode::OK);
    assert!(result.valid, "Expected valid=true for valid input");
    assert_eq!(result.claim_count, 1);
    assert_eq!(result.document_title, "Test Motion");
    assert!(result.errors.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_endpoint_invalid_json_syntax() -> TestResult<()> {
    let app = setup_app().await?;
    let (status, result) = post_validate(app, "{ invalid json }").await?;

    assert_eq!(status, StatusCode::OK); // Validation errors are data, not HTTP errors
    assert!(!result.valid);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(
        result.errors[0].error_type,
        ValidationErrorType::InvalidJson
    );
    assert_eq!(result.errors[0].field, "json");
    Ok(())
}

#[tokio::test]
async fn test_endpoint_missing_schema_version() -> TestResult<()> {
    let app = setup_app().await?;
    // schema_version is wrong (1.0 instead of 2.1)
    let json = r#"{"schema_version":"1.0","extraction_metadata":{"extracted_at":"x","extraction_model":"x"},"source_document":{"id":"d","title":"D","doc_type":"m"},"case":{"id":"c","name":"C"},"parties":{"plaintiffs":[],"defendants":[]},"claims":[{"id":"c1","category":"fraud","quote":"x","source":{"document_id":"d"},"made_by":"p","against":["d"]}]}"#;
    let (status, result) = post_validate(app, json).await?;

    assert_eq!(status, StatusCode::OK);
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.error_type == ValidationErrorType::SchemaVersionMismatch));
    Ok(())
}

#[tokio::test]
async fn test_endpoint_invalid_claim_category() -> TestResult<()> {
    let app = setup_app().await?;
    // Invalid category "bad_category"
    let json = r#"{"schema_version":"2.1","extraction_metadata":{"extracted_at":"x","extraction_model":"x"},"source_document":{"id":"d","title":"D","doc_type":"m"},"case":{"id":"c","name":"C"},"parties":{"plaintiffs":[],"defendants":[]},"claims":[{"id":"CLAIM-001","category":"bad_category","quote":"Test quote","source":{"document_id":"d"},"made_by":"p","against":["d"]}]}"#;
    let (status, result) = post_validate(app, json).await?;

    assert_eq!(status, StatusCode::OK);
    assert!(!result.valid);
    let cat_error = result.errors.iter().find(|e| e.field == "category");
    assert!(cat_error.is_some(), "Expected error for invalid category");
    assert_eq!(
        cat_error.unwrap().error_type,
        ValidationErrorType::InvalidValue
    );
    assert_eq!(cat_error.unwrap().claim_id, Some("CLAIM-001".to_string()));
    Ok(())
}

#[tokio::test]
async fn test_endpoint_duplicate_claim_ids() -> TestResult<()> {
    let app = setup_app().await?;
    // Two claims with same ID "CLAIM-001"
    let json = r#"{"schema_version":"2.1","extraction_metadata":{"extracted_at":"x","extraction_model":"x"},"source_document":{"id":"d","title":"D","doc_type":"m"},"case":{"id":"c","name":"C"},"parties":{"plaintiffs":[],"defendants":[]},"claims":[{"id":"CLAIM-001","category":"fraud","quote":"First","source":{"document_id":"d"},"made_by":"p","against":["d"]},{"id":"CLAIM-001","category":"fraud","quote":"Second","source":{"document_id":"d"},"made_by":"p","against":["d"]}]}"#;
    let (status, result) = post_validate(app, json).await?;

    assert_eq!(status, StatusCode::OK);
    assert!(!result.valid);
    let dup_errors: Vec<_> = result
        .errors
        .iter()
        .filter(|e| e.error_type == ValidationErrorType::DuplicateId)
        .collect();
    assert_eq!(dup_errors.len(), 1, "Expected one duplicate ID error");
    assert!(dup_errors[0].message.contains("CLAIM-001"));
    Ok(())
}

#[tokio::test]
async fn test_endpoint_multiple_validation_errors() -> TestResult<()> {
    let app = setup_app().await?;
    // Multiple errors: empty id, empty quote, bad category, empty against
    let json = r#"{"schema_version":"2.1","extraction_metadata":{"extracted_at":"x","extraction_model":"x"},"source_document":{"id":"d","title":"D","doc_type":"m"},"case":{"id":"c","name":"C"},"parties":{"plaintiffs":[],"defendants":[]},"claims":[{"id":"","category":"invalid","quote":"","source":{"document_id":"d"},"made_by":"p","against":[]}]}"#;
    let (status, result) = post_validate(app, json).await?;

    assert_eq!(status, StatusCode::OK);
    assert!(!result.valid);
    // Should have at least 4 errors: id, quote, category, against
    assert!(
        result.errors.len() >= 4,
        "Expected at least 4 errors, got {}",
        result.errors.len()
    );

    let fields: Vec<&str> = result.errors.iter().map(|e| e.field.as_str()).collect();
    assert!(fields.contains(&"id"), "Expected error for empty id");
    assert!(fields.contains(&"quote"), "Expected error for empty quote");
    assert!(
        fields.contains(&"category"),
        "Expected error for invalid category"
    );
    assert!(
        fields.contains(&"against"),
        "Expected error for empty against"
    );
    Ok(())
}
