//! What one deck row says about a question, and what the start card offers back.
//!
//! Two composed sentences, both of them about a PAST act rather than about the
//! sitting she is in: the status under a row (`answered today · repeat ·
//! attempt 2`) and the detail on the unfinished-session box (`· today 09:57 ·
//! George's side · 1 of 5 answered.`).
//!
//! ## Why they are composed here and not in the browser
//!
//! The practice payload's law, unchanged since v0: every sentence arrives
//! composed. The client holds no templates and no date format, so how a status
//! reads is a Settings edit — and a client that wanted to say something
//! different would have to invent the words itself.
//!
//! ## Why this is a module and not more of `practice_page`
//!
//! Rule 17: that module was at 224 non-comment lines before this task, and the
//! two functions here plus their date arithmetic would have carried it past 300.
//! The seam is honest as well as arithmetical — `practice_page` assembles the
//! payload's SHAPE, and this decides what two of its sentences SAY.

use chrono::{DateTime, Utc};

use crate::domain::settings::Settings;
use crate::domain::wording_templates::render;
use crate::repositories::pipeline_repository::practice_flow::{OpenSessionRecord, RowStatusRecord};

/// The time of day the unfinished line prints: `09:57`.
//
// STRUCTURAL: the same argument the session date format carries in
// `practice_page` — it is the shape of a clock on ONE witness surface, nothing
// about it varies between DEV and PROD, and a strftime string is the one kind of
// stored value the settings store cannot validate. A typo does not fail; it
// renders `09:%M` onto the screen with every other check green.
pub(crate) const CLOCK_FORMAT: &str = "%H:%M";

/// The word one stored mark renders as.
///
/// ## Why this is a match over all three and not "repeat or fine"
///
/// The same defect the sheet's mark cell had before flow v1: an `else` arm made
/// a `skipped` row print as **fine**, which is the screen telling her she
/// answered a question she had set aside. Here `skipped` does not even reach the
/// template — it has a sentence of its own — so the arm exists to make that
/// visible rather than to be used.
fn mark_word(settings: &Settings, mark: &str) -> String {
    let report = &settings.practice_report_wording;
    match mark {
        "repeat" => report.mark_repeat.clone(),
        "skipped" => settings.practice_wording.flow.mark_skipped.clone(),
        // Hidden from the deck while this sitting still had it queued. NOT
        // `skipped`: that is her act, and this was the editor's.
        "hidden" => settings
            .practice_wording
            .flow
            .mark_hidden_before_asked
            .clone(),
        "fine" => report.mark_fine.clone(),
        // Unreachable through the database — the column's CHECK permits three
        // values. It renders the raw value rather than guessing, so a fourth
        // mark added to a migration and not here is VISIBLE on the row.
        other => other.to_string(),
    }
}

/// The status under one deck row, or `None` when nobody has answered it.
///
/// ## Domain note: three sentences, because they are three different facts
///
/// `answered today · fine` is about tonight. `skipped today` is about a question
/// she was dealt and set aside — which is NOT the same as `Skip today` on the
/// start card, an act that writes no row at all. `last: Tue 18 Aug · repeat` is
/// about a sitting that is over, and the date is the point of it: a question
/// answered last week is not one she has done tonight, and a status that read
/// `answered · repeat` for both would hide exactly the difference she opened the
/// screen to see.
///
/// The attempt suffix is withdrawn at one attempt. "attempt 1" on every row is
/// noise, and the number only means something once it is above one.
pub fn row_status(settings: &Settings, record: &RowStatusRecord) -> String {
    let row = &settings.practice_wording.row;
    let mark = mark_word(settings, &record.mark);

    let mut line = if record.answered_today {
        if record.mark == "skipped" {
            row.skipped_today.clone()
        } else {
            render(&row.answered_today_template, &[("mark", &mark)])
        }
    } else {
        render(
            &row.earlier_template,
            &[
                ("when", &super::practice_page::when(record.answered_at)),
                ("mark", &mark),
            ],
        )
    };

    if record.attempts > 1 {
        // The joining space is supplied here and never stored: the store trims
        // every value, so a template could not carry a leading one.
        line.push(' ');
        line.push_str(&render(
            &row.attempt_suffix_template,
            &[("n", &record.attempts.to_string())],
        ));
    }
    line
}

// `same_day` used to live here and compared in UTC. It is gone: Postgres does
// the comparison now, in the case's own zone, and hands back a boolean. The
// reason is in `row_statuses` — Marie practises in the evening in Michigan, and
// a UTC day ended hers at 20:00 local.

/// The detail sentence on the unfinished-session box.
///
/// ## Why `{total}` can be honest about not knowing
///
/// The total is the STORED queue's length, and sessions opened before flow v1
/// carry no queue. Rather than printing `1 of 0 answered.` — which reads as a
/// bug and is a lie about her evening — the count is rendered as the stored
/// dash. The sheet's `Ended early.` clause takes the same position for the same
/// reason: the surface never claims a fact it cannot source.
pub fn open_session_detail(settings: &Settings, record: &OpenSessionRecord) -> String {
    let flow = &settings.practice_wording.flow;
    let total = match record.queue_len {
        Some(n) => n.to_string(),
        None => settings.practice_report_wording.tactic_none.clone(),
    };
    render(
        &flow.unfinished_detail_template,
        &[
            (
                "when",
                &started_at_phrase(settings, record.started_today, record.started_at),
            ),
            ("who", &who_word(settings, &record.who)),
            ("answered", &record.answered.to_string()),
            ("total", &total),
        ],
    )
}

/// `today 09:57`, or `Mon 18 Aug 09:57` for a sitting she left on another day.
fn started_at_phrase(settings: &Settings, today: bool, at: DateTime<Utc>) -> String {
    let clock = at.format(CLOCK_FORMAT).to_string();
    if today {
        format!(
            "{} {clock}",
            settings.practice_wording.row.unfinished_today_word
        )
    } else {
        format!("{} {clock}", super::practice_page::when(at))
    }
}

/// The side she chose, in the words the rest of the screen uses.
///
/// The two pills and the mixed choice's title, rather than three rows of their
/// own: the resume line names the same three things the choice buttons above it
/// name, and a second vocabulary for them is a second thing to keep in step.
fn who_word(settings: &Settings, who: &str) -> String {
    let w = &settings.practice_wording;
    match who {
        "george" => w.pill_george.clone(),
        "chuck" => w.pill_chuck.clone(),
        "mixed" => w.who_mixed_title.clone(),
        // The column's CHECK permits exactly three values. The raw value is
        // rendered rather than guessed at, so a fourth side added to a migration
        // and not here is visible on the screen instead of disguised as George's.
        other => other.to_string(),
    }
}

#[cfg(test)]
#[path = "practice_status_tests.rs"]
mod tests;
