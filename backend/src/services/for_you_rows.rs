//! One waiting item, turned into the three sentences a row shows.
//!
//! CC_TASK_FOR_YOU_v1 L1, from `FOR_YOU_MOCKUP_v1_2026-09-22.html` boards 1-3.
//!
//! ## Why the sentences are built HERE and not in the browser
//!
//! Every string a person reads on this page is a settings row, and a template
//! is only a settings row in a useful sense if the thing that FILLS it can see
//! the store. Composing server-side is what makes "Chuck · on your answer of
//! Mon 21 Sep" a sentence Roman can retune in Settings instead of a sentence
//! assembled from four fragments by a component. The same law
//! `wording_practice_row` states for the deck row.
//!
//! ## Domain note: the same item reads differently to the two sides
//!
//! A note Chuck leaves on Marie's answer is "on your answer of Mon 21 Sep" to
//! her and "note on her answer" to him. One event, two sentences, because a
//! byline is written to its reader. That is why [`RowVoice`] carries the side.

use chrono::{DateTime, NaiveDate, Utc};

use crate::domain::scenario_code::scenario_code;
use crate::domain::wording_for_you::ForYouWording;
use crate::domain::wording_templates::render;
use crate::dto::for_you::{ForYouDay, ForYouRowDto, ForYouSide};
use crate::repositories::pipeline_repository::waiting_items::WaitingItemRow;
use crate::services::practice_clock::{local_clock, local_date, local_day, local_stamp};

/// Everything the composer needs that is the same for every row on one page.
///
/// ## Rust Learning: a borrowed context struct
///
/// Seven values that never change while a page is being built, passed once
/// instead of seven times per row. The `'a` says this struct may not outlive
/// the settings snapshot it points into — which the compiler checks, so a
/// composer that tried to keep a row's words after the snapshot was replaced
/// would not build.
pub struct RowVoice<'a> {
    pub wording: &'a ForYouWording,
    /// `practice_case_timezone`. Every date on this page is read in it.
    pub timezone: &'a str,
    /// Which list is being served — see the module header.
    pub side: ForYouSide,
    /// `practice_reviewer_usernames`, in order.
    pub reviewer_logins: &'a [String],
    /// `practice_reviewer_display_names`, index-aligned with the logins.
    pub reviewer_names: &'a [String],
    /// `practice_witness_username`.
    pub witness_login: &'a str,
    /// Today, in the case's timezone. Passed in rather than read per row so
    /// that a page built across midnight groups all of its rows by one answer.
    pub today: NaiveDate,
}

impl RowVoice<'_> {
    /// What this page calls the person who wrote an item.
    ///
    /// ## Domain note: a login is never printed
    ///
    /// `cpenzien` is a database identifier. The reviewers' display names are an
    /// index-aligned settings row; the witness's and the unattributed case are
    /// stored strings of this block. The login itself is the LAST resort — a
    /// person who is neither a listed reviewer nor the witness has no stored
    /// name anywhere, and showing their login is better than showing nothing,
    /// because the row still says who to ask about it.
    pub fn display_name(&self, login: Option<&str>) -> String {
        let Some(login) = login else {
            return self.wording.unknown_author.clone();
        };
        if let Some(at) = self.reviewer_logins.iter().position(|l| l == login) {
            // Index-aligned by a boot check that refuses two lists of different
            // lengths — `get` rather than `[at]` anyway, because a panic on a
            // settings edit is the one failure this page must not have.
            if let Some(name) = self.reviewer_names.get(at) {
                return name.clone();
            }
        }
        if login == self.witness_login {
            return self.wording.witness_name.clone();
        }
        login.to_string()
    }

    /// Which day heading this moment belongs under, in the case's timezone.
    pub fn day_of(&self, at: DateTime<Utc>) -> ForYouDay {
        let day = local_day(at, self.timezone);
        if day == self.today {
            ForYouDay::Today
        } else if Some(day) == self.today.pred_opt() {
            ForYouDay::Yesterday
        } else {
            // Including a date in the FUTURE, which a clock skew between two
            // machines can produce. "Earlier" is wrong for it and "today" would
            // be a lie; the row is still listed, newest first, which is where a
            // future stamp sorts anyway.
            ForYouDay::Earlier
        }
    }

    /// The row's first line: which deck, and which question.
    fn deck_line(&self, row: &WaitingItemRow) -> String {
        let code = row
            .code_ordinal
            .map(scenario_code)
            // A scenario minted before codes existed has no `S-n`. Its name
            // alone still identifies it, and an empty code renders as nothing
            // rather than as `S-`.
            .unwrap_or_default();
        match row.question_text.as_deref() {
            Some(question) => render(
                &self.wording.deck_line_template,
                &[
                    ("code", &code),
                    ("deck", &row.scenario_name),
                    ("question", question),
                ],
            ),
            None => render(
                &self.wording.deck_line_no_question_template,
                &[("code", &code), ("deck", &row.scenario_name)],
            ),
        }
    }

    /// What the item says, under the template its kind calls for.
    ///
    /// A note is shown as written — it IS the sentence somebody wrote to be
    /// read. An answer and a rewording are quoted, because on this page they
    /// are being reported rather than spoken.
    fn body(&self, row: &WaitingItemRow) -> String {
        let text = row.body.as_deref().unwrap_or_default();
        // STRUCTURAL: `answer`, `note` and `change` are the schema's own
        // vocabulary — the three legs of the `items` CTE in `waiting_items`
        // emit exactly these, and renaming one is a code change and a data
        // migration made together. Not a deployment value: a store that spelled
        // them differently would be a different store.
        match row.kind.as_str() {
            "answer" => render(&self.wording.body_answer_template, &[("text", text)]),
            "change" => render(&self.wording.body_change_template, &[("text", text)]),
            // "note", and anything a future kind might be: shown verbatim. A
            // kind this build does not know is better rendered plainly than
            // wrapped in the wrong sentence.
            _ => text.to_string(),
        }
    }

    /// `{what}` — the clause that names what KIND of thing this row is (ruling Q1).
    fn what(&self, row: &WaitingItemRow) -> String {
        let w = self.wording;
        // STRUCTURAL: the same three schema values as in `body` above, for the
        // same reason. The SENTENCES they choose between are settings rows; the
        // discriminators are not.
        match (row.kind.as_str(), row.subject_at) {
            ("answer", _) => w.byline_answer.clone(),
            ("change", _) => w.byline_change.clone(),
            // A note on one ATTEMPT, to the person whose attempt it was.
            ("note", Some(answered_at)) if self.side == ForYouSide::Witness => render(
                &w.byline_note_on_answer_witness,
                &[("when", &local_date(answered_at, self.timezone))],
            ),
            ("note", Some(_)) => w.byline_note_on_answer_reviewer.clone(),
            ("note", None) => w.byline_note_on_question.clone(),
            (_, _) => w.byline_note_on_question.clone(),
        }
    }

    /// The small line under a row: who wrote it, what it is, and — on the
    /// Everything tab — when this reader last looked at it.
    fn byline(&self, row: &WaitingItemRow) -> String {
        let who = self.display_name(row.author.as_deref());
        let what = self.what(row);
        let line = render(
            &self.wording.byline_template,
            &[("who", &who), ("what", &what)],
        );
        match row.seen_at {
            // The space is supplied HERE, not stored: the settings store trims
            // every value, so a template cannot carry a leading one.
            Some(at) => {
                let suffix = render(
                    &self.wording.byline_read_suffix_template,
                    &[("when", &local_date(at, self.timezone))],
                );
                format!("{line} {suffix}")
            }
            None => line,
        }
    }

    /// One waiting item as the page shows it.
    pub fn compose(&self, row: &WaitingItemRow) -> ForYouRowDto {
        let day = self.day_of(row.at);
        ForYouRowDto {
            kind: row.kind.clone(),
            item_id: row.item_id,
            scenario_id: row.scenario_id,
            question_id: row.question_id,
            deck_line: self.deck_line(row),
            body: self.body(row),
            byline: self.byline(row),
            // Today needs no date — the heading above it already said so. An
            // older row carries its day, because "4:18 pm" on its own is a time
            // with no anchor.
            when: match day {
                ForYouDay::Today => local_clock(row.at, self.timezone),
                _ => local_stamp(row.at, self.timezone),
            },
            day,
            read: row.seen_at.is_some(),
        }
    }
}

#[cfg(test)]
#[path = "for_you_rows_tests.rs"]
mod tests;
