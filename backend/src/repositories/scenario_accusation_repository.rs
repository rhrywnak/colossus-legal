//! Which document each marked statement sits in (task 2.11 B1).
//!
//! One question, one query: *for these evidence ids, what document is each one
//! contained in?* It answers the second half of Roman's sentence — "said 5 times
//! in 5 **different documents**" — and nothing else.
//!
//! ## Why this is its own tiny query and not the card pool
//!
//! The pool read (`bias::repository::all_evidence_about_subject`) returns every
//! candidate the subject touches, with quotes, speakers, pattern tags and page
//! numbers — on S-2 that is 148 rows of prose. Asking it for a count of distinct
//! documents among at most a few dozen marked instances would move a great deal of
//! text across the wire to derive one integer, on a panel that reloads after every
//! marking.
//!
//! It is also the safer shape for §10: this query CANNOT return a quote, a
//! confidence or an annotation, so nothing on the accusation read path is ever
//! holding data the rehearsal surface may not render. Exclusion by projection
//! beats exclusion by discipline.
//!
//! ## Domain note: an evidence item with no document is a real state
//!
//! `OPTIONAL MATCH`, and a row whose `document_id` is null is simply absent from
//! the returned map. The caller counts distinct KNOWN documents, so an
//! unplaceable statement undercounts rather than inventing a source — see
//! `services::scenario_accusation::AccusationState::documents_spoken_in` for why
//! that direction is the only honest one.

use std::collections::HashMap;

use neo4rs::{query, Graph};

use crate::neo4j::schema;
use crate::repositories::scenario_card_repository::ScenarioCardRepoError;

/// Build the document-lookup query.
///
/// A named function rather than an inline `format!` for the same reason
/// `card_extras_query` is one: it is the only part of this module a test can hold
/// without a live Neo4j, and what it must be tested for — that the edge type comes
/// from the schema module and that both columns are projected — is exactly what a
/// test of the returned string can check.
fn documents_query() -> String {
    format!(
        "MATCH (e) WHERE e.id IN $ids \
         OPTIONAL MATCH (e)-[:{contained_in}]->(d:Document) \
         RETURN e.id AS evidence_id, d.id AS document_id",
        contained_in = schema::CONTAINED_IN,
    )
}

/// The document each of `ids` is contained in, keyed by evidence id.
///
/// Ids the graph does not hold, and ids whose evidence sits in no document, are
/// simply absent from the map. That is not an error: re-processing moves nodes,
/// and this scenario has already been taught that lesson with 26 saved references
/// pointing at nodes that no longer resolve.
///
/// ## Rust Learning: `impl Iterator` is not used here on purpose
///
/// The obvious signature returns an iterator and lets the caller collect. It
/// cannot: the rows arrive from an async stream that borrows the connection, so
/// the values have to be owned and gathered before the stream is dropped. A
/// `HashMap` returned by value is the shape that actually survives the await.
///
/// # Errors
/// Returns [`ScenarioCardRepoError`] if the query or a row decode fails. A read
/// that FAILED and a read that found nothing are different answers, and this
/// keeps them so — the caller must not report "0 documents" for a graph it could
/// not reach.
pub(crate) async fn fetch_documents_for_nodes(
    graph: &Graph,
    ids: &[String],
) -> Result<HashMap<String, String>, ScenarioCardRepoError> {
    const OP: &str = "fetch_documents_for_nodes";

    // Nothing marked is the ordinary state of a fresh scenario, not an error —
    // skip the round trip entirely, exactly as `fetch_card_extras` does.
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut stream = graph
        .execute(query(&documents_query()).param("ids", ids.to_vec()))
        .await
        .map_err(|source| ScenarioCardRepoError::Query {
            operation: OP,
            source,
        })?;

    let mut out = HashMap::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(|source| ScenarioCardRepoError::Query {
            operation: OP,
            source,
        })?
    {
        let evidence_id: String =
            row.get("evidence_id")
                .map_err(|source| ScenarioCardRepoError::RowDecode {
                    operation: OP,
                    source,
                })?;
        // A null `document_id` decodes to `None` and is skipped. Skipped, not
        // defaulted to an empty string: an empty document id would collapse every
        // unplaceable statement into ONE fictional shared document, inflating the
        // very count this module exists to keep honest.
        let document_id: Option<String> =
            row.get("document_id")
                .map_err(|source| ScenarioCardRepoError::RowDecode {
                    operation: OP,
                    source,
                })?;

        if let Some(document_id) = document_id {
            out.insert(evidence_id, document_id);
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The query walks the schema module's edge to a Document node.
    ///
    /// The A0 inventory found five mutually inconsistent hand-assembled edge sets
    /// in this codebase; this is deliberately not the sixth. An edge type that
    /// drifted from `schema::CONTAINED_IN` would keep working right up until the
    /// constant moved, and then return an empty map — which reads on screen as "in
    /// 0 documents": a wrong number, confidently stated, with nothing to diagnose.
    #[test]
    fn the_query_walks_the_schema_edge_to_a_document() {
        let cypher = documents_query();
        assert!(
            cypher.contains(&format!("-[:{}]->(d:Document)", schema::CONTAINED_IN)),
            "{cypher}"
        );
    }

    /// The match is OPTIONAL, so an evidence item in no document still returns.
    ///
    /// A plain `MATCH` would silently drop exactly those rows. The map would look
    /// identical — an absent key either way — but the caller could no longer tell
    /// "this statement is in no document" from "this statement is not in the
    /// graph", and a future reader adding a not-found branch would be reading a
    /// lie.
    #[test]
    fn the_document_join_is_optional() {
        assert!(
            documents_query().contains("OPTIONAL MATCH"),
            "a plain MATCH would drop rows"
        );
    }

    /// Both columns are projected under the names the row decode asks for.
    ///
    /// The decode does `row.get("evidence_id")` and `row.get("document_id")`. A
    /// renamed alias is a runtime decode failure on every row at once, which no
    /// type check would find — the same seam the shared SELECT-projection consts
    /// carry on the SQL side, pinned the same way.
    #[test]
    fn the_projection_names_the_two_columns_the_decode_reads() {
        let cypher = documents_query();
        assert!(cypher.contains("AS evidence_id"), "{cypher}");
        assert!(cypher.contains("AS document_id"), "{cypher}");
    }

    /// The id list is a bound parameter, never interpolated.
    ///
    /// These ids come from stored rows a human's clicks created. Building the list
    /// into the string would put caller-supplied text into a query — the injection
    /// shape — and would also defeat the driver's plan cache on a panel that
    /// reloads after every marking.
    #[test]
    fn the_ids_are_bound_and_not_interpolated() {
        assert!(documents_query().contains("$ids"));
    }
}
