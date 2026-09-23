//! Notes, composed for a screen — pure, no I/O.
//!
//! CC_TASK_REVIEW_LOOP_v1 §4. The notes table and its repository were built on
//! 2026-08-19 and routed nowhere after the 2026-08-23 ruling retired them from the
//! interface; this task wires them back, for one purpose: Chuck writes on the
//! answer he is reading, and Marie reads it under her deck row.
//!
//! ## Everything arrives composed
//!
//! The same law the rest of the practice payload follows: the browser holds no
//! templates and no date format. A note's day, and the "struck 18 Sep" line under
//! a withdrawn one, are filled here from stored words in the case's own timezone.
//!
//! ## Which notes a deck row shows
//!
//! Notes on the question itself, and notes on its CURRENT answer. A note on a
//! superseded attempt stays in the table and on the answers page's history, but
//! it is not about what she would say today, so the row does not carry it
//! (GO v1 ruling 6 — the same rule the War Room pill counts by).

use uuid::Uuid;

use crate::domain::settings::Settings;
use crate::domain::wording_templates::render;
use crate::dto::practice_review::PracticeNoteDto;
use crate::repositories::pipeline_repository::practice_notes::NoteRecord;
use crate::services::practice_clock::local_day_month;

/// One stored note as every panel renders it.
///
/// `struck` is `Some("struck 18 Sep")` exactly when the note was withdrawn — one
/// field, so a note cannot render struck through without saying when.
pub fn note_dto(settings: &Settings, note: &NoteRecord) -> PracticeNoteDto {
    let zone = &settings.practice_read.case_timezone;
    PracticeNoteDto {
        id: note.id,
        question_id: note.question_id,
        answer_id: note.answer_id,
        author: note.author.clone(),
        text: note.text.clone(),
        when: local_day_month(note.created_at, zone),
        answers_note_id: note.answers_note_id,
        struck: note.struck_at.map(|at| {
            render(
                &settings.practice_wording.row.note_struck_template,
                // UNBRACED key: this repo's `render` supplies the braces itself.
                &[("when", &local_day_month(at, zone))],
            )
        }),
    }
}

/// The notes one deck row shows: on the question, or on its current answer.
///
/// `notes` is the whole scenario's list (one read for the page); this filters it
/// per row, preserving the read's oldest-first order.
///
/// ## Rust Learning: `Option<Uuid>` compared with `==`
///
/// `note.answer_id == current_answer` compares two `Option<Uuid>`s whole. With no
/// current answer (`None`), a note on an attempt (`Some(..)`) can never match —
/// which is right: a question with no standing answer has no current attempt to
/// be noted on. Question-level notes (`answer_id == None`) are taken by the first
/// arm regardless.
pub fn row_notes(
    settings: &Settings,
    notes: &[NoteRecord],
    question_id: Uuid,
    current_answer: Option<Uuid>,
) -> Vec<PracticeNoteDto> {
    notes
        .iter()
        .filter(|note| note.question_id == Some(question_id))
        .filter(|note| note.answer_id.is_none() || note.answer_id == current_answer)
        .map(|note| note_dto(settings, note))
        .collect()
}

#[cfg(test)]
#[path = "practice_note_view_tests.rs"]
mod tests;
