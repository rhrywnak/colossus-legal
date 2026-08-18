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
pub fn heading(settings: &Settings, answered: usize, repeats: usize) -> String {
    let w = &settings.practice_report_wording;
    let clause = if repeats == 0 {
        w.sheet_nothing_to_repeat.clone()
    } else {
        render(
            &w.sheet_repeat_clause_template,
            &[("n", &repeats.to_string())],
        )
    };
    render(
        &w.sheet_heading_template,
        &[("count", &answered.to_string()), ("repeat", &clause)],
    )
}

/// Render the whole sheet.
pub fn sheet_payload(
    settings: &Settings,
    code: &str,
    ended_at: chrono::DateTime<chrono::Utc>,
    rows: Vec<PracticeSheetRow>,
) -> PracticeSheetPayload {
    let w = &settings.practice_report_wording;
    let repeats = rows.iter().filter(|r| r.mark == "repeat").count();

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
            mark: if row.mark == "repeat" {
                w.mark_repeat.clone()
            } else {
                w.mark_fine.clone()
            },
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
        heading: heading(settings, rendered.len(), repeats),
        rows: rendered,
    }
}

#[cfg(test)]
#[path = "practice_sheet_tests.rs"]
mod tests;
