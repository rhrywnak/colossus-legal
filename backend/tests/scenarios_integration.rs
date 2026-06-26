//! backend/tests/scenarios_integration.rs
//!
//! Integration tests for `repositories::pipeline_repository::scenario_store`
//! (the `scenarios` table — migration
//! `20260626115557_create_scenarios_table.sql`, in `colossus_legal_v2`).
//!
//! Every test is `#[ignore]` because they require a live `colossus_legal_v2`
//! PostgreSQL database — the project has no `#[sqlx::test]` fixture infra, so
//! CI does NOT run them (same convention as
//! `authored_entities_integration.rs`).
//!
//! Run manually against live DEV infra:
//!   `cargo test -p colossus-legal-backend --test scenarios_integration -- \
//!      --ignored --test-threads=1`
//!
//! ## Case-slug safety
//!
//! These tests DELETE rows by `case_slug`. They therefore must NOT use the bare
//! production slug. Each test derives a per-test `__test_<tag>` slug from the
//! documented base, so cleanup only ever touches the test's own rows and the
//! suite is re-runnable.

use serde_json::json;
use sqlx::PgPool;

use colossus_legal_backend::config::AppConfig;
use colossus_legal_backend::repositories::pipeline_repository::{
    delete_scenarios_for_case, get_scenario, insert_scenario, list_scenarios_for_case,
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

/// A fully-populated authored `definition` body, exercising every key the
/// renderer (task 1.5) will read — so the round-trip proves the jsonb survives
/// intact, not just that *a* json value persists.
fn sample_definition() -> serde_json::Value {
    json!({
        "schema_v": 1,
        "attack_text": "Defendant concealed the fee waiver from the plaintiff.",
        "attack_meaning": "Establishes the concealment element of fraud.",
        "wielders": ["person-marie-awad", "person-jeffrey-humphrey"],
        "target": "org-catholic-family-service",
        "seed_phrases": ["fee waiver", "concealed", "did not disclose"],
        "anti_seed_phrases": ["fully disclosed", "waived in writing"],
        "notes": "Cross-reference the November intake call."
    })
}

// ── Round-trip ───────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn it_round_trips_a_scenario() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("round_trip");
    delete_scenarios_for_case(&pool, &slug).await?;

    let definition = sample_definition();
    let anchors = vec!["alleg-1".to_string(), "alleg-2".to_string()];

    let id = insert_scenario(
        &pool,
        "Concealment of fee waiver",
        "offense",
        "needs_evidence",
        &slug,
        Some("count-1"),
        Some(&anchors),
        &definition,
    )
    .await?;

    let got = get_scenario(&pool, id)
        .await?
        .expect("inserted scenario must be readable by its id");

    // Spine survives intact.
    assert_eq!(got.scenario_id, id);
    assert_eq!(got.name, "Concealment of fee waiver");
    assert_eq!(got.direction, "offense");
    assert_eq!(got.status, "needs_evidence");
    assert_eq!(got.case_slug, slug);
    assert_eq!(got.feeds_count_id.as_deref(), Some("count-1"));
    assert_eq!(
        got.anchor_allegation_ids.as_deref(),
        Some(anchors.as_slice())
    );

    // jsonb survives intact, key for key.
    assert_eq!(got.definition, definition);
    assert_eq!(got.definition["schema_v"], json!(1));
    assert_eq!(got.definition["wielders"][0], json!("person-marie-awad"));
    assert_eq!(got.definition["seed_phrases"].as_array().unwrap().len(), 3);

    delete_scenarios_for_case(&pool, &slug).await?;
    Ok(())
}

// ── Absent row ───────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn it_returns_none_for_unknown_id() -> TestResult<()> {
    let pool = pipeline_pool().await?;

    // A fresh random uuid that was never inserted. `get_scenario` contracts
    // that a missing row is `Ok(None)`, not an error — assert that distinction.
    let unknown = uuid::Uuid::new_v4();
    let got = get_scenario(&pool, unknown).await?;
    assert!(got.is_none(), "an unknown scenario_id must return Ok(None)");

    Ok(())
}

// ── Optional columns as NULL ──────────────────────────────────────

#[tokio::test]
#[ignore]
async fn it_round_trips_with_null_optionals() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("null_optionals");
    delete_scenarios_for_case(&pool, &slug).await?;

    let id = insert_scenario(
        &pool,
        "Minimal scenario",
        "defense",
        "draft",
        &slug,
        None, // feeds_count_id
        None, // anchor_allegation_ids
        &json!({ "schema_v": 1 }),
    )
    .await?;

    let got = get_scenario(&pool, id).await?.expect("must be readable");
    assert!(got.feeds_count_id.is_none(), "NULL feeds_count_id → None");
    assert!(
        got.anchor_allegation_ids.is_none(),
        "NULL anchor_allegation_ids → None (distinct from an empty array)"
    );

    delete_scenarios_for_case(&pool, &slug).await?;
    Ok(())
}

// ── List ─────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn it_lists_scenarios_for_a_case() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("list");
    delete_scenarios_for_case(&pool, &slug).await?;

    let def = json!({ "schema_v": 1 });
    insert_scenario(&pool, "First", "offense", "draft", &slug, None, None, &def).await?;
    insert_scenario(&pool, "Second", "defense", "ready", &slug, None, None, &def).await?;

    let rows = list_scenarios_for_case(&pool, &slug).await?;
    assert_eq!(rows.len(), 2, "both scenarios for the case must be listed");
    assert!(rows.iter().all(|r| r.case_slug == slug));

    delete_scenarios_for_case(&pool, &slug).await?;
    Ok(())
}

// ── CHECK constraints reject out-of-set values ────────────────────

#[tokio::test]
#[ignore]
async fn it_rejects_bad_direction() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("bad_direction");
    delete_scenarios_for_case(&pool, &slug).await?;

    let result = insert_scenario(
        &pool,
        "Bad direction",
        "sideways", // not in ('offense','defense')
        "draft",
        &slug,
        None,
        None,
        &json!({ "schema_v": 1 }),
    )
    .await;

    assert!(
        result.is_err(),
        "the scenarios_direction_check CHECK must reject 'sideways'"
    );

    // And nothing was written.
    let rows = list_scenarios_for_case(&pool, &slug).await?;
    assert!(rows.is_empty(), "a rejected insert must write no row");

    delete_scenarios_for_case(&pool, &slug).await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn it_rejects_bad_status() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("bad_status");
    delete_scenarios_for_case(&pool, &slug).await?;

    let result = insert_scenario(
        &pool,
        "Bad status",
        "offense",
        "in_progress", // not in ('draft','needs_evidence','ready')
        &slug,
        None,
        None,
        &json!({ "schema_v": 1 }),
    )
    .await;

    assert!(
        result.is_err(),
        "the scenarios_status_check CHECK must reject 'in_progress'"
    );

    let rows = list_scenarios_for_case(&pool, &slug).await?;
    assert!(rows.is_empty(), "a rejected insert must write no row");

    delete_scenarios_for_case(&pool, &slug).await?;
    Ok(())
}
