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
use crate::dto::practice::{
    OpenSessionDto, PracticeDeckPayload, PracticePointDto, PracticeQuestionDto,
};
use crate::dto::practice_wording::PracticeWordingDto;
use crate::repositories::pipeline_repository::practice::{
    LastSessionRecord, PracticePointReceipt, PracticePointRecord, PracticeQuestionRecord,
};
use crate::repositories::pipeline_repository::practice_flow::{
    CurrentAnswerRecord, OpenSessionRecord,
};
use crate::services::practice_status::open_session_detail;

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
    // A NEGATIVE card number cannot exist — the column's CHECK constrains it to
    // 1–7 — so `try_from` failing means the store was edited around the API. The
    // honest rendering is the same as for a card the vocabulary is too short to
    // name: NO TAG. See this function's doc for why that is better than printing
    // the number or inventing a name.
    // best-effort: a card number the store cannot name yields no tag, never a guess.
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

/// One deck row for a screen that has no statuses and no badges to apply.
///
/// The review page shows ONE question and never a list, so neither its status
/// (which is about the row on the start card) nor its `changed` badge (which is
/// about re-reading the list) means anything there. Passing empty slices rather
/// than a second mapper keeps ONE function deciding what a question is on the
/// wire — which is what stops the review page and the start card disagreeing
/// about a redirect's tag.
pub fn question_dto_for(
    settings: &Settings,
    record: PracticeQuestionRecord,
) -> PracticeQuestionDto {
    question_dto(settings, &[], record)
}

/// One talking point with its receipt, for a caller outside this module.
pub fn point_dto(
    point: PracticePointRecord,
    receipts: &[PracticePointReceipt],
) -> PracticePointDto {
    PracticePointDto {
        position: point.position,
        exhibit: point_receipt(&point, receipts),
        text: point.text,
    }
}

/// `Answered on 22 Aug` for one timestamp.
///
/// ## Rust Learning: why the whole `Settings` and not just the template
///
/// It needs two stored values — the template and the case's timezone — from two
/// different blocks. Threading both in as parameters would put the CALLER in
/// charge of pairing them, and a caller that passed UTC with a Michigan template
/// would render a date four hours wrong with nothing to catch it. One argument,
/// one place the pairing is made.
fn answered_on_line(settings: &Settings, at: DateTime<Utc>) -> String {
    render(
        &settings.practice_wording.row.answered_on_template,
        // UNBRACED key: this repo's `render` matches `when`, not `{when}`. A
        // braced key here matches nothing and ships a raw `{when}` to screen —
        // which has happened, and which nothing in the build can warn about
        // because the string is well-typed either way.
        &[(
            "when",
            &crate::services::practice_clock::local_day_month(
                at,
                &settings.practice_read.case_timezone,
            ),
        )],
    )
}

/// One deck row, as the list receives it.
///
/// ## What a row no longer carries
///
/// `status` — "answered today · repeat · attempt 2" — and its raw mark are gone
/// from the wire, retired with the sitting apparatus by
/// CC_TASK_PRACTICE_ONE_PAGE §3, along with the `changed` badge and the box it
/// belonged to. `answered_on` is the ONE status a row has left, and the stored
/// footnote under the list is what tells a reader that its absence means "not
/// answered yet" rather than "failed to load".
fn question_dto(
    settings: &Settings,
    current: &[CurrentAnswerRecord],
    record: PracticeQuestionRecord,
) -> PracticeQuestionDto {
    // Composed here for the reason this module's header gives: the client holds
    // no templates and no date format, so how this line reads is a Settings
    // edit. `None` when nobody has answered — the row then renders NOTHING, and
    // an empty line under a question would read as a status that failed to load.
    let answered_on = current
        .iter()
        .find(|a| a.question_id == record.id)
        .map(|a| answered_on_line(settings, a.answered_at));
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
        flag_note: record.flag_note,
        hidden: record.hidden_at.is_some(),
        draft_by: record.draft_by,
        answered_on,
        kind: record.kind,
        deck_key: record.deck_key,
        follows_key: record.follows_key,
    }
}

/// The receipts the "I'd point to…" picker offers, in the order it lists them.
///
/// Her three points' receipts first — those are the exhibits her own case rests
/// on — then the documents her questions stand on, in deck order. De-duplicated
/// by exact text, because the same hearing page backs more than one question and
/// a list offering it twice is a list she stops reading.
///
/// ## Domain note: nothing here is derived from prose
///
/// Both halves are AUTHORED strings — the seeded point receipts and the deck's
/// own `source_line`. The alternative considered and rejected was cutting the
/// citation off the end of each `receipt` paragraph: it works on George's five
/// rows and produces "establishes point 1" on Chuck's, which is the machine
/// putting a phrase in front of a witness that nobody wrote.
fn picker_receipts(
    deck: &[PracticeQuestionRecord],
    seeded: &[PracticePointReceipt],
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |value: &str| {
        let value = value.trim();
        if !value.is_empty() && !out.iter().any(|held| held == value) {
            out.push(value.to_string());
        }
    };
    for receipt in seeded {
        push(&receipt.text);
    }
    for question in deck {
        if let Some(line) = question.source_line.as_deref() {
            push(line);
        }
    }
    out
}

/// The start screen's one line about the last time she sat down.
///
/// ## Why "no session yet" is a stored sentence and not an empty string
///
/// An empty line reads as a page that failed to load something. A witness
/// opening this screen for the first time should be told she has not been here
/// before, in words — the honest-gap law applied to the smallest possible gap.
pub fn last_session_line(
    wording: &PracticeWording,
    last: Option<&LastSessionRecord>,
    timezone: &str,
) -> String {
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
            ("when", &when(record.ended_at, timezone)),
            ("count", &record.answered.to_string()),
            ("repeat", &record.repeats.to_string()),
        ],
    )
}

/// A timestamp as this surface says it out loud, in the CASE's day.
///
/// A thin alias over `practice_clock::local_date` kept because six modules call
/// it by this name. The zone is threaded in rather than read from a global: a
/// formatter that reached for settings itself could not be tested against two
/// zones, and the DST test is the one that matters.
pub fn when(at: DateTime<Utc>, timezone: &str) -> String {
    crate::services::practice_clock::local_date(at, timezone)
}

/// The receipt one point shows, and the order of precedence behind it.
///
/// ## Domain note: the PAIRING wins, the seeded receipt is the stand-in
///
/// Three sources, in this order:
///
/// 1. `exhibit` — the phrase a human authored when they PAIRED this point to the
///    exhibit behind it (`response_item_fact_refs.note`). That pairing is the
///    record, so it wins wherever it exists.
/// 2. the seeded deck receipt (Roman's ruling, 2026-08-17), which exists because
///    the editor that authors (1) is v1 work and Marie needs the line on Tuesday.
/// 3. `None` — and the screen prints the stored named-absence sentence.
///
/// Putting the pairing first is what stops the stand-in becoming a second truth:
/// the v1 editor takes over by being used, with nothing to migrate and no stale
/// row left speaking over a human's own words.
///
/// ## Why this is `pub(crate)` since T1
///
/// The READ cites these receipts by key, and the model must cite the phrase Marie
/// is looking at. A second copy of this precedence in the read path would agree
/// with the screen today — **[measured 2026-08-20: no point on either live
/// scenario has a paired `exhibit`, so every receipt comes from the seeded
/// table]** — and diverge silently the moment Roman's backfill lands. One
/// function, two callers.
pub(crate) fn point_receipt(
    point: &PracticePointRecord,
    seeded: &[PracticePointReceipt],
) -> Option<String> {
    point.exhibit.clone().or_else(|| {
        seeded
            .iter()
            .find(|r| r.position == point.position)
            .map(|r| r.text.clone())
    })
}

/// Everything one practice page is built from, besides the settings snapshot.
///
/// ## Rust Learning: a parameter struct, and when it earns its place
///
/// This was eight positional arguments, three of them `String` and two of them
/// collections — the shape where a transposition compiles and puts the scenario's
/// title where its code belongs. Grouping them costs one struct and buys named
/// fields at the call site. Clippy's `too_many_arguments` is the mechanical
/// prompt; the readability is the actual reason.
///
/// It borrows nothing it does not have to: the three owned collections are moved
/// in and consumed, while `receipts` and `last` are read and left alone.
pub struct DeckSources<'a> {
    pub scenario_id: Uuid,
    /// `S-5` — the handle a human reads aloud.
    pub code: String,
    /// The accusation, as the page titles itself.
    pub title: String,
    pub deck: Vec<PracticeQuestionRecord>,
    pub points: Vec<PracticePointRecord>,
    /// The seeded stand-in receipts. See [`point_receipt`] for the precedence.
    pub receipts: &'a [PracticePointReceipt],
    pub last: Option<&'a LastSessionRecord>,
    /// The answer that stands for each question right now, for the row's
    /// `Answered on …` line. Scenario-wide and NOT scoped to the requester —
    /// see `practice_flow::current_answers` for why that is the whole point.
    pub current: &'a [CurrentAnswerRecord],
    /// The sitting she walked out of, if there is one.
    pub open: Option<&'a OpenSessionRecord>,
    /// What the editor's add form may attach a new question to.
    pub attach_options: Vec<crate::dto::practice_review::PracticeAttachOptionDto>,
}

/// Build the whole payload.
///
/// An empty `questions` is a LEGITIMATE state, not an error: the page shows the
/// stored "no practice deck yet — seed it" line. That is the S-6 case, and it is
/// why this function has no "not seeded" failure mode to return.
pub fn deck_payload(settings: &Settings, sources: DeckSources<'_>) -> PracticeDeckPayload {
    let DeckSources {
        scenario_id,
        code,
        title,
        deck,
        points,
        receipts,
        last,
        current,
        open,
        attach_options,
    } = sources;

    let picker = picker_receipts(&deck, receipts);
    // Computed from the deck already in hand — no second query. `max` over an
    // empty deck is `None`, which is the honest answer for a scenario nobody has
    // seeded: there is no date on which nothing last changed.
    let deck_as_of = deck.iter().map(|q| q.updated_at).max();
    PracticeDeckPayload {
        deck_as_of,
        scenario_id,
        code,
        title,
        questions: deck
            .into_iter()
            .map(|record| question_dto(settings, current, record))
            .collect(),
        points: points
            .into_iter()
            .map(|p| PracticePointDto {
                position: p.position,
                exhibit: point_receipt(&p, receipts),
                text: p.text,
            })
            .collect(),
        last_session_line: last_session_line(
            &settings.practice_wording,
            last,
            &settings.practice_read.case_timezone,
        ),
        receipts: picker,
        attach_options,
        open_session: open.map(|record| OpenSessionDto {
            session_id: record.id,
            detail: open_session_detail(settings, record),
        }),
        wording: PracticeWordingDto::from_blocks(
            &settings.practice_wording,
            &settings.practice_report_wording,
        ),
    }
}

#[cfg(test)]
#[path = "practice_page_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "practice_picker_tests.rs"]
mod picker_tests;
