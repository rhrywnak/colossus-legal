//! Live-database proofs for the review cursor and the reviewer's queue
//! (CC_TASK_REVIEW_LOOP_v1 §1; CC_TASK_SIMPLE_COUNTS_v1).
//!
//! `#[ignore]`d, and run by hand against a SCRATCH copy of the pipeline schema
//! (`PIPELINE_DATABASE_URL=… cargo test --lib review_cursor -- --ignored
//! --test-threads=1`), never against `colossus_legal_v2`.
//!
//! Each `(M)` test is mutation-proved in the report.

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::super::war_room_status::live_tests::{
    answer_by, cleanup, hide, pipeline_pool, question, scenario, TestResult,
};
use super::{awaiting_review, mark_reviewed};

/// The reviewer the fixtures use — a test literal standing in for the settings row.
const REVIEWER: &str = "cpenzien";

async fn queue(pool: &PgPool, s: Uuid, reviewer: &str) -> TestResult<(i64, Option<DateTime<Utc>>)> {
    let rows = awaiting_review(pool, &[s], reviewer).await?;
    assert_eq!(rows.len(), 1, "one row per scenario asked for");
    Ok((rows[0].awaiting, rows[0].oldest))
}

async fn cursor_rows(pool: &PgPool, s: Uuid) -> TestResult<i64> {
    let row: (i64,) =
        sqlx::query_as("SELECT count(*) FROM practice_review_cursor WHERE scenario_id = $1")
            .bind(s)
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}

/// Two presses leave ONE row, and the mark moves forward.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn cursor_upsert_is_idempotent() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "cursor_upsert").await?;
    let first = mark_reviewed(&pool, REVIEWER, s).await?;
    let second = mark_reviewed(&pool, REVIEWER, s).await?;
    assert_eq!(cursor_rows(&pool, s).await?, 1);
    assert!(second >= first, "the mark never moves backwards");
    cleanup(&pool, s, "cursor_upsert").await
}

/// No reviewer cursor: every answer waits. After the reviewer's Done: 0. A later
/// answer: 1 — the answer newer than the reviewer's cursor counts.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn answer_newer_than_reviewers_cursor_counts() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "queue_done").await?;
    let t0 = Utc::now() - Duration::hours(1);
    for order in 1..=3 {
        let q = question(&pool, s, "chuck", order).await?;
        answer_by(&pool, s, q, t0, Some("docmarie")).await?;
    }
    assert_eq!(
        queue(&pool, s, REVIEWER).await?.0,
        3,
        "no row = all waiting"
    );

    mark_reviewed(&pool, REVIEWER, s).await?;
    assert_eq!(
        queue(&pool, s, REVIEWER).await?.0,
        0,
        "after the reviewer's Done"
    );

    let q = question(&pool, s, "george", 4).await?;
    answer_by(
        &pool,
        s,
        q,
        Utc::now() + Duration::seconds(5),
        Some("docmarie"),
    )
    .await?;
    assert_eq!(queue(&pool, s, REVIEWER).await?.0, 1, "a later answer");
    cleanup(&pool, s, "queue_done").await
}

/// (M) The reviewer's own answer never counts; anyone else's does — including an
/// unattributed one (`IS DISTINCT FROM`).
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn reviewers_own_answer_never_counts() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "queue_own").await?;
    let t0 = Utc::now();
    let own = question(&pool, s, "chuck", 1).await?;
    answer_by(&pool, s, own, t0, Some(REVIEWER)).await?;
    assert_eq!(
        queue(&pool, s, REVIEWER).await?.0,
        0,
        "the reviewer's own answer"
    );
    let other = question(&pool, s, "chuck", 2).await?;
    answer_by(&pool, s, other, t0, None).await?;
    assert_eq!(
        queue(&pool, s, REVIEWER).await?.0,
        1,
        "an unattributed answer waits"
    );
    cleanup(&pool, s, "queue_own").await
}

/// (M) THE REGRESSION THAT MATTERS: a non-reviewer's Done press changes nothing.
/// Roman presses; the reviewer's queue is unchanged. The reviewer presses; it clears.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn non_reviewer_done_press_changes_nothing() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "queue_nonreviewer").await?;
    let t0 = Utc::now() - Duration::hours(1);
    for order in 1..=2 {
        let q = question(&pool, s, "chuck", order).await?;
        answer_by(&pool, s, q, t0, Some("docmarie")).await?;
    }
    mark_reviewed(&pool, "roman", s).await?;
    mark_reviewed(&pool, "docmarie", s).await?;
    assert_eq!(
        queue(&pool, s, REVIEWER).await?.0,
        2,
        "their presses are ignored"
    );
    mark_reviewed(&pool, REVIEWER, s).await?;
    assert_eq!(
        queue(&pool, s, REVIEWER).await?.0,
        0,
        "the reviewer's press clears it"
    );
    cleanup(&pool, s, "queue_nonreviewer").await
}

/// Whoever the reviewer IS decides whose cursor is read: the same data counted
/// for a different reviewer gives that reviewer's number.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn the_queue_follows_the_reviewer_passed_in() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "queue_follows").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    answer_by(
        &pool,
        s,
        q,
        Utc::now() - Duration::hours(1),
        Some("docmarie"),
    )
    .await?;
    mark_reviewed(&pool, REVIEWER, s).await?;
    assert_eq!(
        queue(&pool, s, REVIEWER).await?.0,
        0,
        "cpenzien has reviewed"
    );
    assert_eq!(
        queue(&pool, s, "roman").await?.0,
        1,
        "roman, as reviewer, has not"
    );
    cleanup(&pool, s, "queue_follows").await
}

/// The oldest date is the MIN over waiting answers only: a reviewed older answer
/// does not set it.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn oldest_waiting_is_the_min_unreviewed_answered_at() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "queue_oldest").await?;
    let reviewed = question(&pool, s, "chuck", 1).await?;
    answer_by(
        &pool,
        s,
        reviewed,
        Utc::now() - Duration::days(3),
        Some("docmarie"),
    )
    .await?;
    mark_reviewed(&pool, REVIEWER, s).await?;

    let first_wait = Utc::now() + Duration::seconds(10);
    let a = question(&pool, s, "chuck", 2).await?;
    answer_by(&pool, s, a, first_wait, Some("docmarie")).await?;
    let b = question(&pool, s, "george", 3).await?;
    answer_by(
        &pool,
        s,
        b,
        first_wait + Duration::hours(1),
        Some("docmarie"),
    )
    .await?;

    let (count, oldest) = queue(&pool, s, REVIEWER).await?;
    assert_eq!(count, 2);
    let oldest = oldest.ok_or("an oldest date while answers wait")?;
    assert!(
        (oldest - first_wait).num_milliseconds().abs() < 1,
        "oldest = {oldest}"
    );
    assert_eq!(
        queue(&pool, s, "nobody").await?.0,
        3,
        "sanity: nothing reviewed by nobody"
    );
    cleanup(&pool, s, "queue_oldest").await
}

/// Nothing waiting: count 0 and no date. A hidden question never counts.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn nothing_waiting_has_no_date_and_hidden_never_counts() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "queue_hidden").await?;
    assert_eq!(queue(&pool, s, REVIEWER).await?, (0, None));
    let q = question(&pool, s, "chuck", 1).await?;
    answer_by(&pool, s, q, Utc::now(), Some("docmarie")).await?;
    assert_eq!(queue(&pool, s, REVIEWER).await?.0, 1);
    hide(&pool, q).await?;
    assert_eq!(queue(&pool, s, REVIEWER).await?, (0, None), "hidden");
    cleanup(&pool, s, "queue_hidden").await
}
