//! Assembling the practice page's one payload (screens S0–S2).
//!
//! ## What this reads, and the law it is obeying
//!
//! The scenario row, the scenario's talking points, the deck, and the session
//! log. Nothing else — no graph, no candidate pool, no included facts.
//! PRACTICE_SESSION_DESIGN_v1 §5 states that as a rule; this module is where it
//! is either kept or broken, because a payload is the only place an extra read
//! could hide.
//!
//! ## Why the last-session line is composed HERE and not in the browser
//!
//! The rehearsal payload's law, unchanged: every sentence arrives composed. The
//! client holds no templates, so a change to how that line reads is a Settings
//! edit, and a client that wanted to say something different would have to
//! invent the words itself.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::settings::Settings;
use crate::domain::wording_practice::PracticeWording;
use crate::domain::wording_templates::render;
use crate::dto::practice::{PracticeDeckPayload, PracticePointDto, PracticeQuestionDto};
use crate::dto::practice_wording::PracticeWordingDto;
use crate::repositories::pipeline_repository::practice::{
    LastSessionRecord, PracticePointRecord, PracticeQuestionRecord,
};

/// The date format the drill's two composed lines use: `Sun 16 Aug`.
///
// CONST (structural): not a per-deployment value, and deliberately not a settings
// row. Two reasons, and the second is the one that decides it.
//
// It is the shape of a date on ONE witness surface — the "last session" line and
// the sheet's eyebrow — chosen to read like something a person says out loud
// rather than like a timestamp. Nothing about it varies between DEV and PROD, and
// this case has one locale.
//
// And a strftime string is the one kind of stored value the settings store cannot
// validate. A typo does not fail: `%a %-d %v` renders "Sun 16 %v" onto Chuck's
// printed sheet, silently, with every other check green. The store's whole
// promise is that a value it accepts is a value that works, and it could not keep
// that promise for this one.
const SESSION_DATE_FORMAT: &str = "%a %-d %b";

/// The tactic's NAME for a card number, or `None` when the question carries none.
///
/// ## Domain note: an unknown card number yields no tag, and that is deliberate
///
/// The column's CHECK already refuses anything outside 1–7, so reaching the
/// `None` arm means the stored vocabulary is SHORTER than the deck's numbering —
/// a settings row someone trimmed. Rendering no tag is then the honest outcome:
/// the alternative, printing "card 6", tells Marie a number that means nothing to
/// her, and inventing a name would be worse than either.
pub fn tactic_name(settings: &Settings, tactic: Option<i16>) -> Option<String> {
    let card = tactic?;
    // best-effort: a NEGATIVE card number cannot exist — the column's CHECK
    // constrains it to 1–7 — so `try_from` failing means the store was edited
    // around the API. The honest rendering is the same as for a card the
    // vocabulary is too short to name: NO TAG. See this function's doc for why
    // that is better than printing the number or inventing a name.
    let index = usize::try_from(card).ok()?.checked_sub(1)?;
    settings.practice_read.tactic_names.get(index).cloned()
}

/// The tag a question wears: the card's name, plus the braid suffix when it
/// braids. `compound` becomes `compound · braid`.
fn tactic_tag(settings: &Settings, record: &PracticeQuestionRecord) -> Option<String> {
    let name = tactic_name(settings, record.tactic)?;
    match record.braid_rows.as_deref() {
        // The renderer supplies the joining space; the stored suffix cannot carry
        // a leading one, because the store trims every value.
        Some(_) => Some(format!(
            "{name} {}",
            settings.practice_wording.tactic_braid_suffix
        )),
        None => Some(name),
    }
}

/// One deck row, as the two screens receive it.
fn question_dto(settings: &Settings, record: PracticeQuestionRecord) -> PracticeQuestionDto {
    PracticeQuestionDto {
        tactic: tactic_tag(settings, &record),
        braid: record.braid_rows.is_some(),
        id: record.id,
        side: record.side,
        text: record.text,
        receipt: record.receipt,
        braid_rows: record.braid_rows,
        watch_for: record.watch_for,
        pair_said: record.pair_said,
        pair_admitted: record.pair_admitted,
        stronger: record.stronger,
        stronger_lean: record.stronger_lean,
    }
}

/// The start screen's one line about the last time she sat down.
///
/// ## Why "no session yet" is a stored sentence and not an empty string
///
/// An empty line reads as a page that failed to load something. A witness
/// opening this screen for the first time should be told she has not been here
/// before, in words — the honest-gap law applied to the smallest possible gap.
pub fn last_session_line(wording: &PracticeWording, last: Option<&LastSessionRecord>) -> String {
    let Some(record) = last else {
        return wording.no_last_session.clone();
    };
    // `render` takes UNBRACED keys and supplies the braces itself. A braced key
    // here would match nothing and ship a raw `{when}` to Marie's screen — which
    // this repo has done before, and which nothing in the build can warn about
    // because the string is well-typed either way. The test below asserts no `{`
    // survives, which is the only thing that catches it.
    render(
        &wording.last_session_template,
        &[
            ("when", &when(record.ended_at)),
            ("count", &record.answered.to_string()),
            ("repeat", &record.repeats.to_string()),
        ],
    )
}

/// A timestamp as this surface says it out loud.
pub fn when(at: DateTime<Utc>) -> String {
    at.format(SESSION_DATE_FORMAT).to_string()
}

/// Build the whole payload.
///
/// An empty `questions` is a LEGITIMATE state, not an error: the page shows the
/// stored "no practice deck yet — seed it" line. That is the S-6 case, and it is
/// why this function has no "not seeded" failure mode to return.
pub fn deck_payload(
    settings: &Settings,
    scenario_id: Uuid,
    code: String,
    title: String,
    deck: Vec<PracticeQuestionRecord>,
    points: Vec<PracticePointRecord>,
    last: Option<&LastSessionRecord>,
) -> PracticeDeckPayload {
    PracticeDeckPayload {
        scenario_id,
        code,
        title,
        questions: deck
            .into_iter()
            .map(|record| question_dto(settings, record))
            .collect(),
        points: points
            .into_iter()
            .map(|p| PracticePointDto {
                position: p.position,
                text: p.text,
                exhibit: p.exhibit,
            })
            .collect(),
        last_session_line: last_session_line(&settings.practice_wording, last),
        wording: PracticeWordingDto::from_blocks(
            &settings.practice_wording,
            &settings.practice_report_wording,
        ),
    }
}

#[cfg(test)]
#[path = "practice_page_tests.rs"]
mod tests;
