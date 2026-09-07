// Tests for `repositories::element_detail_repository`.
//
// Split out under the `#[path]` convention when task 396's ranking pass pushed
// this module further past Rule 17's limit. The file was ALREADY over (388
// non-comment lines at beta.394); moving the tests takes the module itself well
// under, which is the R8 discipline that no file a task touches ends over the
// limit.
//
// PROOF_MATRIX_v2 took four tests OUT of this file rather than adding to it. They
// exercised `rank_supporting_evidence`, which §3 superseded: the drill-down's
// order is now `services::matrix_order` (the four sort keys, the hide rules and
// the `duplicate_of` fold) applied by `services::matrix_detail`, and both have
// their own test files. The near-duplicate collapse those tests pinned still runs
// — it is what produces the matrix ROW's two numbers — and is still tested in
// `services::matrix_strength`.

use super::super::element_detail_cypher::element_detail_cypher;
use super::*;
use crate::neo4j::schema;
use serde_json::json;

/// One fully-populated supporting item, for the wire-shape tests.
///
/// Split out of the assertion when PROOF_MATRIX_v2 grew `EvidenceRef` from
/// thirteen fields to twenty-six: the literal and the expectation it is compared
/// against were one 130-line function, and a fixture nobody can read is a
/// fixture nobody checks.
fn wire_supporting_item() -> EvidenceRef {
    EvidenceRef {
        id: "evidence-074".to_string(),
        verbatim_quote: Some("That is my recollection.".to_string()),
        page_number: Some(22),
        paragraph: Some("Q74".to_string()),
        page_note: None,
        source_document_id: Some("doc-phillips".to_string()),
        source_document_title: Some("Phillips Discovery Response".to_string()),
        statement_type: Some("partial_admission".to_string()),
        evidence_strength: Some("sworn_party_admission".to_string()),
        speaker: Some("George Phillips".to_string()),
        question: Some("Did you review the accounting?".to_string()),
        answer: None,
        rank: Some(3),
        role: Some("their_own_words".to_string()),
        confidence: Some("high".to_string()),
        rank_reason: Some("His own sworn answer.".to_string()),
        why: None,
        conflict: false,
        duplicate_of_card_id: None,
        document_date: Some("2016-08-08".to_string()),
        ruling: Some("keep".to_string()),
        ruled_by: Some("roman".to_string()),
        hidden_reason: None,
        rfa_line: None,
        occurrences: 1,
    }
}

/// One disputing item carrying nothing the linking pass wrote — the shape of the
/// 286 edges that predate it.
fn wire_disputing_item() -> EvidenceRef {
    EvidenceRef {
        id: "evidence-041".to_string(),
        verbatim_quote: Some("No, that never happened.".to_string()),
        page_number: Some(15),
        paragraph: Some("Q41".to_string()),
        page_note: None,
        source_document_id: Some("doc-affidavit".to_string()),
        source_document_title: Some("Humphrey Affidavit".to_string()),
        statement_type: Some("denial".to_string()),
        evidence_strength: Some("sworn_party_denial".to_string()),
        speaker: Some("Marie Awad".to_string()),
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

/// The Allegation carrying both fixture items.
fn wire_summary() -> AllegationSummary {
    AllegationSummary {
        allegation_id: "allegation-42".to_string(),
        paragraph_number: "10".to_string(),
        summary: Some("Defendant did the thing.".to_string()),
        title: Some("Title".to_string()),
        verbatim_quote: None,
        source_section: "Common",
        supporting_evidence: vec![wire_supporting_item()],
        disputing_evidence: vec![wire_disputing_item()],
    }
}

/// Pins the ALLEGATION-level JSON shape. The frontend reads exactly these
/// snake_case keys; a typo in a struct field name would silently break the panel.
#[test]
fn allegation_summary_serializes_with_expected_keys() {
    let value = serde_json::to_value(wire_summary()).expect("serializes cleanly");
    let object = value.as_object().expect("an object body");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "allegation_id",
            "disputing_evidence",
            "paragraph_number",
            "source_section",
            "summary",
            "supporting_evidence",
            "title",
            "verbatim_quote",
        ]
    );
    assert_eq!(value["allegation_id"], json!("allegation-42"));
    assert_eq!(value["paragraph_number"], json!("10"));
    assert_eq!(value["source_section"], json!("Common"));
    // Absent is `null`, not omitted — the panel distinguishes "no quote" from
    // "the key is missing" (Rule 1).
    assert_eq!(value["verbatim_quote"], json!(null));
}

/// Pins the EVIDENCE-level JSON shape, on both legs.
///
/// Split from the assertion above so a failure names which half of the wire moved
/// — and because the two answer different questions: one is about the panel's
/// card, the other about every row inside it.
#[test]
fn evidence_ref_serializes_with_expected_keys() {
    let value = serde_json::to_value(wire_summary()).expect("serializes cleanly");
    assert_eq!(
        value["supporting_evidence"][0],
        json!({
            "id": "evidence-074",
            "verbatim_quote": "That is my recollection.",
            "page_number": 22,
            "paragraph": "Q74",
            "page_note": null,
            "source_document_id": "doc-phillips",
            "source_document_title": "Phillips Discovery Response",
            "statement_type": "partial_admission",
            "evidence_strength": "sworn_party_admission",
            "speaker": "George Phillips",
            "question": "Did you review the accounting?",
            "answer": null,
            "rank": 3,
            "role": "their_own_words",
            "confidence": "high",
            "rank_reason": "His own sworn answer.",
            "why": null,
            "conflict": false,
            "duplicate_of_card_id": null,
            "document_date": "2016-08-08",
            "ruling": "keep",
            "ruled_by": "roman",
            "hidden_reason": null,
            "rfa_line": null,
            "occurrences": 1,
        })
    );
}

/// An item from BEFORE the linking pass: every edge column `null`, `conflict`
/// false, and the row still complete on the wire. That is 286 of the graph's
/// 1,230 edges, and they must be distinguishable from items the pass rated —
/// `null` is a state the page renders differently, not an absence.
#[test]
fn an_unranked_item_serializes_every_edge_column_as_null() {
    let value = serde_json::to_value(wire_summary()).expect("serializes cleanly");
    let disputing = &value["disputing_evidence"][0];
    for absent in [
        "rank",
        "role",
        "confidence",
        "rank_reason",
        "why",
        "duplicate_of_card_id",
        "document_date",
        "ruling",
        "ruled_by",
        "hidden_reason",
        "rfa_line",
    ] {
        assert_eq!(disputing[absent], json!(null), "{absent} must be null here");
    }
    assert_eq!(disputing["conflict"], json!(false));
    assert_eq!(disputing["occurrences"], json!(1));
}

/// An Allegation with no Evidence on either leg serializes BOTH buckets as
/// **empty arrays, present** — never omitted. Those empty arrays are the
/// visible gaps the panel renders, and they mean different things: nothing
/// corroborates this Allegation, and nothing disputes it (Rule 1: empty and
/// absent stay distinguishable).
#[test]
fn allegation_with_no_evidence_serializes_empty_array_not_omitted() {
    let summary = AllegationSummary {
        allegation_id: "allegation-99".to_string(),
        paragraph_number: "73".to_string(),
        summary: None,
        title: None,
        verbatim_quote: None,
        source_section: "Dedicated",
        supporting_evidence: vec![],
        disputing_evidence: vec![],
    };
    let value = serde_json::to_value(&summary).expect("serializes cleanly");
    assert_eq!(value["supporting_evidence"], json!([]));
    assert_eq!(value["disputing_evidence"], json!([]));
}

/// The detail Cypher must carry BOTH evidence legs, each optional and each
/// hanging off the Allegation.
///
/// ## Why this test matters more than it looks
///
/// If the REBUTS branch were dropped, nothing would fail loudly:
/// `decode_evidence(row, EvidenceLeg::Disputing, op)` would read a missing
/// `disputing_id` as `None` and return `Ok(None)` on every row, so every
/// Allegation would report `disputing_evidence: []` — an empty list that
/// renders identically to "nothing disputes this Allegation". That silent
/// zero over real edges is exactly the defect the Disputes work exists to
/// end, so the query's shape is pinned rather than trusted.
#[test]
fn detail_cypher_carries_both_evidence_legs_optionally_off_the_allegation() {
    let q = element_detail_cypher();

    // The relationships are BOUND (`cr` / `dr`) since PROOF_MATRIX_v2 — the rank,
    // role and confidence live on the edge, so the edge needs a name to project
    // from. Everything else about the match is unchanged.
    assert!(q.contains(&format!(
        "OPTIONAL MATCH (a)<-[cr:{}]-(ev)",
        schema::CORROBORATES
    )));
    assert!(q.contains(&format!("OPTIONAL MATCH (a)<-[dr:{}]-(dv)", schema::REBUTS)));
    // Evidence reaches an Element only THROUGH an Allegation — never a
    // direct edge, which is the traversal that produced the false
    // `to_element: 0` finding on 2026-07-26.
    for rel in [schema::REBUTS, schema::CORROBORATES] {
        assert!(!q.contains(&format!("(e)<-[:{rel}]-")));
        assert!(!q.contains(&format!("(e)<-[cr:{rel}]-")));
        assert!(!q.contains(&format!("(e)<-[dr:{rel}]-")));
    }
}

/// Every `disputing_*` alias the fold decodes must be projected. A renamed
/// or dropped alias would decode to `None` and silently empty the bucket.
#[test]
fn detail_cypher_projects_every_disputing_alias_the_fold_reads() {
    let q = element_detail_cypher();
    for alias in [
        "AS disputing_id",
        "AS disputing_quote",
        "AS disputing_page_number",
        "AS disputing_paragraph",
        "AS disputing_page_note",
        "AS disputing_document_id",
        "AS disputing_document_title",
        "AS disputing_document_date",
        "AS disputing_answer",
        "AS disputing_rank",
        "AS disputing_role",
        "AS disputing_confidence",
        "AS disputing_rank_reason",
        "AS disputing_why",
        "AS disputing_conflict",
        "AS disputing_duplicate_of",
    ] {
        assert!(q.contains(alias), "missing RETURN alias `{alias}`");
    }
}

/// Node labels stay parameter-bound on BOTH legs — including the dispute
/// leg's Evidence and its source Document — so no domain label is inlined
/// into the Cypher (Rule 12 / Rule 16).
#[test]
fn detail_cypher_parameterizes_every_node_label_on_both_legs() {
    let q = element_detail_cypher();
    for binding in [
        "labels(e)[0] = $element_label",
        "labels(lc)[0] = $count_label",
        "labels(a)[0] = $allegation_label",
        "labels(ev)[0] = $evidence_label",
        "labels(dv)[0] = $evidence_label",
        "labels(d)[0] = $document_label",
        "labels(dd)[0] = $document_label",
    ] {
        assert!(q.contains(binding), "missing label binding `{binding}`");
    }
}

/// An Evidence item with no source Document keeps `source_document_id: null`
/// (the warn-logged data-gap state) rather than being dropped or erroring.
#[test]
fn evidence_ref_without_document_serializes_null_source_id() {
    let ev = EvidenceRef {
        id: "evidence-041".to_string(),
        verbatim_quote: Some("The accountings speak for themselves.".to_string()),
        page_number: Some(15),
        paragraph: Some("Q41".to_string()),
        page_note: Some("pages 15-16".to_string()),
        source_document_id: None,
        source_document_title: None,
        statement_type: Some("evasive".to_string()),
        evidence_strength: Some("sworn_party_evasion".to_string()),
        speaker: Some("George Phillips".to_string()),
        question: Some("What was the basis for the valuation?".to_string()),
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
    };
    let value = serde_json::to_value(&ev).expect("serializes cleanly");
    assert_eq!(value["source_document_id"], json!(null));
    assert_eq!(value["page_note"], json!("pages 15-16"));
}

/// ¶10 falls inside `COMMON_PARA_START..=COMMON_PARA_END`. Documents the
/// classifier's lower-half behavior.
#[test]
fn source_section_common_for_paragraph_10() {
    assert_eq!(source_section_for("10"), "Common");
}

/// ¶73 falls above `DEDICATED_PARA_START`. Documents the classifier's
/// upper-half behavior.
#[test]
fn source_section_dedicated_for_paragraph_73() {
    assert_eq!(source_section_for("73"), "Dedicated");
}

/// A non-numeric paragraph_number must classify as "Unknown" rather than
/// silently defaulting (Rule 1: distinct observables for distinct
/// states).
#[test]
fn source_section_unknown_for_non_numeric() {
    assert_eq!(source_section_for("abc"), "Unknown");
}

/// A range like "16-18" must classify by its starting paragraph (16 →
/// Common). This is the case the in-code comment explicitly calls out.
#[test]
fn source_section_handles_range_prefix() {
    assert_eq!(source_section_for("16-18"), "Common");
}

/// Boundary pins so a future paragraph-range tweak shows up as a test
/// failure rather than a silent classification drift.
#[test]
fn source_section_boundaries_pinned() {
    // Just below the Common range — pre-Common (¶6) is undefined territory
    // for the panel, classify as "Unknown".
    assert_eq!(source_section_for("6"), "Unknown");
    // Lower edge of Common.
    assert_eq!(source_section_for("7"), "Common");
    // Upper edge of Common.
    assert_eq!(source_section_for("71"), "Common");
    // Lower edge of Dedicated.
    assert_eq!(source_section_for("72"), "Dedicated");
}

/// Empty string is not a number; classify as "Unknown".
#[test]
fn source_section_empty_string() {
    assert_eq!(source_section_for(""), "Unknown");
}
