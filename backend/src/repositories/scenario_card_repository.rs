//! Neo4j read for the per-candidate card extras (task 1.2, v2 §7).
//!
//! The candidate pool itself comes from `bias::repository` (quote, speaker,
//! document, page). This module reads the three §7 elements that pool query does
//! not project, all keyed by evidence id:
//!
//! * `statement_type` — §7.3, the kind of statement;
//! * `grounding_status` — §7.7, whether the quote was located in its source;
//! * the Evidence→Allegation links and their bears-on chain — §7.5 and §7.6.
//!
//! ## Why a separate query rather than widening the pool projection
//!
//! The pool query (`all_evidence_about_subject`) is shared with the Theme Scan
//! and the Bias Explorer, and it returns one row per Evidence. Adding the
//! allegation join to it would fan every consumer's rows out by the number of
//! allegations an item touches, and every one of them would have to learn to
//! collapse that. Reading the extras separately and joining in Rust leaves those
//! shipped paths untouched.
//!
//! ## Domain note: the stance edge IS the object
//!
//! §7.5 forbids a bare stance. The July card said "contradicts" and named nothing
//! it contradicted, so it could not be ruled on. An Evidence→Allegation edge
//! carries both halves at once — the class says the stance, the target says what
//! it is a stance ABOUT — so the card's stance line is built from the edge and
//! never from a role in isolation. A candidate with no such edge has no object,
//! and that is exactly what the payload's defer flag reports.

use neo4rs::{query, Graph, Row};

use crate::domain::case_state::partition::ConnectionTier;
use crate::models::document_status::ENTITY_ALLEGATION;
use crate::neo4j::schema;

/// Errors from the card-extras read.
#[derive(Debug, thiserror::Error)]
pub enum ScenarioCardRepoError {
    #[error("Neo4j query failed during {operation}: {source}")]
    Query {
        operation: &'static str,
        #[source]
        source: neo4rs::Error,
    },
    #[error("Failed to decode Neo4j row during {operation}: {source}")]
    RowDecode {
        operation: &'static str,
        #[source]
        source: neo4rs::DeError,
    },
}

/// One (evidence, allegation) pairing as the graph returns it.
///
/// The query fans out: an Evidence touching three Allegations yields three rows,
/// each repeating the evidence-level columns. The service collapses them. A row
/// with `edge_class: None` is an Evidence with NO allegation link at all — the
/// `OPTIONAL MATCH` keeps it in the result rather than dropping it, which matters
/// because that item is precisely the one needing a defer flag.
#[derive(Debug, Clone)]
pub(crate) struct CardExtrasRow {
    pub evidence_id: String,
    pub statement_type: Option<String>,
    pub grounding_status: Option<String>,
    /// The relationship type of the Evidence→Allegation edge, or `None` when the
    /// item links to no allegation.
    pub edge_class: Option<String>,
    pub allegation_id: Option<String>,
    pub allegation_summary: Option<String>,
    pub allegation_title: Option<String>,
    pub allegation_paragraph: Option<String>,
    pub element_name: Option<String>,
    pub count_number: Option<i64>,
    pub count_name: Option<String>,
}

/// Build the card-extras query.
///
/// ## Why the edge classes come from the partition, not from a literal list
///
/// `type(r) IN $stance_edge_types` is bound from
/// `ConnectionTier::Topical.edge_types()` — the A0 partition's public accessor —
/// rather than from a hand-written `[CORROBORATES, REBUTS, …]` here. The A0
/// inventory found five mutually inconsistent hand-assembled edge sets already in
/// this codebase; this is deliberately not the sixth. Routing through the
/// accessor also means a change to what counts as a connection reaches this card
/// automatically.
///
/// ## Rust Learning: `OPTIONAL MATCH` chains and why every join is optional
///
/// Each successive `OPTIONAL MATCH` may fail without dropping the row. An
/// Evidence with no allegation still returns (with nulls), an allegation with no
/// element still returns, and an element with no parent count still returns. That
/// is required, not defensive: the item with NO links is the one the card most
/// needs to describe, and a plain `MATCH` would silently exclude exactly it —
/// the same silent-drop that made the July failure invisible.
fn card_extras_query() -> String {
    format!(
        "MATCH (e) WHERE e.id IN $ids \
         OPTIONAL MATCH (e)-[r]->(a) \
           WHERE labels(a)[0] = $allegation_label AND type(r) IN $stance_edge_types \
         OPTIONAL MATCH (a)-[:{bears_on}]->(el) \
         OPTIONAL MATCH (lc)-[:{has_element}]->(el) \
         RETURN e.id                AS evidence_id, \
                e.statement_type    AS statement_type, \
                e.grounding_status  AS grounding_status, \
                type(r)             AS edge_class, \
                a.id                AS allegation_id, \
                a.summary           AS allegation_summary, \
                a.title             AS allegation_title, \
                a.paragraph_number  AS allegation_paragraph, \
                el.element_name     AS element_name, \
                lc.count_number     AS count_number, \
                lc.title            AS count_name \
         ORDER BY evidence_id, count_number, allegation_id",
        bears_on = schema::BEARS_ON,
        has_element = schema::HAS_ELEMENT,
    )
}

/// Read the card extras for a batch of evidence ids.
///
/// Returns one row per (evidence, allegation, element, count) combination, plus
/// one row with null link columns for every evidence that links to nothing. An
/// id that the graph no longer holds simply yields no rows — the caller treats
/// that as content-unavailable, exactly as the facts list already does.
///
/// # Errors
/// Returns [`ScenarioCardRepoError`] if the query or a row decode fails.
pub(crate) async fn fetch_card_extras(
    graph: &Graph,
    ids: &[String],
) -> Result<Vec<CardExtrasRow>, ScenarioCardRepoError> {
    const OP: &str = "fetch_card_extras";

    // An empty pool is a normal state (a scenario whose subject has no evidence
    // yet), not an error — skip the round trip entirely.
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    // The stance classes, via the partition. See `card_extras_query`'s doc for
    // why this is not a literal list.
    let stance_edge_types: Vec<String> = ConnectionTier::Topical
        .edge_types()
        .iter()
        .map(|s| s.to_string())
        .collect();

    let q = query(&card_extras_query())
        .param("ids", ids.to_vec())
        .param("allegation_label", ENTITY_ALLEGATION)
        .param("stance_edge_types", stance_edge_types);

    let mut stream = graph
        .execute(q)
        .await
        .map_err(|source| ScenarioCardRepoError::Query {
            operation: OP,
            source,
        })?;

    let mut rows = Vec::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(|source| ScenarioCardRepoError::Query {
            operation: OP,
            source,
        })?
    {
        rows.push(decode_row(&row)?);
    }
    Ok(rows)
}

/// Decode one row, naming the operation on failure.
///
/// ## Rust Learning: `.get::<Option<T>>()` versus `.get::<T>()`
///
/// Every column here is read as `Option<_>` because every one of them can be
/// null: the optional joins produce nulls by design, and `statement_type` /
/// `grounding_status` are absent on older nodes. Reading a nullable column as a
/// bare `T` would turn "this item has no page" into a DECODE ERROR — a failure
/// of the read rather than a fact about the data.
fn decode_row(row: &Row) -> Result<CardExtrasRow, ScenarioCardRepoError> {
    const OP: &str = "decode_card_extras_row";
    let decode = |source: neo4rs::DeError| ScenarioCardRepoError::RowDecode {
        operation: OP,
        source,
    };

    Ok(CardExtrasRow {
        evidence_id: row.get("evidence_id").map_err(decode)?,
        statement_type: row.get("statement_type").map_err(decode)?,
        grounding_status: row.get("grounding_status").map_err(decode)?,
        edge_class: row.get("edge_class").map_err(decode)?,
        allegation_id: row.get("allegation_id").map_err(decode)?,
        allegation_summary: row.get("allegation_summary").map_err(decode)?,
        allegation_title: row.get("allegation_title").map_err(decode)?,
        allegation_paragraph: row.get("allegation_paragraph").map_err(decode)?,
        element_name: row.get("element_name").map_err(decode)?,
        count_number: row.get("count_number").map_err(decode)?,
        count_name: row.get("count_name").map_err(decode)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The allegation join is OPTIONAL, so an unlinked item survives the query.
    ///
    /// This is the single most important property of this read. The card that
    /// needs a defer flag is exactly the one with no allegation edge; a plain
    /// `MATCH` would drop it from the result, and the workbench would show the
    /// human nothing at all rather than showing them why they cannot rule.
    #[test]
    fn an_unlinked_item_is_not_dropped_by_the_query() {
        let q = card_extras_query();
        assert!(
            q.contains("OPTIONAL MATCH (e)-[r]->(a)"),
            "the allegation join must be OPTIONAL: {q}"
        );
        // Every occurrence of the allegation join must be the OPTIONAL one. A
        // substring test for " MATCH " cannot express this — "OPTIONAL MATCH …"
        // contains it — so compare counts instead.
        assert_eq!(
            q.matches("MATCH (e)-[r]->(a)").count(),
            q.matches("OPTIONAL MATCH (e)-[r]->(a)").count(),
            "a plain MATCH here would silently drop exactly the items needing a \
             defer flag: {q}"
        );
    }

    /// The whole bears-on chain is optional too.
    #[test]
    fn the_bears_on_chain_is_optional_at_every_hop() {
        let q = card_extras_query();
        assert_eq!(
            q.matches("OPTIONAL MATCH").count(),
            3,
            "allegation, element and count must each be optional: {q}"
        );
    }

    /// The edge classes are parameterized, never written into the Cypher.
    ///
    /// Two reasons, both load-bearing: a literal list here would be the sixth
    /// hand-assembled edge set in this codebase (the A0 inventory found five), and
    /// bound parameters keep the statement one fixed prepared query.
    #[test]
    fn the_stance_edge_classes_are_bound_not_interpolated() {
        let q = card_extras_query();
        assert!(
            q.contains("type(r) IN $stance_edge_types"),
            "stance classes must be a bound parameter: {q}"
        );
        for edge in ConnectionTier::Topical.edge_types() {
            assert!(
                !q.contains(edge),
                "{edge} is interpolated into the Cypher; it must come from the \
                 partition accessor as a bound parameter instead: {q}"
            );
        }
    }

    /// Every §7 column this read owns is projected.
    #[test]
    fn the_projection_carries_every_element_this_read_owns() {
        let q = card_extras_query();
        for column in [
            "statement_type",     // §7.3
            "grounding_status",   // §7.7
            "edge_class",         // §7.5 — the stance
            "allegation_summary", // §7.5 — its object
            "element_name",       // §7.6
            "count_name",         // §7.6
        ] {
            assert!(
                q.contains(column),
                "the projection must return {column}: {q}"
            );
        }
    }

    /// Node labels are bound as parameters, not written as `:Allegation`.
    ///
    /// The house convention (`labels(x)[0] = $param`) — see
    /// `case_health_repository`'s module doc for why label literals are avoided.
    #[test]
    fn the_allegation_label_is_a_bound_parameter() {
        let q = card_extras_query();
        assert!(q.contains("labels(a)[0] = $allegation_label"));
        assert!(!q.contains(":Allegation"));
    }
}
