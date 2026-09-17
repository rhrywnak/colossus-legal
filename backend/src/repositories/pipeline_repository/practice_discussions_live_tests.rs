//! Live-database proofs for `practice_discussions` (CC_TASK_QUESTION_CHAT_v1).
//!
//! `#[ignore]`d, run by hand against a SCRATCH copy of the pipeline schema —
//! never `colossus_legal_v2`.

use chrono::{Duration, Utc};

use super::super::war_room_status::live_tests::{
    answer, cleanup, pipeline_pool, question, scenario, TestResult,
};
use super::{insert_turn, list_thread, model_turn_count, standing_answer, NewTurn, TurnRole};

fn user_turn<'a>(q: uuid::Uuid, text: &'a str) -> NewTurn<'a> {
    NewTurn {
        question_id: q,
        author_user_id: "docmarie",
        author_name: "Marie",
        role: TurnRole::User,
        model_id: None,
        text,
        input_tokens: None,
        output_tokens: None,
        ms: None,
    }
}

/// A thread round-trips oldest first, with authors, roles and the model stamped.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_thread_round_trips_in_order_with_its_stamps() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "discuss_roundtrip").await?;
    let q = question(&pool, s, "george", 1).await?;
    insert_turn(&pool, &user_turn(q, "How do I say this?")).await?;
    let reply = insert_turn(
        &pool,
        &NewTurn {
            author_name: "Claude Opus 5",
            role: TurnRole::Model,
            model_id: Some("claude-opus-5"),
            text: "Lead with the letter.",
            input_tokens: Some(2100),
            output_tokens: Some(180),
            ms: Some(4200),
            ..user_turn(q, "")
        },
    )
    .await?;
    assert_eq!(reply.model_id.as_deref(), Some("claude-opus-5"));

    let thread = list_thread(&pool, q).await?;
    let shape: Vec<(&str, &str, Option<&str>)> = thread
        .iter()
        .map(|t| {
            (
                t.role.as_str(),
                t.author_name.as_str(),
                t.model_id.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            ("user", "Marie", None),
            ("model", "Claude Opus 5", Some("claude-opus-5"))
        ]
    );
    assert_eq!(thread[1].input_tokens, Some(2100));
    assert_eq!(model_turn_count(&pool, q).await?, 1);
    cleanup(&pool, s, "discuss_roundtrip").await
}

/// The table refuses a model turn with no model, and a user turn with one.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_turn_must_carry_a_model_exactly_when_it_is_a_model_turn() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "discuss_check").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    let unnamed = NewTurn {
        role: TurnRole::Model,
        ..user_turn(q, "a reply")
    };
    assert!(insert_turn(&pool, &unnamed).await.is_err());
    let stamped_user = NewTurn {
        model_id: Some("claude-opus-5"),
        ..user_turn(q, "hi")
    };
    assert!(insert_turn(&pool, &stamped_user).await.is_err());
    cleanup(&pool, s, "discuss_check").await
}

/// The standing answer is the newest one, with its analysis.
#[tokio::test]
#[ignore = "needs a live pipeline database — point PIPELINE_DATABASE_URL at a scratch copy"]
async fn the_standing_answer_is_the_newest() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let s = scenario(&pool, "discuss_answer").await?;
    let q = question(&pool, s, "chuck", 1).await?;
    assert_eq!(standing_answer(&pool, q).await?, None);
    answer(&pool, s, q, Utc::now() - Duration::hours(1)).await?;
    let newest = answer(&pool, s, q, Utc::now()).await?;
    let standing = standing_answer(&pool, q).await?.ok_or("an answer stands")?;
    assert_eq!(standing.answer_id, newest);
    cleanup(&pool, s, "discuss_answer").await
}
