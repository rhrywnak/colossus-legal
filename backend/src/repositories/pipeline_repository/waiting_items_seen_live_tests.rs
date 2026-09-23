//! Live-database proofs of the SEEN record and the sweep (CC_TASK_FOR_YOU_v1 L0).
//!
//! The sibling of `waiting_items_live_tests`, split from it under Rule 17. That
//! file proves WHICH items wait for whom; this one proves what happens when
//! somebody reads them — that one seen row clears one item, that the sweep
//! marks only what it was handed, and that a second press changes nothing.
//!
//! `#[ignore]`d and run the same way, against a SCRATCH copy of the pipeline
//! schema, never against `colossus_legal_v2`.
//!
//! Each `(M)` test is mutation-proved in the report.

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use super::super::item_seen::{mark_scenario_notes_seen, mark_seen, seen_count, ItemRef};
use super::super::war_room_status::live_tests::{
    answer_by, cleanup, pipeline_pool, question, scenario, TestResult,
};
use super::live_tests::{bench, count, items, note, ADMIN, REVIEWER, WITNESS};
use super::{
    waiting_counts, waiting_items, waiting_total, WaitingQuery, WaitingScope, WaitingSide,
};

/// (M) Reading one item clears that item and nothing else.
///
/// The whole point of the finer grain: a watermark could only clear a whole
/// deck, so the one question Chuck wrote on could not be distinguished from the
/// 184 he did not.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn reading_one_item_clears_only_that_item() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "waiting_clear_one").await?;
    let t0 = Utc::now() - Duration::hours(3);
    let q1 = question(&pool, s, "chuck", 1).await?;
    let a1 = answer_by(&pool, s, q1, t0, Some(WITNESS)).await?;
    let q2 = question(&pool, s, "chuck", 2).await?;
    let a2 = answer_by(&pool, s, q2, t0 + Duration::minutes(5), Some(WITNESS)).await?;
    assert_eq!(count(&pool, s, REVIEWER, WaitingSide::Reviewers).await?, 2);

    assert_eq!(mark_seen(&pool, REVIEWER, &[ItemRef::Answer(a2)]).await?, 1);
    assert_eq!(
        items(&pool, s, REVIEWER, WaitingSide::Reviewers).await?,
        vec![("answer".to_string(), a1)],
        "the OTHER answer is still waiting"
    );
    assert_eq!(
        count(&pool, s, ADMIN, WaitingSide::Reviewers).await?,
        2,
        "and it is cleared for that reader only"
    );
    cleanup(&pool, s, "waiting_clear_one").await
}

/// (M) The sweep marks exactly what it was given — never what arrived after.
///
/// Ruling 1: "Done reviewing marks exactly the items that page showed." The page
/// sends the ids it rendered; an answer written between the render and the press
/// is not among them and must still be waiting afterwards. A watermark press
/// (`looked_at = now()`) would have swallowed it unread, which is the race this
/// design exists to close.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn the_sweep_never_marks_an_item_it_was_not_given() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "waiting_sweep").await?;
    let t0 = Utc::now() - Duration::hours(4);
    let q1 = question(&pool, s, "chuck", 1).await?;
    answer_by(&pool, s, q1, t0, Some(WITNESS)).await?;
    let q2 = question(&pool, s, "chuck", 2).await?;
    answer_by(&pool, s, q2, t0 + Duration::minutes(1), Some(WITNESS)).await?;

    // What the page rendered.
    let shown: Vec<ItemRef> = items(&pool, s, REVIEWER, WaitingSide::Reviewers)
        .await?
        .into_iter()
        .map(|(_, id)| ItemRef::Answer(id))
        .collect();
    assert_eq!(shown.len(), 2);

    // What arrived while he was reading it.
    let q3 = question(&pool, s, "chuck", 3).await?;
    let late = answer_by(&pool, s, q3, Utc::now(), Some(WITNESS)).await?;

    assert_eq!(mark_seen(&pool, REVIEWER, &shown).await?, 2);
    assert_eq!(
        items(&pool, s, REVIEWER, WaitingSide::Reviewers).await?,
        vec![("answer".to_string(), late)],
        "the answer written after the page loaded is still waiting"
    );
    cleanup(&pool, s, "waiting_sweep").await
}

/// (M) A second press keeps the FIRST moment, and writes no second row.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_second_press_keeps_the_first_moment() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "waiting_idempotent").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    let a = answer_by(&pool, s, q, Utc::now() - Duration::hours(1), Some(WITNESS)).await?;
    // A DELTA, not an absolute: this scratch database carries the back-fill's
    // rows too, and a test that asserted a total would be asserting the fixture
    // rather than the behaviour.
    let before = seen_count(&pool, REVIEWER).await?;

    assert_eq!(mark_seen(&pool, REVIEWER, &[ItemRef::Answer(a)]).await?, 1);
    let first: (DateTime<Utc>,) =
        sqlx::query_as("SELECT seen_at FROM practice_item_seen WHERE answer_id = $1")
            .bind(a)
            .fetch_one(&pool)
            .await?;

    assert_eq!(
        mark_seen(&pool, REVIEWER, &[ItemRef::Answer(a)]).await?,
        0,
        "the second press writes nothing, and says so"
    );
    let again: (DateTime<Utc>,) =
        sqlx::query_as("SELECT seen_at FROM practice_item_seen WHERE answer_id = $1")
            .bind(a)
            .fetch_one(&pool)
            .await?;
    assert_eq!(first.0, again.0, "the first reading is the one kept");
    assert_eq!(
        seen_count(&pool, REVIEWER).await? - before,
        1,
        "one row, not two"
    );
    cleanup(&pool, s, "waiting_idempotent").await
}

/// An empty sweep writes nothing and is not an error.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn an_empty_sweep_is_a_zero_and_not_a_failure() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    assert_eq!(mark_seen(&pool, REVIEWER, &[]).await?, 0);
    Ok(())
}

/// (M) A count comes back for every deck ASKED ABOUT, including the empty one,
/// and the oldest date is the oldest WAITING item's.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn every_deck_asked_about_gets_a_row_and_an_honest_oldest() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let busy = scenario(&pool, "waiting_counts_busy").await?;
    let quiet = scenario(&pool, "waiting_counts_quiet").await?;
    let oldest = Utc::now() - Duration::days(2);
    let q1 = question(&pool, busy, "chuck", 1).await?;
    let a1 = answer_by(&pool, busy, q1, oldest, Some(WITNESS)).await?;
    let q2 = question(&pool, busy, "chuck", 2).await?;
    answer_by(&pool, busy, q2, Utc::now(), Some(WITNESS)).await?;

    let reviewers = bench();
    let query = WaitingQuery {
        scenario_ids: &[busy, quiet],
        viewer: REVIEWER,
        reviewers: &reviewers,
        side: WaitingSide::Reviewers,
        scope: WaitingScope::Unseen,
    };
    let rows = waiting_counts(&pool, &query).await?;
    assert_eq!(rows.len(), 2, "one row per scenario asked for");
    let busy_row = rows
        .iter()
        .find(|r| r.scenario_id == busy)
        .ok_or("a row for the busy deck")?;
    let quiet_row = rows
        .iter()
        .find(|r| r.scenario_id == quiet)
        .ok_or("a row for the quiet deck")?;
    assert_eq!(busy_row.waiting, 2);
    assert_eq!(quiet_row.waiting, 0);
    assert_eq!(quiet_row.oldest, None, "no date when nothing waits");
    let stamped = busy_row.oldest.ok_or("an oldest while items wait")?;
    assert!(
        (stamped - oldest).num_milliseconds().abs() < 1,
        "oldest = {stamped}"
    );
    assert_eq!(waiting_total(&pool, &query).await?, 2, "the badge's number");

    // Reading the older one moves the date forward to the one still waiting.
    mark_seen(&pool, REVIEWER, &[ItemRef::Answer(a1)]).await?;
    let after = waiting_counts(&pool, &query).await?;
    let busy_after = after
        .iter()
        .find(|r| r.scenario_id == busy)
        .ok_or("a row for the busy deck")?;
    assert_eq!(busy_after.waiting, 1);
    assert!(
        busy_after.oldest.ok_or("a date")? > stamped,
        "the oldest date follows the set, never a stored value"
    );
    cleanup(&pool, busy, "waiting_counts_busy").await?;
    cleanup(&pool, quiet, "waiting_counts_quiet").await
}

/// An id that names no item is REFUSED, not quietly recorded as read.
///
/// ## Domain note: why this is worth a database round trip
///
/// The sweep is the one write in this design, and its ids come from a page. An
/// id that matches no answer, note or change is either a stale page or a caller
/// inventing one — and the wrong answer to both is to write a row saying
/// somebody read a thing that does not exist. Nothing would ever match it
/// again, and nothing would ever say so. The three foreign keys are what make
/// that impossible, and this is the proof they are really there: `mark_seen`
/// returns `Err`, and no row is written.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn an_id_that_names_no_item_is_refused() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let before = seen_count(&pool, REVIEWER).await?;
    let invented = Uuid::new_v4();

    for item in [
        ItemRef::Answer(invented),
        ItemRef::Note(invented),
        ItemRef::Change(invented),
    ] {
        let outcome = mark_seen(&pool, REVIEWER, &[item]).await;
        assert!(
            outcome.is_err(),
            "an id naming no item was accepted: {item:?} -> {outcome:?}"
        );
    }
    assert_eq!(
        seen_count(&pool, REVIEWER).await?,
        before,
        "a refused sweep writes nothing"
    );
    Ok(())
}

/// A blank reader is REFUSED by the CHECK, not stored as a person called "".
///
/// `user_id` is the signed-in login. A blank one means the caller lost track of
/// who is asking — and a row under `''` would be read-state belonging to
/// nobody, invisible to every real reader and impossible to clear.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_blank_reader_is_refused() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "waiting_blank_reader").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    let a = answer_by(&pool, s, q, Utc::now() - Duration::hours(1), Some(WITNESS)).await?;

    for blank in ["", "   "] {
        let outcome = mark_seen(&pool, blank, &[ItemRef::Answer(a)]).await;
        assert!(
            outcome.is_err(),
            "a blank reader was accepted: {blank:?} -> {outcome:?}"
        );
    }
    let rows: (i64,) =
        sqlx::query_as("SELECT count(*) FROM practice_item_seen WHERE answer_id = $1")
            .bind(a)
            .fetch_one(&pool)
            .await?;
    assert_eq!(rows.0, 0, "nothing was written under a blank reader");
    cleanup(&pool, s, "waiting_blank_reader").await
}

/// (M) The EVERYTHING tab keeps a read item, and says when it was read.
///
/// The two tabs are one predicate with one clause different (`WaitingScope`).
/// What this proves is that the difference is exactly the read filter: the same
/// item, present in both lists before it is read and in one of them after —
/// carrying, in the wider list, the moment it was read.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn everything_keeps_what_unread_drops() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "waiting_everything").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    let a = answer_by(&pool, s, q, Utc::now() - Duration::hours(1), Some(WITNESS)).await?;
    let reviewers = bench();
    let ask = |scope| WaitingQuery {
        scenario_ids: std::slice::from_ref(&s),
        viewer: REVIEWER,
        reviewers: &reviewers,
        side: WaitingSide::Reviewers,
        scope,
    };

    assert_eq!(
        waiting_total(&pool, &ask(WaitingScope::Unseen)).await?,
        1,
        "unread before it is read"
    );
    assert_eq!(
        waiting_total(&pool, &ask(WaitingScope::Everything)).await?,
        1,
        "and present in everything"
    );

    mark_seen(&pool, REVIEWER, &[ItemRef::Answer(a)]).await?;
    assert_eq!(
        waiting_total(&pool, &ask(WaitingScope::Unseen)).await?,
        0,
        "read, so it has left the unread tab"
    );
    let rows = waiting_items(&pool, &ask(WaitingScope::Everything), None).await?;
    assert_eq!(rows.len(), 1, "and stayed in everything");
    assert!(
        rows[0].seen_at.is_some(),
        "the wider tab says WHEN it was read"
    );
    cleanup(&pool, s, "waiting_everything").await
}

/// A note about the WHOLE SCENARIO, stamped `at`. No question, by design.
async fn scenario_note(
    pool: &sqlx::PgPool,
    scenario_id: Uuid,
    author_id: &str,
    at: DateTime<Utc>,
) -> TestResult<Uuid> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO practice_notes (scenario_id, question_id, author, author_id, text, created_at) \
         VALUES ($1, NULL, $2, $2, 'about this whole deck', $3) RETURNING id",
    )
    .bind(scenario_id)
    .bind(author_id)
    .bind(at)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Has this person seen that note?
async fn note_is_seen(pool: &sqlx::PgPool, user_id: &str, note_id: Uuid) -> TestResult<bool> {
    let row: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM practice_item_seen WHERE user_id = $1 AND note_id = $2",
    )
    .bind(user_id)
    .bind(note_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0 > 0)
}

/// (M) THE DECK SWEEP CLEARS THE ONE ITEM NO ROW CAN NAME — and stops at the
/// moment the page was served.
///
/// A note with `question_id IS NULL` is about the whole deck: it appears on no
/// question row, opening a question never clears it, and its For You row opens
/// the deck rather than a question. Before this, nothing in the product could
/// ever mark it read (ruled 2026-09-22: the deck sweep does).
///
/// The CUTOFF is the half that matters. `served_at` is the server's own stamp
/// from the payload the page was drawn from, so a note written AFTER the page
/// was served is not swept — the same promise the ids keep for every other
/// kind, kept here by a moment because there is no id to send.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn the_deck_sweep_clears_scenario_notes_up_to_the_moment_it_was_served() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "waiting_scenario_notes").await?;
    let served_at = Utc::now();

    let early = scenario_note(&pool, s, WITNESS, served_at - Duration::hours(1)).await?;
    let late = scenario_note(&pool, s, WITNESS, served_at + Duration::hours(1)).await?;
    // A note ON A QUESTION, at the same early moment: this sweep must not touch
    // it. Its own row on the page names it, and the ids are what clear it.
    let q = question(&pool, s, "chuck", 1).await?;
    let on_question = note(&pool, s, q, Some(WITNESS), served_at - Duration::hours(1)).await?;

    let written = mark_scenario_notes_seen(&pool, REVIEWER, s, served_at).await?;
    assert_eq!(written, 1, "the early scenario note, and only it");
    assert!(note_is_seen(&pool, REVIEWER, early).await?, "swept");
    assert!(
        !note_is_seen(&pool, REVIEWER, late).await?,
        "written after the page was served — it is still waiting"
    );
    assert!(
        !note_is_seen(&pool, REVIEWER, on_question).await?,
        "a note on a QUESTION is named by a row and swept by its id, not by this"
    );

    // Idempotent: a second press writes nothing and says so.
    assert_eq!(
        mark_scenario_notes_seen(&pool, REVIEWER, s, served_at).await?,
        0,
        "a second press keeps the first reading"
    );

    // And a deck nobody wrote a scenario note on sweeps nothing, which is a
    // number rather than an error.
    let quiet = scenario(&pool, "waiting_scenario_notes_quiet").await?;
    assert_eq!(
        mark_scenario_notes_seen(&pool, REVIEWER, quiet, served_at).await?,
        0
    );
    cleanup(&pool, quiet, "waiting_scenario_notes_quiet").await?;
    cleanup(&pool, s, "waiting_scenario_notes").await
}
