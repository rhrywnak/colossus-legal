//! Neo4j read layer for `GET /api/cases/:slug/proof-matrix/rollup`.
//!
//! A single fixed query returns, per `LegalCount`, the number of DISTINCT
//! `Allegation`s bearing on that Count's `Element`s. Rows map straight into the
//! response DTO in the handler — there is no shaping/builder step because the
//! result is already a flat, ordered list.
//!
//! ## Domain note — what `count(DISTINCT a)` per Count means, and what it does NOT
//!
//! This number is the count of DISTINCT Allegations bearing (via `BEARS_ON`) on
//! ANY Element of a Count, where the Count owns its Elements via `HAS_ELEMENT`.
//! An Allegation that bears on several Elements of the *same* Count is counted
//! **once** here — `DISTINCT a` collapses it.
//!
//! This is deliberately NOT the same number as summing the per-Element
//! allegation counts that `causes_of_action_repository::fetch_elements` returns:
//! there, the same Allegation is counted once *per Element* it touches, so its
//! contribution to a Count's total is multiplied by the number of that Count's
//! Elements it bears on. A future reader must NOT try to reconcile the two —
//! they answer different questions (Count-level reach vs. per-Element support).
//!
//! ## Domain note — mandatory match, not OPTIONAL
//!
//! The relationships are a mandatory `MATCH` (not `OPTIONAL MATCH`): a Count
//! with zero bearing Allegations produces no row and is **omitted** from the
//! response, rather than appearing with `deduped_allegations = 0`. This
//! reproduces the proven query exactly (all four current Counts have non-zero
//! totals: 51 / 41 / 19 / 34). If a future requirement needs zero-Counts shown,
//! switch to `OPTIONAL MATCH` — but that is a deliberate behavior change, not a
//! tidy-up.

use neo4rs::{query, Graph};

use crate::models::document_status::{ENTITY_ALLEGATION, ENTITY_ELEMENT, ENTITY_LEGAL_COUNT};
use crate::neo4j::schema;

/// Errors from the proof-matrix rollup read. Context (operation + source) is
/// for operator logs, never the HTTP body (Standing Rule 1).
///
/// ## Rust Learning: `thiserror` with `#[source]`
///
/// `#[derive(thiserror::Error)]` generates the `std::error::Error` impl from the
/// `#[error("…")]` format strings. A field tagged `#[source]` becomes the error
/// *cause* in the chain, so a `tracing::error!` at the handler can walk and log
/// the underlying neo4rs failure while the variant's own message names which
/// operation failed. Mirrors `causes_of_action_repository::CausesRepoError`.
#[derive(Debug, thiserror::Error)]
pub enum ProofMatrixRepoError {
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

/// One rollup row: a Count's number, id, and deduped allegation total. The
/// handler maps this 1:1 into [`crate::dto::proof_matrix::CountRollup`].
#[derive(Debug, Clone)]
pub(crate) struct RollupRow {
    pub count_number: i64,
    pub count_id: String,
    pub deduped_allegations: i64,
}

/// Build the rollup query.
///
/// ## Rust Learning: why a `fn -> String` and not a `const`
///
/// A Rust `const` must be a compile-time literal — it cannot call `format!`. To
/// keep the relationship-type names in exactly one place we interpolate the
/// `schema::*` constants, which forces an owned `String` returned from a
/// function. Cypher cannot *parameterize* a relationship type (only node
/// properties and, via `labels(x)[0] = $param`, label tests), so interpolation
/// of the trusted, code-defined `schema` constants is the established pattern
/// (see `causes_of_action_repository::elements_query`). There are no literal
/// `{ }` braces in the Cypher, so no `{{`/`}}` escaping is needed.
///
/// `count(DISTINCT a)` is load-bearing — see the module-level domain note. It
/// must not be relaxed to a plain `count(a)` or a sum.
fn rollup_query() -> String {
    format!(
        "MATCH (lc)-[:{has_element}]->(el)<-[:{bears_on}]-(a) \
       WHERE labels(lc)[0] = $count_label \
         AND labels(el)[0] = $element_label \
         AND labels(a)[0] = $allegation_label \
     RETURN lc.count_number    AS count_number, \
            lc.id              AS count_id, \
            count(DISTINCT a)  AS deduped_allegations \
     ORDER BY lc.count_number",
        has_element = schema::HAS_ELEMENT,
        bears_on = schema::BEARS_ON,
    )
}

/// Map a row-decode failure to the typed error.
fn decode(operation: &'static str) -> impl Fn(neo4rs::DeError) -> ProofMatrixRepoError {
    move |source| ProofMatrixRepoError::RowDecode { operation, source }
}

/// Fetch the per-Count deduped allegation rollup, ordered by `count_number`.
///
/// Returns one [`RollupRow`] per Count that has at least one bearing Allegation
/// (zero-Counts are omitted; see the module domain note). An empty `Vec` means
/// the graph has no such Counts — the handler treats that as 404 ("case
/// structure not loaded"), distinct from a query error which surfaces as 500.
///
/// ## Rust Learning: streaming rows with `while let Some(row) = stream.next().await?`
///
/// `graph.execute(q)` returns a `RowStream`; `.next().await` yields
/// `Result<Option<Row>, _>`. The `?` propagates a transport error as
/// [`ProofMatrixRepoError::Query`]; `Some(row)` is a row to decode, `None` ends
/// the stream. Each `row.get::<T>("alias")` is fallible (the column may be
/// missing or the wrong type), so every decode is `?`-propagated through
/// [`decode`] — no value is silently defaulted (Standing Rule 1).
pub(crate) async fn fetch_rollup(graph: &Graph) -> Result<Vec<RollupRow>, ProofMatrixRepoError> {
    const OP: &str = "fetch_rollup";
    let q = query(&rollup_query())
        .param("count_label", ENTITY_LEGAL_COUNT)
        .param("element_label", ENTITY_ELEMENT)
        .param("allegation_label", ENTITY_ALLEGATION);
    let mut stream = graph
        .execute(q)
        .await
        .map_err(|source| ProofMatrixRepoError::Query {
            operation: OP,
            source,
        })?;

    let mut rows = Vec::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(|source| ProofMatrixRepoError::Query {
            operation: OP,
            source,
        })?
    {
        rows.push(RollupRow {
            count_number: row.get("count_number").map_err(decode(OP))?,
            count_id: row.get("count_id").map_err(decode(OP))?,
            deduped_allegations: row.get("deduped_allegations").map_err(decode(OP))?,
        });
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The query is built from the shared schema constants and keeps the
    /// load-bearing aggregation. This guards two regressions a clean compile
    /// would miss: a relationship type drifting away from `neo4j::schema`
    /// (Rule 12), and `count(DISTINCT a)` being relaxed to a plain count/sum.
    ///
    /// `fetch_rollup` itself needs a live `neo4rs::Graph`, so it is exercised by
    /// the DEV verification curl, not a unit test — matching the existing
    /// `causes_of_action_repository`, which has no row-decode unit test for the
    /// same reason (a `neo4rs::Row` cannot be constructed off a live driver).
    #[test]
    fn rollup_query_uses_schema_constants_and_distinct() {
        let q = rollup_query();
        // Relationship types come from neo4j::schema, never inline literals.
        assert!(q.contains(&format!("-[:{}]->", schema::HAS_ELEMENT)));
        assert!(q.contains(&format!("<-[:{}]-", schema::BEARS_ON)));
        // The deduping aggregation is intact.
        assert!(q.contains("count(DISTINCT a)  AS deduped_allegations"));
        // Result aliases match the DTO/wire field names.
        assert!(q.contains("AS count_number"));
        assert!(q.contains("AS count_id"));
        // Deterministic ordering for the response.
        assert!(q.contains("ORDER BY lc.count_number"));
        // Labels are parameter-bound, not inline node-label literals.
        assert!(q.contains("labels(lc)[0] = $count_label"));
        assert!(q.contains("labels(el)[0] = $element_label"));
        assert!(q.contains("labels(a)[0] = $allegation_label"));
    }
}
