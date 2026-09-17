//! Live-database proofs for the review cursor (CC_TASK_REVIEW_LOOP_v1 §1–§2).
//!
//! `#[ignore]`d, and run by hand against a SCRATCH copy of the pipeline schema
//! (`PIPELINE_DATABASE_URL=… cargo test --lib review_cursor -- --ignored
//! --test-threads=1`), never against `colossus_legal_v2`.
//!
//! Each `(M)` test is mutation-proved in the report: removing the clause it
//! names from `new_answers_for_viewer` turns it red.

use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::super::war_room_status::live_tests::{
    answer_by, cleanup, hide, pipeline_pool, question, scenario, TestResult,
};
use super::{mark_reviewed, new_answers_for_viewer};

async fn new_for(pool: &PgPool, s: Uuid, viewer: Option<&str>) -> TestResult<i64> {
    let rows = new_answers_for_viewer(pool, &[s], viewer).await?;
    assert_eq!(rows.len(), 1, "one row per scenario asked for");
    Ok(rows[0].new_answers)
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
    let first = mark_reviewed(&pool, "chuck", s).await?;
    let second = mark_reviewed(&pool, "chuck", s).await?;
    assert_eq!(cursor_rows(&pool, s).await?, 1);
    assert!(second >= first, "the mark never moves backwards");
    cleanup(&pool, s, "cursor_upsert").await
}

/// No cursor row: every qualifying answer is new. After Done: 0. A later answer: 1.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn no_cursor_counts_all_then_done_zeroes_it() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "cursor_done").await?;
    let t0 = Utc::now() - Duration::hours(1);
    for order in 1..=3 {
        let q = question(&pool, s, "chuck", order).await?;
        answer_by(&pool, s, q, t0, Some("marie")).await?;
    }
    assert_eq!(
        new_for(&pool, s, Some("chuck")).await?,
        3,
        "no row = all new"
    );

    mark_reviewed(&pool, "chuck", s).await?;
    assert_eq!(new_for(&pool, s, Some("chuck")).await?, 0, "after Done");

    let q = question(&pool, s, "george", 4).await?;
    answer_by(
        &pool,
        s,
        q,
        Utc::now() + Duration::seconds(5),
        Some("marie"),
    )
    .await?;
    assert_eq!(new_for(&pool, s, Some("chuck")).await?, 1, "a later answer");
    cleanup(&pool, s, "cursor_done").await
}

/// (M) The viewer's own answers never count; another viewer's cursor is theirs.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn viewers_own_answers_never_count() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "cursor_own").await?;
    let t0 = Utc::now();
    let q = question(&pool, s, "chuck", 1).await?;
    answer_by(&pool, s, q, t0, Some("chuck")).await?;
    assert_eq!(new_for(&pool, s, Some("chuck")).await?, 0, "his own answer");
    assert_eq!(new_for(&pool, s, Some("roman")).await?, 1, "someone else's");
    cleanup(&pool, s, "cursor_own").await
}

/// An unattributed (pre-2026-08-19) answer counts as "not you" (GO v1 ruling 1).
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn an_unattributed_answer_counts_as_not_you() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "cursor_null").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    answer_by(&pool, s, q, Utc::now(), None).await?;
    assert_eq!(new_for(&pool, s, Some("chuck")).await?, 1);
    cleanup(&pool, s, "cursor_null").await
}

/// No signed-in viewer: 0. A hidden question never counts.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn no_viewer_and_hidden_questions_are_zero() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "cursor_none").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    answer_by(&pool, s, q, Utc::now(), Some("marie")).await?;
    assert_eq!(new_for(&pool, s, None).await?, 0, "no viewer");
    assert_eq!(new_for(&pool, s, Some("chuck")).await?, 1);
    hide(&pool, q).await?;
    assert_eq!(new_for(&pool, s, Some("chuck")).await?, 0, "hidden");
    cleanup(&pool, s, "cursor_none").await
}
