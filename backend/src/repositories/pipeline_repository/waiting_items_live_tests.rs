//! Live-database proofs of the one waiting-items predicate (CC_TASK_FOR_YOU_v1 L0).
//!
//! `#[ignore]`d and run by hand against a SCRATCH copy of the pipeline schema
//! (`PIPELINE_DATABASE_URL=… cargo test --lib waiting_items -- --ignored
//! --test-threads=1`), never against `colossus_legal_v2` — the reason
//! `war_room_status_live_tests` gives: this project has no `#[sqlx::test]`
//! fixture.
//!
//! What only a database can prove here: that the audience rules really split the
//! three item kinds between two lists, that the author exclusion survives a NULL
//! author, and that a seen row really removes ONE item and leaves its neighbours
//! alone. None of that is visible to the compiler — every one of those rules
//! lives inside a `&str`.
//!
//! Each `(M)` test is mutation-proved in the report.

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::super::item_seen::{mark_seen, ItemRef};
use super::super::war_room_status::live_tests::{
    answer_by, cleanup, pipeline_pool, question, scenario, TestResult,
};
use super::{waiting_items, waiting_total, WaitingQuery, WaitingScope, WaitingSide};

/// The LISTED reviewer. A test literal standing in for the settings row.
pub(super) const REVIEWER: &str = "cpenzien";
/// An administrator who is NOT on the list — ruling R2's case, carried forward.
pub(super) const ADMIN: &str = "roman";
/// The witness. Also a settings row in production.
pub(super) const WITNESS: &str = "docmarie";
/// Long enough that no fixture below is ever truncated by it. The production
/// caller reads its page length from configuration; this is a test literal.
pub(super) const PLENTY: i64 = 100;

/// The listed bench, in the shape the repository takes it.
pub(super) fn bench() -> Vec<String> {
    vec![REVIEWER.to_string()]
}

/// Everything waiting for `viewer` on `side`, newest first.
pub(super) async fn items(
    pool: &PgPool,
    s: Uuid,
    viewer: &str,
    side: WaitingSide,
) -> TestResult<Vec<(String, Uuid)>> {
    let reviewers = bench();
    let query = WaitingQuery {
        scenario_ids: &[s],
        viewer,
        reviewers: &reviewers,
        side,
        // Every audience test below asks the UNREAD tab: what waits is the
        // question these rules answer. `Everything` has its own proof, in the
        // seen file beside this one.
        scope: WaitingScope::Unseen,
    };
    let rows = waiting_items(pool, &query, Some(PLENTY)).await?;
    // The total is asked on every call, so every assertion about a list below is
    // also an assertion that the badge agrees with it. One predicate, two
    // readings: if they could disagree, this is where it would show.
    let total = waiting_total(pool, &query).await?;
    assert_eq!(
        total,
        rows.len() as i64,
        "the badge total and the list disagree"
    );
    Ok(rows.into_iter().map(|r| (r.kind, r.item_id)).collect())
}

/// How many items wait for `viewer` on `side`.
pub(super) async fn count(
    pool: &PgPool,
    s: Uuid,
    viewer: &str,
    side: WaitingSide,
) -> TestResult<usize> {
    Ok(items(pool, s, viewer, side).await?.len())
}

/// A note by `author_id` (or by nobody, before attribution existed), stamped `at`.
pub(super) async fn note(
    pool: &PgPool,
    scenario_id: Uuid,
    question_id: Uuid,
    author_id: Option<&str>,
    at: DateTime<Utc>,
) -> TestResult<Uuid> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO practice_notes \
         (scenario_id, question_id, author, author_id, text, created_at) \
         VALUES ($1, $2, COALESCE($3, 'unknown'), $3, 'look at this one again', $4) \
         RETURNING id",
    )
    .bind(scenario_id)
    .bind(question_id)
    .bind(author_id)
    .bind(at)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// (M) An answer waits for the reviewers, never for the person who wrote it, and
/// never for the witness.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn an_answer_waits_for_a_reviewer_and_not_for_its_author() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "waiting_answer").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    let a = answer_by(&pool, s, q, Utc::now() - Duration::hours(1), Some(WITNESS)).await?;

    assert_eq!(
        items(&pool, s, REVIEWER, WaitingSide::Reviewers).await?,
        vec![("answer".to_string(), a)],
        "her answer waits for the reviewer"
    );
    assert_eq!(
        count(&pool, s, WITNESS, WaitingSide::Reviewers).await?,
        0,
        "never for its own author"
    );
    assert_eq!(
        count(&pool, s, WITNESS, WaitingSide::Witness).await?,
        0,
        "an answer is not on the witness's side at all"
    );
    cleanup(&pool, s, "waiting_answer").await
}

/// (M) A LISTED reviewer's note waits for the witness, and for nobody else.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_reviewers_note_waits_for_the_witness_only() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "waiting_note_side").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    let n = note(&pool, s, q, Some(REVIEWER), Utc::now()).await?;

    assert_eq!(
        items(&pool, s, WITNESS, WaitingSide::Witness).await?,
        vec![("note".to_string(), n)],
        "Chuck's note is what Marie is waiting on"
    );
    assert_eq!(
        count(&pool, s, REVIEWER, WaitingSide::Reviewers).await?,
        0,
        "not for the reviewer who wrote it"
    );
    assert_eq!(
        count(&pool, s, ADMIN, WaitingSide::Reviewers).await?,
        0,
        "and not for the reviewers' list at all — it has a side, and it is hers"
    );
    cleanup(&pool, s, "waiting_note_side").await
}

/// (M) An UNLISTED administrator's note waits for the listed reviewers.
///
/// Ruling R2 of CC_TASK_REVIEW_PERMISSION_v1, carried forward: SQL cannot see
/// Authentik groups, so the only thing it can read is the stored list. Roman's
/// note therefore reads as "not from a reviewer" and goes to the reviewers'
/// side — deliberately, and never to the witness, who did not write to him.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn an_unlisted_admins_note_waits_for_the_listed_reviewers() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "waiting_note_admin").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    let n = note(&pool, s, q, Some(ADMIN), Utc::now()).await?;

    assert_eq!(
        items(&pool, s, REVIEWER, WaitingSide::Reviewers).await?,
        vec![("note".to_string(), n)],
        "the listed reviewer sees it"
    );
    assert_eq!(
        count(&pool, s, ADMIN, WaitingSide::Reviewers).await?,
        0,
        "its own author does not"
    );
    assert_eq!(
        count(&pool, s, WITNESS, WaitingSide::Witness).await?,
        0,
        "and the witness does not"
    );
    cleanup(&pool, s, "waiting_note_admin").await
}

/// (M) A note from before attribution existed waits for the reviewers until a
/// seen row says otherwise — which is exactly what the L0 back-fill writes.
///
/// Ruled blind on 2026-09-22 (GO, Q2): an item with no author has no side, so it
/// can belong to nobody's list, and the migration marks every one of them seen
/// for everybody this store knows about. This test proves both halves of that
/// sentence: that the predicate does NOT special-case a NULL author (it reads as
/// "not a reviewer", per the three-valued-logic clause), and that a seen row is
/// what silences it.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn an_author_less_note_waits_until_it_is_marked_seen() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "waiting_note_authorless").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    let n = note(&pool, s, q, None, Utc::now() - Duration::days(400)).await?;

    assert_eq!(
        items(&pool, s, REVIEWER, WaitingSide::Reviewers).await?,
        vec![("note".to_string(), n)],
        "a NULL author reads as 'not a reviewer', not as 'skip this row'"
    );
    assert_eq!(mark_seen(&pool, REVIEWER, &[ItemRef::Note(n)]).await?, 1);
    assert_eq!(
        count(&pool, s, REVIEWER, WaitingSide::Reviewers).await?,
        0,
        "what the back-fill does, done by hand"
    );
    cleanup(&pool, s, "waiting_note_authorless").await
}
