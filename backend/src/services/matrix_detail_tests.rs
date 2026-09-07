// Tests for `services::matrix_detail`.
//
// The ordering itself is proved in `matrix_order_tests`; what is proved here is
// the WIRING — that a stored ruling reaches the item it belongs to, that the leg
// really is rebuilt in the ordered sequence, and that an unreadable stored
// verdict leaves the item unruled rather than removing it.

use super::*;
use crate::repositories::element_detail_repository::AllegationSummary;
use chrono::{DateTime, Utc};

fn at() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_757_000_000, 0).expect("a valid instant")
}

/// A bare evidence row: no edge properties, no ruling, no Q&A.
fn evidence(id: &str) -> EvidenceRef {
    EvidenceRef {
        id: id.to_string(),
        verbatim_quote: Some(format!("quote for {id}")),
        page_number: None,
        paragraph: None,
        page_note: None,
        source_document_id: None,
        source_document_title: None,
        statement_type: None,
        evidence_strength: None,
        speaker: None,
        question: None,
        answer: None,
        rank: None,
        role: None,
        confidence: None,
        rank_reason: None,
        why: None,
        conflict: false,
        duplicate_of_card_id: None,
        document_date: None,
        ruling: None,
        ruled_by: None,
        hidden_reason: None,
        rfa_line: None,
        occurrences: 1,
    }
}

/// One Allegation carrying the given supporting items and nothing disputing.
fn response_with(supporting: Vec<EvidenceRef>) -> ElementDetailResponse {
    ElementDetailResponse {
        element_id: "element-1-1".to_string(),
        element_name: "Duty".to_string(),
        what_plaintiff_must_prove: "A duty was owed.".to_string(),
        order_in_count: Some(1),
        count_number: Some(1),
        count_name: Some("Breach of Fiduciary Duty".to_string()),
        review_notes: None,
        allegation_count: 1,
        common_count: 1,
        dedicated_count: 0,
        visible_items: 5,
        allegations: vec![AllegationSummary {
            allegation_id: "allegation-41".to_string(),
            paragraph_number: "41".to_string(),
            summary: Some("They ignored her".to_string()),
            title: None,
            verbatim_quote: Some("The Defendants ignored…".to_string()),
            source_section: "Common",
            supporting_evidence: supporting,
            disputing_evidence: Vec::new(),
        }],
    }
}

fn ruling(evidence_id: &str, token: &str) -> RulingRecord {
    RulingRecord {
        evidence_id: evidence_id.to_string(),
        allegation_id: "allegation-41".to_string(),
        ruling: token.to_string(),
        ruled_by: "roman".to_string(),
        ruled_at: at(),
        updated_at: at(),
        note: None,
    }
}

fn ids(response: &ElementDetailResponse) -> Vec<&str> {
    response.allegations[0]
        .supporting_evidence
        .iter()
        .map(|e| e.id.as_str())
        .collect()
}

/// A stored `keep` reaches the item, marks it, and floats it to the top.
#[test]
fn a_stored_keep_reaches_its_item_and_leads_the_list() {
    let mut ranked = evidence("machine-best");
    ranked.rank = Some(1);
    let mut kept = evidence("human-kept");
    kept.rank = Some(20);

    let mut response = response_with(vec![ranked, kept]);
    apply_rulings_and_order(
        &mut response,
        &[ruling("human-kept", "keep")],
        &MatrixWording::for_test(),
    );

    assert_eq!(ids(&response), vec!["human-kept", "machine-best"]);
    let leader = &response.allegations[0].supporting_evidence[0];
    assert_eq!(leader.ruling.as_deref(), Some("keep"));
    assert_eq!(leader.ruled_by.as_deref(), Some("roman"));
    assert_eq!(response.allegations[0].supporting_evidence[1].ruling, None);
}

/// A ruling for a DIFFERENT allegation does not reach this one.
///
/// The index is keyed by the pair for exactly this reason: the same statement
/// commonly bears on several accusations, and a keep under ¶41 says nothing about
/// the same words under ¶61.
#[test]
fn a_ruling_for_another_allegation_does_not_leak_across() {
    let mut stray = ruling("item", "keep");
    stray.allegation_id = "allegation-61".to_string();

    let mut response = response_with(vec![evidence("item")]);
    apply_rulings_and_order(&mut response, &[stray], &MatrixWording::for_test());

    assert_eq!(response.allegations[0].supporting_evidence[0].ruling, None);
}

/// A stored `remove` marks the item hidden and moves it to the end — it is never
/// deleted from the payload, because the page has to be able to reveal it.
#[test]
fn a_stored_remove_hides_the_item_without_dropping_it() {
    let mut response = response_with(vec![evidence("struck"), evidence("shown")]);
    apply_rulings_and_order(
        &mut response,
        &[ruling("struck", "remove")],
        &MatrixWording::for_test(),
    );

    assert_eq!(ids(&response), vec!["shown", "struck"]);
    let leg = &response.allegations[0].supporting_evidence;
    assert_eq!(leg[0].hidden_reason, None);
    assert_eq!(leg[1].hidden_reason.as_deref(), Some("removed"));
}

/// AN UNREADABLE STORED VERDICT LEAVES THE ITEM UNRULED — never hidden.
///
/// The failure direction matters more than the failure. A token this build cannot
/// read must not be treated as `remove`: that would take a piece of evidence off a
/// proof surface on the strength of a value nobody can interpret.
#[test]
fn an_unreadable_stored_verdict_leaves_the_item_visible_and_unruled() {
    let mut response = response_with(vec![evidence("item")]);
    apply_rulings_and_order(
        &mut response,
        &[ruling("item", "obliterate")],
        &MatrixWording::for_test(),
    );

    let item = &response.allegations[0].supporting_evidence[0];
    assert_eq!(
        item.ruling, None,
        "the unreadable token is not passed through"
    );
    assert_eq!(
        item.hidden_reason, None,
        "and it certainly does not hide the item"
    );
}

/// A folded duplicate leaves the leg and its target carries the "×N".
#[test]
fn a_folded_duplicate_leaves_the_leg_and_its_target_counts_it() {
    let mut dup = evidence("dup");
    dup.role = Some("duplicate_of".to_string());
    dup.duplicate_of_card_id = Some("target".to_string());

    let mut response = response_with(vec![evidence("target"), dup]);
    apply_rulings_and_order(&mut response, &[], &MatrixWording::for_test());

    assert_eq!(ids(&response), vec!["target"]);
    assert_eq!(
        response.allegations[0].supporting_evidence[0].occurrences,
        2
    );
}

/// A Q&A card gets its finished one-line rendering; an ordinary card does not.
#[test]
fn a_q_and_a_card_gets_its_rfa_line_and_a_plain_card_does_not() {
    let mut rfa = evidence("rfa");
    rfa.question = Some("Admit that you were engaged.".to_string());
    rfa.answer = Some("Admitted".to_string());
    rfa.paragraph = Some("RFA 8".to_string());

    let mut response = response_with(vec![rfa, evidence("plain")]);
    apply_rulings_and_order(&mut response, &[], &MatrixWording::for_test());

    let leg = &response.allegations[0].supporting_evidence;
    let card = leg
        .iter()
        .find(|e| e.id == "rfa")
        .expect("the RFA card survives");
    let line = card
        .rfa_line
        .as_deref()
        .expect("an RFA card renders a line");
    assert!(line.contains("RFA 8"), "{line}");
    assert!(line.contains("Admitted"), "{line}");

    let plain = leg
        .iter()
        .find(|e| e.id == "plain")
        .expect("the plain card survives");
    assert_eq!(plain.rfa_line, None);
}

/// The DISPUTING leg is ordered and ruled by the same rules as the supporting one.
///
/// §3 describes "both stance lists", and the two legs are rendered by one
/// component. A pass that only touched `supporting_evidence` would leave the
/// disputing list in graph order with no Keep/Remove marks — and nothing would
/// say so.
#[test]
fn the_disputing_leg_is_ordered_and_ruled_too() {
    let mut first = evidence("rebut-rank-1");
    first.rank = Some(1);
    let mut second = evidence("rebut-rank-2");
    second.rank = Some(2);

    let mut response = response_with(Vec::new());
    response.allegations[0].disputing_evidence = vec![second, first];
    apply_rulings_and_order(
        &mut response,
        &[RulingRecord {
            evidence_id: "rebut-rank-2".to_string(),
            ..ruling("rebut-rank-2", "keep")
        }],
        &MatrixWording::for_test(),
    );

    let leg = &response.allegations[0].disputing_evidence;
    let order: Vec<&str> = leg.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(
        order,
        vec!["rebut-rank-2", "rebut-rank-1"],
        "the kept item leads"
    );
    assert_eq!(leg[0].ruled_by.as_deref(), Some("roman"));
}

/// An Allegation with no evidence on either leg survives the overlay untouched.
///
/// The empty list is the visible gap this panel exists to show; a pass that
/// panicked or dropped the Allegation would remove the gap along with it.
#[test]
fn an_allegation_with_no_evidence_survives() {
    let mut response = response_with(Vec::new());
    apply_rulings_and_order(&mut response, &[], &MatrixWording::for_test());

    assert_eq!(response.allegations.len(), 1);
    assert!(response.allegations[0].supporting_evidence.is_empty());
    assert!(response.allegations[0].disputing_evidence.is_empty());
}

/// Nothing is lost: every item that went in comes out as a row or inside one
/// row's occurrence count.
#[test]
fn the_overlay_loses_nothing() {
    let mut dup = evidence("dup");
    dup.role = Some("duplicate_of".to_string());
    dup.duplicate_of_card_id = Some("target".to_string());
    let mut excluded = evidence("excluded");
    excluded.role = Some("does_not_belong".to_string());

    let mut response = response_with(vec![evidence("target"), dup, excluded, evidence("plain")]);
    apply_rulings_and_order(
        &mut response,
        &[ruling("plain", "remove")],
        &MatrixWording::for_test(),
    );

    let accounted: usize = response.allegations[0]
        .supporting_evidence
        .iter()
        .map(|e| e.occurrences)
        .sum();
    assert_eq!(accounted, 4, "an item was dropped by the overlay");
}
