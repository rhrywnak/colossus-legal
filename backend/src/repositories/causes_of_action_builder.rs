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
    ElementDetail {
        element_id: e.element_id,
        order_in_count: e.order_in_count,
        element_name: e.element_name,
        what_plaintiff_must_prove: e.what_plaintiff_must_prove,
        controlling_authority: e.controlling_authority,
        theory_variant: e.theory_variant,
        allegation_count: e.allegation_count,
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
        ElementRow {
            count_number,
            element_id: id.into(),
            order_in_count: order,
            element_name: name.into(),
            what_plaintiff_must_prove: Some("prove it".into()),
            controlling_authority: None,
            theory_variant: None,
            allegation_count: alleg,
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
