//! Pure shaping for the causes-of-action endpoint: join the two raw row sets
//! (Counts + Elements) into the response DTO, decoding the JSON-encoded
//! `LegalCount` properties along the way. No database access here — so this
//! logic is unit-testable without Neo4j.

use std::cmp::Ordering;
use std::collections::HashMap;

use super::causes_of_action_decode::{decode_authorities, decode_doctrinal, CausesShapeError};
use super::causes_of_action_repository::{CountRow, ElementRow};
use crate::dto::causes_of_action::{CausesOfActionResponse, CountDetail, ElementDetail};

/// Build the response from the raw rows.
///
/// Elements are grouped by `count_number` and sorted by `order_in_count`
/// ascending (Elements with no order sort last, alphabetically by name); Counts
/// are sorted by `count_number` ascending. Sorting is done here (not in Cypher)
/// so it is verifiable in a unit test.
pub(crate) fn build_causes_of_action(
    case_slug: &str,
    counts: Vec<CountRow>,
    elements: Vec<ElementRow>,
) -> Result<CausesOfActionResponse, CausesShapeError> {
    let mut elements_by_count: HashMap<i64, Vec<ElementDetail>> = HashMap::new();
    for e in elements {
        elements_by_count
            .entry(e.count_number)
            .or_default()
            .push(to_element_detail(e));
    }
    for group in elements_by_count.values_mut() {
        group.sort_by(element_order);
    }

    let mut counts: Vec<CountDetail> = counts
        .into_iter()
        .map(|c| {
            let elements = elements_by_count
                .remove(&c.count_number)
                .unwrap_or_default();
            build_count(c, elements)
        })
        .collect::<Result<_, _>>()?;
    counts.sort_by_key(|c| c.count_number);

    Ok(CausesOfActionResponse {
        case_slug: case_slug.to_string(),
        counts,
    })
}

/// Order Elements: those with an `order_in_count` first (ascending), then
/// unordered ones alphabetically by name.
fn element_order(a: &ElementDetail, b: &ElementDetail) -> Ordering {
    match (a.order_in_count, b.order_in_count) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.element_name.cmp(&b.element_name),
    }
}

/// Assemble one `CountDetail`, decoding its JSON-encoded properties.
fn build_count(c: CountRow, elements: Vec<ElementDetail>) -> Result<CountDetail, CausesShapeError> {
    let controlling_authorities =
        decode_authorities(c.count_number, &c.controlling_authorities_json)?;
    // Primary authority is the first entry's citation — the loader writes them
    // in designed (most-relevant-first) order.
    let controlling_authority_primary = controlling_authorities.first().map(|a| a.citation.clone());
    let doctrinal_requirements = decode_doctrinal(c.count_number, &c.doctrinal_requirements_json)?;

    Ok(CountDetail {
        count_number: c.count_number,
        count_name: c.count_name,
        burden_of_proof: c.burden_of_proof,
        m_civ_ji_reference: c.m_civ_ji_reference,
        controlling_authority_primary,
        controlling_authorities,
        doctrinal_requirements,
        // Absent flag → "review not required" (a meaningful default, not a
        // swallowed error).
        chuck_review_required: c.chuck_review_required.unwrap_or(false),
        chuck_review_note: c.chuck_review_note,
        special_note: c.special_note,
        elements,
    })
}

fn to_element_detail(e: ElementRow) -> ElementDetail {
    // Derive the coverage label here (DB-free) so it is unit-testable without
    // Neo4j, alongside the rest of the shaping.
    let proof_status = derive_proof_status(e.allegation_count, e.covered_allegation_count);
    ElementDetail {
        element_id: e.element_id,
        order_in_count: e.order_in_count,
        element_name: e.element_name,
        what_plaintiff_must_prove: e.what_plaintiff_must_prove,
        controlling_authority: e.controlling_authority,
        theory_variant: e.theory_variant,
        allegation_count: e.allegation_count,
        supporting_evidence_count: e.supporting_evidence_count,
        covered_allegation_count: e.covered_allegation_count,
        proof_status: proof_status.to_string(),
    }
}

/// Derive the Element's coverage label from `total` allegations (`T`) and
/// `covered` allegations (`C`, those with >=1 corroborating Evidence).
///
/// Domain note: the label reports **presence of evidence**, not legal
/// sufficiency — there is intentionally no `"proven"`. The four states are
/// mutually exclusive and total over the valid input domain (`0 <= C <= T`):
///
/// - `T == 0` → `"no_allegations"` — an Element with nothing mapped to it yet.
///   This is a *distinct* state, NOT a gap: you cannot have an evidence gap on
///   an Element that has no allegations to cover (Rule 1: distinct observables).
/// - `C == 0 && T > 0` → `"gap"` — allegations exist, none are corroborated.
/// - `0 < C < T` → `"partial"` — some allegations corroborated, some not.
/// - `C == T && T > 0` → `"supported"` — every allegation has corroboration.
///
/// ## Rust Learning: `&'static str` return for an enum-like label
///
/// The four outputs are compile-time string literals living in the binary, so
/// we return a borrowed `&'static str` (zero allocation). The caller
/// `.to_string()`s it into the owned `ElementDetail.proof_status` for the wire.
///
/// ## Behavior on impossible input
///
/// `covered` is a `count(DISTINCT …)` over a *subset* of the allegations that
/// `total` counts, so valid graph data always satisfies `0 <= C <= T`. The
/// match still totals over all `i64` pairs without panicking:
/// - `C > T` (e.g. a hypothetical double-count bug) is absorbed by the
///   `c >= t` arm → `"supported"`, the nearest honest label (everything mapped
///   is corroborated).
/// - The trailing `_` arm catches any other malformed pair (e.g. a negative
///   `covered` from a corrupt query) → `"partial"`, a benign label rather than
///   a crash.
fn derive_proof_status(total: i64, covered: i64) -> &'static str {
    match (total, covered) {
        (0, _) => "no_allegations",
        (t, 0) if t > 0 => "gap",
        (t, c) if c >= t => "supported",
        // 0 < C < T (normal partial coverage), plus the defensive catch for any
        // impossible pair the arms above don't claim — both shape to "partial".
        _ => "partial",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_row(count_number: i64) -> CountRow {
        CountRow {
            count_number,
            count_name: Some(format!("Count {count_number}")),
            burden_of_proof: Some("preponderance".into()),
            m_civ_ji_reference: None,
            controlling_authorities_json: None,
            doctrinal_requirements_json: None,
            chuck_review_required: None,
            chuck_review_note: None,
            special_note: None,
        }
    }

    fn element_row(
        count_number: i64,
        id: &str,
        name: &str,
        order: Option<i64>,
        alleg: i64,
    ) -> ElementRow {
        // Existing tests cover sorting/grouping where the evidence metrics are
        // irrelevant, so default them to 0. The proof-status / evidence flow is
        // exercised by `element_row_with_evidence` below.
        element_row_with_evidence(count_number, id, name, order, alleg, 0, 0)
    }

    #[allow(clippy::too_many_arguments)]
    fn element_row_with_evidence(
        count_number: i64,
        id: &str,
        name: &str,
        order: Option<i64>,
        alleg: i64,
        supporting: i64,
        covered: i64,
    ) -> ElementRow {
        ElementRow {
            count_number,
            element_id: id.into(),
            order_in_count: order,
            element_name: name.into(),
            what_plaintiff_must_prove: Some("prove it".into()),
            controlling_authority: None,
            theory_variant: None,
            allegation_count: alleg,
            supporting_evidence_count: supporting,
            covered_allegation_count: covered,
        }
    }

    fn build(counts: Vec<CountRow>, elements: Vec<ElementRow>) -> CausesOfActionResponse {
        build_causes_of_action("slug", counts, elements).expect("shaping succeeds")
    }

    #[test]
    fn counts_returned_in_count_number_ascending_order() {
        let counts = vec![count_row(3), count_row(1), count_row(4), count_row(2)];
        let r = build(counts, vec![]);
        let numbers: Vec<i64> = r.counts.iter().map(|c| c.count_number).collect();
        assert_eq!(numbers, vec![1, 2, 3, 4]);
    }

    #[test]
    fn elements_within_a_count_sorted_by_order_in_count() {
        let elements = vec![
            element_row(1, "e2", "Second", Some(2), 0),
            element_row(1, "e1", "First", Some(1), 0),
            element_row(1, "e3", "Third", Some(3), 0),
        ];
        let r = build(vec![count_row(1)], elements);
        let orders: Vec<Option<i64>> = r.counts[0]
            .elements
            .iter()
            .map(|e| e.order_in_count)
            .collect();
        assert_eq!(orders, vec![Some(1), Some(2), Some(3)]);
    }

    #[test]
    fn controlling_authority_primary_is_first_entry_citation() {
        let mut c = count_row(1);
        c.controlling_authorities_json = Some(
            r#"[{"citation":"First v Case","authority_type":"case","role":"r1"},
                {"citation":"Second v Case","authority_type":"case","role":"r2"},
                {"citation":"MCL 1.2.3","authority_type":"statute","role":"r3"}]"#
                .into(),
        );
        let r = build(vec![c], vec![]);
        assert_eq!(
            r.counts[0].controlling_authority_primary.as_deref(),
            Some("First v Case")
        );
    }

    #[test]
    fn json_encoded_properties_decoded_not_passed_through_as_strings() {
        let mut c = count_row(1);
        c.controlling_authorities_json = Some(
            r#"[{"citation":"C","authority_type":"case","court":"COA","year":2020,"role":"r"}]"#
                .into(),
        );
        let r = build(vec![c], vec![]);
        // Structured array, not a string: assert a decoded field value.
        assert_eq!(r.counts[0].controlling_authorities.len(), 1);
        assert_eq!(
            r.counts[0].controlling_authorities[0].authority_type,
            "case"
        );
        assert_eq!(r.counts[0].controlling_authorities[0].year, Some(2020));
    }

    #[test]
    fn count_with_no_elements_returns_empty_array_not_null() {
        let r = build(vec![count_row(1)], vec![]);
        assert!(r.counts[0].elements.is_empty());
    }

    #[test]
    fn element_with_no_proves_element_edges_has_allegation_count_zero() {
        let r = build(
            vec![count_row(1)],
            vec![element_row(1, "e1", "E", Some(1), 0)],
        );
        assert_eq!(r.counts[0].elements[0].allegation_count, 0);
    }

    #[test]
    fn element_with_three_proves_element_edges_has_allegation_count_three() {
        let r = build(
            vec![count_row(1)],
            vec![element_row(1, "e1", "E", Some(1), 3)],
        );
        assert_eq!(r.counts[0].elements[0].allegation_count, 3);
    }

    #[test]
    fn count_two_chuck_review_required_is_true() {
        let mut c = count_row(2);
        c.chuck_review_required = Some(true);
        c.chuck_review_note = Some("confirm dual theory".into());
        let r = build(vec![c], vec![]);
        assert!(r.counts[0].chuck_review_required);
        assert_eq!(
            r.counts[0].chuck_review_note.as_deref(),
            Some("confirm dual theory")
        );
    }

    #[test]
    fn count_three_special_note_present_when_set() {
        let mut c = count_row(3);
        c.special_note = Some("jurisdictional prerequisites, not tort elements".into());
        let r = build(vec![c], vec![]);
        assert_eq!(
            r.counts[0].special_note.as_deref(),
            Some("jurisdictional prerequisites, not tort elements")
        );
    }

    #[test]
    fn count_four_doctrinal_requirements_decoded_as_array() {
        let mut c = count_row(4);
        c.doctrinal_requirements_json = Some(
            r#"[{"requirement":"specificity","description":"must plead specific acts",
                 "satisfied_in_case":true,"satisfaction_evidence":"complaint paras"}]"#
                .into(),
        );
        let r = build(vec![c], vec![]);
        let reqs = r.counts[0]
            .doctrinal_requirements
            .as_ref()
            .expect("array, not null");
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].requirement, "specificity");
        assert!(reqs[0].satisfied_in_case);
        assert_eq!(reqs[0].satisfaction_evidence, "complaint paras");
    }

    #[test]
    fn absent_doctrinal_requirements_is_null_absent_authorities_is_empty_array() {
        let r = build(vec![count_row(1)], vec![]);
        assert!(r.counts[0].doctrinal_requirements.is_none());
        assert!(r.counts[0].controlling_authorities.is_empty());
        assert!(r.counts[0].controlling_authority_primary.is_none());
    }

    #[test]
    fn elements_with_missing_order_sort_after_ordered_then_alphabetically() {
        // Covers all of element_order's arms: Some<->Some, Some<->None,
        // None<->Some, None<->None (alphabetical fallback on element_name).
        let elements = vec![
            element_row(1, "e_b", "Beta", None, 0),
            element_row(1, "e2", "Ordered Two", Some(2), 0),
            element_row(1, "e_a", "Alpha", None, 0),
            element_row(1, "e1", "Ordered One", Some(1), 0),
        ];
        let r = build(vec![count_row(1)], elements);
        let names: Vec<&str> = r.counts[0]
            .elements
            .iter()
            .map(|e| e.element_name.as_str())
            .collect();
        assert_eq!(
            names,
            vec!["Ordered One", "Ordered Two", "Alpha", "Beta"],
            "ordered (asc) first, then unordered alphabetically"
        );
    }

    // ── proof_status derivation ───────────────────────────────────

    #[test]
    fn proof_status_no_allegations_when_total_zero() {
        // T == 0: no mapped allegations — a distinct state, NOT a gap.
        assert_eq!(derive_proof_status(0, 0), "no_allegations");
    }

    #[test]
    fn proof_status_gap_when_allegations_exist_but_none_covered() {
        // C == 0 && T > 0.
        assert_eq!(derive_proof_status(5, 0), "gap");
        assert_eq!(derive_proof_status(1, 0), "gap");
    }

    #[test]
    fn proof_status_partial_when_some_but_not_all_covered() {
        // 0 < C < T.
        assert_eq!(derive_proof_status(5, 2), "partial");
        assert_eq!(derive_proof_status(2, 1), "partial");
    }

    #[test]
    fn proof_status_supported_when_all_allegations_covered() {
        // C == T && T > 0.
        assert_eq!(derive_proof_status(5, 5), "supported");
        assert_eq!(derive_proof_status(1, 1), "supported");
    }

    #[test]
    fn proof_status_c_greater_than_t_absorbed_as_supported() {
        // C > T cannot arise from the DISTINCT-counted Cypher (covered is a
        // subset of total). If a future query bug produced it, the `c >= t` arm
        // claims it → "supported" (everything mapped is corroborated), not a panic.
        assert_eq!(derive_proof_status(2, 3), "supported");
    }

    #[test]
    fn proof_status_negative_covered_falls_to_partial_not_panic() {
        // The trailing `_` defensive arm: a corrupt negative `covered` is shaped
        // into the benign "partial" label rather than crashing the read.
        assert_eq!(derive_proof_status(5, -1), "partial");
    }

    #[test]
    fn evidence_metrics_and_status_flow_through_to_element_detail() {
        // 3 allegations, 2 covered by 4 distinct evidence items → "partial",
        // and the two raw counts surface verbatim on the DTO.
        let r = build(
            vec![count_row(1)],
            vec![element_row_with_evidence(1, "e1", "E", Some(1), 3, 4, 2)],
        );
        let el = &r.counts[0].elements[0];
        assert_eq!(el.allegation_count, 3);
        assert_eq!(el.supporting_evidence_count, 4);
        assert_eq!(el.covered_allegation_count, 2);
        assert_eq!(el.proof_status, "partial");
    }

    #[test]
    fn element_with_no_evidence_is_gap_with_zero_supporting() {
        let r = build(
            vec![count_row(1)],
            vec![element_row_with_evidence(1, "e1", "E", Some(1), 4, 0, 0)],
        );
        let el = &r.counts[0].elements[0];
        assert_eq!(el.supporting_evidence_count, 0);
        assert_eq!(el.proof_status, "gap");
    }

    #[test]
    fn element_with_no_allegations_is_no_allegations_status() {
        let r = build(
            vec![count_row(1)],
            vec![element_row_with_evidence(1, "e1", "E", Some(1), 0, 0, 0)],
        );
        assert_eq!(r.counts[0].elements[0].proof_status, "no_allegations");
    }

    #[test]
    fn malformed_controlling_authorities_json_surfaces_as_an_error() {
        // The decode functions own the malformed-input cases (tested directly
        // in `causes_of_action_decode`); here we pin that build_causes_of_action
        // propagates the error rather than swallowing it.
        let mut c = count_row(1);
        c.controlling_authorities_json = Some("{ this is not valid json ]".into());
        let err = build_causes_of_action("slug", vec![c], vec![]).unwrap_err();
        match err {
            CausesShapeError::DecodeAuthorities { count_number, .. } => assert_eq!(count_number, 1),
            other => panic!("expected DecodeAuthorities, got {other:?}"),
        }
    }
}
