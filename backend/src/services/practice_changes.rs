//! What changed since she was last here, in plain words.
//!
//! Two readers, one vocabulary: the blue box on Marie's start card (task B2) and
//! the change list at the foot of Chuck's sheet. Both are composed here, so the
//! sentence a witness reads and the line a lawyer reads about the same edit
//! cannot drift apart.
//!
//! ## Why "changed" is a comparison and not a flag
//!
//! A flag on the question would have to be cleared by something, and the only
//! honest thing to clear it is Marie answering that question again — which is
//! already recorded, on the answer. So there is nothing to keep in step:
//! `changed` is `the newest change is newer than her newest answer`, computed
//! from two timestamps that exist for other reasons.
//!
//! ## Why the box names ONE editor and ONE day
//!
//! Marie needs to know whether to re-read the deck, not who did what when. The
//! newest change's author and day answer that; an audit trail on the start card
//! would be a screen she stops reading. The full list is one fold away, and
//! Chuck's sheet carries the day's changes in full.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

use crate::domain::settings::Settings;
use crate::domain::wording_templates::render;
use crate::dto::practice_review::PracticeChangedDto;
use crate::repositories::pipeline_repository::practice::PracticeQuestionRecord;
use crate::repositories::pipeline_repository::practice_editor::DeckChangeRecord;
use crate::services::practice_page::when;

/// The printed position of each question, by id: the number beside it on screen.
///
/// ## Domain note: the DECK's order, not the filtered list's
///
/// A change list saying "Q3 re-worded" has to mean the same Q3 whichever side
/// she is looking at, so the number counts the whole deck in `sort_order` — the
/// same order the seed writes and the editor's arrows rewrite.
fn positions(deck: &[PracticeQuestionRecord]) -> HashMap<Uuid, usize> {
    deck.iter()
        .enumerate()
        .map(|(i, q)| (q.id, i + 1))
        .collect()
}

/// One change, as a sentence.
///
/// ## Why an unknown kind renders its own name
///
/// The column's CHECK permits six values. A seventh added to a migration and not
/// to this match must be VISIBLE in the list — a witness seeing `superseded` and
/// asking what it means is a far better outcome than a change that silently
/// vanishes from the list of what changed.
fn line(settings: &Settings, change: &DeckChangeRecord, position: usize) -> String {
    let w = &settings.practice_wording.editor;
    let n = position.to_string();
    match change.change_kind.as_str() {
        "added" => render(
            &w.change_added_template,
            &[
                ("n", &n),
                // The side is stored as the change's `after_value` when a
                // question is added, because it is the one fact about a new
                // question the list needs and the question row may have moved
                // by the time this is read.
                ("side", change.after_value.as_deref().unwrap_or_default()),
            ],
        ),
        "reworded" => render(&w.change_reworded_template, &[("n", &n)]),
        "edited" => render(
            &w.change_edited_template,
            &[
                ("n", &n),
                ("field", change.field.as_deref().unwrap_or_default()),
            ],
        ),
        "moved" => render(&w.change_moved_template, &[("n", &n)]),
        "hidden" => render(&w.change_hidden_template, &[("n", &n)]),
        "unhidden" => render(&w.change_unhidden_template, &[("n", &n)]),
        other => other.to_string(),
    }
}

/// The blue box, or `None` when nothing has changed.
///
/// `None` withdraws it entirely: a box saying "0 questions changed" is a screen
/// telling a witness to re-read a deck that is exactly as she left it.
///
/// `new_notes` is counted by the caller from the same instant, and appended as
/// its own clause — a sitting where only notes arrived still deserves the box,
/// which is why the two counts are separate arguments rather than one total.
pub fn changed_box(
    settings: &Settings,
    deck: &[PracticeQuestionRecord],
    changes: &[DeckChangeRecord],
    new_notes: usize,
    newest_note_author: Option<&str>,
) -> Option<PracticeChangedDto> {
    if changes.is_empty() && new_notes == 0 {
        return None;
    }
    let w = &settings.practice_wording.editor;
    let at = positions(deck);

    // `changes` arrives newest-first, so the head is the newest — the one editor
    // and day the heading names.
    let newest = changes.first();
    let mut heading = render(
        &w.changed_heading_template,
        &[
            ("n", &changes.len().to_string()),
            (
                "who",
                newest
                    .map(|c| c.changed_by.as_str())
                    .or(newest_note_author)
                    .unwrap_or_default(),
            ),
            (
                "when",
                &newest
                    .map(|c| when(c.changed_at, &settings.practice_read.case_timezone))
                    .unwrap_or_default(),
            ),
        ],
    );
    if new_notes > 0 {
        // The joining separator is supplied here and never stored: a stored
        // value is trimmed, so a template could not carry its own leading space.
        heading.push_str(" · ");
        heading.push_str(&render(
            &w.changed_notes_template,
            &[
                ("n", &new_notes.to_string()),
                ("who", newest_note_author.unwrap_or_default()),
            ],
        ));
    }

    Some(PracticeChangedDto {
        heading,
        items: changes
            .iter()
            .map(|c| line(settings, c, at.get(&c.question_id).copied().unwrap_or(0)))
            .collect(),
    })
}

/// Which questions wear the `changed` badge.
///
/// A question is badged when it has changed and she has NOT answered it since.
/// Answering is what retires the badge, because answering is the thing the badge
/// is asking her to do.
pub fn badged(changes: &[DeckChangeRecord], last_answered: &[(Uuid, DateTime<Utc>)]) -> Vec<Uuid> {
    let mut out: Vec<Uuid> = Vec::new();
    for change in changes {
        let answered_since = last_answered
            .iter()
            .find(|(id, _)| *id == change.question_id)
            .is_some_and(|(_, at)| *at > change.changed_at);
        if !answered_since && !out.contains(&change.question_id) {
            out.push(change.question_id);
        }
    }
    out
}

/// The change list at the foot of Chuck's sheet: what was edited that day, and
/// by whom.
///
/// Oldest first, unlike Marie's box — he is reading a day's work in the order it
/// happened, she is being told what is new.
pub fn sheet_lines(
    settings: &Settings,
    deck: &[PracticeQuestionRecord],
    changes: &[DeckChangeRecord],
) -> Vec<String> {
    let w = &settings.practice_wording.editor;
    let at = positions(deck);
    changes
        .iter()
        .map(|c| {
            render(
                &w.sheet_change_item_template,
                &[
                    (
                        "what",
                        &line(settings, c, at.get(&c.question_id).copied().unwrap_or(0)),
                    ),
                    ("who", &c.changed_by),
                ],
            )
        })
        .collect()
}

#[cfg(test)]
#[path = "practice_changes_tests.rs"]
mod tests;
