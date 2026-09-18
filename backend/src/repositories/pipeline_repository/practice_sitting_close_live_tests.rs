//! Live-database proofs for the day-boundary sitting closer
//! (CC_TASK_DEFECT_SWEEP_v1 defect 1).
//!
//! `#[ignore]`d and run by hand against a SCRATCH copy of the pipeline schema
//! (`PIPELINE_DATABASE_URL=… cargo test --lib practice_sitting_close --
//! --ignored --test-threads=1`), never against `colossus_legal_v2`.
//!
//! The day boundary is decided in SQL, in the case's timezone, against the
//! database's own clock — so there is no pure half to test. A fixture that
//! backdates `started_at` and asks what happened is the honest proof.
//!
//! Each `(M)` test is mutation-proved in the report.

use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::super::war_room_status::live_tests::{cleanup, pipeline_pool, question, TestResult};
use super::close_sittings_before_today;

/// The case's own timezone, as the settings row carries it.
const TZ: &str = "America/Detroit";

/// A scenario for this file's fixtures.
async fn scenario(pool: &PgPool, tag: &str) -> TestResult<Uuid> {
    super::super::war_room_status::live_tests::scenario(pool, tag).await
}

/// One open sitting, started `days_ago`, owned by `user_id`. Returns its id.
async fn sitting(
    pool: &PgPool,
    scenario_id: Uuid,
    user_id: &str,
    days_ago: i64,
) -> TestResult<Uuid> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO practice_sessions (scenario_id, who, user_id, started_at) \
         VALUES ($1, 'mixed', $2, $3) RETURNING id",
    )
    .bind(scenario_id)
    .bind(user_id)
    .bind(Utc::now() - Duration::days(days_ago))
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// An answer on `session_id`, stamped `days_ago`.
async fn answer(
    pool: &PgPool,
    session_id: Uuid,
    question_id: Uuid,
    days_ago: i64,
) -> TestResult<()> {
    sqlx::query(
        "INSERT INTO practice_answers \
         (session_id, question_id, answer_text, self_check, mark, answered_at) \
         VALUES ($1, $2, 'her words', '{}'::jsonb, 'fine', $3)",
    )
    .bind(session_id)
    .bind(question_id)
    .bind(Utc::now() - Duration::days(days_ago))
    .execute(pool)
    .await?;
    Ok(())
}

/// `ended_at` on one sitting, or `None` while it stands.
async fn ended_at(pool: &PgPool, id: Uuid) -> TestResult<Option<chrono::DateTime<Utc>>> {
    let row: (Option<chrono::DateTime<Utc>>,) =
        sqlx::query_as("SELECT ended_at FROM practice_sessions WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}

/// Yesterday's sitting is over; today's is not. (M)
///
/// The whole rule in one test. If the predicate ever loosened to "close every
/// open sitting", the second assertion is what goes red — and closing the
/// sitting Marie is CURRENTLY answering into would end her day's work under her
/// while she typed.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_sitting_from_yesterday_closes_and_todays_does_not() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "close_yesterday").await?;

    let old = sitting(&pool, s, "docmarie", 3).await?;
    let current = sitting(&pool, s, "docmarie", 0).await?;

    let closed = close_sittings_before_today(&pool, s, "docmarie", TZ).await?;

    assert_eq!(closed, 1, "only the one that did not start today");
    assert!(ended_at(&pool, old).await?.is_some(), "yesterday's is over");
    assert!(
        ended_at(&pool, current).await?.is_none(),
        "she is still in today's — closing it would end her work under her"
    );

    cleanup(&pool, s, "close_yesterday").await
}

/// A closed sitting is stamped at its own last answer, not at now. (M)
///
/// Ruling 1(b), 2026-09-17. `ended_at` is the moment the work stopped, and it
/// is what `last_ended_session` reports to the deck's changed-box. Stamping
/// `NOW()` would record a month of silence as practice.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_closed_sitting_is_stamped_at_its_own_last_answer() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "close_stamp").await?;
    let q = question(&pool, s, "george", 0).await?;

    let old = sitting(&pool, s, "docmarie", 10).await?;
    answer(&pool, old, q, 9).await?;
    answer(&pool, old, q, 8).await?;

    close_sittings_before_today(&pool, s, "docmarie", TZ).await?;

    let stamped = ended_at(&pool, old).await?.expect("it closed");
    let expected = Utc::now() - Duration::days(8);
    let drift = (stamped - expected).num_seconds().abs();
    assert!(
        drift < 60,
        "expected the LAST answer (8 days ago), got {stamped} — a NOW() stamp \
         would be eight days out"
    );

    cleanup(&pool, s, "close_stamp").await
}

/// A sitting that holds no answers falls back to when it was opened.
///
/// 18 of the 26 strays on DEV were empty. `COALESCE` to `started_at` is the only
/// honest thing left to say about a sitting where nothing happened — and without
/// it the UPDATE would write NULL and the row would still read as open.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn an_empty_sitting_is_stamped_at_its_start() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "close_empty").await?;

    let empty = sitting(&pool, s, "docmarie", 5).await?;
    close_sittings_before_today(&pool, s, "docmarie", TZ).await?;

    let stamped = ended_at(&pool, empty).await?.expect("it closed, not NULL");
    let drift = (stamped - (Utc::now() - Duration::days(5)))
        .num_seconds()
        .abs();
    assert!(drift < 60, "expected its own started_at, got {stamped}");

    cleanup(&pool, s, "close_empty").await
}

/// One user's day ending never ends another's sitting. (M)
///
/// Chuck opening the page must not close Marie's work, which is the same rule
/// `practice_flow::newest_open_session` carries: a sitting belongs to whoever
/// opened it.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn another_users_sitting_is_never_touched() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "close_other_user").await?;

    let hers = sitting(&pool, s, "docmarie", 4).await?;
    let his = sitting(&pool, s, "cpenzien", 4).await?;

    let closed = close_sittings_before_today(&pool, s, "cpenzien", TZ).await?;

    assert_eq!(closed, 1, "his own, and only his own");
    assert!(ended_at(&pool, his).await?.is_some());
    assert!(
        ended_at(&pool, hers).await?.is_none(),
        "he closed her sitting — a sitting belongs to whoever opened it"
    );

    cleanup(&pool, s, "close_other_user").await
}
