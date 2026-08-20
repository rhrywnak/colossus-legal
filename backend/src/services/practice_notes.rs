//! One note, as every panel renders it — and who is allowed to sign one.
//!
//! Small on purpose. The note itself is a stored sentence somebody typed and
//! this service must not touch it; what it composes is the two things AROUND it
//! that the browser holds no templates for — the day it was written, and the
//! "struck Tue 19 Aug" line under a note somebody has withdrawn.

use crate::auth::AuthUser;
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
        when: when(record.created_at, &settings.practice_read.case_timezone),
        struck: record.struck_at.map(|at| {
            render(
                &w.notes_struck_template,
                &[("when", &when(at, &settings.practice_read.case_timezone))],
            )
        }),
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

/// Who a write is attributed to: the stable id, and the name a screen prints.
///
/// ## Why there are no selectors any more
///
/// There used to be two — "Editing as" on the deck editor and an author picker
/// on every note — and behind them a stored allow-list of display names. The
/// premise was that this build has one shared login and therefore cannot know
/// who is acting. That premise was WRONG: Chuck and Marie have had logins since
/// March, and every request already arrives with an authenticated user. The
/// selectors were asking a question the server could already answer — and then,
/// worse, silently refusing to work until somebody answered it.
///
/// ## Rust Learning: returning a tuple struct's worth of data without the struct
///
/// Two `String`s that always travel together would normally earn a struct. They
/// do not here because every caller destructures them immediately into two
/// columns (`author` and `author_id`), and a struct would add a name to import
/// at eight call sites to save nothing at any of them. The ORDER is the risk —
/// both are strings — so the return is `(id, name)` in the same order the
/// columns are declared everywhere, and every call site binds them by name.
pub fn attribution(user: &AuthUser) -> (String, String) {
    // `display_name` is what the screen prints beside a note; `username` is what
    // identifies the person when somebody is renamed in Authentik. A display
    // name that is blank — possible, if Authentik has no name for an account —
    // falls back to the username rather than rendering an empty author.
    let name = if user.display_name.trim().is_empty() {
        user.username.clone()
    } else {
        user.display_name.clone()
    };
    (user.username.clone(), name)
}

#[cfg(test)]
#[path = "practice_notes_tests.rs"]
mod tests;
