//! Live-database proofs for the half of the minter that talks to the store
//! (CC_TASK_DEFECT_SWEEP_v1 defect 4).
//!
//! `#[ignore]`d and run by hand against a SCRATCH copy of the pipeline schema
//! (`PIPELINE_DATABASE_URL=… cargo test --lib deck_key_mint -- --ignored
//! --test-threads=1`), never against `colossus_legal_v2`.
//!
//! `next_hand_key` is pure and pinned without a database in
//! `deck_key_mint_tests.rs`. What needs a store is the READ it is fed and the
//! INSERT that consumes it: a SELECT that returned the wrong scenario's keys, or
//! an insert that stopped carrying the minted one, would leave every pure test
//! green and every hand-added question back where it started.

use sqlx::PgPool;
use uuid::Uuid;

use super::super::war_room_status::live_tests::{cleanup, pipeline_pool, scenario, TestResult};
use super::{keys_in_scenario, next_hand_key};
use crate::repositories::pipeline_repository::practice_editor::{insert_question, NewQuestion};

/// One question carrying `key` (or none), so the read has something to find.
async fn question_with_key(
    pool: &PgPool,
    scenario_id: Uuid,
    key: Option<&str>,
    order: i32,
) -> TestResult<Uuid> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO practice_questions \
         (scenario_id, side, kind, text, source_kind, sort_order, created_by, deck_key) \
         VALUES ($1, 'george', 'cross', $2, 'manual', $3, 'test', $4) RETURNING id",
    )
    .bind(scenario_id)
    .bind(format!("question {order}"))
    .bind(order)
    .bind(key)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// The read returns this scenario's keys, and only this scenario's. (M)
///
/// The scenario fence is the half a pure test cannot see. Without it the minter
/// would number against every deck in the case at once: harmless-looking (the
/// keys would still be unique) right up until two decks disagree about which
/// `x3` a redirect means.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn the_read_returns_this_scenarios_keys_and_no_others() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let mine = scenario(&pool, "mint_read_mine").await?;
    let other = scenario(&pool, "mint_read_other").await?;

    question_with_key(&pool, mine, Some("g1"), 1).await?;
    question_with_key(&pool, mine, Some("x4"), 2).await?;
    // A NULL key, which the read must simply not return rather than choke on.
    question_with_key(&pool, mine, None, 3).await?;
    question_with_key(&pool, other, Some("x99"), 1).await?;

    let mut tx = pool.begin().await?;
    let mut keys = keys_in_scenario(&mut tx, mine).await?;
    tx.rollback().await?;
    keys.sort();

    assert_eq!(keys, vec!["g1".to_string(), "x4".to_string()]);
    assert_eq!(
        next_hand_key(&keys),
        "x5",
        "the other deck's x99 must not number this one"
    );

    cleanup(&pool, mine, "mint_read_mine").await?;
    cleanup(&pool, other, "mint_read_other").await
}

/// A hand-added question is written WITH a minted key. (M)
///
/// End to end through the real insert: this is the defect itself — every
/// question added on the page carried `deck_key = NULL`, which froze
/// `seed_practice_deck --update` on that deck and left the question with no
/// handle a redirect could anchor to.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_hand_added_question_is_written_with_a_minted_key() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "mint_insert").await?;

    let mut tx = pool.begin().await?;
    let first = insert_question(
        &mut tx,
        &NewQuestion {
            scenario_id: s,
            side: "george",
            kind: "cross",
            text: "the first one somebody typed",
            tactic: None,
            follows_key: None,
            watch_for: None,
            source_kind: "manual",
            source_ref: None,
            receipt: None,
            sort_order: 1,
            created_by: "test",
        },
    )
    .await?;
    let second = insert_question(
        &mut tx,
        &NewQuestion {
            scenario_id: s,
            side: "chuck",
            kind: "direct",
            text: "and the second",
            tactic: None,
            follows_key: None,
            watch_for: None,
            source_kind: "manual",
            source_ref: None,
            receipt: None,
            sort_order: 2,
            created_by: "test",
        },
    )
    .await?;
    tx.commit().await?;

    assert_eq!(stored_key(&pool, first).await?.as_deref(), Some("x1"));
    assert_eq!(
        stored_key(&pool, second).await?.as_deref(),
        Some("x2"),
        "the second add must see the first one's key, inside the same transaction"
    );

    cleanup(&pool, s, "mint_insert").await
}

/// The minted key never lands in the deck file's own namespace. (M)
///
/// The same invariant the pure test pins, asserted against what the DATABASE
/// actually holds — because the failure that matters is a stored `g6`, not a
/// returned one.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn the_stored_key_is_never_one_the_deck_file_could_write() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "mint_namespace").await?;

    // A deck already carrying the file's own keys, as every seeded deck does.
    question_with_key(&pool, s, Some("g1"), 1).await?;
    question_with_key(&pool, s, Some("c1"), 2).await?;

    let mut tx = pool.begin().await?;
    let added = insert_question(
        &mut tx,
        &NewQuestion {
            scenario_id: s,
            side: "george",
            kind: "cross",
            text: "typed onto a seeded deck",
            tactic: None,
            follows_key: None,
            watch_for: None,
            source_kind: "manual",
            source_ref: None,
            receipt: None,
            sort_order: 3,
            created_by: "test",
        },
    )
    .await?;
    tx.commit().await?;

    let key = stored_key(&pool, added).await?.expect("a key was minted");
    assert_eq!(key, "x1", "it numbers its OWN namespace, not the file's");
    assert!(
        !key.starts_with('g') && !key.starts_with('c') && !key.starts_with('r'),
        "a stored {key} would collide with the architect's next file key, and \
         --update would rewrite the wrong question's text"
    );

    cleanup(&pool, s, "mint_namespace").await
}

/// The `deck_key` one question holds now.
async fn stored_key(pool: &PgPool, id: Uuid) -> TestResult<Option<String>> {
    let row: (Option<String>,) =
        sqlx::query_as("SELECT deck_key FROM practice_questions WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}
