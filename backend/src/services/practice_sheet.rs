//! Composing Chuck's sheet from the log (screen S3).
//!
//! ## Why every cell is a WORD and not a boolean
//!
//! This sheet is printed and handed to a lawyer. "true" in a Help column is not
//! something anybody reads; "opened" is. The client receives cells, not flags to
//! translate — the same rule the rehearsal payload follows, and the reason the
//! browser holds none of these templates.
//!
//! ## Domain note: this is the only thing that leaves the screen
//!
//! FRE/MRE 612: what a witness reviews to refresh memory may be discoverable.
//! Until Chuck rules, the session is on screen only and the printed sheet goes to
//! him. There is no "print for Marie" anywhere in this build, and the absence is
//! deliberate rather than unbuilt.

use crate::domain::settings::Settings;
use crate::domain::wording_templates::render;
use crate::dto::practice::{PracticeSheetPayload, PracticeSheetRowDto};
use crate::repositories::pipeline_repository::practice::PracticeSheetRow;
use crate::repositories::pipeline_repository::practice_flow::FlaggedQuestionRecord;
use crate::services::practice_page::{tactic_name, when};

/// The "From" cell: which side asked, and whether it was a braid.
fn from_cell(settings: &Settings, row: &PracticeSheetRow) -> String {
    let w = &settings.practice_report_wording;
    match (row.side.as_str(), row.braid_rows.is_some()) {
        ("george", true) => w.sheet_from_george_braid.clone(),
        ("george", false) => w.sheet_from_george.clone(),
        // Anything that is not George is Chuck: the column has a CHECK with two
        // values, so there is no third case to invent a word for.
        _ => w.sheet_from_chuck.clone(),
    }
}

/// The heading: "Six questions. Two to repeat." — or "Nothing to repeat."
///
/// ## Why the repeat clause is a whole stored clause and not a number
///
/// "0 to repeat." is a sentence nobody writes by hand, and this heading is the
/// first thing Chuck reads. The zero case gets its own row for the same reason
/// the .392 count line's singular forms did: a surface a professional reads stops
/// being trusted the moment it produces a sentence a person would not.
/// Mockup v3 adds two clauses: `s skipped.` when she set questions aside, and
/// `Ended early.` when she stopped before the queue was exhausted.
///
/// ## Why they are APPENDED rather than folded into the heading template
///
/// The template is a stored row that has been correct since v0, and two more
/// placeholders in it would render `. .` on the common path where neither
/// clause applies. Appending only the clauses that are true keeps the ordinary
/// sentence exactly what Chuck already reads.
///
/// ## Domain note: `Ended early.` is a fact, not a fault
///
/// The sheet says what happened and grades nothing. A witness who stopped after
/// three questions did three questions; the clause is there so Chuck is not
/// misled into thinking the deck was exhausted, not to mark her down.
pub fn heading(
    settings: &Settings,
    answered: usize,
    repeats: usize,
    skipped: usize,
    ended_early: bool,
) -> String {
    let w = &settings.practice_report_wording;
    let flow = &settings.practice_wording.flow;
    let clause = if repeats == 0 {
        w.sheet_nothing_to_repeat.clone()
    } else {
        render(
            &w.sheet_repeat_clause_template,
            &[("n", &repeats.to_string())],
        )
    };
    let mut line = render(
        &w.sheet_heading_template,
        &[("count", &answered.to_string()), ("repeat", &clause)],
    );
    if skipped > 0 {
        // The joining space is supplied HERE and never stored: a stored value is
        // trimmed, so a template could not carry a leading one.
        line.push(' ');
        line.push_str(&render(
            &flow.sheet_skipped_clause_template,
            &[("s", &skipped.to_string())],
        ));
    }
    if ended_early {
        line.push(' ');
        line.push_str(&flow.sheet_ended_early_clause);
    }
    line
}

/// The word one row's mark renders as.
///
/// ## Why this is a match and not an `if repeat { … } else { … }`
///
/// It was that, and widening the stored vocabulary to three values in the flow
/// v1 migration turned the `else` into a silent liar: a `skipped` row printed on
/// Chuck's sheet as **fine**, which is the sheet telling him she answered a
/// question she had set aside. An exhaustive match over the three stored values
/// cannot do that, and the fallback arm now names what it saw.
fn mark_cell(settings: &Settings, mark: &str) -> String {
    let w = &settings.practice_report_wording;
    match mark {
        "repeat" => w.mark_repeat.clone(),
        "skipped" => settings.practice_wording.flow.mark_skipped.clone(),
        "fine" => w.mark_fine.clone(),
        // The CHECK constraint permits exactly three values, so this is
        // unreachable through the database. It renders the raw value rather
        // than guessing at "fine": a fourth mark added to the migration and not
        // to this match must be VISIBLE on the sheet, not disguised as a pass.
        other => other.to_string(),
    }
}

/// Render the whole sheet.
/// The label the flag list prints for one question — `G2`, `C4`.
///
/// ## Why it is composed here and not stored
///
/// It is a POSITION, not an identity: "the second of George's questions". The
/// row's identity is its uuid, which is no use to Roman reading a printed sheet.
/// Composing it from the side and the per-side ordinal means it stays correct
/// when the deck is re-seeded, and there is no second column to keep in step.
///
/// The letters come from the stored side pills' first character rather than
/// from a literal, so a deck that renamed a side cannot print a letter from a
/// vocabulary nobody uses.
fn flag_label(side: &str, ordinal: usize, settings: &Settings) -> String {
    let w = &settings.practice_report_wording;
    let word = match side {
        "chuck" => w.sheet_from_chuck.as_str(),
        "george" => w.sheet_from_george.as_str(),
        _ => "?",
    };
    let initial = word
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();
    format!("{initial}{ordinal}")
}

/// The flag list at the foot of the sheet, already composed into sentences.
///
/// Returns EMPTY when nothing is flagged, and the caller withdraws the whole
/// block — heading included. A heading over an empty list reads as a list that
/// failed to load.
pub fn flag_lines(settings: &Settings, flagged: &[FlaggedQuestionRecord]) -> Vec<String> {
    let flow = &settings.practice_wording.flow;
    // Per-SIDE ordinals: George's second question is G2 whether or not Chuck's
    // questions are interleaved with it in the deck's sort order.
    let mut seen_george = 0usize;
    let mut seen_chuck = 0usize;
    flagged
        .iter()
        .map(|q| {
            // An unknown side counts on George's tally rather than opening a
            // third: the LABEL above is what makes it visible, and a counter
            // per unrecognised value would renumber the real ones.
            let ordinal = match q.side.as_str() {
                "chuck" => {
                    seen_chuck += 1;
                    seen_chuck
                }
                _ => {
                    seen_george += 1;
                    seen_george
                }
            };
            render(
                &flow.flag_summary_item_template,
                &[
                    ("id", &flag_label(&q.side, ordinal, settings)),
                    ("question", &q.text),
                    // The query returns only rows carrying a note; the default is
                    // unreachable and is an empty string rather than a guess.
                    ("note", q.flag_note.as_deref().unwrap_or_default()),
                ],
            )
        })
        .collect()
}

pub fn sheet_payload(
    settings: &Settings,
    code: &str,
    ended_at: chrono::DateTime<chrono::Utc>,
    rows: Vec<PracticeSheetRow>,
    ended_early: bool,
    flagged: &[FlaggedQuestionRecord],
) -> PracticeSheetPayload {
    let w = &settings.practice_report_wording;
    let repeats = rows.iter().filter(|r| r.mark == "repeat").count();
    // Counted apart from `repeats`, and neither is counted as the other: a
    // question she set aside is not one she stumbled on.
    let skipped = rows.iter().filter(|r| r.mark == "skipped").count();
    let flow = &settings.practice_wording.flow;
    let lines = flag_lines(settings, flagged);

    let rendered = rows
        .iter()
        .enumerate()
        .map(|(i, row)| PracticeSheetRowDto {
            number: i + 1,
            from: from_cell(settings, row),
            // A question with no card gets the stored dash, not an empty cell —
            // an empty cell in a printed table reads as data that went missing.
            tactic: tactic_name(settings, row.tactic).unwrap_or_else(|| w.tactic_none.clone()),
            question: row.question.clone(),
            answer: row.answer_text.clone(),
            mark: mark_cell(settings, &row.mark),
            help_opened: row.help_opened,
            help: if row.help_opened {
                w.help_opened.clone()
            } else {
                w.help_none.clone()
            },
        })
        .collect::<Vec<_>>();

    PracticeSheetPayload {
        kicker: render(
            &w.sheet_kicker_template,
            &[("code", code), ("when", &when(ended_at))],
        ),
        heading: heading(settings, rendered.len(), repeats, skipped, ended_early),
        rows: rendered,
        flagged: lines,
        flagged_heading: if flagged.is_empty() {
            String::new()
        } else {
            flow.flag_summary_heading.clone()
        },
        flagged_hint: if flagged.is_empty() {
            String::new()
        } else {
            flow.flag_summary_hint.clone()
        },
    }
}

#[cfg(test)]
#[path = "practice_sheet_tests.rs"]
mod tests;
