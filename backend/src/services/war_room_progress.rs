//! The War Room status card's numbers, folded — pure, no I/O.
//!
//! CC_TASK_WAR_ROOM_v1. The reads live in `api::war_room_progress` (the evidence
//! counts, which reuse the card queue's assembly) and
//! `pipeline_repository::war_room_status` / `review_cursor` (the four Postgres families). This
//! module turns what they return into one [`ScenarioProgress`] per scenario, and
//! is where "a scenario the reads forgot" becomes an error rather than a card of
//! zeroes.
//!
//! ## Why the fold is its own module
//!
//! Everything a test needs to assert about the card's numbers — the hidden
//! question moves nothing, the no-deck scenario is zeroes not blanks, the counts
//! equal the scenario page's — can be asserted here without a database or a
//! graph. The handler only does I/O and calls these.

use std::collections::HashMap;

use uuid::Uuid;

use crate::domain::fact_status::FactStatus;
use crate::dto::scenario_card::ScenarioCardsResponse;
use crate::dto::war_room_progress::{
    AnsweredSplit, DeckSummary, LastScan, MatrixLinked, ScenarioProgress,
};
use crate::repositories::pipeline_repository::review_cursor::AwaitingReviewRow;
use crate::repositories::pipeline_repository::war_room_status::{
    ChangedCountRow, DeckCountsRow, LastScanRow,
};
use crate::services::scenario_card_assembly::count_proposed;
use crate::services::scenario_human_links::link_counts;

/// Whose Done reviewing marks the review queue is counted against.
///
/// ## Domain note: ONE place names the reviewer BENCH
///
/// The `practice_reviewer_usernames` settings row — never a literal and never
/// the signed-in user (CC_TASK_SIMPLE_COUNTS_v1; a list since
/// CC_TASK_REVIEW_PAGE_v1). Both readers of the queue — the War Room's cards and
/// the deck's review bar — call this, so adding an attorney reaches both from
/// one Settings edit.
///
/// ## Rust Learning: returning `&[String]`, not `&Vec<String>`
///
/// A slice is what every caller actually needs (iterate it, bind it, ask whether
/// it contains a name), and `&Vec<T>` would force any future caller holding an
/// array or a slice of its own to build a `Vec` to call this. Rust's deref
/// coercion turns the `&Vec<String>` this function has into the `&[String]` it
/// returns with no code and no cost — which is why the API guidelines say to
/// take and return the slice.
pub fn review_queue_reviewer(settings: &crate::domain::settings::Settings) -> &[String] {
    &settings.practice_read.reviewer_usernames
}

/// The bench as screens print it: every reviewer's display name, joined.
///
/// ## Domain note: `{reviewer}` names WHOEVER owes the work
///
/// One reviewer prints one name, exactly as it always did. Two print
/// `Chuck · Roman`, because the review queue is one shared number and a pill
/// naming only the first of them would tell a reader the work belongs to
/// somebody it does not. The joiner is a stored row and carries no spaces of its
/// own — the store trims every value — so they are supplied here.
///
/// An EMPTY bench cannot reach this: `settings_practice::reviewer_bench`
/// refuses a snapshot whose lists are misaligned, and a blank row is refused by
/// `token_list_of` before that. So this never returns an empty string over a
/// store the boot check accepted.
pub fn reviewer_display_line(settings: &crate::domain::settings::Settings) -> String {
    let joiner = format!(" {} ", settings.practice_wording.review.name_joiner);
    settings.practice_read.reviewer_display_names.join(&joiner)
}

/// A read returned something the fold cannot turn into a card.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProgressError {
    /// A family read returned no row for a scenario it was asked about. Every
    /// family except the scan starts from `unnest(ids)`, so this is a query
    /// defect, never "that scenario has none" — and it must not render as zero.
    #[error("the {family} read returned no row for scenario {scenario_id}")]
    MissingRow {
        family: &'static str,
        scenario_id: Uuid,
    },
    /// A count did not fit the card's `u32` (negative, or absurdly large).
    #[error("{field} for scenario {scenario_id} is {value}, which is not a count")]
    NotACount {
        field: &'static str,
        scenario_id: Uuid,
        value: i64,
    },
}

/// The three evidence numbers, taken from one scenario's assembled cards.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EvidenceCounts {
    pub facts_included: usize,
    pub candidates_to_rule: usize,
    pub linked: usize,
    pub stuck: usize,
}

/// Count the card payload the scenario page is served — the ONE authority.
///
/// - `facts_included` — working-pool cards with status Included (set-aside cards
///   are Dropped by construction);
/// - `candidates_to_rule` — `count_proposed`, the same number the route attaches
///   to `proposal_source`;
/// - `linked` / `stuck` — `link_counts`, the same loop `link_progress` words.
pub fn evidence_counts(response: &ScenarioCardsResponse) -> EvidenceCounts {
    let (linked, stuck) = link_counts(response.pool.iter().chain(response.set_aside.iter()));
    EvidenceCounts {
        facts_included: response
            .pool
            .iter()
            .filter(|card| card.status == FactStatus::Included)
            .count(),
        candidates_to_rule: count_proposed(response),
        linked,
        stuck,
    }
}

/// Everything the Postgres families returned, by family.
pub struct FamilyRows {
    pub scans: Vec<LastScanRow>,
    pub deck: Vec<DeckCountsRow>,
    pub changed: Vec<ChangedCountRow>,
    /// The reviewer's queue per scenario (`review_cursor::awaiting_review`).
    pub awaiting: Vec<AwaitingReviewRow>,
}

/// One [`ScenarioProgress`] per scenario id, or the first thing that is wrong.
///
/// A scenario absent from `evidence` or from any unnest-first family is an error
/// (see [`ProgressError::MissingRow`]); absent from `scans` is `last_scan: None`.
///
/// # Errors
/// [`ProgressError`] naming the family or field and the scenario.
pub fn fold_progress(
    scenario_ids: &[Uuid],
    evidence: &HashMap<Uuid, EvidenceCounts>,
    rows: FamilyRows,
) -> Result<HashMap<Uuid, ScenarioProgress>, ProgressError> {
    // ## Rust Learning: collecting into a `HashMap` keyed by id
    //
    // Each family arrives as a `Vec` in whatever order Postgres chose. Turning each
    // into a map once makes every per-scenario lookup O(1) and — more to the point
    // — makes "no row for this id" a `None` the code has to handle, instead of an
    // index that silently points at the wrong scenario.
    let scans: HashMap<Uuid, LastScanRow> =
        rows.scans.into_iter().map(|r| (r.scenario_id, r)).collect();
    let deck: HashMap<Uuid, DeckCountsRow> =
        rows.deck.into_iter().map(|r| (r.scenario_id, r)).collect();
    let changed: HashMap<Uuid, i64> = rows
        .changed
        .into_iter()
        .map(|r| (r.scenario_id, r.changed))
        .collect();
    let awaiting: HashMap<Uuid, AwaitingReviewRow> = rows
        .awaiting
        .into_iter()
        .map(|r| (r.scenario_id, r))
        .collect();

    let mut out = HashMap::with_capacity(scenario_ids.len());
    for &id in scenario_ids {
        let missing = |family| ProgressError::MissingRow {
            family,
            scenario_id: id,
        };
        let one = OneScenario {
            id,
            evidence: evidence.get(&id).ok_or_else(|| missing("evidence"))?,
            scan: scans.get(&id),
            deck: deck.get(&id).ok_or_else(|| missing("deck"))?,
            changed: *changed.get(&id).ok_or_else(|| missing("changed"))?,
            awaiting: awaiting
                .get(&id)
                .ok_or_else(|| missing("awaiting review"))?,
        };
        out.insert(id, build_progress(&one)?);
    }
    Ok(out)
}

/// Everything read about ONE scenario, borrowed from the family maps.
///
/// ## Rust Learning: a struct of borrows (`&'a T`)
///
/// The rows stay owned by `fold_progress`'s maps; this only points at them, so
/// building a card copies nothing. The lifetime `'a` says the struct cannot
/// outlive the maps it points into — which the compiler checks for us.
struct OneScenario<'a> {
    id: Uuid,
    evidence: &'a EvidenceCounts,
    scan: Option<&'a LastScanRow>,
    deck: &'a DeckCountsRow,
    changed: i64,
    awaiting: &'a AwaitingReviewRow,
}

/// One scenario's card numbers, every count checked on the way in.
fn build_progress(one: &OneScenario<'_>) -> Result<ScenarioProgress, ProgressError> {
    let id = one.id;
    let count = |field, value: i64| to_count(id, field, value);
    let (ev, d) = (one.evidence, one.deck);
    Ok(ScenarioProgress {
        facts_included: count("facts_included", usize_to_i64(ev.facts_included))?,
        candidates_to_rule: count("candidates_to_rule", usize_to_i64(ev.candidates_to_rule))?,
        matrix_linked: MatrixLinked {
            linked: count("matrix_linked.linked", usize_to_i64(ev.linked))?,
            total: count("matrix_linked.total", usize_to_i64(ev.stuck))?,
        },
        last_scan: one.scan.map(|s| LastScan { when: s.started_at }),
        deck: DeckSummary {
            questions: count("deck.questions", d.questions)?,
            built_on: d.built_on,
        },
        answered: AnsweredSplit {
            total: count("answered.total", d.answered)?,
            of: count("answered.of", d.questions)?,
            chuck_answered: count("answered.chuck_answered", d.chuck_answered)?,
            chuck_total: count("answered.chuck_total", d.chuck_total)?,
            defense_answered: count("answered.defense_answered", d.defense_answered)?,
            defense_total: count("answered.defense_total", d.defense_total)?,
        },
        marie_changed: count("marie_changed", one.changed)?,
        awaiting_review: count("awaiting_review", one.awaiting.awaiting)?,
        oldest_awaiting_review: one.awaiting.oldest,
    })
}

/// A database count as the card's `u32`, or a named error.
///
/// ## Rust Learning: `TryFrom` instead of `as`
///
/// `value as u32` would turn `-1` into `4294967295` and print it on a card
/// without complaint. `u32::try_from` returns an `Err` for anything out of range,
/// which this maps to an error that names the field and the scenario.
fn to_count(scenario_id: Uuid, field: &'static str, value: i64) -> Result<u32, ProgressError> {
    u32::try_from(value).map_err(|_| ProgressError::NotACount {
        field,
        scenario_id,
        value,
    })
}

/// A `usize` count widened for [`to_count`]; saturates rather than wraps, and a
/// saturated value then fails `to_count` loudly.
fn usize_to_i64(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

#[cfg(test)]
#[path = "war_room_progress_tests.rs"]
mod tests;

#[cfg(test)]
mod reviewer_tests {
    use super::*;

    /// (M) The reviewer bench comes from the settings row, not a literal: change
    /// the row and the queue's reviewers follow.
    #[test]
    fn the_review_queue_reviewer_is_the_settings_row() {
        let mut settings = crate::domain::settings::Settings::for_test();
        assert_eq!(review_queue_reviewer(&settings), ["cpenzien".to_string()]);
        settings.practice_read.reviewer_usernames =
            vec!["roman".to_string(), "docmarie".to_string()];
        assert_eq!(
            review_queue_reviewer(&settings),
            ["roman".to_string(), "docmarie".to_string()]
        );
    }

    /// One reviewer prints one name — the behaviour every screen had before the
    /// bench existed, unchanged.
    #[test]
    fn one_reviewer_prints_one_name_with_no_joiner() {
        let settings = crate::domain::settings::Settings::for_test();
        assert_eq!(reviewer_display_line(&settings), "Chuck");
    }

    /// Two reviewers print both names, and the SPACES come from here.
    ///
    /// The joiner row is stored as a bare `·` because the store trims every
    /// value. A `join` using it directly would render `Chuck·Roman`, which is
    /// the defect this asserts against.
    #[test]
    fn two_reviewers_print_both_names_around_the_stored_joiner() {
        let mut settings = crate::domain::settings::Settings::for_test();
        settings.practice_read.reviewer_display_names =
            vec!["Chuck".to_string(), "Roman".to_string()];
        assert_eq!(reviewer_display_line(&settings), "Chuck \u{b7} Roman");
    }

    /// The joiner is the STORED row, not a literal: change it and the line
    /// follows, which is what makes the separator a Settings edit.
    #[test]
    fn the_joiner_comes_from_the_store() {
        let mut settings = crate::domain::settings::Settings::for_test();
        settings.practice_read.reviewer_display_names =
            vec!["Chuck".to_string(), "Roman".to_string()];
        settings.practice_wording.review.name_joiner = "and".to_string();
        assert_eq!(reviewer_display_line(&settings), "Chuck and Roman");
    }
}
