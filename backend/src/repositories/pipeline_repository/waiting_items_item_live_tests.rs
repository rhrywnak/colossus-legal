//! Live-database proofs of WHAT COUNTS AS AN ITEM (CC_TASK_FOR_YOU_v1).
//!
//! Split from `waiting_items_live_tests` under Rule 17, on a seam worth having:
//! that file answers "who is this waiting FOR", and this one answers the prior
//! question — is there anything waiting at all. A struck note, a hidden
//! question, a superseded answer and a deck change to a field nobody reads are
//! all events in the store that must reach nobody's list.
//!
//! `#[ignore]`d and run the same way, against a SCRATCH copy of the pipeline
//! schema, never against `colossus_legal_v2`.
//!
//! Each `(M)` test is mutation-proved in the report.

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::super::war_room_status::live_tests::{
    answer_by, cleanup, hide, pipeline_pool, question, scenario, TestResult,
};
use super::live_tests::{count, items, note, REVIEWER, WITNESS};
use super::WaitingSide;

async fn strike(pool: &PgPool, note_id: Uuid) -> TestResult<()> {
    sqlx::query("UPDATE practice_notes SET struck_at = now(), struck_by = 'chuck' WHERE id = $1")
        .bind(note_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// A change to the question's TEXT — the only kind that waits.
async fn change_by(
    pool: &PgPool,
    scenario_id: Uuid,
    question_id: Uuid,
    kind: &str,
    changed_by_id: &str,
    at: DateTime<Utc>,
) -> TestResult<Uuid> {
    change_field(
        pool,
        scenario_id,
        question_id,
        kind,
        Some("text"),
        changed_by_id,
        at,
    )
    .await
}

/// One deck change of `kind` to `field`, made by `changed_by_id`, at `at`.
///
/// The sibling helper in `war_room_status::live_tests` writes neither the login
/// nor the field; every rule under test here is about one or the other, so this
/// one writes both.
async fn change_field(
    pool: &PgPool,
    scenario_id: Uuid,
    question_id: Uuid,
    kind: &str,
    field: Option<&str>,
    changed_by_id: &str,
    at: DateTime<Utc>,
) -> TestResult<Uuid> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO practice_deck_changes \
         (scenario_id, question_id, change_kind, field, changed_by, changed_by_id, changed_at) \
         VALUES ($1, $2, $3, $4, $5, $5, $6) RETURNING id",
    )
    .bind(scenario_id)
    .bind(question_id)
    .bind(kind)
    .bind(field)
    .bind(changed_by_id)
    .bind(at)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// A struck note waits for nobody: striking IS the withdrawal.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_struck_note_waits_for_nobody() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "waiting_note_struck").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    let n = note(&pool, s, q, Some(REVIEWER), Utc::now()).await?;
    assert_eq!(count(&pool, s, WITNESS, WaitingSide::Witness).await?, 1);

    strike(&pool, n).await?;
    assert_eq!(
        count(&pool, s, WITNESS, WaitingSide::Witness).await?,
        0,
        "withdrawn"
    );
    assert_eq!(count(&pool, s, REVIEWER, WaitingSide::Reviewers).await?, 0);
    cleanup(&pool, s, "waiting_note_struck").await
}

/// (M) A change waits for the witness, in the two kinds that change what she was
/// asked — and never for whoever made it (ruled 2026-09-22, Q3).
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_change_waits_for_the_witness_and_never_for_its_author() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "waiting_change").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    let t0 = Utc::now() - Duration::hours(2);
    let reworded = change_by(&pool, s, q, "reworded", REVIEWER, t0).await?;

    assert_eq!(
        items(&pool, s, WITNESS, WaitingSide::Witness).await?,
        vec![("change".to_string(), reworded)],
        "a rewording is a question she has not read in that form"
    );
    assert_eq!(
        count(&pool, s, REVIEWER, WaitingSide::Reviewers).await?,
        0,
        "a change is not on the reviewers' side"
    );

    // Her OWN edit never waits for her. Before this rule the war room's amber
    // count read "Marie changed this — waiting for Marie".
    change_by(&pool, s, q, "edited", WITNESS, t0 + Duration::minutes(1)).await?;
    assert_eq!(
        count(&pool, s, WITNESS, WaitingSide::Witness).await?,
        1,
        "her own edit adds nothing to her list"
    );

    // And the bookkeeping kinds are not items at all.
    change_by(&pool, s, q, "moved", REVIEWER, t0 + Duration::minutes(2)).await?;
    assert_eq!(
        count(&pool, s, WITNESS, WaitingSide::Witness).await?,
        1,
        "moving a question does not change what it asks"
    );

    // (M) NOR IS AN EDIT TO ANOTHER FIELD. `stronger` is the model answer Chuck
    // keeps beside a question and `tactic` is his note on how it will be asked:
    // both are the reviewer's own craft, and neither changes the question she
    // reads. Measured on a copy of DEV, where fifty-one of the 146 deck changes
    // are exactly these two — every one of which this page claimed was a
    // reworded question until the clause was added.
    change_field(
        &pool,
        s,
        q,
        "edited",
        Some("stronger"),
        REVIEWER,
        t0 + Duration::minutes(3),
    )
    .await?;
    change_field(
        &pool,
        s,
        q,
        "edited",
        Some("tactic"),
        REVIEWER,
        t0 + Duration::minutes(4),
    )
    .await?;
    assert_eq!(
        count(&pool, s, WITNESS, WaitingSide::Witness).await?,
        1,
        "the reviewer's own notes on a question are not a change to it"
    );
    cleanup(&pool, s, "waiting_change").await
}

/// (M) A hidden question is not an item, and neither is a SUPERSEDED answer.
///
/// Re-answering replaces what is waiting rather than adding to it — the deck
/// would otherwise accumulate one waiting item per attempt, and a witness who
/// revised an answer three times would look like three questions of work.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn hidden_questions_and_superseded_answers_are_not_items() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "waiting_superseded").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    answer_by(&pool, s, q, Utc::now() - Duration::hours(2), Some(WITNESS)).await?;
    let current = answer_by(&pool, s, q, Utc::now(), Some(WITNESS)).await?;

    assert_eq!(
        items(&pool, s, REVIEWER, WaitingSide::Reviewers).await?,
        vec![("answer".to_string(), current)],
        "the current answer, once"
    );

    hide(&pool, q).await?;
    assert_eq!(
        count(&pool, s, REVIEWER, WaitingSide::Reviewers).await?,
        0,
        "a hidden question waits for nobody"
    );
    cleanup(&pool, s, "waiting_superseded").await
}
