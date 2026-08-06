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
        if let Some((evidence_id, document_id)) = decode_document_row(&row)? {
            out.insert(evidence_id, document_id);
        }
    }

    Ok(out)
}

/// One (evidence, document) pair, or `None` when the statement sits in no
/// document.
///
/// Skipped rather than defaulted to an empty string: an empty document id would
/// collapse every unplaceable statement into ONE fictional shared document,
/// inflating the very count this module exists to keep honest.
fn decode_document_row(
    row: &neo4rs::Row,
) -> Result<Option<(String, String)>, ScenarioCardRepoError> {
    let evidence_id: String = column(row, "evidence_id")?;
    let document_id: Option<String> = column(row, "document_id")?;
    Ok(document_id.map(|d| (evidence_id, d)))
}

/// Read one column, naming IT — not the query — when the value is a shape this
/// build cannot read.
///
/// ## Why the column name is the operation
///
/// "Failed to decode a row during fetch_documents_for_nodes" is the same sentence
/// for every column in the query, and it leaves an operator guessing which
/// property is mistyped. "…during document_id" names the property, and on these
/// two queries that is the whole diagnosis: `evidence_id` is set by every write
/// path, while the nullable columns are where a type mismatch actually happens.
fn column<T: for<'a> serde::Deserialize<'a>>(
    row: &neo4rs::Row,
    name: &'static str,
) -> Result<T, ScenarioCardRepoError> {
    row.get(name)
        .map_err(|source| ScenarioCardRepoError::RowDecode {
            operation: name,
            source,
        })
}

/// One placed statement, as the rehearsal page needs to see it (task 2.11 B2).
///
/// ## Domain note: what this struct deliberately cannot carry
///
/// No confidence, no grounding status, no tier, no allegation links, no ruling.
/// The rehearsal page is forbidden every one of them (§10), and the cheapest way
/// to keep them off it is a query that never projects them — exclusion by
/// projection beats exclusion by discipline. Contrast `CardExtrasRow`, which
/// carries exactly that machinery because the working view needs it.
#[derive(Debug, Clone)]
pub(crate) struct RehearsalFactRow {
    pub graph_node_id: String,
    /// The statement, verbatim. `None` on a node carrying no quote — which the
    /// caller must treat as unusable rather than as an empty quote.
    pub quote: Option<String>,
    /// Who said it. `None` when the record does not record it — measured on S-2,
    /// one of forty-six.
    pub speaker: Option<String>,
    /// The extraction's kind token, humanized by `card_language` at the boundary.
    pub statement_type: Option<String>,
    /// The statement's OWN date. Never the document's — measured on this case,
    /// document titles carry years that disagree with the statements inside them.
    pub occurred_on: Option<String>,
    pub document_id: Option<String>,
    pub document_title: Option<String>,
    pub page: Option<i64>,
}

/// Build the placed-statement query.
///
/// A named function for the same reason [`documents_query`] is one: it is the
/// part of this module a test can hold without a live Neo4j, and what it must be
/// tested for — the schema edge, the optional joins, the projected names, and
/// what it does NOT project — is exactly what a test of the string can check.
fn rehearsal_facts_query() -> String {
    format!(
        "MATCH (e) WHERE e.id IN $ids \
         OPTIONAL MATCH (e)-[:{stated_by}]->(p) \
         OPTIONAL MATCH (e)-[:{contained_in}]->(d:Document) \
         RETURN e.id                AS graph_node_id, \
                e.verbatim_quote    AS quote, \
                p.name              AS speaker, \
                e.statement_type    AS statement_type, \
                coalesce(e.event_date, e.sent_date, e.statement_date) AS occurred_on, \
                d.id                AS document_id, \
                d.title             AS document_title, \
                e.page_number       AS page \
         ORDER BY occurred_on, graph_node_id",
        stated_by = schema::STATED_BY,
        contained_in = schema::CONTAINED_IN,
    )
}

/// Read the placed statements — the marked instances and their paired answers.
///
/// Only what a human PLACED is looked up, never the whole pool: the rehearsal
/// page renders only what somebody deliberately put on it, so a query that read
/// more would be loading data the page may not show.
///
/// An id the graph no longer holds is simply absent from the result. That is not
/// an error and must not be treated as one — measured on DEV, six of S-2's
/// fifty-two included refs no longer resolve. The caller renders each absence as
/// a named gap.
///
/// # Errors
/// Returns [`ScenarioCardRepoError`] if the query or a row decode fails. A read
/// that FAILED and a read that found nothing are different answers, and this
/// keeps them so.
pub(crate) async fn fetch_rehearsal_facts(
    graph: &Graph,
    ids: &[String],
) -> Result<Vec<RehearsalFactRow>, ScenarioCardRepoError> {
    const OP: &str = "fetch_rehearsal_facts";

    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut stream = graph
        .execute(query(&rehearsal_facts_query()).param("ids", ids.to_vec()))
        .await
        .map_err(|source| ScenarioCardRepoError::Query {
            operation: OP,
            source,
        })?;

    let mut out = Vec::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(|source| ScenarioCardRepoError::Query {
            operation: OP,
            source,
        })?
    {
        out.push(decode_row(&row)?);
    }

    Ok(out)
}

/// One returned row, or a named decode failure.
///
/// ## Why a wrong TYPE refuses where a missing property does not
///
/// Every nullable column decodes into an `Option`, so a property the extraction
/// never set is `Ok(None)` — a normal answer, and the caller decides what each
/// absence MEANS: no quote is unusable, no speaker is a named gap, no date is a
/// named gap that still renders the row.
///
/// A property of the WRONG TYPE is not an absence. `.unwrap_or(None)` would
/// collapse the two, and the failure would be invisible in exactly the places
/// that matter: a mistyped `quote` would make a real statement look unloadable,
/// and a mistyped `page` would silently drop the citation a witness needs to
/// produce the document. Both are data-integrity faults, and both refuse here
/// with the column named (Standing Rule 1).
fn decode_row(row: &neo4rs::Row) -> Result<RehearsalFactRow, ScenarioCardRepoError> {
    Ok(RehearsalFactRow {
        graph_node_id: column(row, "graph_node_id")?,
        quote: column(row, "quote")?,
        speaker: column(row, "speaker")?,
        statement_type: column(row, "statement_type")?,
        occurred_on: column(row, "occurred_on")?,
        document_id: column(row, "document_id")?,
        document_title: column(row, "document_title")?,
        page: column(row, "page")?,
    })
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

    /// The placed-statement query walks BOTH schema edges.
    ///
    /// Same drift risk as its sibling above, doubled: an edge type that no longer
    /// matched `schema::` would return empty columns on every row, and the page
    /// would render "Speaker not recorded" against forty-six statements whose
    /// speakers the record knows perfectly well.
    #[test]
    fn the_rehearsal_query_walks_both_schema_edges() {
        let cypher = rehearsal_facts_query();
        assert!(
            cypher.contains(&format!("-[:{}]->(p)", schema::STATED_BY)),
            "{cypher}"
        );
        assert!(
            cypher.contains(&format!("-[:{}]->(d:Document)", schema::CONTAINED_IN)),
            "{cypher}"
        );
    }

    /// Both joins are OPTIONAL, so a statement with no speaker or no document
    /// still returns.
    ///
    /// A plain MATCH would silently DROP exactly those rows — and the caller reads
    /// an absent row as "the record cannot produce this statement", so a statement
    /// whose speaker is merely unrecorded would be reported as unloadable. Measured
    /// on S-2, one of forty-six statements has no speaker; it must render with its
    /// named gap, not vanish into one.
    #[test]
    fn both_rehearsal_joins_are_optional() {
        let cypher = rehearsal_facts_query();
        assert_eq!(
            cypher.matches("OPTIONAL MATCH").count(),
            2,
            "the speaker and the document must both be optional: {cypher}"
        );
    }

    /// Every projected alias is one `decode_row` actually reads.
    ///
    /// A renamed alias is a runtime decode failure on every row at once, which no
    /// type check would find — the same seam the shared SQL projection consts
    /// carry, pinned the same way. The list below is copied from `decode_row`'s
    /// `column(row, …)` calls; if the two ever disagree this test is what says so.
    #[test]
    fn the_rehearsal_projection_names_every_column_the_decode_reads() {
        let cypher = rehearsal_facts_query();
        for column in [
            "graph_node_id",
            "quote",
            "speaker",
            "statement_type",
            "occurred_on",
            "document_id",
            "document_title",
            "page",
        ] {
            assert!(
                cypher.contains(&format!("AS {column}")),
                "the decode reads {column} but the query never projects it: {cypher}"
            );
        }
    }

    /// The date is the STATEMENT's own, and all three spellings are tried.
    ///
    /// Measured on this case, the extraction stores a statement's date under three
    /// different property names. Dropping one would silently undate a third of the
    /// timeline — and the document's date is deliberately NOT among them, because
    /// the titles carry years that disagree with the statements inside them.
    #[test]
    fn the_date_coalesces_the_statement_s_own_three_spellings_and_no_document_date() {
        let cypher = rehearsal_facts_query();
        assert!(cypher.contains("e.event_date"), "{cypher}");
        assert!(cypher.contains("e.sent_date"), "{cypher}");
        assert!(cypher.contains("e.statement_date"), "{cypher}");
        assert!(
            !cypher.contains("d.document_date"),
            "a document's date must never stand in for a statement's: {cypher}"
        );
    }

    /// The rehearsal query binds its ids too.
    #[test]
    fn the_rehearsal_ids_are_bound_and_not_interpolated() {
        assert!(rehearsal_facts_query().contains("$ids"));
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
