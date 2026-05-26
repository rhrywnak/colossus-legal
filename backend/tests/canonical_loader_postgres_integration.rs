//! backend/tests/canonical_loader_postgres_integration.rs
//!
//! Integration tests for the canonical loader's Tier-1 Postgres writes
//! (`canonical_elements::authored::write_authored_entities`). They exercise
//! the Postgres path directly (no Neo4j needed) against a live
//! `colossus_legal_v2` database, so every test is `#[ignore]` (no
//! `#[sqlx::test]` fixture infra in this repo). CI does NOT run them.
//!
//! Run manually:
//!   `cargo test -p colossus-legal-backend --test canonical_loader_postgres_integration -- \
//!      --ignored --test-threads=1`
//!
//! ## Case-slug safety
//!
//! These tests DELETE authored rows by `case_slug`, so they use a per-test
//! `awad_v_catholic_family_service__test_loader_<tag>` slug — never the bare
//! production slug — to avoid wiping real authored data.

use sqlx::PgPool;

use colossus_legal_backend::canonical_elements::authored::{
    count_authored, write_authored_entities,
};
use colossus_legal_backend::canonical_elements::schema::{CountFile, CountMetadata, ElementDef};
use colossus_legal_backend::config::AppConfig;
use colossus_legal_backend::repositories::pipeline_repository::{
    delete_authored_entities_for_case, delete_authored_relationships_by_type,
    list_authored_entities, list_authored_relationships,
};

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const CASE_SLUG_BASE: &str = "awad_v_catholic_family_service";

fn test_slug(tag: &str) -> String {
    format!("{CASE_SLUG_BASE}__test_loader_{tag}")
}

async fn pipeline_pool() -> TestResult<PgPool> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&config.pipeline_database_url)
        .await?;
    Ok(pool)
}

async fn cleanup(pool: &PgPool, slug: &str) -> TestResult<()> {
    delete_authored_relationships_by_type(pool, slug, "HAS_ELEMENT").await?;
    delete_authored_entities_for_case(pool, slug).await?;
    Ok(())
}

// ── Fixture builders (construct the parsed YAML shapes directly) ──

fn element(id: &str, order: u32) -> ElementDef {
    ElementDef {
        id: id.to_string(),
        order_in_count: order,
        element_name: format!("Element {id}"),
        title: format!("Title {id}"),
        theory_variant: None,
        what_plaintiff_must_prove: "must prove X".into(),
        controlling_authority: "Some v Case".into(),
        statutory_anchor: None,
        case_specific_notes: Some("note".into()),
    }
}

fn count_file(n: u32, element_ids: &[&str]) -> CountFile {
    CountFile {
        count: CountMetadata {
            count_number: n,
            count_name: format!("Count {n}"),
            template_name: format!("template_{n}"),
            burden_of_proof: "preponderance".into(),
            m_civ_ji_reference: None,
            chuck_review_required: None,
            chuck_review_note: None,
            special_note: None,
            controlling_authorities: vec![],
            doctrinal_requirements: vec![],
        },
        elements: element_ids
            .iter()
            .enumerate()
            .map(|(i, id)| element(id, i as u32 + 1))
            .collect(),
        breach_theories: vec![],
        improper_act_theories: vec![],
        declarations_sought: vec![],
    }
}

// ── (1) authored_entities rows for LegalCounts + Elements ────────

#[tokio::test]
#[ignore]
async fn write_produces_legalcount_and_element_entities() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("entities");
    cleanup(&pool, &slug).await?;

    let files = vec![
        count_file(1, &["element-1-1", "element-1-2"]),
        count_file(2, &["element-2-1"]),
    ];
    write_authored_entities(&pool, &slug, &files).await?;

    let entities = list_authored_entities(&pool, &slug, None).await?;
    assert_eq!(entities.len(), 5, "2 LegalCounts + 3 Elements");

    let counts: Vec<_> = entities
        .iter()
        .filter(|e| e.entity_type == "LegalCount")
        .collect();
    assert_eq!(counts.len(), 2);
    let c1 = counts
        .iter()
        .find(|c| c.entity_id == "count-1")
        .expect("count-1 present");
    assert_eq!(c1.item_data["count_number"], 1);
    assert_eq!(c1.item_data["template_name"], "template_1");
    assert_eq!(c1.provenance, "canonical");
    assert_eq!(c1.created_by.as_deref(), Some("loader"));
    assert!(counts.iter().any(|c| c.entity_id == "count-2"));

    let elements: Vec<_> = entities
        .iter()
        .filter(|e| e.entity_type == "Element")
        .collect();
    assert_eq!(elements.len(), 3);
    let e = elements
        .iter()
        .find(|e| e.entity_id == "element-2-1")
        .expect("element-2-1 present");
    assert_eq!(
        e.item_data["parent_count_number"], 2,
        "Element ties back to its Count"
    );
    assert_eq!(e.item_data["order_in_count"], 1);

    cleanup(&pool, &slug).await?;
    Ok(())
}

// ── (2) authored_relationships HAS_ELEMENT rows ──────────────────

#[tokio::test]
#[ignore]
async fn write_produces_has_element_relationships() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("rels");
    cleanup(&pool, &slug).await?;

    let files = vec![count_file(1, &["element-1-1", "element-1-2"])];
    write_authored_entities(&pool, &slug, &files).await?;

    let rels = list_authored_relationships(&pool, &slug, Some("HAS_ELEMENT")).await?;
    assert_eq!(rels.len(), 2);
    assert!(
        rels.iter().all(|r| r.from_entity_id == "count-1"),
        "HAS_ELEMENT from the Count"
    );
    assert!(rels.iter().any(|r| r.to_entity_id == "element-1-1"));
    assert!(rels.iter().any(|r| r.to_entity_id == "element-1-2"));
    assert!(rels.iter().all(|r| r.provenance == "canonical"));
    // order_in_count rides on the edge.
    let r1 = rels
        .iter()
        .find(|r| r.to_entity_id == "element-1-1")
        .unwrap();
    assert_eq!(r1.properties.as_ref().expect("props")["order_in_count"], 1);

    cleanup(&pool, &slug).await?;
    Ok(())
}

// ── (3) entity_id values are the cross-tier ids (count-{N} / YAML id) ──

#[tokio::test]
#[ignore]
async fn write_uses_cross_tier_entity_ids() -> TestResult<()> {
    // The entity_id values written here are the SAME strings the Neo4j side
    // uses as node `id` (count-{N} via set_legal_count_id; element-x-y via
    // upsert_element MERGE on {id}), which is how the tiers connect.
    let pool = pipeline_pool().await?;
    let slug = test_slug("ids");
    cleanup(&pool, &slug).await?;

    let files = vec![count_file(3, &["element-3-1"])];
    write_authored_entities(&pool, &slug, &files).await?;

    let entities = list_authored_entities(&pool, &slug, None).await?;
    let ids: Vec<&str> = entities.iter().map(|e| e.entity_id.as_str()).collect();
    assert!(
        ids.contains(&"count-3"),
        "LegalCount entity_id is count-{{N}}"
    );
    assert!(
        ids.contains(&"element-3-1"),
        "Element entity_id is the YAML id"
    );

    let rels = list_authored_relationships(&pool, &slug, Some("HAS_ELEMENT")).await?;
    assert_eq!(rels.len(), 1);
    assert_eq!(rels[0].from_entity_id, "count-3");
    assert_eq!(rels[0].to_entity_id, "element-3-1");

    cleanup(&pool, &slug).await?;
    Ok(())
}

// ── (4) idempotent across runs ───────────────────────────────────

#[tokio::test]
#[ignore]
async fn write_is_idempotent_across_runs() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("idempotent");
    cleanup(&pool, &slug).await?;

    let files = vec![
        count_file(1, &["element-1-1"]),
        count_file(2, &["element-2-1", "element-2-2"]),
    ];

    write_authored_entities(&pool, &slug, &files).await?;
    let e1 = list_authored_entities(&pool, &slug, None).await?.len();
    let r1 = list_authored_relationships(&pool, &slug, None).await?.len();

    write_authored_entities(&pool, &slug, &files).await?; // second run, same YAML
    let e2 = list_authored_entities(&pool, &slug, None).await?.len();
    let r2 = list_authored_relationships(&pool, &slug, None).await?.len();

    assert_eq!(e1, e2, "entity count stable across runs");
    assert_eq!(r1, r2, "relationship count stable across runs");

    // And the row counts match the pure predictor used for the report.
    let predicted = count_authored(&files);
    assert_eq!(e2 as u64, predicted.entities); // 2 counts + 3 elements = 5
    assert_eq!(r2 as u64, predicted.relationships); // 3 HAS_ELEMENT

    cleanup(&pool, &slug).await?;
    Ok(())
}

// ── (reconcile) removing an Element from the YAML drops its rows ─

#[tokio::test]
#[ignore]
async fn write_reconciles_removed_elements() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("reconcile");
    cleanup(&pool, &slug).await?;

    write_authored_entities(
        &pool,
        &slug,
        &[count_file(1, &["element-1-1", "element-1-2"])],
    )
    .await?;
    assert_eq!(
        list_authored_entities(&pool, &slug, Some("Element"))
            .await?
            .len(),
        2
    );

    // Re-run with element-1-2 dropped: delete-then-insert reconciles it away.
    write_authored_entities(&pool, &slug, &[count_file(1, &["element-1-1"])]).await?;
    let elems = list_authored_entities(&pool, &slug, Some("Element")).await?;
    assert_eq!(elems.len(), 1, "removed Element is reconciled away");
    assert_eq!(elems[0].entity_id, "element-1-1");
    assert_eq!(
        list_authored_relationships(&pool, &slug, Some("HAS_ELEMENT"))
            .await?
            .len(),
        1,
        "its HAS_ELEMENT edge is gone too"
    );

    cleanup(&pool, &slug).await?;
    Ok(())
}
