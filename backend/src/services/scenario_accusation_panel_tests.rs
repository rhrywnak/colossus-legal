//! Tests for [`super`] — what the accusation section is actually told to say.
//!
//! The derivation next door proves the LAW (which absences exist). These prove the
//! RENDER: that the sentences a human reads are the stored ones with the right
//! numbers in them, that exactly one of the two summary lines is ever present, and
//! that no template escapes to the browser.

use super::*;
use crate::domain::human_authored::HumanFactKind;
use crate::services::scenario_accusation::{derive, StoredJudgment};
use std::collections::HashSet;

fn included(ids: &[&str]) -> HashSet<String> {
    ids.iter().map(|s| (*s).to_string()).collect()
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

fn ordinals(pairs: &[(&str, i32)]) -> HashMap<String, i32> {
    pairs
        .iter()
        .map(|(node, n)| ((*node).to_string(), *n))
        .collect()
}

fn documents(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(node, doc)| ((*node).to_string(), (*doc).to_string()))
        .collect()
}

/// A panel over one derived state, with the seeded words.
fn panel(
    state: &AccusationState,
    ords: &HashMap<String, i32>,
    docs: &HashMap<String, String>,
    included_count: usize,
    accusation_text: Option<&str>,
) -> AccusationPanelDto {
    let wording = AccusationWording::for_test();
    build_panel(PanelInput {
        accusation_text,
        state,
        ordinals: ords,
        documents: docs,
        included_count,
        wording: &wording,
    })
}

// ── The two summary lines, and the rule that exactly one shows ───────────────

#[test]
fn the_count_line_carries_both_real_numbers() {
    // Roman's sentence, in the product: "he mentioned it 5 times in 5 different
    // documents". Two marked instances in two documents must read as two of each,
    // from the STORED template — not from a sentence assembled in code.
    let state = derive(
        &[instance("ev-a"), instance("ev-b")],
        &included(&["ev-a", "ev-b"]),
    );
    let dto = panel(
        &state,
        &ordinals(&[("ev-a", 3), ("ev-b", 9)]),
        &documents(&[("ev-a", "doc-1"), ("ev-b", "doc-2")]),
        46,
        None,
    );

    assert_eq!(
        dto.count_line.as_deref(),
        Some("Said 2 times, in 2 documents.")
    );
    assert_eq!(dto.no_instances_notice, None);
}

#[test]
fn the_count_line_never_inflates_when_a_statement_has_left() {
    // The honest render the design names explicitly: "said 2 times" with gaps
    // showing. The departed instance is a gap, not a tally mark, and the document
    // it used to sit in is not a document the accusation was said in.
    let state = derive(
        &[instance("ev-a"), instance("ev-b"), instance("ev-gone")],
        &included(&["ev-a", "ev-b"]),
    );
    let dto = panel(
        &state,
        &ordinals(&[]),
        &documents(&[("ev-a", "doc-1"), ("ev-b", "doc-1"), ("ev-gone", "doc-9")]),
        46,
        None,
    );

    assert_eq!(
        dto.count_line.as_deref(),
        Some("Said 2 times, in 1 documents.")
    );
}

#[test]
fn nothing_marked_shows_the_notice_with_the_waiting_count_and_no_count_line() {
    // "No instances marked" and "no instances marked, out of forty-six waiting"
    // are different states of the same scenario, and the second is the one that
    // says what to do next.
    let state = derive(&[], &included(&["ev-a", "ev-b"]));
    let dto = panel(&state, &ordinals(&[]), &documents(&[]), 46, None);

    assert_eq!(
        dto.no_instances_notice.as_deref(),
        Some("No instances marked yet. 46 included facts are waiting here.")
    );
    assert_eq!(dto.count_line, None);
}

#[test]
fn exactly_one_summary_line_is_present_in_every_state() {
    // The contract stated as a test. Both would say two things about one state;
    // neither would leave the section silent about whether anything is marked —
    // and "nobody has started" must never look like "the read failed".
    let states = [
        derive(&[], &included(&[])),
        derive(&[], &included(&["ev-a"])),
        derive(&[instance("ev-a")], &included(&["ev-a"])),
        derive(&[instance("ev-a")], &included(&[])),
        derive(
            &[instance("ev-a"), pairing("ev-a", "ev-b")],
            &included(&["ev-a", "ev-b"]),
        ),
    ];

    for state in states {
        let dto = panel(&state, &ordinals(&[]), &documents(&[]), 4, None);
        assert!(
            dto.count_line.is_some() ^ dto.no_instances_notice.is_some(),
            "exactly one summary line must be present: {:?} / {:?}",
            dto.count_line,
            dto.no_instances_notice
        );
    }
}

// ── The gaps, as a human reads them ──────────────────────────────────────────

#[test]
fn the_prep_list_names_the_fact_by_the_handle_a_human_says() {
    // A gap that cannot say WHICH fact is useless on a list of forty-six that look
    // alike — and the design calls this list the single most useful thing on the
    // page.
    let state = derive(&[instance("ev-a")], &included(&["ev-a"]));
    let dto = panel(
        &state,
        &ordinals(&[("ev-a", 14)]),
        &documents(&[]),
        46,
        None,
    );

    assert_eq!(dto.gaps.len(), 1);
    assert_eq!(dto.gaps[0].kind, GAP_KIND_NO_ANSWER);
    assert_eq!(dto.gaps[0].message, "NO ANSWER PREPARED — C-14");
    assert_eq!(dto.gaps[0].graph_node_id, "ev-a");
}

#[test]
fn an_unnumbered_fact_falls_back_to_its_id_rather_than_a_hole_in_the_sentence() {
    // A candidate gather has not numbered yet still has to be nameable. An ugly id
    // a human can search for beats "NO ANSWER PREPARED — ", which names nothing.
    let state = derive(&[instance("ev-unnumbered")], &included(&["ev-unnumbered"]));
    let dto = panel(&state, &ordinals(&[]), &documents(&[]), 1, None);

    assert_eq!(dto.gaps[0].message, "NO ANSWER PREPARED — ev-unnumbered");
    assert!(!dto.gaps[0].message.contains("{code}"));
}

#[test]
fn the_three_gap_kinds_carry_three_different_tokens_and_three_different_sentences() {
    // The Remove law's two halves have different remedies — one needs a new
    // answer, the other needs the statement back or the pairing withdrawn — so a
    // human must be able to tell them apart, and a client must be able to without
    // parsing prose.
    let state = derive(
        &[
            instance("ev-unanswered"),
            instance("ev-answer-gone"),
            pairing("ev-answer-gone", "ev-departed-answer"),
            pairing("ev-never-marked", "ev-some-answer"),
        ],
        &included(&["ev-unanswered", "ev-answer-gone", "ev-some-answer"]),
    );
    let dto = panel(&state, &ordinals(&[]), &documents(&[]), 3, None);

    let kinds: Vec<&str> = dto.gaps.iter().map(|g| g.kind.as_str()).collect();
    assert!(kinds.contains(&GAP_KIND_NO_ANSWER), "{kinds:?}");
    assert!(kinds.contains(&GAP_KIND_ANSWER_REMOVED), "{kinds:?}");
    assert!(kinds.contains(&GAP_KIND_ACCUSATION_REMOVED), "{kinds:?}");

    let messages: HashSet<&str> = dto.gaps.iter().map(|g| g.message.as_str()).collect();
    assert_eq!(
        messages.len(),
        dto.gaps.len(),
        "two gaps rendered the same sentence: {messages:?}"
    );
}

#[test]
fn the_prep_list_is_first_so_the_actionable_gap_is_the_one_read_first() {
    // The derivation pins the order; this asserts the panel preserves it. A render
    // that re-sorted would bury the list a human acts on beneath two they cannot.
    let state = derive(
        &[
            instance("ev-z-unanswered"),
            pairing("ev-a-orphan", "ev-answer"),
        ],
        &included(&["ev-z-unanswered", "ev-answer"]),
    );
    let dto = panel(&state, &ordinals(&[]), &documents(&[]), 2, None);

    assert_eq!(dto.gaps[0].kind, GAP_KIND_NO_ANSWER);
}

// ── The instances, and the answer-present flag ───────────────────────────────

#[test]
fn a_paired_instance_reports_its_answer_and_that_the_answer_is_here() {
    let state = derive(
        &[instance("ev-a"), pairing("ev-a", "ev-answer")],
        &included(&["ev-a", "ev-answer"]),
    );
    let dto = panel(
        &state,
        &ordinals(&[("ev-a", 14), ("ev-answer", 31)]),
        &documents(&[("ev-a", "doc-1")]),
        2,
        None,
    );

    assert_eq!(dto.instances.len(), 1);
    assert_eq!(dto.instances[0].code.as_deref(), Some("C-14"));
    assert_eq!(dto.instances[0].answer_code.as_deref(), Some("C-31"));
    assert_eq!(
        dto.instances[0].answers_graph_node_id.as_deref(),
        Some("ev-answer")
    );
    assert!(dto.instances[0].answer_present);
    assert!(dto.gaps.is_empty());
}

#[test]
fn an_instance_whose_answer_left_keeps_the_pairing_and_reports_it_broken() {
    // The Remove law end to end: the row is KEPT, the instance still counts as
    // said, the answer is still named, and the flag says it is not here. A client
    // must not have to cross-reference the gap list to know that.
    let state = derive(
        &[instance("ev-a"), pairing("ev-a", "ev-answer")],
        &included(&["ev-a"]),
    );
    let dto = panel(
        &state,
        &ordinals(&[]),
        &documents(&[("ev-a", "d1")]),
        1,
        None,
    );

    assert_eq!(
        dto.count_line.as_deref(),
        Some("Said 1 times, in 1 documents.")
    );
    assert_eq!(
        dto.instances[0].answers_graph_node_id.as_deref(),
        Some("ev-answer"),
        "the pairing must not vanish"
    );
    assert!(!dto.instances[0].answer_present);
    assert_eq!(dto.gaps[0].kind, GAP_KIND_ANSWER_REMOVED);
}

#[test]
fn an_unpaired_instance_is_not_reported_as_having_a_present_answer() {
    // `answer_present` must mean "there is an answer and it is here", not "no gap
    // mentions this anchor". An unpaired instance has no answer at all.
    let state = derive(&[instance("ev-a")], &included(&["ev-a"]));
    let dto = panel(&state, &ordinals(&[]), &documents(&[]), 1, None);

    assert_eq!(dto.instances[0].answers_graph_node_id, None);
    assert!(!dto.instances[0].answer_present);
}

// ── What crosses the wire, and what does not ─────────────────────────────────

#[test]
fn no_template_placeholder_escapes_to_the_browser_except_the_one_it_must_fill() {
    // §10 exclusion, made structural. The count template and the gap templates
    // have no field to travel in, so a browser cannot recompute a count — and
    // would need the instances and the document map to try. `{detail}` is the one
    // exception: it is the failure's own words, which exist only over there.
    let state = derive(&[instance("ev-a")], &included(&["ev-a"]));
    let dto = panel(&state, &ordinals(&[("ev-a", 1)]), &documents(&[]), 1, None);
    let json = serde_json::to_string(&dto).expect("the panel serializes");

    assert!(!json.contains("{times}"), "{json}");
    assert!(!json.contains("{documents}"), "{json}");
    assert!(!json.contains("{code}"), "{json}");
    assert!(!json.contains("{included}"), "{json}");
    assert!(
        json.contains("{detail}"),
        "the failure template keeps its one placeholder"
    );
}

#[test]
fn an_unwritten_accusation_travels_as_null_and_is_never_stood_in_for() {
    // The beta.371 defect at its root: nothing may fill this slot but a human's
    // sentence. `None` here is what makes the section render the stored gap
    // notice rather than a quote from the record.
    let state = derive(&[], &included(&[]));
    let dto = panel(&state, &ordinals(&[]), &documents(&[]), 0, None);

    assert_eq!(dto.accusation_text, None);
    assert_eq!(
        dto.wording.text_missing_notice,
        AccusationWording::for_test().text_missing_notice
    );
}

#[test]
fn an_authored_accusation_travels_verbatim() {
    let state = derive(&[], &included(&[]));
    let dto = panel(
        &state,
        &ordinals(&[]),
        &documents(&[]),
        0,
        Some("They say Marie refused to divide the property."),
    );

    assert_eq!(
        dto.accusation_text.as_deref(),
        Some("They say Marie refused to divide the property.")
    );
}
