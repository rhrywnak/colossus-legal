//! backend/tests/timeline_subsets_integration.rs
//!
//! ⚑ T1.4 — THE COUNT PROOF for timeline subsets.
//!
//! One test walks the whole feature against a live database, in the order the
//! task specifies it:
//!
//! 1. create a subset with three events at positions 3, 1, 2
//! 2. read it back — they come out 1, 2, 3
//! 3. soft-delete one event ON THE CHRONOLOGY — the subset still returns three,
//!    one marked `removed`
//! 4. undelete it — `removed` is false again
//! 5. attach the subset to a scenario — the scenario's subsets read returns it
//!    with `event_count: 3`
//! 6. replace the event set with two
//! 7. the subset's history holds exactly the rows the design says it should
//!
//! Every test is `#[ignore]` because it requires a live `colossus_legal_v2`
//! PostgreSQL database — the project has no `#[sqlx::test]` fixture infra, so CI
//! does NOT run them (the same convention as `scenarios_integration.rs`). The
//! CI-runnable coverage is the pure composition and validation unit tests in
//! `services::chronology_subset_read` / `_validate` / `_guard`, plus the two
//! source scans that prove the one write path and the write guard. THIS is the
//! behavioural proof that the queries, the transaction and the seal do what
//! those describe.
//!
//! Run manually against a database carrying migration
//! `20260830122249_timeline_subsets`:
//!   `cargo test -p colossus-legal-backend --test timeline_subsets_integration -- \
//!      --ignored --test-threads=1`
//!
//! ## Why the HTTP layer is not exercised
//!
//! Every handler takes `State<AppState>`, and an `AppState` carries two pools, a
//! Neo4j graph, a Qdrant client and a settings snapshot — this project has no
//! tier that builds one. So this test drives the layer immediately beneath the
//! handlers: the same service functions and the same composition each handler
//! calls, in the same order. That is the house pattern every other
//! `*_integration.rs` in this directory follows. The handler-shaped half — that
//! a write handler cannot be reached anonymously — is proved twice, in two other
//! places: `tests/timeline_subsets_auth.rs`, which exercises the real extractor,
//! and the source scan in `api::timeline_subsets::writes::tests`, which proves
//! every write handler declares it. That is step 8 of the task, and it lives in
//! its own binary because it needs no database and DOES need to write
//! `AUTH_MODE` into the process environment — which no test sharing a process
//! with six database steps should be allowed to do.
//!
//! ## Case-slug safety
//!
//! This test DELETES its own rows by `case_slug` and by scenario id. It
//! therefore must NOT use the bare production slug: it derives a
//! `__test_subsets` slug so cleanup only ever touches its own rows and the suite
//! is re-runnable.

use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use colossus_legal_backend::config::AppConfig;
use colossus_legal_backend::repositories::pipeline_repository::chronology_subsets::{
    carriers_for_subsets, count_subset_history, events_any_state_by_ids, get_subset_any_state,
    links_for_events, list_scenario_subsets, list_subset_event_ids, note_counts_for_events,
    subset_counts,
};
use colossus_legal_backend::repositories::pipeline_repository::chronology_write::{
    insert_event, soft_delete_event, undelete_event, NewChronologyEvent,
};
use colossus_legal_backend::repositories::pipeline_repository::{delete_scenario, insert_scenario};
use colossus_legal_backend::services::chronology_guard::ChronologyWriter;
use colossus_legal_backend::services::chronology_subset_read::{
    build_scenario_subsets, build_subset_detail, carriers_by_subset, SubsetDetailSources,
};
use colossus_legal_backend::services::chronology_subset_validate::ValidSubsetEvent;
use colossus_legal_backend::services::chronology_subset_write as subset_write;

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Documented base slug (the real matter). This test appends `__test_subsets` so
/// destructive cleanup never touches production rows. See the module doc.
const CASE_SLUG_BASE: &str = "awad_v_catholic_family_service";

/// The phase every seeded event is filed under. `estate` is one of the four
/// slugs `chronology_phases` carries in every environment, and
/// `chronology_events.phase` is a foreign key onto it — so an event cannot be
/// written without naming a real one.
const TEST_PHASE: &str = "estate";

fn test_slug() -> String {
    format!("{CASE_SLUG_BASE}__test_subsets")
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

/// The writer every step stamps with, as `open_write` would build it from a login.
fn writer() -> ChronologyWriter {
    ChronologyWriter {
        by_id: "test_roman".to_string(),
        by: "Test Roman".to_string(),
    }
}

/// Seed one chronology event and return its id.
async fn seed_event(pool: &PgPool, slug: &str, day: u32, title: &str) -> TestResult<Uuid> {
    let id = insert_event(
        pool,
        &NewChronologyEvent {
            case_slug: slug,
            event_date: NaiveDate::from_ymd_opt(2009, 3, day).expect("a real day"),
            date_precision: "day",
            approximate: false,
            phase: TEST_PHASE,
            title,
            fact: None,
            attributes: &json!({}),
            created_by: "test_roman",
        },
    )
    .await?;
    Ok(id)
}

/// Remove everything this test wrote, in foreign-key order.
///
/// Runs at the START of the test as well as the end, so a previous run that
/// died half way does not make the next one fail on a name clash — a suite that
/// is not re-runnable is a suite people stop running.
async fn cleanup(pool: &PgPool, slug: &str, scenario_id: Option<Uuid>) -> TestResult<()> {
    if let Some(id) = scenario_id {
        delete_scenario(pool, id, slug).await?;
    }
    sqlx::query(
        "DELETE FROM scenario_subsets WHERE subset_id IN \
           (SELECT id FROM chronology_subsets WHERE case_slug = $1)",
    )
    .bind(slug)
    .execute(pool)
    .await?;
    sqlx::query(
        "DELETE FROM chronology_subset_history WHERE subset_id IN \
           (SELECT id FROM chronology_subsets WHERE case_slug = $1)",
    )
    .bind(slug)
    .execute(pool)
    .await?;
    sqlx::query(
        "DELETE FROM chronology_subset_events WHERE subset_id IN \
           (SELECT id FROM chronology_subsets WHERE case_slug = $1)",
    )
    .bind(slug)
    .execute(pool)
    .await?;
    sqlx::query("DELETE FROM chronology_subsets WHERE case_slug = $1")
        .bind(slug)
        .execute(pool)
        .await?;
    sqlx::query(
        "DELETE FROM chronology_event_links WHERE event_id IN \
           (SELECT id FROM chronology_events WHERE case_slug = $1)",
    )
    .bind(slug)
    .execute(pool)
    .await?;
    sqlx::query(
        "DELETE FROM chronology_event_history WHERE event_id IN \
           (SELECT id FROM chronology_events WHERE case_slug = $1)",
    )
    .bind(slug)
    .execute(pool)
    .await?;
    sqlx::query("DELETE FROM chronology_events WHERE case_slug = $1")
        .bind(slug)
        .execute(pool)
        .await?;
    Ok(())
}

/// Compose one subset exactly as `api::timeline_subsets::support::subset_response`
/// does — the same five reads, the same composer, in the same order.
///
/// Duplicated here rather than exported from the handler module because that
/// function takes an `AppState`. The five calls below are what it makes; if it
/// ever grows a sixth, this test composing five is the visible difference.
async fn read_subset_detail(
    pool: &PgPool,
    subset_id: Uuid,
) -> TestResult<colossus_legal_backend::dto::chronology_subset::SubsetDetailDto> {
    let subset = get_subset_any_state(pool, subset_id)
        .await?
        .ok_or("the subset was written and cannot be read back")?;
    let refs = list_subset_event_ids(pool, subset_id).await?;
    let ids: Vec<Uuid> = refs.iter().map(|r| r.event_id).collect();
    let events = events_any_state_by_ids(pool, &ids).await?;
    let links = links_for_events(pool, &ids).await?;
    let counts: HashMap<Uuid, i64> = note_counts_for_events(pool, &ids)
        .await?
        .into_iter()
        .collect();
    let carriers = carriers_for_subsets(pool, &[subset_id]).await?;
    let carried_by = carriers_by_subset(&carriers)
        .remove(&subset_id)
        .unwrap_or_default();

    let composed = build_subset_detail(SubsetDetailSources {
        subset: &subset,
        refs: &refs,
        events: &events,
        links: &links,
        note_counts: &counts,
        resolved_documents: &HashSet::new(),
        carried_by: &carried_by,
    });
    assert!(
        composed.warnings.is_empty(),
        "the composition warned, which means a reference lost its event: {:?}",
        composed.warnings
    );
    Ok(composed.payload)
}

fn reference(event_id: Uuid, position: i32, note: &str) -> ValidSubsetEvent {
    ValidSubsetEvent {
        event_id,
        position,
        note: note.to_string(),
    }
}

#[tokio::test]
#[ignore = "requires a live colossus_legal_v2 database carrying 20260830122249_timeline_subsets"]
async fn a_subset_orders_its_events_marks_its_gaps_and_records_every_write() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug();
    cleanup(&pool, &slug, None).await?;

    // ── 1 · three events, added to a subset at positions 3, 1, 2 ─────────────
    //
    // Deliberately NOT in date order and NOT in insertion order: the whole
    // ruling of 2026-08-30 (1) is that the author may move a line, so the proof
    // has to be that the STORED position wins over both.
    let first = seed_event(&pool, &slug, 16, "the transfer").await?;
    let second = seed_event(&pool, &slug, 18, "the withdrawal").await?;
    let third = seed_event(&pool, &slug, 20, "the check").await?;

    let created = subset_write::create(
        &pool,
        &slug,
        "The $50,000",
        "What the money did.",
        &[
            reference(first, 3, "last in the story"),
            reference(second, 1, "first in the story"),
            reference(third, 2, ""),
        ],
        &writer(),
    )
    .await?;
    let subset_id = created.subset.id;

    // ── 2 · they come back in STORY order, not date order ────────────────────
    let detail = read_subset_detail(&pool, subset_id).await?;
    assert_eq!(detail.event_count, 3);
    assert_eq!(detail.gap_count, 0);
    let order: Vec<Uuid> = detail.events.iter().map(|e| e.event.id).collect();
    assert_eq!(
        order,
        vec![second, third, first],
        "the subset must read in position order (1, 2, 3), not date or insertion order"
    );
    assert_eq!(detail.events[0].subset_note, "first in the story");
    assert!(detail.events.iter().all(|e| !e.removed));

    // ── 3 · soft-delete one event ON THE CHRONOLOGY ──────────────────────────
    //
    // The subset still returns THREE. Design R1: the row is marked, never
    // dropped — dropping it would silently shorten a story somebody counted.
    soft_delete_event(&pool, second, "test_roman").await?;
    let with_gap = read_subset_detail(&pool, subset_id).await?;
    assert_eq!(
        with_gap.event_count, 3,
        "a removed event stays in the story"
    );
    assert_eq!(with_gap.gap_count, 1);
    assert!(with_gap.events[0].removed, "the deleted event is marked");
    assert!(!with_gap.events[1].removed);
    assert!(!with_gap.events[2].removed);
    // The count read the LIST endpoint uses must agree with the detail's.
    let counted = subset_counts(&pool, &[subset_id]).await?;
    assert_eq!(counted[0].event_count, 3);
    assert_eq!(counted[0].gap_count, 1);

    // ── 4 · undelete it — the mark goes away ─────────────────────────────────
    undelete_event(&pool, second, "test_roman").await?;
    let healed = read_subset_detail(&pool, subset_id).await?;
    assert_eq!(healed.gap_count, 0);
    assert!(healed.events.iter().all(|e| !e.removed));

    // ── 5 · attach it to a scenario ──────────────────────────────────────────
    let (scenario_id, _code) = insert_scenario(
        &pool,
        "Subset attachment proof",
        "offense",
        "draft",
        &slug,
        None,
        None,
        &json!({ "schema_v": 1 }),
    )
    .await?;
    subset_write::attach(&pool, scenario_id, subset_id, 0, &writer()).await?;

    let attached = build_scenario_subsets(&list_scenario_subsets(&pool, scenario_id).await?);
    assert_eq!(
        attached.len(),
        1,
        "the scenario carries exactly the one subset"
    );
    assert_eq!(attached[0].id, subset_id);
    assert_eq!(attached[0].event_count, 3);
    assert_eq!(attached[0].gap_count, 0);
    assert_eq!(attached[0].position, 0);

    // And the subset now knows who carries it, by scenario CODE.
    let carried = read_subset_detail(&pool, subset_id).await?;
    assert_eq!(carried.carried_by.len(), 1);
    assert!(
        carried.carried_by[0].starts_with("S-"),
        "the backend spells the handle, never a screen: {:?}",
        carried.carried_by
    );

    // ── 6 · replace the set with TWO ─────────────────────────────────────────
    //
    // The third event leaves the story; the two that stay swap positions, which
    // is the reorder the DEFERRABLE unique constraint exists for — an IMMEDIATE
    // one would abort this transaction half way.
    subset_write::replace_events(
        &pool,
        subset_id,
        &[reference(third, 1, "now first"), reference(second, 2, "")],
        &writer(),
    )
    .await?;

    let replaced = read_subset_detail(&pool, subset_id).await?;
    assert_eq!(replaced.event_count, 2);
    assert_eq!(
        replaced
            .events
            .iter()
            .map(|e| e.event.id)
            .collect::<Vec<_>>(),
        vec![third, second],
        "the replaced set reads in its new story order"
    );
    assert_eq!(replaced.events[0].subset_note, "now first");
    // The subset's own row was restamped, so the list shows a real "last
    // touched by" rather than the creator forever.
    assert_eq!(replaced.updated_by, "test_roman");
    // The scenario's read follows the new count with no second write.
    let after = build_scenario_subsets(&list_scenario_subsets(&pool, scenario_id).await?);
    assert_eq!(after[0].event_count, 2);

    // ── 7 · the history, counted ─────────────────────────────────────────────
    //
    // ⚑ TWO rows: `created` and `events_replaced`. The task's text says four and
    // then names two, and its own parenthetical settles it — the attach is a
    // `scenario_subsets` row and is NOT a subset history row, because it is the
    // SCENARIO's fact about the subset rather than a change to the subset's
    // content. The two chronology-event writes in steps 3 and 4 land in
    // `chronology_event_history`, which is a different table and a different
    // subject. See the T1 report's stated choice.
    let history = count_subset_history(&pool, subset_id).await?;
    assert_eq!(
        history, 2,
        "one history row per act on the subset: created, events_replaced"
    );
    let actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM chronology_subset_history WHERE subset_id = $1 ORDER BY changed_at",
    )
    .bind(subset_id)
    .fetch_all(&pool)
    .await?;
    assert_eq!(actions, vec!["created", "events_replaced"]);

    // Every row is stamped and carries the ordered list — a history that
    // recorded the act and not its content would look healthy and say nothing.
    let snapshots: Vec<(String, serde_json::Value)> = sqlx::query_as(
        "SELECT changed_by, snapshot FROM chronology_subset_history \
         WHERE subset_id = $1 ORDER BY changed_at",
    )
    .bind(subset_id)
    .fetch_all(&pool)
    .await?;
    assert_eq!(snapshots[0].0, "test_roman");
    assert_eq!(snapshots[0].1["events"].as_array().map(Vec::len), Some(3));
    assert_eq!(snapshots[1].1["events"].as_array().map(Vec::len), Some(2));

    cleanup(&pool, &slug, Some(scenario_id)).await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires a live colossus_legal_v2 database carrying 20260830122249_timeline_subsets"]
async fn a_deleted_subset_leaves_the_scenario_link_alone_and_undo_brings_it_back() -> TestResult<()>
{
    // The delete's stated contract: "soft delete; detaches nothing". The way
    // that stays coherent is that the scenario read simply does not see a
    // deleted subset — so the button hides, the link row survives, and Undo
    // brings the attachment back with it. A detach-on-delete could not have.
    let pool = pipeline_pool().await?;
    let slug = format!("{}_undo", test_slug());
    cleanup(&pool, &slug, None).await?;

    let event = seed_event(&pool, &slug, 16, "the transfer").await?;
    let created = subset_write::create(
        &pool,
        &slug,
        "Undo proof",
        "",
        &[reference(event, 1, "")],
        &writer(),
    )
    .await?;
    let (scenario_id, _code) = insert_scenario(
        &pool,
        "Subset undo proof",
        "offense",
        "draft",
        &slug,
        None,
        None,
        &json!({ "schema_v": 1 }),
    )
    .await?;
    subset_write::attach(&pool, scenario_id, created.subset.id, 0, &writer()).await?;
    assert_eq!(list_scenario_subsets(&pool, scenario_id).await?.len(), 1);

    subset_write::soft_delete(&pool, created.subset.id, &writer()).await?;
    assert!(
        list_scenario_subsets(&pool, scenario_id).await?.is_empty(),
        "a deleted subset is not read, so the View Timeline button hides"
    );

    subset_write::restore(&pool, created.subset.id, &writer()).await?;
    assert_eq!(
        list_scenario_subsets(&pool, scenario_id).await?.len(),
        1,
        "Undo brings the attachment back, because the delete detached nothing"
    );

    let actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM chronology_subset_history WHERE subset_id = $1 ORDER BY changed_at",
    )
    .bind(created.subset.id)
    .fetch_all(&pool)
    .await?;
    assert_eq!(actions, vec!["created", "deleted", "restored"]);

    cleanup(&pool, &slug, Some(scenario_id)).await?;
    Ok(())
}

/// ⚑ THE FALSE-HISTORY GUARD: no write seals a history row it did not earn.
///
/// The four write functions that can meet an already-deleted (or already-live)
/// subset must all refuse to seal, because `chronology_subset_history` is
/// append-only evidence — there is no `delete_subset_history` and there must not
/// be one. A row saying "deleted" for a delete that did not happen cannot be
/// taken back, and an operator reading the history of a disputed change would
/// see an act nobody performed.
///
/// The two edit paths answer with a 409-shaped `Deleted` error; the two
/// idempotent paths answer `Ok(None)`, meaning "nothing to do, nothing
/// recorded". Both are refusals to seal. What is proved here is the count: the
/// history holds exactly the rows the acts earned, and not one more.
///
/// Each check runs against the state INSIDE the write's own transaction, which
/// is the only place the answer cannot go stale between a handler's pre-check
/// and its write.
#[tokio::test]
#[ignore = "requires a live colossus_legal_v2 database"]
async fn no_write_onto_a_deleted_subset_ever_seals_a_history_row() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug();
    cleanup(&pool, &slug, None).await?;

    let event = seed_event(&pool, &slug, 4, "The letter").await?;
    let created = subset_write::create(
        &pool,
        &slug,
        "The false-history guard",
        "",
        &[ValidSubsetEvent {
            event_id: event,
            position: 1,
            note: String::new(),
        }],
        &writer(),
    )
    .await?;
    let subset_id = created.subset.id;
    assert_eq!(
        count_subset_history(&pool, subset_id).await?,
        1,
        "the create earned exactly one row"
    );

    // ── the subset goes away ────────────────────────────────────────────────
    let deleted = subset_write::soft_delete(&pool, subset_id, &writer()).await?;
    assert!(deleted.is_some(), "the first delete is a real act");
    assert_eq!(count_subset_history(&pool, subset_id).await?, 2);

    // ── 1 · a second delete is a no-op, NOT a second `deleted` row ──────────
    // This is the race the handler's pre-check cannot close: it reads on its own
    // connection, before the transaction opens.
    let again = subset_write::soft_delete(&pool, subset_id, &writer()).await?;
    assert!(
        again.is_none(),
        "deleting an already-deleted subset performed no act, so it must answer None"
    );
    assert_eq!(
        count_subset_history(&pool, subset_id).await?,
        2,
        "a second delete must not seal a second `deleted` row"
    );

    // ── 2 · rename onto a deleted subset is refused, not sealed ─────────────
    let refused = subset_write::rename(&pool, subset_id, Some("A new name"), None, &writer()).await;
    assert!(
        matches!(refused, Err(ref e) if e.to_string().contains("is deleted")),
        "rename onto a deleted subset must be refused with the Undo message, got: {refused:?}"
    );
    assert_eq!(
        count_subset_history(&pool, subset_id).await?,
        2,
        "a refused rename must seal nothing"
    );
    let still = get_subset_any_state(&pool, subset_id)
        .await?
        .expect("the subset row is still there, soft-deleted");
    assert_eq!(
        still.name, "The false-history guard",
        "a refused rename must not have changed the name either"
    );

    // ── 3 · replace_events onto a deleted subset is refused, not sealed ─────
    let refused = subset_write::replace_events(&pool, subset_id, &[], &writer()).await;
    assert!(
        matches!(refused, Err(ref e) if e.to_string().contains("is deleted")),
        "replace_events onto a deleted subset must be refused, got: {refused:?}"
    );
    assert_eq!(
        count_subset_history(&pool, subset_id).await?,
        2,
        "a refused replace must seal nothing"
    );
    assert_eq!(
        list_subset_event_ids(&pool, subset_id).await?.len(),
        1,
        "and it must not have emptied the event set on the way out"
    );

    // ── 4 · the Undo is a real act; a second Undo is not ────────────────────
    let restored = subset_write::restore(&pool, subset_id, &writer()).await?;
    assert!(restored.is_some(), "the first Undo is a real act");
    assert_eq!(count_subset_history(&pool, subset_id).await?, 3);

    let again = subset_write::restore(&pool, subset_id, &writer()).await?;
    assert!(
        again.is_none(),
        "undoing a live subset performed no act, so it must answer None"
    );
    assert_eq!(
        count_subset_history(&pool, subset_id).await?,
        3,
        "a second Undo must not seal a second `restored` row"
    );

    // ── the whole record, in order: only the acts that happened ─────────────
    let actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM chronology_subset_history WHERE subset_id = $1 ORDER BY changed_at",
    )
    .bind(subset_id)
    .fetch_all(&pool)
    .await?;
    assert_eq!(
        actions,
        vec!["created", "deleted", "restored"],
        "the history is a record of acts; four refusals added nothing to it"
    );

    cleanup(&pool, &slug, None).await?;
    Ok(())
}
