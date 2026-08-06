//! The honest-gap law, as assertions (task 2.11).
//!
//! Every test here is about an ABSENCE — the thing the page must name rather
//! than hide. A count that inflates or a pairing that vanishes are both failures
//! a reader cannot detect from the screen, which is exactly why they are pinned
//! here instead of left to review.

use super::*;

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

#[test]
fn an_unanswered_instance_is_the_prep_list() {
    // The design's words: the gap list "is the list of what still needs
    // preparing before trial", and the single most useful thing on the page.
    let state = derive(&[instance("ev-a")], &included(&["ev-a"]));

    assert_eq!(state.times_said(), 1);
    assert_eq!(state.unanswered(), 1);
    assert_eq!(
        state.gaps,
        vec![AccusationGap::NoAnswerPrepared {
            anchor_graph_node_id: "ev-a".to_string()
        }]
    );
}

#[test]
fn an_answered_instance_produces_no_gap() {
    let state = derive(
        &[instance("ev-a"), pairing("ev-a", "ev-answer")],
        &included(&["ev-a", "ev-answer"]),
    );

    assert_eq!(state.times_said(), 1);
    assert_eq!(state.unanswered(), 0);
    assert!(state.gaps.is_empty());
    assert_eq!(
        state.instances[0].answers_graph_node_id.as_deref(),
        Some("ev-answer")
    );
}

#[test]
fn the_count_never_inflates_past_what_is_placed() {
    // "The count is computed from what is actually placed — 'said 2 times' with
    // 3 gaps showing. It never inflates." Two marked, one of them removed from
    // the scenario: the page may claim ONE.
    let state = derive(
        &[instance("ev-a"), instance("ev-gone")],
        &included(&["ev-a"]),
    );

    assert_eq!(
        state.times_said(),
        1,
        "an instance whose statement left the scenario is not a tally mark",
    );
}

#[test]
fn a_pairing_whose_answer_left_is_shown_not_deleted() {
    // THE Remove law (ruled 2026-08-06). The human's pairing survives and says
    // what happened. Deleting it would make their judgment disappear with
    // nothing on screen to say so.
    let state = derive(
        &[instance("ev-a"), pairing("ev-a", "ev-answer")],
        &included(&["ev-a"]),
    );

    assert_eq!(state.times_said(), 1, "the accusation was still said");
    assert_eq!(
        state.gaps,
        vec![AccusationGap::AnswerRemoved {
            anchor_graph_node_id: "ev-a".to_string(),
            answers_graph_node_id: "ev-answer".to_string(),
        }],
        "the answer leaving must be NAMED, not silently unpaired",
    );
}

#[test]
fn a_pairing_whose_accusation_left_is_also_named() {
    // The other side of the same law. Neither direction may vanish quietly.
    let state = derive(
        &[instance("ev-gone"), pairing("ev-gone", "ev-answer")],
        &included(&["ev-answer"]),
    );

    assert_eq!(state.times_said(), 0);
    assert_eq!(
        state.gaps,
        vec![AccusationGap::AccusationRemoved {
            anchor_graph_node_id: "ev-gone".to_string()
        }]
    );
}

#[test]
fn a_pairing_with_no_instance_behind_it_is_still_visible() {
    // Re-including a removed fact and not re-marking it leaves a pairing hanging
    // off an anchor nobody has marked. It is still a human's judgment; dropping
    // it from the derivation would be the silent vanish in a subtler costume.
    let state = derive(
        &[pairing("ev-a", "ev-answer")],
        &included(&["ev-a", "ev-answer"]),
    );

    assert_eq!(state.times_said(), 0, "nothing is marked as an instance");
    assert_eq!(
        state.gaps,
        vec![AccusationGap::AccusationRemoved {
            anchor_graph_node_id: "ev-a".to_string()
        }]
    );
}

#[test]
fn nothing_marked_yields_nothing_claimed() {
    // A scenario with 46 included facts and no instances marked must claim
    // nothing at all — the page's own empty state, not a count of zero dressed
    // as a finding.
    let state = derive(&[], &included(&["ev-a", "ev-b"]));
    assert_eq!(state.times_said(), 0);
    assert!(state.gaps.is_empty());
    assert!(state.instances.is_empty());
}

#[test]
fn the_order_is_total_so_two_renders_cannot_disagree() {
    // The derivation walks a HashMap. Without the sorts, gap order would follow
    // hash iteration and the prep list would reshuffle itself between reloads.
    let judgments = vec![
        instance("ev-c"),
        instance("ev-a"),
        instance("ev-b"),
        pairing("ev-b", "ev-answer"),
    ];
    let present = included(&["ev-a", "ev-b", "ev-c", "ev-answer"]);

    let first = derive(&judgments, &present);
    // Same facts, different input order — the output must be identical.
    let shuffled = vec![
        pairing("ev-b", "ev-answer"),
        instance("ev-b"),
        instance("ev-c"),
        instance("ev-a"),
    ];
    assert_eq!(derive(&shuffled, &present), first);

    let anchors: Vec<&str> = first
        .instances
        .iter()
        .map(|i| i.anchor_graph_node_id.as_str())
        .collect();
    assert_eq!(anchors, vec!["ev-a", "ev-b", "ev-c"]);
}

#[test]
fn the_prep_list_comes_first_among_the_gaps() {
    // Three kinds of absence, and they are not equally urgent: an unanswered
    // accusation is work to do before trial, while a broken pairing is a repair.
    // The order is the design's priority, not the enum's declaration order.
    let state = derive(
        &[
            instance("ev-unanswered"),
            instance("ev-broken"),
            pairing("ev-broken", "ev-gone-answer"),
            pairing("ev-never-marked", "ev-answer"),
        ],
        &included(&["ev-unanswered", "ev-broken", "ev-answer"]),
    );

    assert!(matches!(
        state.gaps.first(),
        Some(AccusationGap::NoAnswerPrepared { .. })
    ));
    assert_eq!(state.gaps.len(), 3);
    assert_eq!(state.unanswered(), 1);
}

#[test]
fn marking_the_same_statement_twice_counts_once() {
    // The database's partial unique index makes this unreachable through the
    // write path, but a derivation that double-counted would inflate the very
    // number the design says must never inflate — so it is asserted rather than
    // assumed from a constraint one layer away.
    let state = derive(&[instance("ev-a"), instance("ev-a")], &included(&["ev-a"]));
    assert_eq!(state.times_said(), 1);
    assert_eq!(state.instances.len(), 1);
}

// ── The document count (task 2.11 B1, the second half of Roman's sentence) ────

/// A document map, in the shape the read path builds it.
fn documents(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(node, doc)| ((*node).to_string(), (*doc).to_string()))
        .collect()
}

#[test]
fn the_same_accusation_twice_in_one_document_is_one_document() {
    // "Said 2 times, in 1 document." The force of Roman's model is the SPREAD —
    // two sentences on one page of one deposition is one witness on one day, and
    // reporting it as two documents would overstate the pattern.
    let state = derive(
        &[instance("ev-a"), instance("ev-b")],
        &included(&["ev-a", "ev-b"]),
    );

    assert_eq!(state.times_said(), 2);
    assert_eq!(
        state.documents_spoken_in(&documents(&[("ev-a", "doc-1"), ("ev-b", "doc-1")])),
        1
    );
}

#[test]
fn instances_across_three_documents_count_three() {
    let state = derive(
        &[instance("ev-a"), instance("ev-b"), instance("ev-c")],
        &included(&["ev-a", "ev-b", "ev-c"]),
    );

    assert_eq!(
        state.documents_spoken_in(&documents(&[
            ("ev-a", "doc-1"),
            ("ev-b", "doc-2"),
            ("ev-c", "doc-3"),
        ])),
        3
    );
}

#[test]
fn an_instance_whose_document_is_unknown_undercounts_rather_than_guessing() {
    // The honest-gap law's direction of travel. An instance the graph cannot place
    // in a document still counts as SAID — a human marked it and the statement is
    // there — but it adds no document, because the page would then claim a source
    // it cannot produce on the spot.
    let state = derive(
        &[instance("ev-a"), instance("ev-orphan")],
        &included(&["ev-a", "ev-orphan"]),
    );

    assert_eq!(
        state.times_said(),
        2,
        "both were marked and both are present"
    );
    assert_eq!(
        state.documents_spoken_in(&documents(&[("ev-a", "doc-1")])),
        1,
        "the unplaceable instance must not invent a second document"
    );
}

#[test]
fn a_removed_instance_is_counted_in_neither_number() {
    // The count never inflates, and the document count is bound to the same rule:
    // an instance whose statement has left the scenario is a gap, not a tally
    // mark, even when the map still remembers which document it was in.
    let state = derive(
        &[
            instance("ev-a"),
            instance("ev-gone"),
            pairing("ev-gone", "ev-x"),
        ],
        &included(&["ev-a"]),
    );

    assert_eq!(state.times_said(), 1);
    assert_eq!(
        state.documents_spoken_in(&documents(&[("ev-a", "doc-1"), ("ev-gone", "doc-2")])),
        1,
        "a statement that left the scenario cannot still be a document it was said in"
    );
}

#[test]
fn no_instances_means_no_documents_and_no_empty_map_confusion() {
    // Zero and zero, from a map that DOES know about facts in the scenario — so a
    // future implementation that counted map entries instead of instances would
    // fail here rather than passing on an empty fixture.
    let state = derive(&[], &included(&["ev-a", "ev-b"]));

    assert_eq!(state.times_said(), 0);
    assert_eq!(
        state.documents_spoken_in(&documents(&[("ev-a", "doc-1"), ("ev-b", "doc-2")])),
        0
    );
}
