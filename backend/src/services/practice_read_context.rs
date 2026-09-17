//! The three things the read learned to see on 2026-09-17: the scenario's attack,
//! a redirect's parent question, and the standing notes.
//!
//! CC_TASK_QUESTION_CHAT_ADDENDUM_v1. Split from [`super::practice_read_gather`]
//! so that module stays the §4 table it documents; the loaders here feed the same
//! [`super::practice_read_payload::ReadPayload`].
//!
//! ## The pure half is where the rules live
//!
//! [`parent_from`] and [`standing_notes`] decide what the model sees and are unit
//! tests; the async loaders only fetch rows and name a failure. "A struck note is
//! never sent" and "a non-redirect carries no parent" are therefore properties of
//! two functions a test can reach, not of a query a test cannot.

use uuid::Uuid;

use crate::repositories::pipeline_repository::practice::{list_deck, PracticeQuestionRecord};
use crate::repositories::pipeline_repository::practice_flow::current_answer_for;
use crate::repositories::pipeline_repository::practice_notes::{notes_for_question, NoteRecord};
use crate::repositories::pipeline_repository::scenario_store::get_scenario;
use crate::services::practice_clock::local_day_month;
use crate::services::practice_read_gather::PayloadFailure;
use crate::services::practice_read_payload::{Parent, PayloadNote};
use crate::state::AppState;

// STRUCTURAL: the `practice_questions.kind` CHECK vocabulary — `redirect` is what
// the column MEANS by a question that repairs a defense question. A new kind is a
// migration plus a code change.
const KIND_REDIRECT: &str = "redirect";

/// The question a redirect repairs, from the deck it lives in.
///
/// Not a redirect → [`Parent::NotARedirect`] (no section at all). A redirect
/// whose `follows_key` is missing, or names no question in this deck →
/// [`Parent::Unresolved`] (the named absence — the known deck_key=null defect of
/// hand-added questions, deliberately not fixed here).
pub fn parent_from(question: &PracticeQuestionRecord, deck: &[PracticeQuestionRecord]) -> Parent {
    if question.kind != KIND_REDIRECT {
        return Parent::NotARedirect;
    }
    let Some(follows) = question.follows_key.as_deref() else {
        return Parent::Unresolved;
    };
    deck.iter()
        .find(|q| q.deck_key.as_deref() == Some(follows))
        .map_or(Parent::Unresolved, |q| Parent::Repairs(q.text.clone()))
}

/// The notes the model is shown: UNSTRUCK, on the question or its CURRENT
/// answer, every author named and dated, newest first.
///
/// ## Domain note: why every author, and why never a struck note
///
/// Counsel leads, but Roman's guidance is guidance too, and the model sees who
/// said what (GO ruling 4). A struck note is guidance somebody withdrew — sending
/// it would let the model enforce advice the team no longer stands behind.
pub fn standing_notes(
    notes: &[NoteRecord],
    current_answer: Option<Uuid>,
    timezone: &str,
) -> Vec<PayloadNote> {
    let mut standing: Vec<&NoteRecord> = notes
        .iter()
        .filter(|n| n.struck_at.is_none())
        .filter(|n| n.answer_id.is_none() || n.answer_id == current_answer)
        .collect();
    // Newest first; the id breaks a same-instant tie so the order is stable.
    standing.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
    standing
        .into_iter()
        .map(|n| PayloadNote {
            author: n.author.clone(),
            when: local_day_month(n.created_at, timezone),
            text: n.text.clone(),
        })
        .collect()
}

/// The scenario's attack — its theme statement — or `None` when unwritten.
///
/// # Errors
/// [`PayloadFailure::Scenario`] when the scenario row cannot be read, or is gone.
pub async fn attack_of(
    state: &AppState,
    scenario_id: Uuid,
) -> Result<Option<String>, PayloadFailure> {
    let record = get_scenario(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| PayloadFailure::Scenario {
            scenario_id,
            source: anyhow::Error::new(e),
        })?
        .ok_or_else(|| PayloadFailure::Scenario {
            scenario_id,
            source: anyhow::anyhow!("no such scenario"),
        })?;
    Ok(record
        .theme_statement
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty()))
}

/// Load the deck only when the question IS a redirect, then resolve its parent.
///
/// # Errors
/// [`PayloadFailure::Deck`] when a redirect's deck cannot be read.
pub async fn parent_of(
    state: &AppState,
    scenario_id: Uuid,
    question: &PracticeQuestionRecord,
) -> Result<Parent, PayloadFailure> {
    if question.kind != KIND_REDIRECT {
        return Ok(Parent::NotARedirect);
    }
    let deck = list_deck(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| PayloadFailure::Deck {
            scenario_id,
            source: anyhow::Error::new(e),
        })?;
    Ok(parent_from(question, &deck))
}

/// Load the question's notes and its current answer, then filter them.
///
/// # Errors
/// [`PayloadFailure::Notes`] when either read fails.
pub async fn notes_of(
    state: &AppState,
    question_id: Uuid,
    timezone: &str,
) -> Result<Vec<PayloadNote>, PayloadFailure> {
    let failure =
        |e: crate::repositories::pipeline_repository::PipelineRepoError| PayloadFailure::Notes {
            question_id,
            source: anyhow::Error::new(e),
        };
    let current = current_answer_for(&state.pipeline_pool, question_id)
        .await
        .map_err(failure)?
        .map(|(id, _)| id);
    let notes = notes_for_question(&state.pipeline_pool, question_id)
        .await
        .map_err(failure)?;
    Ok(standing_notes(&notes, current, timezone))
}

#[cfg(test)]
#[path = "practice_read_context_tests.rs"]
mod tests;
