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

use super::live_tests::{
    answer, change, changed, changed_for, cleanup, pipeline_pool, question, scenario, TestResult,
    REVIEWER_LOGIN, WITNESS,
};

/// A note written by the LISTED REVIEWER at `at`, on the question or on one
/// answer.
///
/// ## ⚑ Why the author is `REVIEWER_LOGIN` and not the literal "chuck"
///
/// It was "chuck" until 2026-09-23, and every test in this file turned red the
/// first time the whole `--ignored` target was run. The reason is a rule these
/// fixtures pre-date: since CC_TASK_FOR_YOU_v1 L2 the witness's count is the
/// `waiting_items` predicate, where a note waits for HER only when a LISTED
/// reviewer wrote it — a note by anybody else waits for the reviewers instead
/// (ruling R2, because SQL cannot read Authentik groups). "chuck" is not on
/// this file's bench (`REVIEWER_LOGIN` is `cpenzien`), so every note here was
/// being read as a stranger's and counted for the wrong side.
///
/// The fixtures were wrong, not the rule: each caller below means "the reviewer
/// left her a note". Written through the same const the bench is built from, so
/// the two cannot drift apart again.
async fn note(
    pool: &PgPool,
    scenario_id: Uuid,
    question_id: Uuid,
    answer_id: Option<Uuid>,
    at: DateTime<Utc>,
) -> TestResult<Uuid> {
    note_by(
        pool,
        scenario_id,
        question_id,
        answer_id,
        at,
        REVIEWER_LOGIN,
    )
    .await
}

/// The same note, written by a NAMED login.
///
/// Split out for CC_TASK_REVIEW_COUNTS_HONEST_v1: the witness's own note must
/// not badge her, and proving that needs a note nobody else wrote. Every caller
/// above keeps Chuck, so nothing they assert moves.
///
/// `author_id` is the login and `author` is the display name — the filter reads
/// the ID, and the two are written together here so a fixture cannot accidentally
/// prove the filter works by leaving the id NULL.
async fn note_by(
    pool: &PgPool,
    scenario_id: Uuid,
    question_id: Uuid,
    answer_id: Option<Uuid>,
    at: DateTime<Utc>,
    author_id: &str,
) -> TestResult<Uuid> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO practice_notes \
         (scenario_id, question_id, answer_id, author, author_id, text, created_at) \
         VALUES ($1, $2, $3, $5, $5, 'hold the number', $4) RETURNING id",
    )
    .bind(scenario_id)
    .bind(question_id)
    .bind(answer_id)
    .bind(at)
    .bind(author_id)
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

// ── The witness's own notes (CC_TASK_REVIEW_COUNTS_HONEST_v1) ────────────────
//
// J1's defect, in one line: Marie wrote Chuck a note on 19 September and her own
// War Room tile came back reading "1 new or changed for Marie". The note LATERAL
// in `changed_counts` had no author leg at all, so a message she sent was
// counted as a message she had not read.
//
// The filter is the count's OWNER — the `practice_witness_username` row — and
// never the signed-in viewer: the number is one global fact by ruling
// (2026-09-17), so a per-viewer filter would give three people three different
// truths about one deck.

/// **J1** (M) Her own unstruck note on a question she answered: 0.
///
/// Red on main before this task — the same fixture read 1.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn witness_own_note_does_not_badge_her() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "note_own").await?;
    let t0 = Utc::now();
    let q = question(&pool, s, "chuck", 1).await?;
    let a = answer(&pool, s, q, t0).await?;
    note_by(&pool, s, q, Some(a), t0 + Duration::minutes(1), WITNESS).await?;
    assert_eq!(
        changed(&pool, s).await?,
        0,
        "a note she wrote herself is a message to somebody else, not work waiting on her"
    );
    cleanup(&pool, s, "note_own").await
}

/// (M) The SAME store, counted against a different witness: 1.
///
/// The mutation proof for the row itself. If the filter named a login in code
/// rather than reading `practice_witness_username`, this would still read 0 and
/// the test above would be proving nothing about the setting.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn the_filter_follows_the_witness_row() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "note_own_other").await?;
    let t0 = Utc::now();
    let q = question(&pool, s, "chuck", 1).await?;
    let a = answer(&pool, s, q, t0).await?;
    note_by(&pool, s, q, Some(a), t0 + Duration::minutes(1), WITNESS).await?;
    assert_eq!(
        changed_for(&pool, s, "somebody_else").await?,
        1,
        "pointed at another login, her note is a stranger's note and counts again"
    );
    cleanup(&pool, s, "note_own_other").await
}

/// ANTI-VACUITY: the filter is about the AUTHOR, not the clock.
///
/// Her own note written BEFORE her answer already read 0 on main, for the
/// ordinary reason. This asserts the new leg did not merely re-prove that: the
/// note here post-dates the answer and is still dropped.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn her_note_is_dropped_by_the_author_and_not_the_order() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "note_own_order").await?;
    let t0 = Utc::now();
    let q = question(&pool, s, "george", 1).await?;
    let a = answer(&pool, s, q, t0).await?;
    // An hour AFTER the answer — the arm that makes a stranger's note count.
    note_by(&pool, s, q, Some(a), t0 + Duration::hours(1), WITNESS).await?;
    assert_eq!(changed(&pool, s).await?, 0);
    // And Chuck's note at the very same instant DOES count, on the same store.
    note_by(&pool, s, q, Some(a), t0 + Duration::hours(1), "chuck").await?;
    assert_eq!(
        changed(&pool, s).await?,
        1,
        "the leg drops HER note, not every note newer than her answer"
    );
    cleanup(&pool, s, "note_own_order").await
}

/// An unattributed note (pre-2026-08-19, `author_id IS NULL`) still counts.
///
/// `NULL <> 'marie'` is NULL, which a WHERE drops — so without the explicit NULL
/// arm every note written before attribution existed would silently stop
/// counting. NULL means "not the witness", never "skip it".
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn an_unattributed_note_still_counts() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "note_null_author").await?;
    let t0 = Utc::now();
    let q = question(&pool, s, "chuck", 1).await?;
    let a = answer(&pool, s, q, t0).await?;
    sqlx::query(
        "INSERT INTO practice_notes \
         (scenario_id, question_id, answer_id, author, author_id, text, created_at) \
         VALUES ($1, $2, $3, 'unknown', NULL, 'from before attribution', $4)",
    )
    .bind(s)
    .bind(q)
    .bind(a)
    .bind(t0 + Duration::minutes(1))
    .execute(&pool)
    .await?;
    assert_eq!(changed(&pool, s).await?, 1);
    cleanup(&pool, s, "note_null_author").await
}

/// **J3** REGRESSION: somebody else's EDIT still badges her.
///
/// The witness filter is on NOTES only. A question Chuck reworded after she
/// answered it is still a question whose words have changed under her answer,
/// and that is the badge doing its job.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn another_persons_edit_still_badges_her() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "edit_after_answer").await?;
    let t0 = Utc::now();
    let q = question(&pool, s, "chuck", 1).await?;
    answer(&pool, s, q, t0).await?;
    changed(&pool, s).await?;
    change(&pool, s, q, "reworded", t0 + Duration::minutes(1)).await?;
    assert_eq!(changed(&pool, s).await?, 1);
    cleanup(&pool, s, "edit_after_answer").await
}

/// And her OWN edit badges her too — edits are not filtered by author.
///
/// Deliberate, and the line the ruling draws: a note is a message, an edit is a
/// change to the words she answered. Re-reading a question she reworded herself
/// is still work, and `chg` carries no author leg for that reason.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn her_own_edit_still_badges_her() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "own_edit").await?;
    let t0 = Utc::now();
    let q = question(&pool, s, "george", 1).await?;
    answer(&pool, s, q, t0).await?;
    change(&pool, s, q, "reworded", t0 + Duration::minutes(1)).await?;
    assert_eq!(changed(&pool, s).await?, 1);
    cleanup(&pool, s, "own_edit").await
}
