//! Live-database proofs that a REPLY behaves like any other item (L3).
//!
//! Split from its three siblings under Rule 17, on the seam those files already
//! draw: who a thing waits for, whether it is an item at all, and whether it
//! has been read. This one answers a fourth question the others cannot — what
//! a reply CARRIES, which is a column and a join rather than a predicate.
//!
//! `#[ignore]`d and run the same way, against a SCRATCH copy of the pipeline
//! schema, never against `colossus_legal_v2`.
//!
//! ## Why this needs a database at all
//!
//! The parent's words reach the row through `LEFT JOIN practice_notes parent ON
//! parent.id = n.answers_note_id`. A join lives inside a `&str`; nothing in the
//! compiler can tell whether it finds the right row, or any row, or the note
//! itself. The mutation that proves it is in the report: pointing the join at
//! `n.id` makes every reply quote ITSELF, and only this file notices.

use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::super::item_seen::{mark_seen, ItemRef};
use super::super::practice_notes::{insert_note, NewNote};
use super::super::war_room_status::live_tests::{
    answer_by, cleanup, pipeline_pool, question, scenario, TestResult,
};
use super::live_tests::{bench, items, note, REVIEWER, WITNESS};
use super::{waiting_items, WaitingQuery, WaitingScope, WaitingSide};

/// One reply, written the way the route writes it.
async fn reply(
    pool: &PgPool,
    scenario_id: Uuid,
    question_id: Uuid,
    parent: Uuid,
    author_id: &str,
    text: &str,
) -> TestResult<Uuid> {
    Ok(insert_note(
        pool,
        &NewNote {
            scenario_id,
            question_id: Some(question_id),
            answer_id: None,
            author: author_id,
            author_id,
            text,
            answers_note_id: Some(parent),
        },
    )
    .await?)
}

/// Every row waiting for `viewer`, with the text of whatever it answers.
async fn rows_with_parents(
    pool: &PgPool,
    s: Uuid,
    viewer: &str,
    side: WaitingSide,
) -> TestResult<Vec<(Uuid, Option<String>)>> {
    let reviewers = bench();
    let rows = waiting_items(
        pool,
        &WaitingQuery {
            scenario_ids: &[s],
            viewer,
            reviewers: &reviewers,
            side,
            scope: WaitingScope::Unseen,
        },
        None,
    )
    .await?;
    Ok(rows.into_iter().map(|r| (r.item_id, r.reply_to)).collect())
}

/// (M) Marie's reply to Chuck's note waits for the REVIEWERS, and carries the
/// words it answers.
///
/// This is journey (b) at the level of the store: she replies without changing
/// her answer, and something arrives on his list that says what it is about.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_reply_waits_for_the_other_side_and_quotes_what_it_answers() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "waiting_reply").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    // She answered yesterday, and he read it (no answer is waiting for him).
    let a = answer_by(&pool, s, q, Utc::now() - Duration::days(1), Some(WITNESS)).await?;
    mark_seen(&pool, REVIEWER, &[ItemRef::Answer(a)]).await?;
    let his = note(&pool, s, q, Some(REVIEWER), Utc::now() - Duration::hours(2)).await?;
    let hers = reply(&pool, s, q, his, WITNESS, "Yes, that is right.").await?;

    let waiting = rows_with_parents(&pool, s, REVIEWER, WaitingSide::Reviewers).await?;
    assert_eq!(
        waiting.len(),
        1,
        "exactly her reply waits for him: her answer is read and his own note is his"
    );
    assert_eq!(waiting[0].0, hers);
    assert_eq!(
        waiting[0].1.as_deref(),
        Some("look at this one again"),
        "the row carries the PARENT's words — see this module's header"
    );

    // And nothing changed on HER side: replying is not answering.
    assert!(
        items(&pool, s, WITNESS, WaitingSide::Witness).await?.len() == 1,
        "his note still waits for her; her own reply never does"
    );

    cleanup(&pool, s, "waiting_reply").await
}

/// A reply clears by being read, exactly like any other item.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn reading_a_reply_clears_it() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "waiting_reply_seen").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    let his = note(&pool, s, q, Some(REVIEWER), Utc::now() - Duration::hours(2)).await?;
    let hers = reply(&pool, s, q, his, WITNESS, "Yes, that is right.").await?;

    assert_eq!(
        items(&pool, s, REVIEWER, WaitingSide::Reviewers)
            .await?
            .len(),
        1
    );
    let marked = mark_seen(&pool, REVIEWER, &[ItemRef::Note(hers)]).await?;
    assert_eq!(marked, 1, "one seen row written for the reply");
    assert_eq!(
        items(&pool, s, REVIEWER, WaitingSide::Reviewers)
            .await?
            .len(),
        0,
        "a reply is an item: reading it clears it"
    );

    cleanup(&pool, s, "waiting_reply_seen").await
}

/// (M) A note that answers nothing carries NO parent text.
///
/// The anti-vacuity half of the first test: if the join returned a row for
/// every note, every note would quote something and the first assertion would
/// pass for the wrong reason.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_plain_note_carries_no_parent_text() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "waiting_plain_note").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    note(&pool, s, q, Some(REVIEWER), Utc::now() - Duration::hours(2)).await?;

    let waiting = rows_with_parents(&pool, s, WITNESS, WaitingSide::Witness).await?;
    assert_eq!(waiting.len(), 1);
    assert_eq!(
        waiting[0].1, None,
        "it answers nothing, so it quotes nothing"
    );

    cleanup(&pool, s, "waiting_plain_note").await
}

/// Striking a reply withdraws it, like striking any other note.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_struck_reply_waits_for_nobody() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "waiting_struck_reply").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    let his = note(&pool, s, q, Some(REVIEWER), Utc::now() - Duration::hours(2)).await?;
    let hers = reply(&pool, s, q, his, WITNESS, "Yes, that is right.").await?;

    sqlx::query("UPDATE practice_notes SET struck_at = now(), struck_by = $2 WHERE id = $1")
        .bind(hers)
        .bind(WITNESS)
        .execute(&pool)
        .await?;

    assert_eq!(
        items(&pool, s, REVIEWER, WaitingSide::Reviewers)
            .await?
            .len(),
        0,
        "a withdrawn reply waits for nobody"
    );

    cleanup(&pool, s, "waiting_struck_reply").await
}
