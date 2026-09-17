//! Live-database proofs for the three note reads the review loop added
//! (`notes_for_question`, `note_by_id`, `answer_home`).
//!
//! `#[ignore]`d, and run by hand against a SCRATCH copy of the pipeline schema,
//! for the reason `war_room_status_live_tests` gives — never `colossus_legal_v2`.

use chrono::Utc;
use uuid::Uuid;

use super::super::war_room_status::live_tests::{
    answer, cleanup, pipeline_pool, question, scenario, TestResult,
};
use super::{answer_home, insert_note, note_by_id, notes_for_question, NewNote};

/// `answer_home` names the answer's scenario and question; an unknown id is `None`.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn answer_home_reads_scenario_and_question() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "notes_home").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    let a = answer(&pool, s, q, Utc::now()).await?;
    assert_eq!(answer_home(&pool, a).await?, Some((s, q)));
    assert_eq!(answer_home(&pool, Uuid::new_v4()).await?, None);
    cleanup(&pool, s, "notes_home").await
}

/// A written note reads back by id with its fields; an unknown id is `None`.
/// `notes_for_question` returns the question's notes — both levels — oldest
/// first, and nothing from another question.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn notes_read_back_by_id_and_by_question() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "notes_read").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    let other = question(&pool, s, "george", 2).await?;
    let a = answer(&pool, s, q, Utc::now()).await?;
    let note = |question_id, answer_id, text| NewNote {
        scenario_id: s,
        question_id: Some(question_id),
        answer_id,
        author: "Chuck",
        author_id: "chuck",
        text,
    };
    let first = insert_note(&pool, &note(q, None, "on the question")).await?;
    let second = insert_note(&pool, &note(q, Some(a), "on the answer")).await?;
    insert_note(&pool, &note(other, None, "elsewhere")).await?;

    let stored = note_by_id(&pool, second)
        .await?
        .ok_or("the note reads back")?;
    assert_eq!(
        (stored.answer_id, stored.text.as_str()),
        (Some(a), "on the answer")
    );
    assert!(stored.struck_at.is_none());
    assert!(note_by_id(&pool, Uuid::new_v4()).await?.is_none());

    let ids: Vec<Uuid> = notes_for_question(&pool, q)
        .await?
        .iter()
        .map(|n| n.id)
        .collect();
    assert_eq!(ids, vec![first, second]);
    cleanup(&pool, s, "notes_read").await
}
