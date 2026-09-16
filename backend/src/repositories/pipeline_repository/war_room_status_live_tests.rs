//! Live-database proofs for the War Room status reads.
//!
//! `#[ignore]`d for the reason `practice_reorder_live_tests` gives: the project
//! has no `#[sqlx::test]` fixture, so these run by hand against a SCRATCH copy of
//! the pipeline schema (`PIPELINE_DATABASE_URL=… cargo test --lib war_room_status
//! -- --ignored --test-threads=1`), never against `colossus_legal_v2`.
//!
//! What only a database can prove: that the `unnest`-first shape really returns a
//! row for a scenario with nothing in it, that `hidden_at` really leaves every
//! count, and that the changed rule reads the right timestamps.

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{changed_counts, deck_counts, last_scans, prep_counts};
use crate::config::AppConfig;
use crate::domain::human_authored::HumanFactKind;
use crate::repositories::pipeline_repository::scenario_store::{delete_scenario, insert_scenario};

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn test_slug(tag: &str) -> String {
    format!("awad_v_catholic_family_service__test_war_room_{tag}")
}

async fn pipeline_pool() -> TestResult<PgPool> {
    // best-effort: a missing .env is normal when the URL comes from the shell,
    // which is how a scratch database is pointed at; the connect below fails
    // loudly either way if it is unset.
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;
    Ok(sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&config.pipeline_database_url)
        .await?)
}

async fn scenario(pool: &PgPool, tag: &str) -> TestResult<Uuid> {
    let (id, _code) = insert_scenario(
        pool,
        &format!("war room {tag}"),
        "offense",
        "draft",
        &test_slug(tag),
        None,
        None,
        &serde_json::json!({ "schema_v": 1 }),
    )
    .await?;
    Ok(id)
}

async fn cleanup(pool: &PgPool, id: Uuid, tag: &str) -> TestResult<()> {
    // Answers RESTRICT their question, so they go first; everything else cascades.
    sqlx::query(
        "DELETE FROM practice_answers WHERE session_id IN \
         (SELECT id FROM practice_sessions WHERE scenario_id = $1)",
    )
    .bind(id)
    .execute(pool)
    .await?;
    assert_eq!(delete_scenario(pool, id, &test_slug(tag)).await?, 1);
    Ok(())
}

async fn question(pool: &PgPool, scenario_id: Uuid, side: &str, order: i32) -> TestResult<Uuid> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO practice_questions \
         (scenario_id, side, kind, text, source_kind, sort_order, created_by) \
         VALUES ($1, $2, $3, $4, 'manual', $5, 'test') RETURNING id",
    )
    .bind(scenario_id)
    .bind(side)
    .bind(if side == "george" { "cross" } else { "direct" })
    .bind(format!("question {order}"))
    .bind(order)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

async fn hide(pool: &PgPool, question_id: Uuid) -> TestResult<()> {
    sqlx::query(
        "UPDATE practice_questions SET hidden_at = NOW(), hidden_by = 'chuck' WHERE id = $1",
    )
    .bind(question_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn answer(
    pool: &PgPool,
    scenario_id: Uuid,
    question_id: Uuid,
    at: DateTime<Utc>,
) -> TestResult<()> {
    let session: (Uuid,) = sqlx::query_as(
        "INSERT INTO practice_sessions (scenario_id, who, user_id) \
         VALUES ($1, 'mixed', 'marie') RETURNING id",
    )
    .bind(scenario_id)
    .fetch_one(pool)
    .await?;
    sqlx::query(
        "INSERT INTO practice_answers \
         (session_id, question_id, answer_text, self_check, mark, answered_at) \
         VALUES ($1, $2, 'her words', '{}'::jsonb, 'fine', $3)",
    )
    .bind(session.0)
    .bind(question_id)
    .bind(at)
    .execute(pool)
    .await?;
    Ok(())
}

async fn change(
    pool: &PgPool,
    scenario_id: Uuid,
    question_id: Uuid,
    kind: &str,
    at: DateTime<Utc>,
) -> TestResult<()> {
    sqlx::query(
        "INSERT INTO practice_deck_changes \
         (scenario_id, question_id, change_kind, changed_by, changed_at) \
         VALUES ($1, $2, $3, 'chuck', $4)",
    )
    .bind(scenario_id)
    .bind(question_id)
    .bind(kind)
    .bind(at)
    .execute(pool)
    .await?;
    Ok(())
}

async fn changed(pool: &PgPool, id: Uuid) -> TestResult<i64> {
    let rows = changed_counts(pool, &[id]).await?;
    assert_eq!(rows.len(), 1, "one row per scenario asked for");
    Ok(rows[0].changed)
}

/// A scenario with NO deck, NO scan and NO augmentation gets a row of zeroes from
/// every unnest-first family, and no scan row.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn aggregate_returns_one_row_per_scenario_including_empty() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let (bare, full) = (
        scenario(&pool, "bare").await?,
        scenario(&pool, "full").await?,
    );
    question(&pool, full, "chuck", 1).await?;
    let ids = [bare, full];

    let deck = deck_counts(&pool, &ids).await?;
    let prep = prep_counts(&pool, &ids, HumanFactKind::WatchList.code()).await?;
    let changed_rows = changed_counts(&pool, &ids).await?;
    assert_eq!((deck.len(), prep.len(), changed_rows.len()), (2, 2, 2));

    let d = deck
        .iter()
        .find(|r| r.scenario_id == bare)
        .expect("bare has a deck row");
    assert_eq!(
        (d.questions, d.answered, d.chuck_total, d.defense_total),
        (0, 0, 0, 0)
    );
    assert_eq!(d.built_on, None);
    let p = prep
        .iter()
        .find(|r| r.scenario_id == bare)
        .expect("bare has a prep row");
    assert_eq!((p.talking_points, p.watch_items), (0, 0));
    assert!(
        last_scans(&pool, &ids).await?.is_empty(),
        "neither was ever scanned"
    );
    let f = deck
        .iter()
        .find(|r| r.scenario_id == full)
        .expect("full has a deck row");
    assert_eq!((f.questions, f.chuck_total), (1, 1));

    cleanup(&pool, bare, "bare").await?;
    cleanup(&pool, full, "full").await
}

/// A hidden question moves no count — and un-hiding it moves them (the mutation).
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn hidden_question_moves_no_count() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "hidden").await?;
    let now = Utc::now();
    let visible = question(&pool, s, "chuck", 1).await?;
    let secret = question(&pool, s, "george", 2).await?;
    answer(&pool, s, visible, now).await?;
    answer(&pool, s, secret, now).await?;
    change(&pool, s, secret, "reworded", now + Duration::minutes(5)).await?;

    let with_both = deck_counts(&pool, &[s]).await?.remove(0);
    assert_eq!(
        (
            with_both.questions,
            with_both.answered,
            with_both.defense_total
        ),
        (2, 2, 1)
    );
    assert_eq!(
        changed(&pool, s).await?,
        1,
        "the reworded question counts while visible"
    );

    hide(&pool, secret).await?;
    let hidden = deck_counts(&pool, &[s]).await?.remove(0);
    assert_eq!((hidden.questions, hidden.answered), (1, 1));
    assert_eq!((hidden.defense_total, hidden.defense_answered), (0, 0));
    assert_eq!(
        changed(&pool, s).await?,
        0,
        "a hidden question never counts"
    );

    cleanup(&pool, s, "hidden").await
}

/// She answered, then Chuck reworded it: 1.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn answered_then_reworded_counts() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "reworded_after").await?;
    let t0 = Utc::now();
    let q = question(&pool, s, "chuck", 1).await?;
    answer(&pool, s, q, t0).await?;
    change(&pool, s, q, "reworded", t0 + Duration::minutes(1)).await?;
    assert_eq!(changed(&pool, s).await?, 1);
    cleanup(&pool, s, "reworded_after").await
}

/// Chuck reworded it, then she answered it: 0.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn reworded_then_answered_does_not_count() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "reworded_before").await?;
    let t0 = Utc::now();
    let q = question(&pool, s, "chuck", 1).await?;
    change(&pool, s, q, "reworded", t0).await?;
    answer(&pool, s, q, t0 + Duration::minutes(1)).await?;
    assert_eq!(changed(&pool, s).await?, 0);
    cleanup(&pool, s, "reworded_before").await
}

/// A question added after her last answer counts; once she answers it, it does not.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn question_added_after_her_last_answer_counts() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "added_after").await?;
    let t0 = Utc::now();
    let old = question(&pool, s, "chuck", 1).await?;
    change(&pool, s, old, "added", t0).await?;
    answer(&pool, s, old, t0 + Duration::minutes(1)).await?;
    let new = question(&pool, s, "george", 2).await?;
    change(&pool, s, new, "added", t0 + Duration::minutes(2)).await?;
    assert_eq!(changed(&pool, s).await?, 1, "the new question counts");

    answer(&pool, s, new, t0 + Duration::minutes(3)).await?;
    assert_eq!(changed(&pool, s).await?, 0, "answering it clears it");
    cleanup(&pool, s, "added_after").await
}

/// **The regression GO v3 exists for.** Seventeen `added` rows, no answers: 0.
///
/// Under the v1/v2 rule (changes since her last ENDED sitting, which is always
/// `None` on the one-page deck) this scenario reads 17 and never clears.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn deck_she_has_never_answered_shows_zero() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "never_answered").await?;
    let t0 = Utc::now();
    for order in 1..=17 {
        let q = question(
            &pool,
            s,
            if order % 2 == 0 { "chuck" } else { "george" },
            order,
        )
        .await?;
        change(&pool, s, q, "added", t0).await?;
    }
    assert_eq!(changed(&pool, s).await?, 0);
    cleanup(&pool, s, "never_answered").await
}

/// A hidden question with a change after her last answer never counts.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn hidden_question_never_counts() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "hidden_change").await?;
    let t0 = Utc::now();
    let answered = question(&pool, s, "chuck", 1).await?;
    answer(&pool, s, answered, t0).await?;
    let q = question(&pool, s, "george", 2).await?;
    change(&pool, s, q, "added", t0 + Duration::minutes(1)).await?;
    assert_eq!(changed(&pool, s).await?, 1, "visible, it would count");
    hide(&pool, q).await?;
    change(&pool, s, q, "hidden", t0 + Duration::minutes(2)).await?;
    assert_eq!(changed(&pool, s).await?, 0);
    cleanup(&pool, s, "hidden_change").await
}
