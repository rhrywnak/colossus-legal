//! Tests for [`super`] — the seven blocks, and the laws they must not break.
//!
//! Two things are proved here above all others:
//!
//! 1. **The §10 exclusion law**, as amended 2026-08-06. The DTO's SHAPE is the
//!    real enforcement — it has no field for a verdict — but shape is a claim
//!    about today's struct. `the_payload_carries_nothing_it_is_excluded_from`
//!    fills every free-text slot with the excluded vocabulary's own words and
//!    scans the serialized bytes, so a leak smuggled inside a string is caught
//!    too, and ADDING a field that carries one fails here rather than in front of
//!    a witness.
//! 2. **The honest-gap law.** The counts never inflate, every absence is named,
//!    and the header a folded section keeps showing still carries its gap count.

use super::*;
use crate::domain::human_authored::HumanFactKind;
use crate::domain::settings::Settings;
use crate::dto::rehearsal::RehearsalWatchItem;
use crate::repositories::scenario_accusation_repository::RehearsalFactRow;
use crate::services::scenario_accusation::{derive, StoredJudgment};
use std::collections::HashSet;

// ── Fixtures ─────────────────────────────────────────────────────────────────

fn fact(id: &str, quote: &str) -> RehearsalFactRow {
    RehearsalFactRow {
        graph_node_id: id.to_string(),
        quote: Some(quote.to_string()),
        speaker: Some("George Phillips".to_string()),
        statement_type: Some("attorney_argument".to_string()),
        occurred_on: Some("2009-12-15".to_string()),
        document_id: Some("doc-hearing".to_string()),
        document_title: Some("Hearing to approve plan".to_string()),
        page: Some(24),
    }
}

fn facts(rows: Vec<RehearsalFactRow>) -> HashMap<String, RehearsalFactRow> {
    rows.into_iter()
        .map(|r| (r.graph_node_id.clone(), r))
        .collect()
}

fn instance(anchor: &str) -> StoredJudgment {
    StoredJudgment {
        kind: HumanFactKind::AccusationInstance,
        anchor_graph_node_id: anchor.to_string(),
        answers_graph_node_id: None,
    }
}

fn pairing(anchor: &str, answer: &str) -> StoredJudgment {
    StoredJudgment {
        kind: HumanFactKind::AnswerPairing,
        anchor_graph_node_id: anchor.to_string(),
        answers_graph_node_id: Some(answer.to_string()),
    }
}

fn included(ids: &[&str]) -> HashSet<String> {
    ids.iter().map(|s| (*s).to_string()).collect()
}

/// Render one scenario over a derived state.
fn render(
    judgments: &[StoredJudgment],
    included_ids: &[&str],
    rows: HashMap<String, RehearsalFactRow>,
    accusation_text: Option<&str>,
) -> RehearsalScenario {
    render_with(
        judgments,
        included_ids,
        rows,
        accusation_text,
        Authored::default(),
    )
}

/// Render one scenario, choosing what the store recorded about authorship.
///
/// Split from [`render`] so the twenty callers that do not care about the
/// attribution lines keep their four arguments, and the three that do can say
/// what the store holds without a fifth `Authored::default()` at every site.
fn render_with(
    judgments: &[StoredJudgment],
    included_ids: &[&str],
    rows: HashMap<String, RehearsalFactRow>,
    accusation_text: Option<&str>,
    authored: Authored<'_>,
) -> RehearsalScenario {
    let settings = Settings::for_test();
    let state = derive(judgments, &included(included_ids));
    render_scenario(ScenarioInput {
        // Task R3's three: the identity line's word, the foldable verbatim
        // attack, and the bears-on chips. Fixed here because these tests are
        // about the BLOCKS, and varying them would only add noise.
        direction_label: "We are answering this".to_string(),
        attack_text: Some("…the parties did not cooperate with each other.".to_string()),
        bears_on: vec!["A-41".to_string()],
        code: "S-2".to_string(),
        scenario_id: "3f1b0a9e-2c4d-4e5f-8a7b-6c5d4e3f2a1b".to_string(),
        title: "Refused to divide property amicably",
        what_this_is: Some("Whether Marie refused to divide the estate."),
        what_authored: authored,
        accusation_text,
        accusation_authored: authored,
        state: &state,
        facts: &rows,
        points: vec![RehearsalPoint {
            position: 1,
            text: "I sent a certified letter in November 2009.".to_string(),
            exhibit: None,
            exhibit_notice: "No exhibit paired yet".to_string(),
        }],
        watch_for: vec![RehearsalWatchItem {
            id: "8b2c1d0e-3f4a-4b5c-9d6e-7f8a9b0c1d2e".to_string(),
            text: "He will say she cannot work with anybody".to_string(),
        }],
        settings: &settings,
    })
}

// ── The exclusion law ────────────────────────────────────────────────────────

#[test]
fn the_payload_carries_nothing_it_is_excluded_from() {
    // Every free-text slot is filled with the excluded vocabulary's own words, so
    // the scan below is testing the STRUCTURE and not the fixture's politeness.
    let scenario = render(
        &[instance("ev-a"), pairing("ev-a", "ev-answer")],
        &["ev-a", "ev-answer"],
        facts(vec![
            fact("ev-a", "They refused."),
            fact("ev-answer", "We did not."),
        ]),
        Some("They say Marie refused to divide the property."),
    );
    let json = serde_json::to_string(&scenario).expect("the scenario serializes");

    // Each entry is a thing §10 forbids this mode from carrying, paired with WHY.
    // `document_id` and `page` are DELIBERATELY absent — see the module header
    // and the DTO's, and the ruling of 2026-08-06.
    //
    // ## `scenario_id` left this list on 2026-08-06 (task 2.11 C, ruling C1)
    //
    // It was banned as an "internal identifier" while this page was read-only.
    // Roman's editing ruling made the page a CALLER of the guarded write routes —
    // the accusation sentence, "What this is", the talking points and the watch
    // items are all edited here — and `/cases/:slug/scenarios/:scenario_id/…` is
    // the address those routes are reached at. A page that may edit needs the
    // address of the thing it edits.
    //
    // The line §10 actually draws is CONTENT, and every content ban below still
    // stands: a UUID renders nothing, says nothing about the evidence, and cannot
    // be read aloud. The alternative offered — pulling the trial-prep payload in
    // to look the id up by code — would have brought readiness VERDICTS into this
    // page's memory, which is §10-worse by a wide margin.
    //
    // `graph_node_id` STAYS banned, and that is the load-bearing half: marking a
    // statement and pairing an answer remain working-view work, so this surface
    // never needs to address a statement. The day it does, this comment is the
    // thing to argue with.
    for (banned, why) in [
        ("motivation", "strategy is not rehearsal material (§10)"),
        ("confidence", "no percentages, no bands (§10)"),
        ("verdict", "no verdicts on a witness surface (§10)"),
        (
            "tier",
            "how much a fact carries is curation vocabulary (§10)",
        ),
        ("sort_ordinal", "an internal ordering number (§10)"),
        (
            "graph_node_id",
            "this page never addresses a STATEMENT — marking and pairing are \
             working-view work (§10, and ruling C1's limit)",
        ),
        ("probative", "internal vocabulary (§10)"),
        ("corroborat", "internal graph vocabulary (§9 canon)"),
        (
            "rebut",
            "the machine never asserts rebuttal (§10, and the law)",
        ),
        ("%", "no percentages (§10)"),
        (
            "\"c-",
            "a candidate handle is working-view vocabulary (§10)",
        ),
    ] {
        assert!(
            !json.to_lowercase().contains(banned),
            "the rehearsal payload contains '{banned}' — {why}. Payload: {json}"
        );
    }
}

#[test]
fn the_source_a_witness_needs_is_carried() {
    // The other half of the same ruling, asserted so a future tightening of the
    // ban list cannot silently take the citation away again. The research is
    // explicit: a witness who cannot produce the source on the spot loses
    // credibility, and this label plus this link is how she produces it.
    let scenario = render(
        &[instance("ev-a")],
        &["ev-a"],
        facts(vec![fact("ev-a", "They refused.")]),
        Some("They say Marie refused."),
    );
    let row = &scenario.accusation.instances[0];

    assert_eq!(row.source.label, "Hearing to approve plan, p. 24");
    assert_eq!(
        row.source.href,
        "/documents/doc-hearing?page=24&tab=document"
    );
    assert_eq!(row.source.open_label, "Open");
}

#[test]
fn a_talking_point_never_gains_a_source() {
    // The ruling permits the citation on instances and answers ONLY. A talking
    // point is HER words; attaching a page to it drags the record into her mouth.
    let scenario = render(&[], &[], facts(vec![]), None);
    let json = serde_json::to_string(&scenario.points).expect("serializes");
    assert!(!json.contains("source"), "{json}");
    assert!(!json.contains("page"), "{json}");
}

// ── The counts never inflate ─────────────────────────────────────────────────

#[test]
fn an_instance_the_record_cannot_produce_is_named_but_never_counted() {
    // The ruling of 2026-08-06, and the six dead refs measured on S-2. It does not
    // count toward "said N times" — the page cannot claim what it cannot load —
    // but it always appears in the gap list AND in the header's gap count.
    let scenario = render(
        &[instance("ev-here"), instance("ev-gone")],
        &["ev-here", "ev-gone"],
        facts(vec![fact("ev-here", "They refused.")]),
        Some("They say Marie refused."),
    );

    assert_eq!(
        scenario.accusation.instances.len(),
        1,
        "only what loaded renders"
    );
    assert_eq!(
        scenario.accusation.count_line.as_deref(),
        Some("Said 1 times, in 1 documents."),
        "the count is what the page can produce, not what was marked"
    );
    let unavailable = scenario
        .accusation
        .gaps
        .iter()
        .find(|g| g.kind == GAP_INSTANCE_UNAVAILABLE)
        .expect("the unloadable instance is named as its own gap kind");
    // It is NOT rendered, so there is no row to jump to. A position here would
    // send a reader from the prep list to somebody else's statement — worse than
    // no link, because they would believe they were looking at the right one.
    assert_eq!(
        unavailable.position, None,
        "an instance the record cannot produce has no row on screen"
    );
    assert_eq!(
        scenario.accusation.gap_count,
        scenario.accusation.gaps.len()
    );
}

#[test]
fn a_statement_with_no_words_is_treated_as_unavailable_and_not_rendered_blank() {
    // An empty pair of quotation marks attributed to a named person reads as us
    // claiming they said nothing. Naming it as unloadable is the honest render.
    let mut empty = fact("ev-a", "");
    empty.quote = Some("   ".to_string());
    let scenario = render(&[instance("ev-a")], &["ev-a"], facts(vec![empty]), None);

    assert!(scenario.accusation.instances.is_empty());
    assert_eq!(scenario.accusation.gaps[0].kind, GAP_INSTANCE_UNAVAILABLE);
    assert_eq!(scenario.accusation.gaps[0].position, None);
}

#[test]
fn the_same_accusation_twice_in_one_document_counts_one_document() {
    // Roman's model turns on the SPREAD — five times in five DIFFERENT documents.
    // Two statements on one page of one hearing is one document, and reporting it
    // as two would overstate the pattern.
    let scenario = render(
        &[instance("ev-a"), instance("ev-b")],
        &["ev-a", "ev-b"],
        facts(vec![fact("ev-a", "They refused."), fact("ev-b", "Again.")]),
        Some("They say Marie refused."),
    );
    assert_eq!(
        scenario.accusation.count_line.as_deref(),
        Some("Said 2 times, in 1 documents.")
    );
}

#[test]
fn nothing_marked_shows_the_notice_and_no_count_line() {
    let scenario = render(&[], &[], facts(vec![]), Some("They say Marie refused."));
    assert_eq!(scenario.accusation.count_line, None);
    assert_eq!(
        scenario.accusation.no_instances_notice.as_deref(),
        Some("Nobody has marked any instances of this accusation yet.")
    );
}

#[test]
fn exactly_one_summary_line_is_present_in_every_state() {
    for (judgments, ids, rows) in [
        (vec![], vec![], facts(vec![])),
        (
            vec![instance("ev-a")],
            vec!["ev-a"],
            facts(vec![fact("ev-a", "They refused.")]),
        ),
        // Marked, but unloadable — the state where a naive implementation renders
        // both lines or neither.
        (vec![instance("ev-a")], vec!["ev-a"], facts(vec![])),
    ] {
        let scenario = render(&judgments, &ids, rows, None);
        assert!(
            scenario.accusation.count_line.is_some()
                ^ scenario.accusation.no_instances_notice.is_some(),
            "exactly one summary line: {:?} / {:?}",
            scenario.accusation.count_line,
            scenario.accusation.no_instances_notice
        );
    }
}

// ── The named gaps ───────────────────────────────────────────────────────────

#[test]
fn the_prep_list_names_who_when_and_where_and_never_a_handle() {
    // §10 keeps internal identifiers off this surface, so the gap cannot say
    // "C-14". Who said it, when, and where is how a witness finds the statement.
    let scenario = render(
        &[instance("ev-a")],
        &["ev-a"],
        facts(vec![fact("ev-a", "They refused.")]),
        Some("They say Marie refused."),
    );

    let gap = &scenario.accusation.gaps[0];
    assert_eq!(gap.kind, GAP_NO_ANSWER);
    assert_eq!(
        gap.message,
        "NO ANSWER PREPARED — George Phillips, 15 Dec 2009, Hearing to approve plan, p. 24"
    );
    assert!(!gap.message.contains("C-"), "{}", gap.message);
}

#[test]
fn an_undated_statement_still_names_its_gap_rather_than_leaving_a_hole() {
    let mut undated = fact("ev-a", "They refused.");
    undated.occurred_on = None;
    let scenario = render(&[instance("ev-a")], &["ev-a"], facts(vec![undated]), None);

    assert!(
        scenario.accusation.gaps[0]
            .message
            .contains("No date on this statement"),
        "{}",
        scenario.accusation.gaps[0].message
    );
    assert_eq!(
        scenario.accusation.instances[0].when_gap.as_deref(),
        Some("No date on this statement")
    );
    assert_eq!(scenario.accusation.instances[0].when, None);
}

#[test]
fn an_answer_that_left_reads_differently_from_one_never_paired() {
    // Different absences, different remedies: one needs an answer prepared, the
    // other needs the departed item back or the pairing withdrawn. A shared
    // sentence would send a human to the wrong one.
    let never = render(
        &[instance("ev-a")],
        &["ev-a"],
        facts(vec![fact("ev-a", "They refused.")]),
        None,
    );
    let departed = render(
        &[instance("ev-a"), pairing("ev-a", "ev-answer")],
        &["ev-a"],
        facts(vec![fact("ev-a", "They refused.")]),
        None,
    );

    assert_eq!(never.accusation.gaps[0].kind, GAP_NO_ANSWER);
    assert_eq!(departed.accusation.gaps[0].kind, GAP_ANSWER_REMOVED);
    assert_ne!(
        never.accusation.gaps[0].message,
        departed.accusation.gaps[0].message
    );
    assert!(departed.accusation.gaps[0]
        .message
        .contains("George Phillips"));
}

#[test]
fn a_pairing_whose_statement_has_left_is_kept_and_named() {
    // The Remove law: a human's decision never vanishes silently. There is no row
    // to hang it from, so it becomes a gap of its own.
    let scenario = render(
        &[pairing("ev-gone", "ev-answer")],
        &["ev-answer"],
        facts(vec![fact("ev-answer", "We did not.")]),
        None,
    );

    let removed = scenario
        .accusation
        .gaps
        .iter()
        .find(|g| g.kind == GAP_ACCUSATION_REMOVED)
        .expect("the Remove law's statement side is named");
    // This gap is about a statement that is not in the instance list at all —
    // there is no row to hang it from, so there is no row to jump to either.
    assert_eq!(
        removed.position, None,
        "a gap about a statement that left the scenario has no row"
    );
}

#[test]
fn an_answered_instance_produces_no_gap_and_carries_the_answer() {
    let scenario = render(
        &[instance("ev-a"), pairing("ev-a", "ev-answer")],
        &["ev-a", "ev-answer"],
        facts(vec![
            fact("ev-a", "They refused."),
            fact("ev-answer", "We did not."),
        ]),
        Some("They say Marie refused."),
    );

    assert!(scenario.accusation.gaps.is_empty());
    assert_eq!(scenario.accusation.gap_count, 0);
    let answer = scenario.accusation.instances[0]
        .answer
        .as_ref()
        .expect("the paired answer");
    assert_eq!(answer.quote, "We did not.");
    assert_eq!(answer.who, "George Phillips");
}

// ── The collapsed headers, which are the whole point of the collapse layer ────

#[test]
fn a_folded_section_still_says_how_many_gaps_are_waiting() {
    // The known hazard of collapsible sections is that content behind a fold gets
    // missed. This is the engineering answer, and the honest-gap law's form of it:
    // the header carries the gap count, and the header is visible either way.
    let scenario = render(
        &[instance("ev-a"), instance("ev-b")],
        &["ev-a", "ev-b"],
        facts(vec![fact("ev-a", "They refused."), fact("ev-b", "Again.")]),
        Some("They say Marie refused."),
    );

    assert_eq!(scenario.headers.accusation, "said 2 times · 2 gaps");
    assert_eq!(scenario.accusation.gap_count, 2);
}

#[test]
fn the_gap_count_travels_as_a_number_not_only_as_a_list() {
    // So a client cannot arrive at it by counting a list it might have filtered —
    // and so the header stays right even if the list is virtualized away.
    let scenario = render(
        &[instance("ev-a"), instance("ev-gone")],
        &["ev-a", "ev-gone"],
        facts(vec![fact("ev-a", "They refused.")]),
        None,
    );
    assert_eq!(
        scenario.accusation.gap_count,
        scenario.accusation.gaps.len()
    );
    assert!(scenario.headers.accusation.contains("2 gaps"));
}

#[test]
fn the_other_three_headers_carry_their_own_honest_counts() {
    let scenario = render(&[], &[], facts(vec![]), None);
    assert_eq!(scenario.headers.points, "1 of 3");
    assert_eq!(scenario.headers.watch_for, "1 to watch for");
    assert_eq!(scenario.headers.timeline, "0 dated items");
}

// ── The blocks that are simply absent ────────────────────────────────────────

#[test]
fn an_unwritten_accusation_is_named_and_never_stood_in_for() {
    // The beta.371 defect at its root. Nothing may fill this slot but a human's
    // sentence — least of all a verbatim quote from the record.
    let scenario = render(&[], &[], facts(vec![]), None);
    assert_eq!(scenario.accusation.text, None);
    assert_eq!(
        scenario.accusation.text_gap.as_deref(),
        Some("Nobody has written the accusation in plain words yet.")
    );
}

#[test]
fn an_authored_accusation_travels_verbatim_and_shows_no_gap() {
    let scenario = render(&[], &[], facts(vec![]), Some("They say Marie refused."));
    assert_eq!(
        scenario.accusation.text.as_deref(),
        Some("They say Marie refused.")
    );
    assert_eq!(scenario.accusation.text_gap, None);
}

#[test]
fn every_block_shows_its_content_or_its_gap_and_never_both() {
    let scenario = render(&[], &[], facts(vec![]), None);

    assert!(scenario.what_this_is.is_some() ^ scenario.what_this_is_gap.is_some());
    assert!(scenario.points.is_empty() == scenario.points_gap.is_some());
    assert!(scenario.watch_for.is_empty() == scenario.watch_for_gap.is_some());
}

#[test]
fn the_row_carries_a_first_line_for_the_collapsed_view() {
    // Composed on the server so the truncation rule is tested in one place; a
    // browser slicing prose would cut differently the moment two components
    // disagreed about the length.
    let long = "They refused every reasonable attempt to divide the property amicably, \
                and continued to refuse for the following eleven months.";
    let scenario = render(
        &[instance("ev-a")],
        &["ev-a"],
        facts(vec![fact("ev-a", long)]),
        None,
    );
    let row = &scenario.accusation.instances[0];

    assert_eq!(row.quote, long);
    assert!(row.quote_first_line.len() < row.quote.len());
    assert!(row.quote_first_line.ends_with('…'));
}

#[test]
fn the_row_positions_are_the_printed_numbers_and_start_at_one() {
    let scenario = render(
        &[instance("ev-a"), instance("ev-b")],
        &["ev-a", "ev-b"],
        facts(vec![fact("ev-a", "One."), fact("ev-b", "Two.")]),
        None,
    );
    assert_eq!(scenario.accusation.instances[0].position, 1);
    assert_eq!(scenario.accusation.instances[1].position, 2);
}

#[test]
fn the_positions_skip_nothing_when_a_statement_could_not_be_loaded() {
    // The rows a human reads are numbered 1, 2, 3 — not 1, 3 with a silent hole
    // where an unloadable statement was. The absence is in the gap list, where it
    // can be acted on, not in a gap in the numbering, where it cannot.
    let scenario = render(
        &[instance("ev-a"), instance("ev-gone"), instance("ev-c")],
        &["ev-a", "ev-gone", "ev-c"],
        facts(vec![fact("ev-a", "One."), fact("ev-c", "Three.")]),
        None,
    );
    let positions: Vec<usize> = scenario
        .accusation
        .instances
        .iter()
        .map(|i| i.position)
        .collect();
    assert_eq!(positions, vec![1, 2]);
}

#[test]
fn the_statement_kind_is_humanized_and_not_translated() {
    // The extraction's set is open — nineteen values measured on this case — so a
    // fixed table of plain names would silently miss the twentieth.
    let scenario = render(
        &[instance("ev-a")],
        &["ev-a"],
        facts(vec![fact("ev-a", "They refused.")]),
        None,
    );
    assert_eq!(
        scenario.accusation.instances[0].kind_label,
        "attorney argument"
    );
}

#[test]
fn the_row_says_it_has_no_answer_without_repeating_the_gap_sentence() {
    // Ruling C5, and the beta.381 defect it closes. The row must SAY it has no
    // answer — a reader meets the row, not the list — but the sentence naming
    // who/when/where lives once, in the prep list. Anything else is the same
    // sentence twice on one screen.
    let scenario = render(
        &[instance("ev-a")],
        &["ev-a"],
        facts(vec![fact("ev-a", "They refused.")]),
        None,
    );

    let row = &scenario.accusation.instances[0];
    let sentence = scenario.accusation.gaps[0].message.as_str();

    assert_eq!(row.answer_tag, "NO ANSWER");
    assert_eq!(row.answer_banner.as_deref(), Some("NO ANSWER PREPARED"));
    assert!(
        sentence.starts_with("NO ANSWER PREPARED"),
        "the prep list still carries the loud sentence: {sentence}"
    );
    assert_ne!(
        row.answer_banner.as_deref(),
        Some(sentence),
        "the row must NOT repeat the prep list's sentence — that is the defect"
    );
    // The distinguishing fact: the sentence names the statement, the banner
    // cannot. If the banner ever grew a `{who}` this assertion fails.
    assert!(
        sentence.contains("George Phillips"),
        "the prep list names who said it: {sentence}"
    );
    assert!(
        !row.answer_banner.as_deref().unwrap().contains("George"),
        "the banner names nobody — that is what keeps it from becoming the \
         duplicate sentence again"
    );
}

#[test]
fn the_prep_list_can_point_at_the_row_it_is_about() {
    // The jump link's target. Rows are numbered from 1, and the entry must name
    // the row it describes — sending a reader to row 1 for row 3's gap would be
    // worse than no link.
    let scenario = render(
        &[instance("ev-a"), instance("ev-b")],
        &["ev-a", "ev-b"],
        facts(vec![
            fact("ev-a", "They refused."),
            fact("ev-b", "They refused again."),
        ]),
        None,
    );

    let positions: Vec<Option<usize>> = scenario
        .accusation
        .gaps
        .iter()
        .map(|gap| gap.position)
        .collect();
    assert_eq!(positions, vec![Some(1), Some(2)]);
}

#[test]
fn an_answered_row_carries_the_answered_tag_and_no_banner() {
    let scenario = render(
        &[instance("ev-a"), pairing("ev-a", "ev-answer")],
        &["ev-a", "ev-answer"],
        facts(vec![
            fact("ev-a", "They refused."),
            fact("ev-answer", "We did not."),
        ]),
        None,
    );
    let row = &scenario.accusation.instances[0];
    assert_eq!(row.answer_tag, "ANSWERED");
    assert_eq!(row.answer_banner, None);
    assert!(row.answer.is_some());
}

// ── Which rows arrive open (task 2.11 C) ─────────────────────────────────────

/// N marked instances, each with its own statement.
fn n_instances(
    n: usize,
) -> (
    Vec<StoredJudgment>,
    Vec<String>,
    HashMap<String, RehearsalFactRow>,
) {
    let ids: Vec<String> = (0..n).map(|i| format!("ev-{i}")).collect();
    let judgments = ids.iter().map(|id| instance(id)).collect();
    let rows = facts(
        ids.iter()
            .map(|id| fact(id, "They refused to divide it."))
            .collect(),
    );
    (judgments, ids, rows)
}

#[test]
fn a_scenario_at_the_cap_arrives_with_its_rows_open() {
    // The seeded cap is 3. Three instances is a page a witness reads straight
    // down; the compact form would hide all of it behind three clicks.
    let (judgments, ids, rows) = n_instances(3);
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let scenario = render(&judgments, &refs, rows, Some("They say she refused."));

    assert_eq!(scenario.accusation.instances.len(), 3);
    assert!(scenario.instances_start_expanded);
}

#[test]
fn a_scenario_over_the_cap_arrives_compact_but_loses_no_rows() {
    // The rule is about the ARRIVAL state and nothing else. Four instances means
    // four rows on the payload — the list is never paginated, at any size,
    // because a page boundary in the middle of "he said it five times" breaks the
    // one thing the block exists to show.
    let (judgments, ids, rows) = n_instances(4);
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let scenario = render(&judgments, &refs, rows, Some("They say she refused."));

    assert!(!scenario.instances_start_expanded);
    assert_eq!(
        scenario.accusation.instances.len(),
        4,
        "every instance is still rendered — compact is a display state, not a cut"
    );
}

// ── The two authorship lines (ruling C2) ─────────────────────────────────────

#[test]
fn both_sentences_carry_who_wrote_them_when_the_store_recorded_it() {
    let at = chrono::NaiveDate::from_ymd_opt(2026, 8, 6)
        .expect("a real date")
        .and_hms_opt(9, 0, 0)
        .expect("a real time")
        .and_utc();
    let scenario = render_with(
        &[instance("ev-a")],
        &["ev-a"],
        facts(vec![fact("ev-a", "They refused.")]),
        Some("They say Marie refused to divide the property."),
        Authored {
            by: Some("Roman"),
            at: Some(at),
        },
    );

    assert_eq!(
        scenario.what_this_is_attribution.as_deref(),
        Some("Written by Roman · 6 Aug 2026")
    );
    assert_eq!(
        scenario.accusation.attribution.as_deref(),
        Some("Written in plain words by Roman · 6 Aug 2026"),
        "the accusation says 'in plain words' — what distinguishes our summary \
         from their verbatim quotes beneath it"
    );
}

#[test]
fn a_sentence_nobody_wrote_carries_no_authorship_line_at_all() {
    // An authorship line over a NAMED GAP would attribute an absence. The gap
    // sentence is the whole content of that block in this state.
    let scenario = render(
        &[instance("ev-a")],
        &["ev-a"],
        facts(vec![fact("ev-a", "They refused.")]),
        None,
    );

    assert_eq!(scenario.accusation.text, None);
    assert_eq!(scenario.accusation.attribution, None);
    assert!(scenario.accusation.text_gap.is_some());
}

#[test]
fn a_sentence_written_before_the_columns_existed_says_the_author_is_unrecorded() {
    // The state every S-2 sentence is in on the day this ships: the migration
    // deliberately backfilled nothing.
    let scenario = render(
        &[instance("ev-a")],
        &["ev-a"],
        facts(vec![fact("ev-a", "They refused.")]),
        Some("They say Marie refused to divide the property."),
    );

    assert_eq!(
        scenario.accusation.attribution.as_deref(),
        Some("Author not recorded — written before authorship was kept.")
    );
}
