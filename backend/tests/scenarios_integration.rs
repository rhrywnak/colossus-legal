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
    delete_scenario, delete_scenarios_for_case, get_scenario, insert_response_item,
    insert_scenario, insert_scenario_response, list_fact_refs_for_item,
    list_fact_refs_for_scenario, list_items_for_response, list_responses_for_scenario,
    list_scenarios_for_case, upsert_fact_ref, upsert_response_item_fact_ref,
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
    // best-effort: a missing .env is normal when the live URL comes from the
    // shell env in CI / live-infra runs; the connect below fails loudly if unset.
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
    upsert_fact_ref(&pool, offense, node, Some("wielder"), true, None, None).await?;
    upsert_fact_ref(&pool, defense, node, Some("target"), true, None, None).await?;

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

    upsert_fact_ref(
        &pool,
        scenario,
        node,
        Some("seed_support"),
        false,
        None,
        None,
    )
    .await?;
    // Re-tag the SAME pair with a different role + confirmed + note.
    upsert_fact_ref(
        &pool,
        scenario,
        node,
        Some("wielder"),
        true,
        Some("reclassified after review"),
        None,
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

/// The `confidence` column (D2a substrate for Theme Scan) round-trips through
/// `upsert_fact_ref`: `Some(x)` stores the float, `None` stores SQL `NULL`, and
/// the `ON CONFLICT` update path overwrites confidence from `EXCLUDED` — so a
/// re-tag can both set and clear it. We read the column back with a raw query
/// because `list_fact_refs_for_scenario` deliberately does NOT surface
/// confidence yet (it stays write-only until D3 chooses to render it), so the
/// record projection is unchanged. Asserting the raw column proves the write
/// reached the row regardless of what the read projection exposes.
#[tokio::test]
#[ignore]
async fn it_round_trips_fact_ref_confidence() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("fact_confidence");
    delete_scenarios_for_case(&pool, &slug).await?;

    let scenario = make_scenario(&pool, "Scan lens", &slug).await?;

    // Reads the raw `confidence` column for one (scenario, node) pair. Returns
    // `Option<f32>`: `None` distinguishes SQL NULL (human/pre-scan) from a stored
    // float — the two states must stay observably distinct (Standing Rule 1).
    async fn read_confidence(
        pool: &sqlx::PgPool,
        scenario: uuid::Uuid,
        node: &str,
    ) -> TestResult<Option<f32>> {
        let value = sqlx::query_scalar::<_, Option<f32>>(
            "SELECT confidence FROM scenario_fact_refs \
             WHERE scenario_id = $1 AND graph_node_id = $2",
        )
        .bind(scenario)
        .bind(node)
        .fetch_one(pool)
        .await?;
        Ok(value)
    }

    // Scan path: a suggestion (confirmed = false) carrying a model confidence.
    let scan_node = "evidence-scan-1";
    upsert_fact_ref(
        &pool,
        scenario,
        scan_node,
        Some("seed_support"),
        false,
        Some("proposed by theme scan"),
        Some(0.87_f32),
    )
    .await?;
    let stored = read_confidence(&pool, scenario, scan_node).await?;
    assert_eq!(
        stored,
        Some(0.87_f32),
        "Some(confidence) must round-trip as the stored REAL value"
    );

    // Human path: no model confidence — the column must be SQL NULL, not 0.0.
    let human_node = "evidence-human-1";
    upsert_fact_ref(
        &pool,
        scenario,
        human_node,
        Some("wielder"),
        true,
        None,
        None,
    )
    .await?;
    assert_eq!(
        read_confidence(&pool, scenario, human_node).await?,
        None,
        "None must write SQL NULL (a hand-curated fact has no model confidence)"
    );

    // Update path: re-tagging the scan node with None must CLEAR confidence to
    // NULL via `confidence = EXCLUDED.confidence` — proving the ON CONFLICT
    // branch is wired, not just the insert branch.
    upsert_fact_ref(
        &pool,
        scenario,
        scan_node,
        Some("wielder"),
        true,
        None,
        None,
    )
    .await?;
    assert_eq!(
        read_confidence(&pool, scenario, scan_node).await?,
        None,
        "the ON CONFLICT update must overwrite confidence from EXCLUDED (here: clear it)"
    );

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
    upsert_fact_ref(&pool, scenario, "node-1", Some("wielder"), true, None, None).await?;
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
    let result = upsert_fact_ref(&pool, orphan, "node-x", Some("wielder"), false, None, None).await;
    assert!(
        result.is_err(),
        "upsert_fact_ref must fail when scenario_id references no scenario (FK violation)"
    );

    Ok(())
}

// ── scenario responses model (task 1.6) ───────────────────────────

/// THE slice test — the storage-layer proof that 1.6 closes the minimal slice.
///
/// scenario (1.1) → tag a graph fact into it (1.2) → response (1.6) → item →
/// reference a HUMAN-AUTHORED graph node id on the item. Asserts the item's
/// fact-ref holds the graph node id and that NO fact content is stored anywhere
/// in the response tables — the id is the only link; content is read live from
/// the graph at compose time.
#[tokio::test]
#[ignore]
async fn it_closes_the_minimal_slice() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("slice");
    delete_scenarios_for_case(&pool, &slug).await?;

    // 1.1: a scenario.
    let scenario = make_scenario(&pool, "Concealment lens", &slug).await?;

    // 1.2: tag a graph fact into the scenario (a human-authored node, 0.2 path).
    let human_node = "human-fact-7f3a";
    upsert_fact_ref(
        &pool,
        scenario,
        human_node,
        Some("wielder"),
        true,
        None,
        None,
    )
    .await?;

    // 1.6: a response, an item, and the item's evidence reference.
    let response = insert_scenario_response(
        &pool,
        scenario,
        Some("Direct answer"),
        "She concealed it.",
        "draft",
        "human",
    )
    .await?;
    let item =
        insert_response_item(&pool, response, 0, "Point one: the waiver was hidden.").await?;
    upsert_response_item_fact_ref(&pool, item, human_node, Some("rests on Humphrey's note"))
        .await?;

    // The item's fact-ref holds ONLY the graph node id (the link) + a note.
    let refs = list_fact_refs_for_item(&pool, item).await?;
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].graph_node_id, human_node);
    assert_eq!(refs[0].response_item_id, item);

    // No fact content leaks into the response tables: the response/item carry the
    // human's authored text, but the EVIDENCE is referenced by id only — there is
    // no quote/citation column to hold graph content (enforced structurally by the
    // Rule-21 scan; here we assert the link round-trips and the id is all we got).
    let responses = list_responses_for_scenario(&pool, scenario).await?;
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].id, response);
    let items = list_items_for_response(&pool, response).await?;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, item);

    delete_scenarios_for_case(&pool, &slug).await?;
    Ok(())
}

/// The four-link cascade: deleting the SCENARIO must wipe its response, that
/// response's item, AND that item's fact-ref — proving cascade reaches from the
/// 1.1 table all the way down. If any link is non-cascading, this fails.
#[tokio::test]
#[ignore]
async fn it_cascades_the_full_chain_on_scenario_delete() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("chain_cascade");
    delete_scenarios_for_case(&pool, &slug).await?;

    let scenario = make_scenario(&pool, "Doomed lens", &slug).await?;
    let response =
        insert_scenario_response(&pool, scenario, None, "answer", "draft", "human").await?;
    let item = insert_response_item(&pool, response, 0, "item").await?;
    upsert_response_item_fact_ref(&pool, item, "node-1", None).await?;

    // Precondition: the full chain exists.
    assert_eq!(list_responses_for_scenario(&pool, scenario).await?.len(), 1);
    assert_eq!(list_items_for_response(&pool, response).await?.len(), 1);
    assert_eq!(list_fact_refs_for_item(&pool, item).await?.len(), 1);

    // Delete the chain-top scenario.
    delete_scenarios_for_case(&pool, &slug).await?;

    // Every descendant must be gone via ON DELETE CASCADE.
    assert!(
        list_responses_for_scenario(&pool, scenario)
            .await?
            .is_empty(),
        "deleting the scenario must cascade to its responses"
    );
    assert!(
        list_items_for_response(&pool, response).await?.is_empty(),
        "deleting the scenario must cascade to response_items"
    );
    assert!(
        list_fact_refs_for_item(&pool, item).await?.is_empty(),
        "deleting the scenario must cascade to response_item_fact_refs"
    );

    Ok(())
}

/// Items list back in `item_index` order regardless of insertion order.
#[tokio::test]
#[ignore]
async fn it_lists_response_items_in_index_order() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("item_order");
    delete_scenarios_for_case(&pool, &slug).await?;

    let scenario = make_scenario(&pool, "Lens", &slug).await?;
    let response =
        insert_scenario_response(&pool, scenario, None, "answer", "draft", "human").await?;

    // Insert out of order: 2, 0, 1.
    insert_response_item(&pool, response, 2, "third").await?;
    insert_response_item(&pool, response, 0, "first").await?;
    insert_response_item(&pool, response, 1, "second").await?;

    let items = list_items_for_response(&pool, response).await?;
    let order: Vec<i32> = items.iter().map(|i| i.item_index).collect();
    assert_eq!(order, vec![0, 1, 2], "items must list in item_index order");
    assert_eq!(items[0].text, "first");
    assert_eq!(items[2].text, "third");

    delete_scenarios_for_case(&pool, &slug).await?;
    Ok(())
}

/// Re-tagging the same (item, node) pair updates the row in place (composite-key
/// upsert) — one row remains, with the new note.
#[tokio::test]
#[ignore]
async fn it_upserts_a_response_item_fact_ref_in_place() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("item_ref_upsert");
    delete_scenarios_for_case(&pool, &slug).await?;

    let scenario = make_scenario(&pool, "Lens", &slug).await?;
    let response =
        insert_scenario_response(&pool, scenario, None, "answer", "draft", "human").await?;
    let item = insert_response_item(&pool, response, 0, "item").await?;
    let node = "node-42";

    upsert_response_item_fact_ref(&pool, item, node, Some("first reason")).await?;
    upsert_response_item_fact_ref(&pool, item, node, Some("revised reason")).await?;

    let refs = list_fact_refs_for_item(&pool, item).await?;
    assert_eq!(
        refs.len(),
        1,
        "re-tagging must update in place, not duplicate"
    );
    assert_eq!(refs[0].note.as_deref(), Some("revised reason"));

    delete_scenarios_for_case(&pool, &slug).await?;
    Ok(())
}

/// A response whose `scenario_id` names no scenario must be rejected by the FK
/// (the violation path called out in `insert_scenario_response`'s `# Errors`).
#[tokio::test]
#[ignore]
async fn it_rejects_a_response_for_a_nonexistent_scenario() -> TestResult<()> {
    let pool = pipeline_pool().await?;

    let orphan = uuid::Uuid::new_v4();
    let result = insert_scenario_response(&pool, orphan, None, "answer", "draft", "human").await;
    assert!(
        result.is_err(),
        "insert_scenario_response must fail when scenario_id references no scenario (FK)"
    );

    Ok(())
}

/// An item whose `response_id` names no response must be rejected by the FK
/// (the violation path called out in `insert_response_item`'s `# Errors`).
#[tokio::test]
#[ignore]
async fn it_rejects_an_item_for_a_nonexistent_response() -> TestResult<()> {
    let pool = pipeline_pool().await?;

    let orphan = uuid::Uuid::new_v4();
    let result = insert_response_item(&pool, orphan, 0, "item").await;
    assert!(
        result.is_err(),
        "insert_response_item must fail when response_id references no response (FK)"
    );

    Ok(())
}

// ── delete_scenario (D1.5) — the case-isolation fence ─────────────────────────

/// The single-scenario hard delete is fenced by `(scenario_id AND case_slug)`:
/// deleting through the OWNING case removes exactly the one row (rows_affected 1),
/// and a delete aimed at a real scenario through the WRONG case slug removes
/// nothing (rows_affected 0) and leaves the row intact. The 0-vs-1 count is the
/// signal the handler turns into 404-vs-204, so this proves the fence at the store
/// layer where it actually lives.
#[tokio::test]
#[ignore]
async fn it_deletes_one_scenario_scoped_by_case() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("delete_fence");
    let other_slug = test_slug("delete_fence_other");
    delete_scenarios_for_case(&pool, &slug).await?;
    delete_scenarios_for_case(&pool, &other_slug).await?;

    let scenario = make_scenario(&pool, "Doomed lens", &slug).await?;

    // Wrong case: the fence matches zero rows and the scenario survives.
    let wrong_case = delete_scenario(&pool, scenario, &other_slug).await?;
    assert_eq!(
        wrong_case, 0,
        "a delete through the wrong case_slug must match zero rows"
    );
    assert!(
        get_scenario(&pool, scenario).await?.is_some(),
        "the scenario must still exist after a wrong-case delete"
    );

    // Owning case: exactly one row deleted, and it is gone.
    let owning_case = delete_scenario(&pool, scenario, &slug).await?;
    assert_eq!(owning_case, 1, "the owning-case delete must remove one row");
    assert!(
        get_scenario(&pool, scenario).await?.is_none(),
        "the scenario must be gone after the owning-case delete"
    );

    // A second delete of the same id now matches nothing (idempotent 0 → 404).
    let repeat = delete_scenario(&pool, scenario, &slug).await?;
    assert_eq!(
        repeat, 0,
        "re-deleting an already-deleted scenario is 0 rows"
    );

    Ok(())
}
