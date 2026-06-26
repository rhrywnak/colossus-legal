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
    delete_scenarios_for_case, get_scenario, insert_scenario, list_fact_refs_for_scenario,
    list_scenarios_for_case, upsert_fact_ref,
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

// ── scenario_fact_refs (task 1.2) ─────────────────────────────────

/// Insert a bare scenario for a slug and return its id.
async fn make_scenario(pool: &PgPool, name: &str, slug: &str) -> TestResult<uuid::Uuid> {
    let id = insert_scenario(
        pool,
        name,
        "offense",
        "draft",
        slug,
        None,
        None,
        &json!({ "schema_v": 1 }),
    )
    .await?;
    Ok(id)
}

/// THE sharing test — the Phase-1 validation primitive.
///
/// The SAME `graph_node_id` is referenced by TWO different scenarios with TWO
/// different roles. Both rows must exist, each carrying its own role. This proves
/// a fact is *shared* across scenarios with a per-scenario role, never *owned*.
#[tokio::test]
#[ignore]
async fn it_shares_a_fact_across_scenarios_with_distinct_roles() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("fact_sharing");
    // Deleting the scenarios cascades to their fact refs (clean start).
    delete_scenarios_for_case(&pool, &slug).await?;

    let offense = make_scenario(&pool, "Offense lens", &slug).await?;
    let defense = make_scenario(&pool, "Defense lens", &slug).await?;

    // One graph node, tagged into both scenarios under different roles.
    let node = "person-jeffrey-humphrey";
    upsert_fact_ref(&pool, offense, node, Some("wielder"), true, None).await?;
    upsert_fact_ref(&pool, defense, node, Some("target"), true, None).await?;

    let offense_refs = list_fact_refs_for_scenario(&pool, offense).await?;
    let defense_refs = list_fact_refs_for_scenario(&pool, defense).await?;

    assert_eq!(offense_refs.len(), 1);
    assert_eq!(defense_refs.len(), 1);
    assert_eq!(offense_refs[0].graph_node_id, node);
    assert_eq!(defense_refs[0].graph_node_id, node);
    // Same node, different role per scenario — the whole point.
    assert_eq!(
        offense_refs[0].role_in_this_scenario.as_deref(),
        Some("wielder")
    );
    assert_eq!(
        defense_refs[0].role_in_this_scenario.as_deref(),
        Some("target")
    );

    delete_scenarios_for_case(&pool, &slug).await?;
    Ok(())
}

/// Re-tagging the same (scenario, node) pair updates the existing row in place
/// (composite-key upsert) — exactly one row remains, with the new role.
#[tokio::test]
#[ignore]
async fn it_upserts_a_fact_ref_in_place() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("fact_upsert");
    delete_scenarios_for_case(&pool, &slug).await?;

    let scenario = make_scenario(&pool, "Lens", &slug).await?;
    let node = "alleg-42";

    upsert_fact_ref(&pool, scenario, node, Some("seed_support"), false, None).await?;
    // Re-tag the SAME pair with a different role + confirmed + note.
    upsert_fact_ref(
        &pool,
        scenario,
        node,
        Some("wielder"),
        true,
        Some("reclassified after review"),
    )
    .await?;

    let refs = list_fact_refs_for_scenario(&pool, scenario).await?;
    assert_eq!(
        refs.len(),
        1,
        "re-tagging must update in place, not duplicate"
    );
    assert_eq!(refs[0].role_in_this_scenario.as_deref(), Some("wielder"));
    assert!(refs[0].confirmed);
    assert_eq!(refs[0].note.as_deref(), Some("reclassified after review"));

    delete_scenarios_for_case(&pool, &slug).await?;
    Ok(())
}

/// Deleting a scenario removes its fact refs via `ON DELETE CASCADE` — no manual
/// cleanup of the child table is needed.
#[tokio::test]
#[ignore]
async fn it_cascades_fact_refs_on_scenario_delete() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("fact_cascade");
    delete_scenarios_for_case(&pool, &slug).await?;

    let scenario = make_scenario(&pool, "Doomed lens", &slug).await?;
    upsert_fact_ref(&pool, scenario, "node-1", Some("wielder"), true, None).await?;
    assert_eq!(
        list_fact_refs_for_scenario(&pool, scenario).await?.len(),
        1,
        "precondition: the fact ref exists before the scenario is deleted"
    );

    // Delete the parent scenario; the FK cascade must remove the child ref.
    delete_scenarios_for_case(&pool, &slug).await?;

    let refs = list_fact_refs_for_scenario(&pool, scenario).await?;
    assert!(
        refs.is_empty(),
        "ON DELETE CASCADE must remove fact refs when their scenario is deleted"
    );

    Ok(())
}

/// A fact ref whose `scenario_id` names no scenario must be rejected by the
/// foreign key — proving `REFERENCES scenarios(scenario_id)` is enforced, not
/// merely declared. (This is the FK-violation path called out in
/// `upsert_fact_ref`'s `# Errors`.)
#[tokio::test]
#[ignore]
async fn it_rejects_a_fact_ref_for_a_nonexistent_scenario() -> TestResult<()> {
    let pool = pipeline_pool().await?;

    // A random uuid that was never inserted into `scenarios`.
    let orphan = uuid::Uuid::new_v4();
    let result = upsert_fact_ref(&pool, orphan, "node-x", Some("wielder"), false, None).await;
    assert!(
        result.is_err(),
        "upsert_fact_ref must fail when scenario_id references no scenario (FK violation)"
    );

    Ok(())
}
