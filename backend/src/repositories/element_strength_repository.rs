//! Per-Element corroborating-Evidence descriptors, for the Proof Matrix's
//! strong-first numbers (task 396, P1).
//!
//! ## Why this is a second read and not two more columns on the elements query
//!
//! `causes_of_action_repository` answers every Element metric with a
//! `count(DISTINCT …)` aggregate, and its own header explains why that is
//! load-bearing: two sibling `OPTIONAL MATCH`es fan one Allegation out to one row
//! per (corroborating × disputing) COMBINATION, so anything that ACCUMULATES per
//! row rather than deduping is silently multiplied.
//!
//! The strong count cannot be a `count(DISTINCT …)`. It needs two things Cypher
//! cannot give it here: the pair→tier map, which lives in the settings store and
//! not in the graph, and the near-duplicate collapse, whose key includes a
//! normalized question and answer. Both are Rust. So this module returns the
//! DESCRIPTORS and `services::matrix_strength` decides the numbers — which is
//! also what makes the row's headline, its depth line and its drill-down
//! provably the same computation rather than three that happen to agree today.
//!
//! ## Why every hop here is a plain MATCH
//!
//! An Element with no corroborating Evidence produces no rows, and the fold hands
//! the caller an empty list for it — which tallies to `strong 0, approved 0`.
//! That is the honest reading and it costs no `OPTIONAL MATCH` fan-out.

use std::collections::HashMap;

use neo4rs::{query, Graph};

use crate::models::document_status::{
    ENTITY_ALLEGATION, ENTITY_ELEMENT, ENTITY_EVIDENCE, ENTITY_LEGAL_COUNT,
};
use crate::neo4j::schema;
use crate::services::matrix_strength::CorroboratingItem;

/// Errors raised by the strength read.
///
/// Deliberately its own type rather than reusing the causes-of-action error: the
/// two reads fail for the same reasons but the operator log must name WHICH read
/// failed, because one of them can degrade and the other cannot (see
/// `api::causes_of_action`).
#[derive(Debug, thiserror::Error)]
pub enum ElementStrengthRepoError {
    /// Neo4j request failed (network, syntax, server-side error).
    #[error("Neo4j query failed during {operation}: {source}")]
    Neo4jQuery {
        operation: &'static str,
        #[source]
        source: neo4rs::Error,
    },

    /// A row decoded at the transport layer but a column could not be
    /// deserialized into the expected Rust type.
    #[error("Failed to decode Neo4j row during {operation}: {source}")]
    Neo4jDecode {
        operation: &'static str,
        #[source]
        source: neo4rs::DeError,
    },
}

/// Build the descriptor query.
///
/// ## Rust Learning: `fn -> String` rather than a `const`
///
/// A Rust `const` cannot call `format!`, and the relationship types come from
/// `neo4j::schema` so this read stays in lockstep with one constant (Rule 16).
/// Same builder pattern as `causes_of_action_repository::elements_query`.
///
/// ## Why `ORDER BY` matters here and not in its sibling
///
/// The sibling returns aggregates, whose value does not depend on row order. This
/// returns items, and the FIRST item of a collapsed group becomes the row a human
/// reads — so an unordered stream would let the displayed quote change between
/// two loads of the same page. Ordering by element then evidence id makes the
/// lead deterministic.
fn element_strength_query() -> String {
    format!(
        "MATCH (lc)-[:{has_element}]->(el) \
       WHERE labels(lc)[0] = $count_label AND labels(el)[0] = $element_label \
     MATCH (a)-[:{bears_on}]->(el) WHERE labels(a)[0] = $allegation_label \
     MATCH (a)<-[:{corroborates}]-(ev) WHERE labels(ev)[0] = $evidence_label \
     OPTIONAL MATCH (ev)-[:{stated_by}]->(sp) \
     RETURN DISTINCT \
       el.id                 AS element_id, \
       ev.id                 AS evidence_id, \
       ev.statement_type     AS statement_type, \
       ev.evidence_strength  AS evidence_strength, \
       sp.name               AS speaker, \
       ev.question           AS question, \
       ev.verbatim_quote     AS quote \
     ORDER BY element_id, evidence_id",
        has_element = schema::HAS_ELEMENT,
        bears_on = schema::BEARS_ON,
        corroborates = schema::CORROBORATES,
        stated_by = schema::STATED_BY,
    )
}

/// Decode helper: maps neo4rs row-decode errors into the typed variant.
///
/// ## Rust Learning: returning `impl Fn(_) -> _`
///
/// `decode(op)` returns a closure capturing the operation name, so every column
/// decode reuses the same context without restating it — the idiom
/// `element_detail_fold` uses.
fn decode(operation: &'static str) -> impl Fn(neo4rs::DeError) -> ElementStrengthRepoError {
    move |source| ElementStrengthRepoError::Neo4jDecode { operation, source }
}

/// Every Element's corroborating Evidence, keyed by `Element.id`.
///
/// An Element with no corroboration is ABSENT from the map rather than present
/// with an empty vec — the caller reads it with `unwrap_or_default()`, and the
/// two readings are identical for a tally. Absence is not a gap here: the map is
/// a lookup, not a census of Elements (that is the causes-of-action read's job).
///
/// # Errors
/// Returns [`ElementStrengthRepoError`] if the query fails or a column cannot be
/// decoded. Every field except the two ids is `Option`, because the graph
/// genuinely lacks them on some nodes — documentary evidence answers no question,
/// and an Evidence node with no `STATED_BY` edge has no speaker.
pub async fn fetch_element_corroborations(
    graph: &Graph,
) -> Result<HashMap<String, Vec<CorroboratingItem>>, ElementStrengthRepoError> {
    const OP: &str = "fetch_element_corroborations";

    let q = query(&element_strength_query())
        .param("count_label", ENTITY_LEGAL_COUNT)
        .param("element_label", ENTITY_ELEMENT)
        .param("allegation_label", ENTITY_ALLEGATION)
        .param("evidence_label", ENTITY_EVIDENCE);

    let mut stream =
        graph
            .execute(q)
            .await
            .map_err(|source| ElementStrengthRepoError::Neo4jQuery {
                operation: OP,
                source,
            })?;

    let mut by_element: HashMap<String, Vec<CorroboratingItem>> = HashMap::new();
    while let Some(row) =
        stream
            .next()
            .await
            .map_err(|source| ElementStrengthRepoError::Neo4jQuery {
                operation: OP,
                source,
            })?
    {
        let element_id: String = row.get("element_id").map_err(decode(OP))?;
        let item = CorroboratingItem {
            id: row.get("evidence_id").map_err(decode(OP))?,
            statement_type: row.get("statement_type").map_err(decode(OP))?,
            evidence_strength: row.get("evidence_strength").map_err(decode(OP))?,
            speaker: row.get("speaker").map_err(decode(OP))?,
            question: row.get("question").map_err(decode(OP))?,
            quote: row.get("quote").map_err(decode(OP))?,
        };
        by_element.entry(element_id).or_default().push(item);
    }

    Ok(by_element)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The query walks the proof chain in the direction the graph models it, and
    /// names every node binding by label.
    ///
    /// House style, stated in `causes_of_action_repository`: an unlabelled binding
    /// would match any node type bearing on an Element. This is the test that says
    /// so for this read, since the query is a `format!` and a typo in it fails at
    /// Neo4j rather than at compile time.
    #[test]
    fn the_query_walks_evidence_to_allegation_to_element_by_label() {
        let q = element_strength_query();
        assert!(q.contains(&format!("-[:{}]->(el)", schema::HAS_ELEMENT)));
        assert!(q.contains(&format!("(a)-[:{}]->(el)", schema::BEARS_ON)));
        assert!(q.contains(&format!("(a)<-[:{}]-(ev)", schema::CORROBORATES)));
        for binding in [
            "labels(lc)[0] = $count_label",
            "labels(el)[0] = $element_label",
            "labels(a)[0] = $allegation_label",
            "labels(ev)[0] = $evidence_label",
        ] {
            assert!(q.contains(binding), "missing label gate: {binding}");
        }
    }

    /// The speaker hop is OPTIONAL and the rest are not.
    ///
    /// An Evidence node with no `STATED_BY` edge must still be counted — dropping
    /// it would silently lower an Element's approved figure — while an Element
    /// with no corroboration simply produces no rows.
    #[test]
    fn only_the_speaker_hop_is_optional() {
        let q = element_strength_query();
        assert!(
            q.contains(&format!(
                "OPTIONAL MATCH (ev)-[:{}]->(sp)",
                schema::STATED_BY
            )),
            "an Evidence with no speaker must not be dropped",
        );
        assert_eq!(
            q.matches("OPTIONAL MATCH").count(),
            1,
            "only the speaker hop may be optional — an optional evidence hop would \
             fan NULL rows into the fold",
        );
    }

    /// Every field the collapse key and the tier lookup need is projected.
    ///
    /// The failure this catches is quiet and severe: a missing `question` column
    /// would make every Q/A item key on speaker + answer alone, which is exactly
    /// the over-collapse `matrix_strength` was corrected for — three distinct
    /// "yes." admissions would merge and two would vanish from the count.
    #[test]
    fn the_projection_carries_every_field_the_collapse_needs() {
        let q = element_strength_query();
        for column in [
            "AS element_id",
            "AS evidence_id",
            "AS statement_type",
            "AS evidence_strength",
            "AS speaker",
            "AS question",
            "AS quote",
        ] {
            assert!(q.contains(column), "the projection is missing {column}");
        }
    }

    /// The stream is ordered, so a collapsed group's lead — the quote a human
    /// actually reads — is the same on every load.
    #[test]
    fn the_rows_are_ordered_so_the_displayed_quote_is_stable() {
        assert!(element_strength_query().contains("ORDER BY element_id, evidence_id"));
    }
}
