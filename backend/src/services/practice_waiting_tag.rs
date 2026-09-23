//! The board-4 mark: what one question's row says is waiting on it.
//!
//! CC_TASK_FOR_YOU_v1 L3, from `FOR_YOU_MOCKUP_v1_2026-09-22.html` board 4.
//!
//! ## What it is for
//!
//! The For you page answers "what is waiting for me?" across the whole case.
//! Board 4 answers the same question one DECK at a time, on the question's own
//! row — so a witness working down her deck can see which questions have
//! something new on them without going to the list first, and a reviewer
//! reading the same deck sees which ones have been answered since he last
//! looked.
//!
//! ## Domain note: the mark is PER PERSON, like everything else in this task
//!
//! It is composed from the same unread items the list is built from
//! (`waiting_items`, scope `Unseen`, this viewer's side), so it disappears the
//! moment she opens the question — which is what `POST /practice/questions/:id/seen`
//! already does. Two readers of one deck see two different sets of marks, and
//! that is the point: there is no shared "read" any more (L2).
//!
//! ## Why the SENTENCE is built here and not in the browser
//!
//! The same law the rest of the practice payload follows. Which of four stored
//! templates a row wears depends on what is unread on it FOR THIS READER, which
//! the browser cannot know; and a component that assembled the sentence from
//! fragments would put four settings rows beyond Roman's reach.

use uuid::Uuid;

use crate::domain::settings::Settings;
use crate::domain::wording_templates::render;
use crate::repositories::pipeline_repository::waiting_items::WaitingItemRow;
use crate::services::for_you_rows::display_name_of;

/// The mark for one question's row, or `None` when nothing waits on it.
///
/// `waiting` is the whole deck's unread list, newest first — ONE read for the
/// payload, filtered per row here, exactly as `row_notes` filters the notes.
///
/// ## Domain note: the NEWEST item names the mark
///
/// A row with a note and two answers under it says what the most recent thing
/// was, and how many others are behind it. Naming the oldest would tell her
/// about something she has probably already been told about; naming all three
/// would be a paragraph on a row that has space for a phrase.
pub fn waiting_tag(
    settings: &Settings,
    waiting: &[WaitingItemRow],
    question_id: Uuid,
) -> Option<String> {
    let mine: Vec<&WaitingItemRow> = waiting
        .iter()
        .filter(|row| row.question_id == Some(question_id))
        .collect();
    let newest = mine.first()?;
    let w = &settings.practice_wording.row;
    let who = display_name_of(
        newest.author.as_deref(),
        &settings.practice_read.reviewer_usernames,
        &settings.practice_read.reviewer_display_names,
        &settings.practice_read.witness_username,
        &settings.for_you_wording,
    );
    // STRUCTURAL: `answer`, `note` and `change` are the schema's own vocabulary
    // — the three legs of the `items` CTE in `waiting_items` emit exactly these.
    // The SENTENCES they choose between are settings rows; the discriminators
    // are not.
    let mark = match newest.kind.as_str() {
        "answer" => render(&w.waiting_answer_template, &[("who", &who)]),
        // A rewording names nobody: what matters to the witness is that the
        // question is not the one she answered last time.
        "change" => w.waiting_change.clone(),
        // "note", and any kind a later build adds. A note is the common case
        // and the one board 4 draws; an unknown kind is better described as
        // "somebody wrote something" than wrapped in the wrong sentence.
        _ => render(&w.waiting_note_template, &[("who", &who)]),
    };
    // `{count}` is how many are BESIDES the one just named, so it is never
    // "+0 more" — the whole clause is withheld at one item.
    if mine.len() > 1 {
        let more = render(
            &w.waiting_more_template,
            &[("count", &(mine.len() - 1).to_string())],
        );
        // The joining space is supplied HERE: the settings store trims every
        // value, so a stored clause cannot carry a leading one.
        return Some(format!("{mark} {more}"));
    }
    Some(mark)
}

#[cfg(test)]
#[path = "practice_waiting_tag_tests.rs"]
mod tests;
