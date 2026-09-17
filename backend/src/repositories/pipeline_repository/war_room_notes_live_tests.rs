//! Live-database proofs that NOTES move Marie's pill (CC_TASK_REVIEW_LOOP_v1 §3).
//!
//! `#[ignore]`d, and run by hand against a SCRATCH copy of the pipeline schema,
//! for the reason `war_room_status_live_tests` gives — never `colossus_legal_v2`.
//!
//! The rule: an UNSTRUCK note on a question, or on its CURRENT answer, counts
//! when it is newer than her answer. Mutation-proved (report): each `(M)` test
//! turns red when its clause is removed from `changed_counts`.

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::live_tests::{answer, changed, cleanup, pipeline_pool, question, scenario, TestResult};

/// A note written by Chuck at `at`, on the question or on one answer.
async fn note(
    pool: &PgPool,
    scenario_id: Uuid,
    question_id: Uuid,
    answer_id: Option<Uuid>,
    at: DateTime<Utc>,
) -> TestResult<Uuid> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO practice_notes \
         (scenario_id, question_id, answer_id, author, author_id, text, created_at) \
         VALUES ($1, $2, $3, 'Chuck', 'chuck', 'hold the number', $4) RETURNING id",
    )
    .bind(scenario_id)
    .bind(question_id)
    .bind(answer_id)
    .bind(at)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

async fn strike(pool: &PgPool, note_id: Uuid) -> TestResult<()> {
    super::super::practice_notes::strike_note(pool, note_id, "chuck").await?;
    Ok(())
}

/// A note on her current answer, written after it: 1.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn note_after_marie_answer_counts() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "note_after").await?;
    let t0 = Utc::now();
    let q = question(&pool, s, "chuck", 1).await?;
    let a = answer(&pool, s, q, t0).await?;
    note(&pool, s, q, Some(a), t0 + Duration::minutes(1)).await?;
    assert_eq!(changed(&pool, s).await?, 1);
    cleanup(&pool, s, "note_after").await
}

/// (M) A question-level note she has since answered past: 0.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn note_before_answer_does_not_count() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "note_before").await?;
    let t0 = Utc::now();
    let q = question(&pool, s, "chuck", 1).await?;
    note(&pool, s, q, None, t0).await?;
    answer(&pool, s, q, t0 + Duration::minutes(1)).await?;
    assert_eq!(changed(&pool, s).await?, 0);
    cleanup(&pool, s, "note_before").await
}

/// (M) Striking the note removes it from the count; a struck note never counts.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn striking_removes_the_note_from_the_count() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "note_struck").await?;
    let t0 = Utc::now();
    let q = question(&pool, s, "george", 1).await?;
    let a = answer(&pool, s, q, t0).await?;
    let n = note(&pool, s, q, Some(a), t0 + Duration::minutes(1)).await?;
    assert_eq!(changed(&pool, s).await?, 1, "standing, it counts");
    strike(&pool, n).await?;
    assert_eq!(changed(&pool, s).await?, 0, "struck, it never counts");
    cleanup(&pool, s, "note_struck").await
}

/// A note on a SUPERSEDED attempt does not count (GO v1 ruling 6).
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn note_on_a_superseded_attempt_does_not_count() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "note_superseded").await?;
    let t0 = Utc::now();
    let q = question(&pool, s, "chuck", 1).await?;
    let first = answer(&pool, s, q, t0).await?;
    answer(&pool, s, q, t0 + Duration::minutes(1)).await?;
    // Written after her NEWEST answer, but about the old one.
    note(&pool, s, q, Some(first), t0 + Duration::minutes(2)).await?;
    assert_eq!(changed(&pool, s).await?, 0);
    cleanup(&pool, s, "note_superseded").await
}

/// An unanswered question with a note newer than her newest answer in the
/// scenario counts — the same arm changes use.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn note_on_an_unanswered_question_uses_her_latest_answer() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "note_unanswered").await?;
    let t0 = Utc::now();
    let answered = question(&pool, s, "chuck", 1).await?;
    answer(&pool, s, answered, t0).await?;
    let open = question(&pool, s, "george", 2).await?;
    note(&pool, s, open, None, t0 + Duration::minutes(1)).await?;
    assert_eq!(changed(&pool, s).await?, 1);
    cleanup(&pool, s, "note_unanswered").await
}
