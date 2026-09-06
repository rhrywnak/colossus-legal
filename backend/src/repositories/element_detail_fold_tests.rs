// Tests for `element_detail_fold`.
//
// Split into a sibling file when PROOF_MATRIX_v2 widened the projection from
// eleven columns per leg to twenty: the fold itself is at the 300-line module
// limit (Rule 17), and its fixtures grew with the struct they build.

use super::*;

fn evidence(id: &str, page: i64) -> EvidenceRef {
    EvidenceRef {
        id: id.to_string(),
        verbatim_quote: None,
        page_number: Some(page),
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

/// A second push with the SAME evidence id is a no-op — the panel must not
/// render an Evidence card twice when a duplicate `CORROBORATES` edge fans
/// the same Evidence onto two rows.
#[test]
fn push_evidence_deduped_skips_duplicate_id() {
    let mut bucket: Vec<EvidenceRef> = Vec::new();
    push_evidence_deduped(&mut bucket, evidence("evidence-074", 22));
    // Same id, different page_number — still treated as the same item.
    push_evidence_deduped(&mut bucket, evidence("evidence-074", 99));
    assert_eq!(bucket.len(), 1);
    assert_eq!(bucket[0].id, "evidence-074");
    assert_eq!(bucket[0].page_number, Some(22), "first write wins");
}

/// The two legs project disjoint column sets — a leg reading the other's
/// aliases would attach disputing items to the supporting bucket, which
/// renders as a rebuttal shown under "supporting evidence": the worst
/// possible display error on a proof surface.
#[test]
fn the_two_evidence_legs_read_disjoint_columns() {
    let supporting = EvidenceLeg::Supporting.columns();
    let disputing = EvidenceLeg::Disputing.columns();
    for s in supporting {
        assert!(
            !disputing.contains(&s),
            "column `{s}` is claimed by both legs"
        );
    }
    assert_eq!(supporting.len(), disputing.len(), "legs must be parallel");
}

/// Each leg names itself in the data-gap warning, so an operator reading the
/// log can tell WHICH side is missing its source document.
#[test]
fn each_leg_labels_itself_for_the_operator_log() {
    assert_eq!(EvidenceLeg::Supporting.label(), "corroborating");
    assert_eq!(EvidenceLeg::Disputing.label(), "disputing");
}

/// Distinct evidence ids both land in the bucket, preserving insertion order.
#[test]
fn push_evidence_deduped_keeps_distinct_ids() {
    let mut bucket: Vec<EvidenceRef> = Vec::new();
    push_evidence_deduped(&mut bucket, evidence("evidence-074", 22));
    push_evidence_deduped(&mut bucket, evidence("evidence-041", 15));
    let ids: Vec<&str> = bucket.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, vec!["evidence-074", "evidence-041"]);
}
