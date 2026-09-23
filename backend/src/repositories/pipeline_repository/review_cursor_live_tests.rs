//! Live-database proofs that the reviewers' per-deck queue is the per-item
//! truth, asked one deck at a time (CC_TASK_FOR_YOU_v1 L2).
//!
//! ## What this file used to hold, and why it is shorter now
//!
//! Eleven tests of a watermark: that two presses left one row, that a
//! non-reviewer's press moved nothing, that the mark was shared across the
//! bench. Every one of them was about `practice_review_cursor`, which nothing
//! reads any more — so they were retired with it rather than rewritten to assert
//! the same sentences about a different mechanism.
//!
//! What replaced them is `waiting_items_live_tests` and its two siblings, which
//! prove the PREDICATE (who an item waits for, what counts as an item, what a
//! seen row clears). This file proves only what is left over: that
//! `awaiting_review` asks that predicate, per deck, about the person asking.
//!
//! `#[ignore]`d and run by hand against a SCRATCH copy of the pipeline schema
//! (`PIPELINE_DATABASE_URL=… cargo test --lib review_cursor -- --ignored
//! --test-threads=1`), never against `colossus_legal_v2`.

use chrono::{Duration, Utc};

use super::super::item_seen::{mark_seen, ItemRef};
use super::super::war_room_status::live_tests::{
    answer_by, cleanup, pipeline_pool, question, scenario, TestResult,
};
use super::awaiting_review;

const REVIEWER: &str = "cpenzien";
const WITNESS: &str = "docmarie";

fn bench() -> Vec<String> {
    vec![REVIEWER.to_string()]
}

/// One row per deck ASKED FOR, including the deck holding nothing.
///
/// The `unnest`-first shape, carried through the adapter. A deck that vanished
/// from the result would leave the war room drawing a card with no pill and no
/// way to tell that from a deck with nothing waiting.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn every_deck_asked_about_comes_back_even_when_it_holds_nothing() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let busy = scenario(&pool, "cursor_busy").await?;
    let quiet = scenario(&pool, "cursor_quiet").await?;
    let q = question(&pool, busy, "chuck", 1).await?;
    answer_by(
        &pool,
        busy,
        q,
        Utc::now() - Duration::hours(1),
        Some(WITNESS),
    )
    .await?;

    let rows = awaiting_review(&pool, &[busy, quiet], REVIEWER, &bench()).await?;
    assert_eq!(rows.len(), 2, "one row per deck asked for");
    let busy_row = rows
        .iter()
        .find(|r| r.scenario_id == busy)
        .ok_or("a row for the busy deck")?;
    assert_eq!(busy_row.awaiting, 1);
    assert!(busy_row.oldest.is_some(), "and the date the count is about");
    let quiet_row = rows
        .iter()
        .find(|r| r.scenario_id == quiet)
        .ok_or("a row for the quiet deck")?;
    assert_eq!(quiet_row.awaiting, 0);
    assert_eq!(quiet_row.oldest, None, "no date when nothing waits");

    cleanup(&pool, busy, "cursor_busy").await?;
    cleanup(&pool, quiet, "cursor_quiet").await
}

/// (M) THE LAYER'S CHANGE, IN ONE TEST: the count is the ASKING PERSON'S.
///
/// Under the retired watermark this number was global — CC_TASK_SIMPLE_COUNTS_v1
/// made it so deliberately, because a shared mark is the only thing a shared
/// count can be derived from. Read-state is per person now, so a seen row
/// written by one reviewer clears that reviewer's queue and leaves the other's
/// exactly where it was.
///
/// If this ever fails by both counts dropping together, the predicate has
/// stopped reading `user_id` and every person in the case is sharing an inbox.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn one_reviewers_reading_does_not_clear_anothers_queue() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "cursor_per_person").await?;
    let bench = vec![REVIEWER.to_string(), "roman".to_string()];
    let q = question(&pool, s, "chuck", 1).await?;
    let answer = answer_by(&pool, s, q, Utc::now() - Duration::hours(2), Some(WITNESS)).await?;

    let count = |who: &'static str, bench: Vec<String>| {
        let pool = pool.clone();
        let ids = [s];
        async move {
            let rows = awaiting_review(&pool, &ids, who, &bench).await?;
            Ok::<i64, Box<dyn std::error::Error + Send + Sync>>(rows[0].awaiting)
        }
    };

    assert_eq!(count(REVIEWER, bench.clone()).await?, 1, "waits for Chuck");
    assert_eq!(count("roman", bench.clone()).await?, 1, "and for Roman");

    mark_seen(&pool, REVIEWER, &[ItemRef::Answer(answer)]).await?;
    assert_eq!(
        count(REVIEWER, bench.clone()).await?,
        0,
        "Chuck has read it"
    );
    assert_eq!(
        count("roman", bench).await?,
        1,
        "Roman has not — the queue is his own, not the bench's"
    );
    cleanup(&pool, s, "cursor_per_person").await
}
