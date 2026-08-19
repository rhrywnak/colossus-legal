//! One note, as every panel renders it — and who is allowed to sign one.
//!
//! Small on purpose. The note itself is a stored sentence somebody typed and
//! this service must not touch it; what it composes is the two things AROUND it
//! that the browser holds no templates for — the day it was written, and the
//! "struck Tue 19 Aug" line under a note somebody has withdrawn.

use crate::domain::settings::Settings;
use crate::domain::wording_templates::render;
use crate::dto::practice_review::PracticeNoteDto;
use crate::repositories::pipeline_repository::practice_notes::NoteRecord;
use crate::services::practice_page::when;

/// One note, composed.
///
/// ## Why `struck` is one `Option<String>` and not a flag plus a date
///
/// Its presence is what tells the screen to strike the text through, and its
/// content is what says when. Two fields would allow a fifth state nobody wants:
/// a note rendered struck with no statement of when it was withdrawn, which
/// invites the reader to assume it never really was.
pub fn note_dto(settings: &Settings, record: &NoteRecord) -> PracticeNoteDto {
    let w = &settings.practice_wording.review;
    PracticeNoteDto {
        id: record.id,
        question_id: record.question_id,
        answer_id: record.answer_id,
        author: record.author.clone(),
        text: record.text.clone(),
        when: when(record.created_at),
        struck: record
            .struck_at
            .map(|at| render(&w.notes_struck_template, &[("when", &when(at))])),
    }
}

/// The notes on the SCENARIO — neither a question's nor an attempt's.
pub fn scenario_notes(settings: &Settings, notes: &[NoteRecord]) -> Vec<PracticeNoteDto> {
    notes
        .iter()
        .filter(|n| n.question_id.is_none())
        .map(|n| note_dto(settings, n))
        .collect()
}

/// How many notes have arrived since one instant, and who wrote the newest.
///
/// ## Domain note: STRUCK notes do not count
///
/// The count is on the start card beside "changed since your last sitting", and
/// it is asking her to go and read something. A note that was written and then
/// withdrawn since she was last here is not something waiting for her — it is
/// still readable, struck, in the panel, which is where a withdrawal belongs.
pub fn new_since(
    notes: &[NoteRecord],
    since: Option<chrono::DateTime<chrono::Utc>>,
) -> (usize, Option<&str>) {
    let fresh: Vec<&NoteRecord> = notes
        .iter()
        .filter(|n| n.struck_at.is_none())
        .filter(|n| since.is_none_or(|at| n.created_at > at))
        .collect();
    // The list arrives oldest first, so the LAST of the fresh ones is the newest.
    let newest = fresh.last().map(|n| n.author.as_str());
    (fresh.len(), newest)
}

/// Is this a name the store lets sign a note?
///
/// ## Why the vocabulary is stored and not a CHECK constraint
///
/// They are real people's names — case-specific data, which Rule 2 keeps out of
/// code. `practice_note_authors` holds them the way `practice_tactic_names`
/// holds the seven cards, and adding a fourth person is a Settings edit rather
/// than a migration plus a deploy.
///
/// The comparison is exact and NOT case-folded: the name is stored on the note
/// and printed beside it, so "chuck" and "Chuck" being the same author would put
/// two spellings of one person on one panel.
pub fn is_note_author(settings: &Settings, author: &str) -> bool {
    names(&settings.practice_wording.editor.note_authors).any(|name| name == author)
}

/// Is this a name the store lets EDIT the deck?
///
/// A shorter list than the note authors, and deliberately so: Marie answers the
/// deck, she does not edit it. Both lists are stored, so that ruling is Roman's
/// to change without a build.
pub fn is_editor(settings: &Settings, author: &str) -> bool {
    names(&settings.practice_wording.editor.editor_authors).any(|name| name == author)
}

/// Split a stored comma-separated vocabulary into its names.
///
/// ## Rust Learning: returning `impl Iterator` borrowed from an argument
///
/// The return type borrows `list`, so nothing is allocated — no `Vec`, no
/// `String` per name — and the compiler proves the caller cannot hold the
/// iterator past the settings snapshot it points into. That is the same
/// guarantee `ReadRules`' borrowed `fine_token` carries, expressed as a return
/// type instead of a struct field.
fn names(list: &str) -> impl Iterator<Item = &str> {
    list.split(',').map(str::trim).filter(|n| !n.is_empty())
}

/// The stored vocabulary, as the API's refusal message lists it.
pub fn author_list(settings: &Settings) -> String {
    names(&settings.practice_wording.editor.note_authors)
        .collect::<Vec<_>>()
        .join(", ")
}

/// The editors, as the API's refusal message lists them.
pub fn editor_list(settings: &Settings) -> String {
    names(&settings.practice_wording.editor.editor_authors)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
#[path = "practice_notes_tests.rs"]
mod tests;
