// Tests for `repositories::element_detail_repository`.
//
// Split out under the `#[path]` convention when task 396's ranking pass pushed
// this module further past Rule 17's limit. The file was ALREADY over (388
// non-comment lines at beta.394); moving the tests takes the module itself well
// under, which is the R8 discipline that no file a task touches ends over the
// limit.

use super::*;
use crate::domain::evidence_tier::EvidenceTierMap;
use serde_json::json;

/// Pins the JSON shape of `AllegationSummary`. The frontend reads
/// exactly these snake_case keys; a typo in the struct field name would
/// silently break the panel.
#[test]
fn allegation_summary_serializes_with_expected_keys() {
    let summary = AllegationSummary {
        allegation_id: "allegation-42".to_string(),
        paragraph_number: "10".to_string(),
        summary: Some("Defendant did the thing.".to_string()),
        title: Some("Title".to_string()),
        verbatim_quote: None,
        source_section: "Common",
        supporting_evidence: vec![EvidenceRef {
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
            tier: Some("hedged".to_string()),
            occurrences: 1,
        }],
        disputing_evidence: vec![EvidenceRef {
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
            // The disputing leg is deliberately never tiered — see
            // `rank_supporting_evidence`.
            tier: None,
            occurrences: 1,
        }],
    };
    let value = serde_json::to_value(&summary).expect("serializes cleanly");
    assert_eq!(
        value,
        json!({
            "allegation_id": "allegation-42",
            "paragraph_number": "10",
            "summary": "Defendant did the thing.",
            "title": "Title",
            "verbatim_quote": null,
            "source_section": "Common",
            "supporting_evidence": [{
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
                "tier": "hedged",
                "occurrences": 1,
            }],
            "disputing_evidence": [{
                "id": "evidence-041",
                "verbatim_quote": "No, that never happened.",
                "page_number": 15,
                "paragraph": "Q41",
                "page_note": null,
                "source_document_id": "doc-affidavit",
                "source_document_title": "Humphrey Affidavit",
                "statement_type": "denial",
                "evidence_strength": "sworn_party_denial",
                "speaker": "Marie Awad",
                "question": null,
                "tier": null,
                "occurrences": 1,
            }],
        })
    );
}

/// Build a supporting-leg item for the ranking tests.
fn supporting(
    id: &str,
    statement_type: &str,
    strength: &str,
    speaker: &str,
    question: Option<&str>,
    quote: &str,
) -> EvidenceRef {
    EvidenceRef {
        id: id.to_string(),
        verbatim_quote: Some(quote.to_string()),
        page_number: Some(1),
        paragraph: None,
        page_note: None,
        source_document_id: None,
        source_document_title: None,
        statement_type: Some(statement_type.to_string()),
        evidence_strength: Some(strength.to_string()),
        speaker: Some(speaker.to_string()),
        question: question.map(str::to_string),
        tier: None,
        occurrences: 1,
    }
}

fn response_with(supporting_evidence: Vec<EvidenceRef>) -> ElementDetailResponse {
    ElementDetailResponse {
        element_id: "element-1-2".to_string(),
        element_name: "Second".to_string(),
        what_plaintiff_must_prove: "prove it".to_string(),
        order_in_count: Some(2),
        count_number: Some(1),
        count_name: Some("Count I".to_string()),
        review_notes: None,
        allegations: vec![AllegationSummary {
            allegation_id: "allegation-42".to_string(),
            paragraph_number: "10".to_string(),
            summary: None,
            title: None,
            verbatim_quote: None,
            source_section: "Common",
            supporting_evidence,
            disputing_evidence: vec![supporting(
                "d-1",
                "denial",
                "sworn_party_denial",
                "George Phillips",
                None,
                "No.",
            )],
        }],
        allegation_count: 1,
        common_count: 1,
        dedicated_count: 0,
    }
}

/// The drill-down opens strongest first, with a chip on every mapped row.
///
/// This is the acceptance sentence for 1.2's drill-down, as a unit test: the
/// items are supplied in the wrong order and must come back ranked.
#[test]
fn ranking_orders_the_drill_down_strongest_first_and_stamps_its_tiers() {
    let mut response = response_with(vec![
        supporting(
            "e-hedged",
            "partial_admission",
            "sworn_party_admission",
            "George Phillips",
            Some("Did you review it?"),
            "To the best of my recollection.",
        ),
        supporting(
            "e-strong",
            "admission",
            "sworn_party_admission",
            "George Phillips",
            Some("Was it received?"),
            "Yes.",
        ),
    ]);
    rank_supporting_evidence(&mut response, &EvidenceTierMap::for_test());

    let leg = &response.allegations[0].supporting_evidence;
    let ids: Vec<&str> = leg.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, vec!["e-strong", "e-hedged"]);
    assert_eq!(leg[0].tier.as_deref(), Some("strong"));
    assert_eq!(leg[1].tier.as_deref(), Some("hedged"));
}

/// A known duplicate renders ONCE, carrying ×2 — and its twin is gone from
/// the list rather than shown twice with a marker.
#[test]
fn a_duplicate_statement_renders_once_carrying_its_count() {
    let mut response = response_with(vec![
        supporting(
            "c-9",
            "admission",
            "sworn_party_admission",
            "George Phillips",
            Some("Was correspondence received?"),
            "Correspondence was received from Marie Awad.",
        ),
        supporting(
            "c-11",
            "admission",
            "sworn_party_admission",
            "George Phillips",
            Some("Was correspondence received?"),
            "Correspondence was received from Marie Awad.",
        ),
    ]);
    rank_supporting_evidence(&mut response, &EvidenceTierMap::for_test());

    let leg = &response.allegations[0].supporting_evidence;
    assert_eq!(leg.len(), 1, "the pair collapses to one row");
    assert_eq!(leg[0].id, "c-9");
    assert_eq!(leg[0].occurrences, 2, "the row carries ×2");
}

/// Three "Yes." answers to three different interrogatories stay three rows.
///
/// The measured over-collapse trap, asserted at the layer the drill-down
/// actually renders from — not only inside `matrix_strength`, because the
/// question column has to survive the Cypher, the fold and this adapter to do
/// its job.
#[test]
fn three_distinct_yes_admissions_survive_as_three_rows() {
    let mut response = response_with(vec![
        supporting(
            "e-1",
            "admission",
            "sworn_party_admission",
            "George Phillips",
            Some("Did you receive the letter?"),
            "Yes.",
        ),
        supporting(
            "e-2",
            "admission",
            "sworn_party_admission",
            "George Phillips",
            Some("Was the auction held?"),
            "Yes.",
        ),
        supporting(
            "e-3",
            "admission",
            "sworn_party_admission",
            "George Phillips",
            Some("Did you sign it?"),
            "Yes.",
        ),
    ]);
    rank_supporting_evidence(&mut response, &EvidenceTierMap::for_test());
    assert_eq!(response.allegations[0].supporting_evidence.len(), 3);
}

/// The disputing leg is untouched: not collapsed, not ranked, not tiered.
///
/// Collapsing rebuttals would quietly reduce the number of things arguing
/// against us, which is the opposite of what an honesty pass is for.
#[test]
fn the_disputing_leg_is_left_exactly_as_it_was() {
    let mut response = response_with(vec![]);
    let before = response.allegations[0].disputing_evidence.clone();
    rank_supporting_evidence(&mut response, &EvidenceTierMap::for_test());
    assert_eq!(response.allegations[0].disputing_evidence, before);
    assert_eq!(
        response.allegations[0].disputing_evidence[0].tier, None,
        "a rebuttal carries no strength tier",
    );
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

    assert!(q.contains(&format!(
        "OPTIONAL MATCH (a)<-[:{}]-(ev)",
        schema::CORROBORATES
    )));
    assert!(q.contains(&format!("OPTIONAL MATCH (a)<-[:{}]-(dv)", schema::REBUTS)));
    // Evidence reaches an Element only THROUGH an Allegation — never a
    // direct edge, which is the traversal that produced the false
    // `to_element: 0` finding on 2026-07-26.
    assert!(!q.contains(&format!("(e)<-[:{}]-", schema::REBUTS)));
    assert!(!q.contains(&format!("(e)<-[:{}]-", schema::CORROBORATES)));
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
        tier: None,
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
