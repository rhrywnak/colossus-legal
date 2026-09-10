//! The LIVE-DATABASE proof that Chuck's drag saves (task DECK_DRAG_AND_ADD, 1).
//!
//! `#[ignore]`d, because it needs a real `colossus_legal_v2` — the project has no
//! `#[sqlx::test]` fixture infrastructure, so CI does not run it (the convention
//! `config_overrides`'s live block and `tests/scan_run_history_integration.rs`
//! already follow).
//!
//! ## Why this one cannot be a unit test, when everything else here is
//!
//! The bug was never in the arithmetic. `resequenced` returned a correct order of
//! one side; `write_order` wrote it correctly; and the transaction rolled back
//! anyway, because `practice_questions_order_unique` spans the whole SCENARIO and
//! the rows left out of that order were still holding numbers the renumber was
//! about to hand out. Every pure test in `practice_place_tests` passed against
//! the broken code, because none of them could see the constraint.
//!
//! So the one thing that proves the fix is a real Postgres refusing or accepting
//! a real `UPDATE`. That is this file, and it is why it exists at all.
//!
//! Run it against a THROWAWAY database — a schema copy, never
//! `colossus_legal_v2` itself:
//!
//! ```text
//! PIPELINE_DATABASE_URL=…/scratch cargo test --lib \
//!     practice_reorder::live_tests -- --ignored --test-threads=1
//! ```
//!
//! Every test seeds its own scenario under a `__test_` slug and deletes it
//! afterwards; the questions cascade with it.

use sqlx::PgPool;
use uuid::Uuid;

use super::{placed_after, resequenced, write_order, NewPosition};
use crate::config::AppConfig;
use crate::repositories::pipeline_repository::practice::list_deck;
use crate::repositories::pipeline_repository::scenario_store::{delete_scenario, insert_scenario};

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// The real matter's slug with a per-test suffix, so the cleanup can only ever
/// reach rows this test wrote (the scan history suite's convention).
fn test_slug(tag: &str) -> String {
    format!("awad_v_catholic_family_service__test_{tag}")
}

async fn pipeline_pool() -> TestResult<PgPool> {
    // best-effort: a missing .env is normal when the URL comes from the shell,
    // which is how a scratch database is pointed at; the connect below fails
    // loudly either way if it is unset.
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&config.pipeline_database_url)
        .await?;
    Ok(pool)
}

/// A scenario holding an INTERLEAVED two-sided deck, in the shape DEV has.
///
/// `sort_order` runs 1..7 across both sides — george, george, chuck, chuck,
/// george, chuck, george — which is what makes the old code collide: George's
/// four rows renumbered to `0..3` while Chuck's still held 3 and 4.
async fn seed(pool: &PgPool, tag: &str) -> TestResult<(Uuid, Vec<Uuid>)> {
    let (scenario_id, _code) = insert_scenario(
        pool,
        &format!("deck drag {tag}"),
        "offense",
        "draft",
        &test_slug(tag),
        None,
        None,
        &serde_json::json!({ "schema_v": 1 }),
    )
    .await?;

    let sides = [
        "george", "george", "chuck", "chuck", "george", "chuck", "george",
    ];
    let mut ids = Vec::new();
    for (index, side) in sides.iter().enumerate() {
        let row: (Uuid,) = sqlx::query_as(
            "INSERT INTO practice_questions \
             (scenario_id, side, kind, text, source_kind, sort_order, created_by) \
             VALUES ($1, $2, $3, $4, 'manual', $5, 'test') RETURNING id",
        )
        .bind(scenario_id)
        .bind(side)
        .bind(if *side == "george" { "cross" } else { "direct" })
        .bind(format!("question {}", index + 1))
        // Deliberately 1-based, because the seeded decks on DEV are.
        .bind(i32::try_from(index).unwrap_or(0) + 1)
        .fetch_one(pool)
        .await?;
        ids.push(row.0);
    }
    Ok((scenario_id, ids))
}

async fn cleanup(pool: &PgPool, scenario_id: Uuid, tag: &str) -> TestResult<()> {
    let deleted = delete_scenario(pool, scenario_id, &test_slug(tag)).await?;
    assert_eq!(deleted, 1, "the test's own scenario must be cleaned up");
    Ok(())
}

/// The order this deck's rows are actually stored in.
async fn stored_order(pool: &PgPool, scenario_id: Uuid) -> TestResult<Vec<(Uuid, String, i32)>> {
    let rows: Vec<(Uuid, String, i32)> = sqlx::query_as(
        "SELECT id, side, sort_order FROM practice_questions \
         WHERE scenario_id = $1 ORDER BY sort_order",
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// **The bug, gone.** A two-sided drag COMMITS against the real constraint.
///
/// This is the assertion that was a 500 this afternoon. Nothing about it is
/// arithmetic: the transaction either survives `practice_questions_order_unique`
/// or it does not.
#[tokio::test]
#[ignore = "needs a live pipeline database — see the module doc; point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_two_sided_drag_commits_against_the_real_constraint() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let (scenario_id, ids) = seed(&pool, "drag_commits").await?;

    let deck = list_deck(&pool, scenario_id).await?;
    // Drag George's last row (slot 6) onto George's first (slot 0).
    let order = resequenced(&deck, ids[6], Some(ids[0])).expect("a real move names a position");

    let mut tx = pool.begin().await?;
    write_order(&mut tx, &order).await?;
    tx.commit().await?;

    let stored = stored_order(&pool, scenario_id).await?;
    assert_eq!(stored.len(), 7, "every row survived: {stored:?}");
    assert_eq!(
        stored.iter().map(|r| r.0).collect::<Vec<_>>(),
        order,
        "the deck is stored in the order that was written"
    );
    // Renumbered 0..6, contiguous, so the next drag has the same room.
    assert_eq!(
        stored.iter().map(|r| r.2).collect::<Vec<_>>(),
        (0..7).collect::<Vec<i32>>(),
        "sort_order is 0..N-1 after a write: {stored:?}"
    );
    // The dragged row is George's first, and Chuck's rows never left their slots.
    assert_eq!(stored[0].0, ids[6], "the dragged row landed on top");
    assert_eq!((stored[2].0, stored[3].0), (ids[2], ids[3]));
    assert_eq!(stored[5].0, ids[5]);

    cleanup(&pool, scenario_id, "drag_commits").await?;
    Ok(())
}

/// The OLD order is the one Postgres refuses — the bug, reproduced.
///
/// A regression test written from the other direction, and the only one that
/// demonstrates *why* the fix is the fix rather than merely that the new code
/// works. It hands `write_order` exactly what the old `resequenced` returned —
/// one side's ids — and asserts the database REFUSES it. If this ever starts
/// passing, either the constraint has been dropped or `write_order` has quietly
/// learned to tolerate a partial list, and both are worth hearing about.
#[tokio::test]
#[ignore = "needs a live pipeline database — see the module doc; point PIPELINE_DATABASE_URL at a scratch copy"]
async fn one_sides_ids_alone_are_still_refused_by_the_constraint() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let (scenario_id, ids) = seed(&pool, "old_order_refused").await?;

    // What the pre-fix `resequenced` returned: George's four rows, re-ordered,
    // and nothing else.
    let georges_only = vec![ids[6], ids[0], ids[1], ids[4]];

    let mut tx = pool.begin().await?;
    let result = write_order(&mut tx, &georges_only).await;
    // The rollback is the point: whether the error surfaces on the UPDATE or the
    // COMMIT, nothing may be left changed.
    let refused = match result {
        Err(_) => true,
        Ok(()) => tx.commit().await.is_err(),
    };
    assert!(
        refused,
        "writing one side's ids alone must still collide — that collision IS the \
         bug this task fixed, and a green here means the guard is gone"
    );

    let stored = stored_order(&pool, scenario_id).await?;
    assert_eq!(
        stored.iter().map(|r| r.2).collect::<Vec<_>>(),
        (1..8).collect::<Vec<i32>>(),
        "the refused write left the seeded 1..7 numbering untouched: {stored:?}"
    );

    cleanup(&pool, scenario_id, "old_order_refused").await?;
    Ok(())
}

/// Adding with `after` lands the new row where it was asked, and commits.
#[tokio::test]
#[ignore = "needs a live pipeline database — see the module doc; point PIPELINE_DATABASE_URL at a scratch copy"]
async fn an_add_after_commits_and_lands_below_that_row() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let (scenario_id, ids) = seed(&pool, "add_after").await?;

    let deck = list_deck(&pool, scenario_id).await?;
    let mut tx = pool.begin().await?;
    // The insert, exactly as the route does it: at MAX + 1.
    let new_row: (Uuid,) = sqlx::query_as(
        "INSERT INTO practice_questions \
         (scenario_id, side, kind, text, source_kind, sort_order, created_by) \
         VALUES ($1, 'george', 'cross', 'the new one', 'manual', \
                 (SELECT MAX(sort_order) + 1 FROM practice_questions WHERE scenario_id = $1), \
                 'test') RETURNING id",
    )
    .bind(scenario_id)
    .fetch_one(&mut *tx)
    .await?;

    let order = placed_after(&deck, new_row.0, "george", NewPosition::After(ids[0]))
        .expect("a same-side after names a position");
    write_order(&mut tx, &order).await?;
    tx.commit().await?;

    let stored = stored_order(&pool, scenario_id).await?;
    assert_eq!(stored.len(), 8, "the deck grew by one: {stored:?}");
    let georges: Vec<Uuid> = stored
        .iter()
        .filter(|r| r.1 == "george")
        .map(|r| r.0)
        .collect();
    let at = georges
        .iter()
        .position(|q| *q == ids[0])
        .expect("the anchor row is still on George's side");
    assert_eq!(
        georges[at + 1],
        new_row.0,
        "the new question is immediately below the row it named: {georges:?}"
    );

    cleanup(&pool, scenario_id, "add_after").await?;
    Ok(())
}
