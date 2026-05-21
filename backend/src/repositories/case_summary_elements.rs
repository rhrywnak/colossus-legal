// Per-LegalCount Element loader for GET /case-summary.
//
// Extracted from `case_summary_repository.rs` to keep modules under the
// 300 code-line cap (CLAUDE.md §4-17). Same precedent as
// `allegation_detail_repository.rs` split from `decomposition_repository.rs`.
//
// This module exposes a single free async function consumed by the
// `CaseSummaryRepository::get_case_summary` orchestrator, plus the
// pure-helper sort/group function that backs the unit tests.

use std::collections::HashMap;

use neo4rs::{query, Graph};

use super::case_summary_repository::CaseSummaryRepositoryError;
use crate::dto::case_summary::ElementInfo;
use crate::models::document_status::{ENTITY_ALLEGATION, ENTITY_ELEMENT, ENTITY_LEGAL_COUNT};

/// Fetch every Element attached to a LegalCount in a single round trip
/// and group the result by `count_id`.
///
/// Returns a map keyed by `count_id` so the orchestrator can attach the
/// Vec to the matching [`crate::dto::case_summary::LegalCountInfo`] without
/// an N+1 query.
///
/// The Cypher is a left-join of Allegations onto Elements: each Element
/// emits exactly one row even when zero allegations prove it (a new
/// Element that hasn't been linked yet). `count(DISTINCT a)` is required —
/// the OPTIONAL MATCH would otherwise count the implicit `null` allegation
/// as 1 for empty-element rows; `count(DISTINCT a)` returns 0 when only
/// nulls are present.
///
/// Sort is done in Rust (not Cypher) so [`group_and_sort_elements`] can
/// be unit-tested independently of Neo4j.
///
/// ## Rust Learning: free fn vs method on the repository
///
/// This is a free function instead of a method on `CaseSummaryRepository`
/// because the only state it needs is the shared `Graph` handle. Keeping
/// it free makes the unit test surface trivial — the helper-under-test
/// doesn't need to construct a repository struct around the Graph. The
/// orchestrator calls it via `super::case_summary_elements::...` so the
/// dependency direction stays one-way (repository → elements helper).
pub(super) async fn get_elements_per_count(
    graph: &Graph,
) -> Result<HashMap<String, Vec<ElementInfo>>, CaseSummaryRepositoryError> {
    let mut rows: Vec<(String, ElementInfo)> = Vec::new();
    let mut result = graph
        .execute(
            query(
                "MATCH (lc)-[:HAS_ELEMENT]->(el)
                   WHERE labels(lc)[0] = $count_label
                     AND labels(el)[0] = $element_label
                 OPTIONAL MATCH (a)-[:PROVES_ELEMENT]->(el)
                   WHERE labels(a)[0] = $allegation_label
                 RETURN lc.id AS count_id,
                        el.id AS element_id,
                        el.element_name AS element_name,
                        el.title AS title,
                        el.order_in_count AS order_in_count,
                        el.controlling_authority AS controlling_authority,
                        count(DISTINCT a) AS allegation_count",
            )
            .param("count_label", ENTITY_LEGAL_COUNT)
            .param("element_label", ENTITY_ELEMENT)
            .param("allegation_label", ENTITY_ALLEGATION),
        )
        .await?;

    while let Some(row) = result.next().await? {
        if let Some(parsed) = row_to_element(&row) {
            rows.push(parsed);
        }
    }

    Ok(group_and_sort_elements(rows))
}

/// Parse a single Neo4j row into a `(count_id, ElementInfo)` pair.
///
/// Returns `None` and logs a warning when structural identity fields
/// (`count_id` or `element_id`) are missing or empty — pushing such rows
/// downstream would either lose the Element entirely (no count_id) or
/// collide React keys (empty element_id). Logging keeps the failure
/// observable instead of silently dropping (Standing Rule 1).
///
/// Extracted from `get_elements_per_count` to keep that function under
/// the 50-line cap (CLAUDE.md §4-18) and to make the row-shape contract
/// explicit at the type signature.
fn row_to_element(row: &neo4rs::Row) -> Option<(String, ElementInfo)> {
    let count_id: String = row.get("count_id").unwrap_or_default();
    let element_id: String = row.get("element_id").unwrap_or_default();

    // Structural identity guards: count_id is the join key the orchestrator
    // uses to attach Elements to their LegalCountInfo; element_id is the
    // React render key. Either being empty is a graph-shape problem worth
    // surfacing in logs (Standing Rule 1).
    if count_id.is_empty() {
        tracing::warn!(
            element_id = %element_id,
            "Element row returned with empty count_id — \
             dropping; check :HAS_ELEMENT relationships for orphaned Elements"
        );
        return None;
    }
    if element_id.is_empty() {
        tracing::warn!(
            count_id = %count_id,
            "Element row returned with empty element_id — \
             dropping; check the Element node MERGE keys in ingest_helpers.rs"
        );
        return None;
    }

    // Descriptive (non-identity) fields below — see field-level comments
    // for the best-effort rationale on the two `.ok()` calls.
    let element = ElementInfo {
        id: element_id,
        element_name: row.get("element_name").unwrap_or_default(),
        title: row.get("title").unwrap_or_default(),
        // best-effort: order metadata is absent on older extractions; sort handles None.
        order_in_count: row.get("order_in_count").ok(),
        allegation_count: row.get("allegation_count").unwrap_or(0),
        // best-effort: null until the canonical Element library lands; UI shows placeholder.
        controlling_authority: row.get("controlling_authority").ok(),
    };
    Some((count_id, element))
}

/// Group `(count_id, ElementInfo)` rows into a HashMap keyed by count_id
/// and sort each Vec by `order_in_count` ascending, with elements missing
/// an `order_in_count` sorted to the bottom and alphabetized by
/// `element_name` among themselves.
///
/// ## Rust Learning: `unwrap_or(i64::MAX)` as a "push to bottom" sort key
///
/// `Option<i64>` doesn't have a useful `Ord` for our needs — `None` sorts
/// *before* `Some(n)` in the stdlib derived ordering, which would put
/// missing-order Elements at the top. Mapping `None -> i64::MAX` flips
/// that and gives us a single `(i64, &str)` tuple key, which `sort_by`
/// can compare cheaply. The fallback `element_name` is the secondary key
/// only when the primary tied, so present-order Elements still order
/// strictly by `order_in_count` first.
///
/// ## Rust Learning: `&str` in tuples vs owned `String`
///
/// The closure builds `(i64, &str)` borrowed from each `ElementInfo` —
/// the borrowed strings live just long enough to be compared. Returning
/// owned `String` would allocate per element with no benefit.
fn group_and_sort_elements(rows: Vec<(String, ElementInfo)>) -> HashMap<String, Vec<ElementInfo>> {
    let mut grouped: HashMap<String, Vec<ElementInfo>> = HashMap::new();
    for (count_id, element) in rows {
        grouped.entry(count_id).or_default().push(element);
    }
    for elements in grouped.values_mut() {
        elements.sort_by(|a, b| {
            let a_key = (
                a.order_in_count.unwrap_or(i64::MAX),
                a.element_name.as_str(),
            );
            let b_key = (
                b.order_in_count.unwrap_or(i64::MAX),
                b.element_name.as_str(),
            );
            a_key.cmp(&b_key)
        });
    }
    grouped
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — pure helpers and serde behavior. Cypher round-trips are covered
// by the orchestrator's integration tests elsewhere; these tests lock the
// ordering contract and the null-property handling.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_element(name: &str, order: Option<i64>) -> ElementInfo {
        ElementInfo {
            id: format!("el-{name}"),
            element_name: name.to_string(),
            title: format!("Title for {name}"),
            order_in_count: order,
            allegation_count: 0,
            controlling_authority: None,
        }
    }

    #[test]
    fn elements_sort_by_order_then_name() {
        // Mixed input: present orders interleave with None, plus two None
        // entries that must alphabetize among themselves at the bottom.
        let rows = vec![
            ("count-A".to_string(), make_element("damages", Some(3))),
            ("count-A".to_string(), make_element("breach", Some(1))),
            ("count-A".to_string(), make_element("zeta", None)),
            ("count-A".to_string(), make_element("causation", Some(2))),
            ("count-A".to_string(), make_element("alpha", None)),
        ];

        let grouped = group_and_sort_elements(rows);
        let names: Vec<&str> = grouped["count-A"]
            .iter()
            .map(|e| e.element_name.as_str())
            .collect();

        assert_eq!(
            names,
            vec!["breach", "causation", "damages", "alpha", "zeta"],
            "ordered elements first (by order_in_count asc), \
             then None-ordered alphabetically at the bottom"
        );
    }

    #[test]
    fn element_grouping_attaches_to_correct_count() {
        // Two counts share zero rows in common — each Vec must contain
        // only its own Elements after grouping.
        let rows = vec![
            ("count-A".to_string(), make_element("a_first", Some(1))),
            ("count-B".to_string(), make_element("b_first", Some(1))),
            ("count-A".to_string(), make_element("a_second", Some(2))),
            ("count-B".to_string(), make_element("b_second", Some(2))),
        ];

        let grouped = group_and_sort_elements(rows);

        assert_eq!(grouped.len(), 2);
        let a_names: Vec<&str> = grouped["count-A"]
            .iter()
            .map(|e| e.element_name.as_str())
            .collect();
        let b_names: Vec<&str> = grouped["count-B"]
            .iter()
            .map(|e| e.element_name.as_str())
            .collect();
        assert_eq!(a_names, vec!["a_first", "a_second"]);
        assert_eq!(b_names, vec!["b_first", "b_second"]);
    }

    #[test]
    fn controlling_authority_round_trips_none_and_some() {
        // None must serialize away (skip_serializing_if) and deserialize
        // back to None when absent. A present empty string must round-trip
        // intact — collapsing it to None here would hide a real graph
        // value from the frontend, which has its own placeholder logic.
        let none_input = ElementInfo {
            id: "x".into(),
            element_name: "e".into(),
            title: "T".into(),
            order_in_count: None,
            allegation_count: 0,
            controlling_authority: None,
        };
        let none_json = serde_json::to_string(&none_input).unwrap();
        assert!(
            !none_json.contains("controlling_authority"),
            "None field must be skipped by serde: {none_json}"
        );

        // Re-deserialize: absent field becomes None again.
        let round_tripped: ElementInfo = serde_json::from_str(&none_json).unwrap();
        assert_eq!(round_tripped.controlling_authority, None);

        // Present empty string survives untouched.
        let empty_string_json = r#"{"id":"x","element_name":"e","title":"T",
            "order_in_count":null,"allegation_count":0,
            "controlling_authority":""}"#;
        let with_empty: ElementInfo = serde_json::from_str(empty_string_json).unwrap();
        assert_eq!(with_empty.controlling_authority, Some(String::new()));
    }

    #[test]
    fn missing_count_id_rows_are_dropped_in_grouping() {
        // The repo code logs and skips rows with empty count_id before
        // ever reaching the helper, so the helper itself never sees them.
        // This test documents that contract: the helper trusts every row
        // has a non-empty count_id and groups accordingly.
        let rows = vec![("count-A".to_string(), make_element("only", Some(1)))];
        let grouped = group_and_sort_elements(rows);
        assert_eq!(grouped.len(), 1);
        assert!(grouped.contains_key("count-A"));
    }
}
