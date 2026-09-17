//! Live-database proofs that Marie's NOTES reach Chuck's review queue
//! (CC_TASK_DEFECT_SWEEP_v1 defect 5).
//!
//! `#[ignore]`d and run by hand against a SCRATCH copy of the pipeline schema
//! (`PIPELINE_DATABASE_URL=… cargo test --lib review_cursor -- --ignored
//! --test-threads=1`), never against `colossus_legal_v2`.
//!
//! The mirror of `war_room_notes_live_tests`, which proves the same rule in the
//! other direction. Until this shipped the loop ran one way: Chuck's notes moved
//! Marie's pill, and Marie's notes moved nothing.
//!
//! Each `(M)` test is mutation-proved in the report.

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::super::war_room_status::live_tests::{
    answer_by, cleanup, pipeline_pool, question, scenario, TestResult,
};
use super::{awaiting_review, mark_reviewed};

/// The reviewer the fixtures use — a test literal standing in for the settings row.
const REVIEWER: &str = "cpenzien";

/// How many questions Chuck's queue holds, and since when.
async fn queue(pool: &PgPool, s: Uuid) -> TestResult<(i64, Option<DateTime<Utc>>)> {
    let rows = awaiting_review(pool, &[s], REVIEWER).await?;
    assert_eq!(rows.len(), 1, "one row per scenario asked for");
    Ok((rows[0].awaiting, rows[0].oldest))
}

/// One second after a mark the SERVER issued.
///
/// ## Why not `Utc::now()`
///
/// `mark_reviewed` stamps the cursor with the DATABASE's `now()`, and these
/// tests run on a different machine. A note stamped from the Mac's clock can
/// land a fraction BEFORE a cursor stamped from the server's, and the test then
/// fails for a reason that has nothing to do with the code under test — which is
/// exactly what it did on the first scratch run. Anchoring to the value the
/// server just returned takes both clocks out of the question.
fn after(mark: DateTime<Utc>) -> DateTime<Utc> {
    mark + Duration::seconds(1)
}

/// A note by `author_id`, on the question or on one answer, stamped `at`.
async fn note(
    pool: &PgPool,
    scenario_id: Uuid,
    question_id: Uuid,
    answer_id: Option<Uuid>,
    author_id: &str,
    at: DateTime<Utc>,
) -> TestResult<Uuid> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO practice_notes \
         (scenario_id, question_id, answer_id, author, author_id, text, created_at) \
         VALUES ($1, $2, $3, $4, $4, 'look at this one again', $5) RETURNING id",
    )
    .bind(scenario_id)
    .bind(question_id)
    .bind(answer_id)
    .bind(author_id)
    .bind(at)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Marie's note on a question Chuck has already reviewed puts it back. (M)
///
/// The defect itself. Before this leg existed, Chuck pressed Done reviewing, the
/// queue went to 0, and Marie's note — the one thing she can say back about a
/// question — reached him nowhere.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn her_note_on_a_reviewed_question_puts_it_back_in_his_queue() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "cursor_note_back").await?;
    let t0 = Utc::now() - Duration::hours(2);
    let q = question(&pool, s, "george", 1).await?;
    let a = answer_by(&pool, s, q, t0, Some("docmarie")).await?;

    assert_eq!(queue(&pool, s).await?.0, 1, "her answer waits");
    let mark = mark_reviewed(&pool, REVIEWER, s).await?;
    assert_eq!(queue(&pool, s).await?.0, 0, "he has read it");

    note(&pool, s, q, Some(a), "docmarie", after(mark)).await?;

    assert_eq!(
        queue(&pool, s).await?.0,
        1,
        "her note after his cursor puts the question back in front of him"
    );
    cleanup(&pool, s, "cursor_note_back").await
}

/// Striking the note withdraws it again. (M)
///
/// The same rule her own pill obeys: nothing is deleted, and striking is how a
/// note stops asking for attention.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn striking_her_note_withdraws_it_from_his_queue() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "cursor_note_struck").await?;
    let t0 = Utc::now() - Duration::hours(2);
    let q = question(&pool, s, "george", 1).await?;
    let a = answer_by(&pool, s, q, t0, Some("docmarie")).await?;
    let mark = mark_reviewed(&pool, REVIEWER, s).await?;

    let n = note(&pool, s, q, Some(a), "docmarie", after(mark)).await?;
    assert_eq!(queue(&pool, s).await?.0, 1, "standing, it counts");

    super::super::practice_notes::strike_note(&pool, n, REVIEWER).await?;
    assert_eq!(queue(&pool, s).await?.0, 0, "struck, it never counts");

    cleanup(&pool, s, "cursor_note_struck").await
}

/// Chuck's OWN note never puts a question in his own queue. (M)
///
/// He writes most of the notes. Counting them would hand him a queue made of his
/// own handwriting, which is the failure the answer leg already avoids with the
/// same `author_id` test.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn his_own_note_never_counts() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "cursor_note_own").await?;
    let t0 = Utc::now() - Duration::hours(2);
    let q = question(&pool, s, "george", 1).await?;
    let a = answer_by(&pool, s, q, t0, Some("docmarie")).await?;
    let mark = mark_reviewed(&pool, REVIEWER, s).await?;

    note(&pool, s, q, Some(a), REVIEWER, after(mark)).await?;

    assert_eq!(queue(&pool, s).await?.0, 0, "his own hand does not queue");
    cleanup(&pool, s, "cursor_note_own").await
}

/// A note OLDER than his cursor does not count.
///
/// The cursor leg. Without it, every historical note on the deck would land in
/// the queue the moment this shipped and Done reviewing could never clear it.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_note_older_than_his_cursor_does_not_count() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "cursor_note_old").await?;
    let t0 = Utc::now() - Duration::hours(2);
    let q = question(&pool, s, "george", 1).await?;
    let a = answer_by(&pool, s, q, t0, Some("docmarie")).await?;

    note(&pool, s, q, Some(a), "docmarie", t0 + Duration::minutes(1)).await?;
    mark_reviewed(&pool, REVIEWER, s).await?;

    assert_eq!(queue(&pool, s).await?.0, 0, "he has read past it");
    cleanup(&pool, s, "cursor_note_old").await
}

/// A note on a SUPERSEDED attempt does not count.
///
/// Mirrors `war_room_notes_live_tests::note_on_a_superseded_attempt_does_not_count`
/// — a note about an answer she has since replaced is about a thing that is no
/// longer on screen.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_note_on_a_superseded_attempt_does_not_count() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "cursor_note_superseded").await?;
    let t0 = Utc::now() - Duration::hours(3);
    let q = question(&pool, s, "george", 1).await?;
    let first = answer_by(&pool, s, q, t0, Some("docmarie")).await?;
    answer_by(&pool, s, q, t0 + Duration::minutes(1), Some("docmarie")).await?;
    let mark = mark_reviewed(&pool, REVIEWER, s).await?;

    note(&pool, s, q, Some(first), "docmarie", after(mark)).await?;

    assert_eq!(queue(&pool, s).await?.0, 0, "it is about a replaced answer");
    cleanup(&pool, s, "cursor_note_superseded").await
}

/// A question waiting on BOTH an answer and a note counts ONCE. (M)
///
/// The join's own hazard. A LATERAL that produced a row per note would multiply
/// the question and the queue would read 2 for one question — the "counts rows,
/// not things" failure, arriving by the back door.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_question_waiting_on_both_counts_once() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "cursor_note_both").await?;
    let q = question(&pool, s, "george", 1).await?;
    let a = answer_by(&pool, s, q, Utc::now(), Some("docmarie")).await?;

    // Two notes AND a fresh answer, all after his (absent) cursor.
    note(&pool, s, q, Some(a), "docmarie", Utc::now()).await?;
    note(&pool, s, q, None, "docmarie", Utc::now()).await?;

    assert_eq!(
        queue(&pool, s).await?.0,
        1,
        "one question is one item however many reasons it has to be there"
    );
    cleanup(&pool, s, "cursor_note_both").await
}

/// "Oldest waiting" names the note when the note is what is waiting. (M)
///
/// The summary card prints this date. A question can wait on the NOTE alone —
/// here the only answer is the reviewer's own, which never counts — and
/// reporting the answer's stamp would name a moment four days older than
/// anything actually waiting: a date that contradicts the count beside it.
///
/// No cursor row and no `mark_reviewed` on purpose: both legs would then be
/// racing the same clock, and a test that depends on which of two `now()`s lands
/// first is a test that fails on a slow afternoon.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn the_oldest_waiting_date_names_the_note_when_the_note_is_what_waits() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "cursor_note_oldest").await?;
    let q = question(&pool, s, "george", 1).await?;

    // HIS answer, four days old: the answer leg can never fire on it.
    let a = answer_by(&pool, s, q, Utc::now() - Duration::days(4), Some(REVIEWER)).await?;
    assert_eq!(
        queue(&pool, s).await?.0,
        0,
        "his own answer waits on nobody"
    );

    // HER note, an hour old: the only thing waiting.
    let when = Utc::now() - Duration::hours(1);
    note(&pool, s, q, Some(a), "docmarie", when).await?;

    let (count, oldest) = queue(&pool, s).await?;
    assert_eq!(count, 1, "her note is what puts it in his queue");
    let oldest = oldest.expect("something waits, so something is the oldest");
    assert!(
        (oldest - when).num_seconds().abs() < 60,
        "expected the note's own stamp ({when}), got {oldest} — the answer is \
         four days older and is his own"
    );
    cleanup(&pool, s, "cursor_note_oldest").await
}
