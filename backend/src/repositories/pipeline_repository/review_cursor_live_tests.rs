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

/// The queue for a bench of ONE — the shape every test below this line asks in,
/// and the shape every store has on the day CC_TASK_REVIEW_PAGE_v1 ships.
async fn queue(pool: &PgPool, s: Uuid, reviewer: &str) -> TestResult<(i64, Option<DateTime<Utc>>)> {
    bench_queue(pool, s, &[reviewer]).await
}

/// The queue for a bench of any size.
async fn bench_queue(
    pool: &PgPool,
    s: Uuid,
    reviewers: &[&str],
) -> TestResult<(i64, Option<DateTime<Utc>>)> {
    let bench: Vec<String> = reviewers.iter().map(|r| (*r).to_string()).collect();
    let rows = awaiting_review(pool, &[s], &bench).await?;
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

// ─── The reviewer BENCH (CC_TASK_REVIEW_PAGE_v1, ruled 2026-09-19) ───────────
//
// Three proofs for the three places the bench replaced one name in the SQL.
// Each `(M)` is mutation-proved in the report.

/// (M) ONE SHARED CURSOR: either listed reviewer's press clears the count for
/// both, and the LATER press is the mark.
///
/// This is the ruling in one test. Read one row per reviewer instead, and there
/// would be as many queues as there are reviewers — the state v2.1.10 was
/// written to end, arriving again by the back door.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn either_reviewers_press_clears_the_shared_count() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "queue_bench_shared").await?;
    let bench = ["cpenzien", "roman"];
    let q = question(&pool, s, "chuck", 1).await?;
    answer_by(
        &pool,
        s,
        q,
        Utc::now() - Duration::hours(2),
        Some("docmarie"),
    )
    .await?;
    assert_eq!(
        bench_queue(&pool, s, &bench).await?.0,
        1,
        "nobody on the bench has reviewed"
    );

    // ROMAN presses — not the login the singular row used to hold.
    mark_reviewed(&pool, "roman", s).await?;
    assert_eq!(
        bench_queue(&pool, s, &bench).await?.0,
        0,
        "the second reviewer's press clears it for the whole bench"
    );

    // A newer answer waits again for everyone.
    let q2 = question(&pool, s, "chuck", 2).await?;
    answer_by(&pool, s, q2, Utc::now(), Some("docmarie")).await?;
    assert_eq!(bench_queue(&pool, s, &bench).await?.0, 1, "a later answer");

    // And the OTHER reviewer's press clears that one — the mark is the latest
    // press by anybody on the bench, not one person's row.
    mark_reviewed(&pool, "cpenzien", s).await?;
    assert_eq!(
        bench_queue(&pool, s, &bench).await?.0,
        0,
        "the first reviewer's press moves the same shared mark"
    );
    cleanup(&pool, s, "queue_bench_shared").await
}

/// (M) An answer written by ANY listed reviewer never waits.
///
/// Left as one name, the second reviewer's own answer would have counted as
/// work waiting for the second reviewer — which is exactly the bug the
/// exclusion leg exists to prevent, reintroduced for everybody but the first
/// name on the list.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn an_answer_by_any_listed_reviewer_never_waits() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "queue_bench_author").await?;
    let bench = ["cpenzien", "roman"];
    let t0 = Utc::now() - Duration::hours(1);

    let first = question(&pool, s, "chuck", 1).await?;
    answer_by(&pool, s, first, t0, Some("cpenzien")).await?;
    let second = question(&pool, s, "chuck", 2).await?;
    answer_by(&pool, s, second, t0, Some("roman")).await?;
    assert_eq!(
        bench_queue(&pool, s, &bench).await?.0,
        0,
        "neither reviewer's own answer waits for the bench"
    );

    // Somebody NOT on the bench does wait — the anti-vacuity half, without
    // which an exclusion that matched everybody would pass the line above.
    let third = question(&pool, s, "chuck", 3).await?;
    answer_by(&pool, s, third, t0, Some("docmarie")).await?;
    assert_eq!(
        bench_queue(&pool, s, &bench).await?.0,
        1,
        "an answer by somebody else still waits"
    );

    // An UNATTRIBUTED answer counts too: sittings from before 2026-08-19 carry
    // no author id, and a NULL there means "not a reviewer", never "skip it".
    // This is the leg that three-valued logic silently drops if the NULL case
    // is not named in the predicate.
    let fourth = question(&pool, s, "chuck", 4).await?;
    answer_by(&pool, s, fourth, t0, None).await?;
    assert_eq!(
        bench_queue(&pool, s, &bench).await?.0,
        2,
        "an unattributed answer is not a reviewer's"
    );
    cleanup(&pool, s, "queue_bench_author").await
}

/// (M) A press by somebody NOT on the bench still moves nothing.
///
/// The regression `non_reviewer_done_press_changes_nothing` guards, asked again
/// of a bench: widening the `mark` CTE from one login to a list must not widen
/// it to everybody, which is the mistake `user_id = ANY($2)` invites if the
/// bind is ever passed the wrong list.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_press_from_outside_the_bench_still_moves_nothing() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "queue_bench_outsider").await?;
    let bench = ["cpenzien", "roman"];
    let q = question(&pool, s, "chuck", 1).await?;
    answer_by(
        &pool,
        s,
        q,
        Utc::now() - Duration::hours(3),
        Some("docmarie"),
    )
    .await?;

    mark_reviewed(&pool, "docmarie", s).await?;
    mark_reviewed(&pool, "somebody_else", s).await?;
    assert_eq!(
        bench_queue(&pool, s, &bench).await?.0,
        1,
        "presses from outside the bench are recorded and ignored"
    );

    mark_reviewed(&pool, "cpenzien", s).await?;
    assert_eq!(
        bench_queue(&pool, s, &bench).await?.0,
        0,
        "a listed reviewer's press clears it"
    );
    cleanup(&pool, s, "queue_bench_outsider").await
}
