//! The review page: every attempt at one question, and the notes on each.
//!
//! Task B3. Read-only by Roman's ruling — nothing on this page edits an answer.
//! An answer is a moment; she answers again instead, and the attempts stack.
//!
//! ## Why the attempts are NUMBERED here and reversed here
//!
//! The repository returns them oldest first, because "attempt 1" must mean her
//! first attempt however the list is sorted. Numbering from the top of a
//! newest-first list would make attempt 1 change its meaning every time she
//! answered again. So: number in the order they happened, then reverse for a
//! page that says "newest first" out loud.

use crate::domain::settings::Settings;
use crate::domain::wording_templates::render;
use crate::dto::practice_review::{PracticeAttemptDto, PracticeNoteDto};
use crate::repositories::pipeline_repository::practice_notes::{AttemptRecord, NoteRecord};
use crate::services::practice_notes::note_dto;
use crate::services::practice_page::when;
// ONE clock format for the whole surface. It was declared twice — here and in
// `practice_status` — which is two places for a strftime string the settings
// store cannot validate to go wrong independently.
use crate::services::practice_status::CLOCK_FORMAT;

/// The stored word for one mark. The same three-way match the sheet makes, and
/// for the same reason: an `else` arm printed a `skipped` row as **fine** before
/// flow v1, which told a reader she answered a question she had set aside.
fn mark_word(settings: &Settings, mark: &str) -> String {
    let report = &settings.practice_report_wording;
    match mark {
        "repeat" => report.mark_repeat.clone(),
        "skipped" => settings.practice_wording.flow.mark_skipped.clone(),
        "fine" => report.mark_fine.clone(),
        other => other.to_string(),
    }
}

/// Which of the four boxes she ticked, as the labels she ticked them under.
///
/// ## Domain note: ticking none is not a fault
///
/// The stored "none ticked" line stands where the list would be. It is a named
/// absence and deliberately neutral: a witness who read her own answer and found
/// nothing wrong with it has done the exercise, and a screen implying otherwise
/// would be grading her, which this product does not do.
fn boxes(settings: &Settings, self_check: &serde_json::Value) -> String {
    let w = &settings.practice_report_wording;
    let ticked: Vec<&str> = [
        ("only_asked", w.check_only_asked.as_str()),
        ("accepted_premise", w.check_accepted_premise.as_str()),
        ("explained_unasked", w.check_explained_unasked.as_str()),
        ("guessed", w.check_guessed.as_str()),
    ]
    .into_iter()
    .filter(|(key, _)| self_check.get(key).and_then(serde_json::Value::as_bool) == Some(true))
    .map(|(_, label)| label)
    .collect();

    if ticked.is_empty() {
        return settings.practice_wording.review.review_boxes_none.clone();
    }
    ticked.join(" · ")
}

/// What she said she would point to, or an empty list.
///
/// A value that will not decode withdraws the clause and is logged, exactly as
/// the sheet's does: this is a page about her own past work, and refusing to
/// render it over one malformed cell would cost her the whole question.
fn picked(record: &AttemptRecord) -> Vec<String> {
    let Some(value) = record.points_to.as_ref() else {
        return Vec::new();
    };
    match serde_json::from_value::<Vec<String>>(value.clone()) {
        Ok(picked) => picked,
        Err(e) => {
            tracing::error!(
                error = %e,
                answer = %record.id,
                "practice review: a stored points_to would not decode; the line is withdrawn"
            );
            Vec::new()
        }
    }
}

/// One attempt, composed.
///
/// Split from [`attempts`] so that function is the three lines it is about —
/// number, compose, reverse — and this is the twelve fields one row carries.
fn one_attempt(
    settings: &Settings,
    record: &AttemptRecord,
    number: usize,
    notes: &[NoteRecord],
) -> PracticeAttemptDto {
    let w = &settings.practice_wording.review;
    let report = &settings.practice_report_wording;
    PracticeAttemptDto {
        answer_id: record.id,
        heading: render(
            &w.review_attempt_template,
            &[
                ("n", &number.to_string()),
                (
                    "when",
                    &format!(
                        "{} {}",
                        when(record.answered_at),
                        record.answered_at.format(CLOCK_FORMAT)
                    ),
                ),
            ],
        ),
        mark: mark_word(settings, &record.mark),
        mark_key: record.mark.clone(),
        answer: record.answer_text.clone(),
        read_text: record.read_text.clone(),
        read_ok: record.read_ok,
        points_to: picked(record),
        detail: render(
            &w.review_detail_template,
            &[
                (
                    "help",
                    if record.help_opened {
                        report.help_opened.as_str()
                    } else {
                        report.help_none.as_str()
                    },
                ),
                ("boxes", &boxes(settings, &record.self_check)),
            ],
        ),
        notes: notes
            .iter()
            .filter(|n| n.answer_id == Some(record.id))
            .map(|n| note_dto(settings, n))
            .collect(),
    }
}

/// Every attempt at one question, numbered from her first and returned newest
/// first.
///
/// `notes` is the whole scenario's note list; the attempt-level ones are
/// partitioned out of it here rather than read per attempt, because the page is
/// one payload by design and a read per attempt would be a round trip per row.
pub fn attempts(
    settings: &Settings,
    records: &[AttemptRecord],
    notes: &[NoteRecord],
) -> Vec<PracticeAttemptDto> {
    let mut out: Vec<PracticeAttemptDto> = records
        .iter()
        .enumerate()
        .map(|(i, record)| one_attempt(settings, record, i + 1, notes))
        .collect();

    // Newest first, which is what the page says out loud. Reversed AFTER the
    // numbering, so attempt 1 is still her first.
    out.reverse();
    out
}

/// The review page's progress line: `Question 3 · review`.
pub fn progress(settings: &Settings, position: usize) -> String {
    render(
        &settings.practice_wording.review.review_progress_template,
        &[("n", &position.to_string())],
    )
}

/// The notes on the QUESTION itself — Roman's amendment 2.
///
/// Neither the scenario's (no `question_id`) nor an attempt's (`answer_id` set).
/// One filter rather than a second query, for the reason [`attempts`] gives.
pub fn question_notes(
    settings: &Settings,
    notes: &[NoteRecord],
    question_id: uuid::Uuid,
) -> Vec<PracticeNoteDto> {
    notes
        .iter()
        .filter(|n| n.question_id == Some(question_id) && n.answer_id.is_none())
        .map(|n| note_dto(settings, n))
        .collect()
}

#[cfg(test)]
#[path = "practice_review_tests.rs"]
mod tests;
