//! backend/tests/authored_entities_integration.rs
//!
//! Integration tests for `repositories::pipeline_repository::authored_entities`
//! (three-tier architecture, Option A — migration
//! `20260526141630_create_authored_entity_tables.sql`).
//!
//! Every test is `#[ignore]` because they require a live `colossus_legal_v2`
//! PostgreSQL database — the project has no `#[sqlx::test]` fixture infra
//! (see `config_overrides.rs` tests for the same convention). CI does NOT
//! run them.
//!
//! Run manually against live DEV infra:
//!   `cargo test -p colossus-legal-backend --test authored_entities_integration -- \
//!      --ignored --test-threads=1`
//!
//! ## Case-slug safety
//!
//! These tests DELETE rows by `case_slug`. They therefore must NOT use the
//! bare production slug `awad_v_catholic_family_service` — doing so would
//! let an accidental run wipe real authored Elements/Counts. Each test uses
//! that slug as a documented base with a per-test `__test_<tag>` suffix, so
//! deletes only ever touch the test's own rows and the suite is
//! re-runnable. `entity_id` is globally UNIQUE, so test ids are namespaced
//! by tag too.

use sqlx::PgPool;

use colossus_legal_backend::config::AppConfig;
use colossus_legal_backend::repositories::pipeline_repository::{
    delete_authored_entities_for_case, delete_authored_relationships_by_type, get_authored_entity,
    list_authored_entities, list_authored_relationships, upsert_authored_entity,
    upsert_authored_relationship,
};

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Documented base slug (the real matter). Tests append `__test_<tag>` so
/// destructive cleanup never touches production rows. See the module doc.
const CASE_SLUG_BASE: &str = "awad_v_catholic_family_service";

fn test_slug(tag: &str) -> String {
    format!("{CASE_SLUG_BASE}__test_{tag}")
}

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

/// Clear any rows left by a prior run so each test starts from a known
/// state. Relationship types must be listed explicitly (delete is by type).
async fn reset_case(pool: &PgPool, slug: &str, rel_types: &[&str]) -> TestResult<()> {
    delete_authored_entities_for_case(pool, slug).await?;
    for t in rel_types {
        delete_authored_relationships_by_type(pool, slug, t).await?;
    }
    Ok(())
}

// ── authored_entities ────────────────────────────────────────────

/// (1) Insert a new entity, retrieve it by entity_id, verify the fields.
#[tokio::test]
#[ignore]
async fn upsert_and_get_authored_entity_roundtrip() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("get_roundtrip");
    reset_case(&pool, &slug, &[]).await?;

    let data = serde_json::json!({ "element_name": "duty", "title": "Duty of Care" });
    let id = upsert_authored_entity(
        &pool,
        &slug,
        "Element",
        "el-get-1",
        &data,
        "authored",
        Some("tester"),
    )
    .await?;
    assert!(id > 0, "insert must return a positive serial id");

    let fetched = get_authored_entity(&pool, "el-get-1")
        .await?
        .expect("entity should exist after insert");
    assert_eq!(fetched.entity_id, "el-get-1");
    assert_eq!(fetched.case_slug, slug);
    assert_eq!(fetched.entity_type, "Element");
    assert_eq!(fetched.provenance, "authored");
    assert_eq!(fetched.created_by.as_deref(), Some("tester"));
    assert_eq!(fetched.item_data["element_name"], "duty");

    delete_authored_entities_for_case(&pool, &slug).await?;
    Ok(())
}

/// (2) Upsert an existing entity_id: row is reused, item_data updated,
/// updated_at advances.
#[tokio::test]
#[ignore]
async fn upsert_authored_entity_updates_existing() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("entity_update");
    reset_case(&pool, &slug, &[]).await?;

    let v1 = serde_json::json!({ "title": "v1" });
    upsert_authored_entity(
        &pool,
        &slug,
        "Element",
        "el-upd-1",
        &v1,
        "authored",
        Some("a"),
    )
    .await?;
    let first = get_authored_entity(&pool, "el-upd-1")
        .await?
        .expect("exists after insert");

    // NOW() is per-statement; a short sleep guarantees a strictly later
    // updated_at so the assertion below is not flaky on a fast DB.
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let v2 = serde_json::json!({ "title": "v2" });
    upsert_authored_entity(
        &pool,
        &slug,
        "Element",
        "el-upd-1",
        &v2,
        "canonical",
        Some("b"),
    )
    .await?;
    let second = get_authored_entity(&pool, "el-upd-1")
        .await?
        .expect("exists after upsert");

    assert_eq!(
        second.id, first.id,
        "upsert must reuse the row, not insert a new one"
    );
    assert_eq!(second.item_data["title"], "v2", "item_data must be updated");
    assert_eq!(second.provenance, "canonical", "provenance must be updated");
    assert!(
        second.updated_at > first.updated_at,
        "updated_at must advance on upsert"
    );

    delete_authored_entities_for_case(&pool, &slug).await?;
    Ok(())
}

/// (3) List returns all entities for a case; filtering by entity_type
/// returns the correct subset.
#[tokio::test]
#[ignore]
async fn list_authored_entities_filters_by_type() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("entity_list");
    reset_case(&pool, &slug, &[]).await?;

    let empty = serde_json::json!({});
    upsert_authored_entity(
        &pool,
        &slug,
        "Element",
        "el-list-1",
        &empty,
        "authored",
        None,
    )
    .await?;
    upsert_authored_entity(
        &pool,
        &slug,
        "Element",
        "el-list-2",
        &empty,
        "authored",
        None,
    )
    .await?;
    upsert_authored_entity(
        &pool,
        &slug,
        "LegalCount",
        "count-list-1",
        &empty,
        "authored",
        None,
    )
    .await?;

    let all = list_authored_entities(&pool, &slug, None).await?;
    assert_eq!(all.len(), 3, "no filter returns all three");

    let elements = list_authored_entities(&pool, &slug, Some("Element")).await?;
    assert_eq!(elements.len(), 2);
    assert!(elements.iter().all(|e| e.entity_type == "Element"));

    let counts = list_authored_entities(&pool, &slug, Some("LegalCount")).await?;
    assert_eq!(counts.len(), 1);
    assert_eq!(counts[0].entity_id, "count-list-1");

    delete_authored_entities_for_case(&pool, &slug).await?;
    Ok(())
}

/// (4) Delete-by-case clears every entity for the case.
#[tokio::test]
#[ignore]
async fn delete_authored_entities_for_case_clears_all() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("entity_delete");
    reset_case(&pool, &slug, &[]).await?;

    let empty = serde_json::json!({});
    upsert_authored_entity(
        &pool, &slug, "Element", "el-del-1", &empty, "authored", None,
    )
    .await?;
    upsert_authored_entity(
        &pool,
        &slug,
        "LegalCount",
        "count-del-1",
        &empty,
        "authored",
        None,
    )
    .await?;
    assert_eq!(list_authored_entities(&pool, &slug, None).await?.len(), 2);

    let removed = delete_authored_entities_for_case(&pool, &slug).await?;
    assert_eq!(removed, 2, "delete must report the two rows removed");
    assert!(
        list_authored_entities(&pool, &slug, None).await?.is_empty(),
        "case must have no entities after delete"
    );
    Ok(())
}

// ── authored_relationships ───────────────────────────────────────

/// (5) Insert a relationship, retrieve it via list.
#[tokio::test]
#[ignore]
async fn upsert_and_list_authored_relationship_roundtrip() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("rel_roundtrip");
    reset_case(&pool, &slug, &["PROVES_ELEMENT"]).await?;

    let props = serde_json::json!({ "confidence": 0.9 });
    let id = upsert_authored_relationship(
        &pool,
        &slug,
        "allegation-1",
        "el-1",
        "PROVES_ELEMENT",
        Some(&props),
        "mapped",
        Some("mapper"),
    )
    .await?;
    assert!(id > 0);

    let rels = list_authored_relationships(&pool, &slug, Some("PROVES_ELEMENT")).await?;
    assert_eq!(rels.len(), 1);
    let r = &rels[0];
    assert_eq!(r.from_entity_id, "allegation-1");
    assert_eq!(r.to_entity_id, "el-1");
    assert_eq!(r.relationship_type, "PROVES_ELEMENT");
    assert_eq!(r.provenance, "mapped");
    assert_eq!(r.created_by.as_deref(), Some("mapper"));
    assert_eq!(
        r.properties.as_ref().expect("properties present")["confidence"],
        0.9
    );

    delete_authored_relationships_by_type(&pool, &slug, "PROVES_ELEMENT").await?;
    Ok(())
}

/// (6) Upsert the same edge: row reused, properties + provenance updated.
#[tokio::test]
#[ignore]
async fn upsert_authored_relationship_updates_properties() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("rel_update");
    reset_case(&pool, &slug, &["PROVES_ELEMENT"]).await?;

    let p1 = serde_json::json!({ "confidence": 0.5 });
    let id1 = upsert_authored_relationship(
        &pool,
        &slug,
        "allegation-2",
        "el-2",
        "PROVES_ELEMENT",
        Some(&p1),
        "mapped",
        None,
    )
    .await?;

    let p2 = serde_json::json!({ "confidence": 0.95, "note": "revised" });
    let id2 = upsert_authored_relationship(
        &pool,
        &slug,
        "allegation-2",
        "el-2",
        "PROVES_ELEMENT",
        Some(&p2),
        "authored",
        None,
    )
    .await?;

    assert_eq!(id1, id2, "same edge must upsert in place, not duplicate");
    let rels = list_authored_relationships(&pool, &slug, Some("PROVES_ELEMENT")).await?;
    assert_eq!(rels.len(), 1, "no duplicate edge created");
    let props = rels[0].properties.as_ref().expect("properties present");
    assert_eq!(props["confidence"], 0.95, "properties must be updated");
    assert_eq!(props["note"], "revised");
    assert_eq!(rels[0].provenance, "authored");

    delete_authored_relationships_by_type(&pool, &slug, "PROVES_ELEMENT").await?;
    Ok(())
}

/// (7) List filters relationships by relationship_type.
#[tokio::test]
#[ignore]
async fn list_authored_relationships_filters_by_type() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("rel_list");
    let types = ["HAS_ELEMENT", "PROVES_ELEMENT"];
    reset_case(&pool, &slug, &types).await?;

    upsert_authored_relationship(
        &pool,
        &slug,
        "count-1",
        "el-a",
        "HAS_ELEMENT",
        None,
        "canonical",
        None,
    )
    .await?;
    upsert_authored_relationship(
        &pool,
        &slug,
        "count-1",
        "el-b",
        "HAS_ELEMENT",
        None,
        "canonical",
        None,
    )
    .await?;
    upsert_authored_relationship(
        &pool,
        &slug,
        "allegation-x",
        "el-a",
        "PROVES_ELEMENT",
        None,
        "mapped",
        None,
    )
    .await?;

    assert_eq!(
        list_authored_relationships(&pool, &slug, None).await?.len(),
        3
    );
    assert_eq!(
        list_authored_relationships(&pool, &slug, Some("HAS_ELEMENT"))
            .await?
            .len(),
        2
    );
    assert_eq!(
        list_authored_relationships(&pool, &slug, Some("PROVES_ELEMENT"))
            .await?
            .len(),
        1
    );

    reset_case(&pool, &slug, &types).await?;
    Ok(())
}

/// (8) Delete-by-type removes only the targeted type, leaving others.
#[tokio::test]
#[ignore]
async fn delete_authored_relationships_by_type_is_selective() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("rel_delete");
    let types = ["HAS_ELEMENT", "PROVES_ELEMENT"];
    reset_case(&pool, &slug, &types).await?;

    upsert_authored_relationship(
        &pool,
        &slug,
        "count-9",
        "el-9",
        "HAS_ELEMENT",
        None,
        "canonical",
        None,
    )
    .await?;
    upsert_authored_relationship(
        &pool,
        &slug,
        "allegation-9",
        "el-9",
        "PROVES_ELEMENT",
        None,
        "mapped",
        None,
    )
    .await?;

    let removed = delete_authored_relationships_by_type(&pool, &slug, "PROVES_ELEMENT").await?;
    assert_eq!(removed, 1, "only the one PROVES_ELEMENT edge is removed");

    let remaining = list_authored_relationships(&pool, &slug, None).await?;
    assert_eq!(remaining.len(), 1, "HAS_ELEMENT edge must remain");
    assert_eq!(remaining[0].relationship_type, "HAS_ELEMENT");

    reset_case(&pool, &slug, &types).await?;
    Ok(())
}
