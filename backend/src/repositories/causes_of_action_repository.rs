//! Neo4j read layer for `GET /api/cases/:slug/causes-of-action`.
//!
//! ## Why two queries instead of one `collect()`
//!
//! The instruction suggested a single query that `collect()`s Elements into a
//! list-of-maps per Count. neo4rs's row decoding for a nested list-of-maps is
//! fragile (no clean `row.get` for `Vec<HashMap<..>>` of mixed types), so we
//! instead issue two **fixed** queries — one for Counts, one for all Elements —
//! and join them by `count_number` in the (DB-free, unit-tested) builder. This
//! is **two round trips total, not N+1**, and mirrors the existing
//! `case_summary_repository` pattern (`get_legal_count_details` +
//! `get_elements_per_count`).

use neo4rs::{query, Graph};

use crate::models::document_status::{ENTITY_ALLEGATION, ENTITY_ELEMENT, ENTITY_LEGAL_COUNT};

/// Errors from the causes-of-action reads. Context (operation + source) is for
/// operator logs, never the HTTP body (Standing Rule 1).
#[derive(Debug, thiserror::Error)]
pub enum CausesRepoError {
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

/// One `LegalCount` with the canonical properties the loader populated.
/// `count_name` is the node's `title`.
#[derive(Debug, Clone)]
pub(crate) struct CountRow {
    pub count_number: i64,
    pub count_name: Option<String>,
    pub burden_of_proof: Option<String>,
    pub m_civ_ji_reference: Option<String>,
    pub controlling_authorities_json: Option<String>,
    pub doctrinal_requirements_json: Option<String>,
    pub chuck_review_required: Option<bool>,
    pub chuck_review_note: Option<String>,
    pub special_note: Option<String>,
}

/// One `Element` row, tagged with its parent `count_number` for joining, and
/// its computed incoming-`PROVES_ELEMENT` count.
#[derive(Debug, Clone)]
pub(crate) struct ElementRow {
    pub count_number: i64,
    pub element_id: String,
    pub order_in_count: Option<i64>,
    pub element_name: String,
    pub what_plaintiff_must_prove: Option<String>,
    pub controlling_authority: Option<String>,
    pub theory_variant: Option<String>,
    pub allegation_count: i64,
}

// Full Cypher as named constants (no magic strings inline). Node labels are
// parameterized via the shared ENTITY_* constants; relationship types
// (HAS_ELEMENT, PROVES_ELEMENT) are fixed schema identifiers and live inside
// the query text (Cypher cannot parameterize relationship types).
const COUNTS_QUERY: &str = "MATCH (lc) WHERE labels(lc)[0] = $count_label \
     RETURN lc.count_number          AS count_number, \
            lc.title                 AS count_name, \
            lc.burden_of_proof       AS burden_of_proof, \
            lc.m_civ_ji_reference    AS m_civ_ji_reference, \
            lc.controlling_authorities_json AS controlling_authorities_json, \
            lc.doctrinal_requirements_json  AS doctrinal_requirements_json, \
            lc.chuck_review_required AS chuck_review_required, \
            lc.chuck_review_note     AS chuck_review_note, \
            lc.special_note          AS special_note \
     ORDER BY lc.count_number";

// `count(DISTINCT a)` is required: the OPTIONAL MATCH yields a null `a` for
// Elements with no Allegations, and DISTINCT collapses that to 0 (a plain
// `count(a)` would also return 0 for nulls, but DISTINCT also de-dupes an
// Allegation that proves the same Element more than once).
const ELEMENTS_QUERY: &str = "MATCH (lc)-[:HAS_ELEMENT]->(el) \
       WHERE labels(lc)[0] = $count_label AND labels(el)[0] = $element_label \
     OPTIONAL MATCH (a)-[:PROVES_ELEMENT]->(el) WHERE labels(a)[0] = $allegation_label \
     RETURN lc.count_number             AS count_number, \
            el.id                       AS element_id, \
            el.order_in_count           AS order_in_count, \
            el.element_name             AS element_name, \
            el.what_plaintiff_must_prove AS what_plaintiff_must_prove, \
            el.controlling_authority    AS controlling_authority, \
            el.theory_variant           AS theory_variant, \
            count(DISTINCT a)           AS allegation_count";

/// Map a row-decode failure to the typed error.
fn decode(operation: &'static str) -> impl Fn(neo4rs::DeError) -> CausesRepoError {
    move |source| CausesRepoError::RowDecode { operation, source }
}

/// Fetch every `LegalCount` with its canonical properties, ordered by number.
pub(crate) async fn fetch_counts(graph: &Graph) -> Result<Vec<CountRow>, CausesRepoError> {
    const OP: &str = "fetch_counts";
    let q = query(COUNTS_QUERY).param("count_label", ENTITY_LEGAL_COUNT);
    let mut stream = graph
        .execute(q)
        .await
        .map_err(|source| CausesRepoError::Query {
            operation: OP,
            source,
        })?;

    let mut rows = Vec::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(|source| CausesRepoError::Query {
            operation: OP,
            source,
        })?
    {
        rows.push(CountRow {
            count_number: row.get("count_number").map_err(decode(OP))?,
            count_name: row.get("count_name").map_err(decode(OP))?,
            burden_of_proof: row.get("burden_of_proof").map_err(decode(OP))?,
            m_civ_ji_reference: row.get("m_civ_ji_reference").map_err(decode(OP))?,
            controlling_authorities_json: row
                .get("controlling_authorities_json")
                .map_err(decode(OP))?,
            doctrinal_requirements_json: row
                .get("doctrinal_requirements_json")
                .map_err(decode(OP))?,
            chuck_review_required: row.get("chuck_review_required").map_err(decode(OP))?,
            chuck_review_note: row.get("chuck_review_note").map_err(decode(OP))?,
            special_note: row.get("special_note").map_err(decode(OP))?,
        });
    }
    Ok(rows)
}

/// Fetch all `Element`s (across every Count) with their allegation counts. The
/// builder groups them by `count_number`.
pub(crate) async fn fetch_elements(graph: &Graph) -> Result<Vec<ElementRow>, CausesRepoError> {
    const OP: &str = "fetch_elements";
    let q = query(ELEMENTS_QUERY)
        .param("count_label", ENTITY_LEGAL_COUNT)
        .param("element_label", ENTITY_ELEMENT)
        .param("allegation_label", ENTITY_ALLEGATION);
    let mut stream = graph
        .execute(q)
        .await
        .map_err(|source| CausesRepoError::Query {
            operation: OP,
            source,
        })?;

    let mut rows = Vec::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(|source| CausesRepoError::Query {
            operation: OP,
            source,
        })?
    {
        rows.push(ElementRow {
            count_number: row.get("count_number").map_err(decode(OP))?,
            element_id: row.get("element_id").map_err(decode(OP))?,
            order_in_count: row.get("order_in_count").map_err(decode(OP))?,
            element_name: row.get("element_name").map_err(decode(OP))?,
            what_plaintiff_must_prove: row.get("what_plaintiff_must_prove").map_err(decode(OP))?,
            controlling_authority: row.get("controlling_authority").map_err(decode(OP))?,
            theory_variant: row.get("theory_variant").map_err(decode(OP))?,
            allegation_count: row.get("allegation_count").map_err(decode(OP))?,
        });
    }
    Ok(rows)
}
