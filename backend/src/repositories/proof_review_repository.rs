//! Neo4j read layer for `GET /api/cases/:slug/proof-review`.
//!
//! Two fixed, read-only queries over the discovery proof edges:
//!
//! 1. [`fetch_proof_edges`] — every `Evidence -[:CORROBORATES]-> Allegation`
//!    edge, one row each (answer side + allegation side).
//! 2. [`fetch_excluded`] — every preserved *non-answer* `Evidence`
//!    (`statement_type` in the non-answer set) that produced **no**
//!    `CORROBORATES` edge.
//!
//! The page's Summary, Proof-edges, Excluded, and Borderline sub-views are all
//! shaped from these two row sets by [`super::proof_review_builder`] — the
//! repository does no grouping or filtering beyond the two queries, so the wire
//! payload can never drift from the rows the database actually returned.
//!
//! ## Read-only / encapsulation
//!
//! Both queries are pure `MATCH … RETURN` reads — no `MERGE`/`CREATE`/`SET`/
//! `DELETE`. Every relationship type and node label comes from a code-defined
//! constant (`neo4j::schema` for relationships, `models::document_status` for
//! labels and the non-answer `statement_type` set) — there are no bare schema
//! string literals in the Cypher (Rule 12). The frontend receives only the
//! labeled DTO rows the builder produces and knows none of these field names.

use neo4rs::{query, Graph, Row};

use crate::models::document_status::{
    ENTITY_ALLEGATION, ENTITY_EVIDENCE, NON_ANSWER_STATEMENT_TYPES,
};
use crate::neo4j::schema;

/// Errors from the proof-review reads. Context (operation + source) is for
/// operator logs, never the HTTP body (Standing Rule 1). Mirrors
/// `proof_matrix_repository::ProofMatrixRepoError`.
///
/// ## Rust Learning: `thiserror` with `#[source]`
///
/// `#[derive(thiserror::Error)]` generates the `std::error::Error` impl from the
/// `#[error("…")]` format strings. A field tagged `#[source]` becomes the error
/// *cause* in the chain, so the handler's `tracing::error!` can walk and log the
/// underlying neo4rs failure while the variant names which operation failed.
#[derive(Debug, thiserror::Error)]
pub enum ProofReviewRepoError {
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

/// One `CORROBORATES` edge: the discovery answer (Evidence) and the complaint
/// allegation it corroborates. Maps 1:1 into [`crate::dto::proof_review::ProofEdge`].
///
/// `statement_type` / `evidence_strength` are required (the categorization
/// backbone — a null is a real data problem, surfaced as a logged 500). Every
/// display/locator field is `Option`: a single missing field must not fail the
/// whole read, and `None` stays observable as JSON `null` (Standing Rule 1).
#[derive(Debug, Clone)]
pub(crate) struct EvidenceProofRow {
    pub statement_type: String,
    pub evidence_strength: String,
    pub answer: Option<String>,
    pub question: Option<String>,
    pub evidence_verbatim_quote: Option<String>,
    pub paragraph: Option<String>,
    pub page_number: Option<i64>,
    pub source_document: Option<String>,
    pub allegation_summary: Option<String>,
    pub allegation_title: Option<String>,
    pub allegation_paragraph_number: Option<String>,
    pub allegation_id: Option<String>,
}

/// One preserved non-answer with no `CORROBORATES` edge. Maps 1:1 into
/// [`crate::dto::proof_review::ExcludedEvidence`]. Same answer-side fields as
/// [`EvidenceProofRow`] minus the (non-existent) allegation side.
#[derive(Debug, Clone)]
pub(crate) struct ExcludedEvidenceRow {
    pub statement_type: String,
    pub answer: Option<String>,
    pub question: Option<String>,
    pub evidence_verbatim_quote: Option<String>,
    pub paragraph: Option<String>,
    pub page_number: Option<i64>,
    pub source_document: Option<String>,
}

/// Build the proof-edges query.
///
/// ## Rust Learning: `fn -> String`, not a `const`, for an interpolated query
///
/// A Rust `const` must be a compile-time literal and cannot call `format!`. To
/// keep the relationship type and node labels in exactly one place we
/// interpolate the `schema::CORROBORATES` constant (Cypher cannot *parameterize*
/// a relationship type) and bind the labels as `$param`s tested with
/// `labels(x)[0] = $param` — the same split `proof_matrix_repository` uses. No
/// literal `{ }` braces appear in the Cypher, so no `{{`/`}}` escaping is needed.
///
/// The optional document filter is `($document_id IS NULL OR e.source_document =
/// $document_id)`: when the caller binds `document_id` to NULL (no `?document_id`
/// query param) the clause is always true and every document is included; when
/// bound to a value it scopes the read to that one document. This mirrors the
/// `$actor_id IS NULL OR …` pattern in `bias::repository`.
fn proof_edges_query() -> String {
    format!(
        "MATCH (e)-[:{corroborates}]->(a) \
       WHERE labels(e)[0] = $evidence_label \
         AND labels(a)[0] = $allegation_label \
         AND ($document_id IS NULL OR e.source_document = $document_id) \
     RETURN e.statement_type    AS statement_type, \
            e.evidence_strength AS evidence_strength, \
            e.answer            AS answer, \
            e.question          AS question, \
            e.verbatim_quote    AS evidence_verbatim_quote, \
            e.paragraph         AS paragraph, \
            e.page_number       AS page_number, \
            e.source_document   AS source_document, \
            a.summary           AS allegation_summary, \
            a.title             AS allegation_title, \
            a.paragraph_number  AS allegation_paragraph_number, \
            a.id                AS allegation_id \
     ORDER BY e.source_document, e.page_number, e.paragraph",
        corroborates = schema::CORROBORATES,
    )
}

/// Build the excluded-non-answers query.
///
/// Selects `Evidence` whose `statement_type` is in the non-answer set and which
/// has **no** outgoing `CORROBORATES` edge. The target label is omitted from the
/// `NOT (e)-[:CORROBORATES]->()` predicate on purpose: `CORROBORATES` only ever
/// points at an `Allegation`, so the bare arrow is equivalent and avoids an
/// inline `:Allegation` label literal (Rule 12). The non-answer set is bound as
/// a list parameter, never spliced into the string.
fn excluded_query() -> String {
    format!(
        "MATCH (e) \
       WHERE labels(e)[0] = $evidence_label \
         AND e.statement_type IN $non_answer_types \
         AND NOT (e)-[:{corroborates}]->() \
         AND ($document_id IS NULL OR e.source_document = $document_id) \
     RETURN e.statement_type  AS statement_type, \
            e.answer          AS answer, \
            e.question        AS question, \
            e.verbatim_quote  AS evidence_verbatim_quote, \
            e.paragraph       AS paragraph, \
            e.page_number     AS page_number, \
            e.source_document AS source_document \
     ORDER BY e.source_document, e.page_number, e.paragraph",
        corroborates = schema::CORROBORATES,
    )
}

/// Map a row-decode failure to the typed error, tagging the operation.
fn decode(operation: &'static str) -> impl Fn(neo4rs::DeError) -> ProofReviewRepoError {
    move |source| ProofReviewRepoError::RowDecode { operation, source }
}

/// Decode one proof-edge row. Required fields use `String`; display/locator
/// fields use `Option`, so a missing property in the graph yields `None` (and a
/// later JSON `null`) rather than failing the read.
fn decode_proof_edge_row(
    row: &Row,
    op: &'static str,
) -> Result<EvidenceProofRow, ProofReviewRepoError> {
    Ok(EvidenceProofRow {
        statement_type: row.get("statement_type").map_err(decode(op))?,
        evidence_strength: row.get("evidence_strength").map_err(decode(op))?,
        answer: row.get("answer").map_err(decode(op))?,
        question: row.get("question").map_err(decode(op))?,
        evidence_verbatim_quote: row.get("evidence_verbatim_quote").map_err(decode(op))?,
        paragraph: row.get("paragraph").map_err(decode(op))?,
        page_number: row.get("page_number").map_err(decode(op))?,
        source_document: row.get("source_document").map_err(decode(op))?,
        allegation_summary: row.get("allegation_summary").map_err(decode(op))?,
        allegation_title: row.get("allegation_title").map_err(decode(op))?,
        allegation_paragraph_number: row.get("allegation_paragraph_number").map_err(decode(op))?,
        allegation_id: row.get("allegation_id").map_err(decode(op))?,
    })
}

/// Decode one excluded-evidence row.
fn decode_excluded_row(
    row: &Row,
    op: &'static str,
) -> Result<ExcludedEvidenceRow, ProofReviewRepoError> {
    Ok(ExcludedEvidenceRow {
        statement_type: row.get("statement_type").map_err(decode(op))?,
        answer: row.get("answer").map_err(decode(op))?,
        question: row.get("question").map_err(decode(op))?,
        evidence_verbatim_quote: row.get("evidence_verbatim_quote").map_err(decode(op))?,
        paragraph: row.get("paragraph").map_err(decode(op))?,
        page_number: row.get("page_number").map_err(decode(op))?,
        source_document: row.get("source_document").map_err(decode(op))?,
    })
}

/// Fetch every `CORROBORATES` proof edge, optionally scoped to one source
/// document.
///
/// ## Rust Learning: streaming rows with `while let Some(row) = stream.next().await?`
///
/// `graph.execute(q)` returns a `RowStream`; `.next().await` yields
/// `Result<Option<Row>, _>`. The `?` propagates a transport error as
/// [`ProofReviewRepoError::Query`]; `Some(row)` is a row to decode, `None` ends
/// the stream. `document_id: Option<&str>` binds to a Cypher NULL when `None`
/// (neo4rs maps `Option` → NULL), which is exactly what the
/// `$document_id IS NULL OR …` clause relies on.
pub(crate) async fn fetch_proof_edges(
    graph: &Graph,
    document_id: Option<&str>,
) -> Result<Vec<EvidenceProofRow>, ProofReviewRepoError> {
    const OP: &str = "fetch_proof_edges";
    let q = query(&proof_edges_query())
        .param("evidence_label", ENTITY_EVIDENCE)
        .param("allegation_label", ENTITY_ALLEGATION)
        .param("document_id", document_id);
    let mut stream = graph
        .execute(q)
        .await
        .map_err(|source| ProofReviewRepoError::Query {
            operation: OP,
            source,
        })?;

    let mut rows = Vec::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(|source| ProofReviewRepoError::Query {
            operation: OP,
            source,
        })?
    {
        rows.push(decode_proof_edge_row(&row, OP)?);
    }
    Ok(rows)
}

/// Fetch every preserved non-answer `Evidence` with no `CORROBORATES` edge,
/// optionally scoped to one source document.
pub(crate) async fn fetch_excluded(
    graph: &Graph,
    document_id: Option<&str>,
) -> Result<Vec<ExcludedEvidenceRow>, ProofReviewRepoError> {
    const OP: &str = "fetch_excluded";
    let q = query(&excluded_query())
        .param("evidence_label", ENTITY_EVIDENCE)
        // `&[&str]` → owned `Vec<&str>`, which neo4rs binds as a Cypher list for
        // the `IN $non_answer_types` test. The set lives in one constant.
        .param("non_answer_types", NON_ANSWER_STATEMENT_TYPES.to_vec())
        .param("document_id", document_id);
    let mut stream = graph
        .execute(q)
        .await
        .map_err(|source| ProofReviewRepoError::Query {
            operation: OP,
            source,
        })?;

    let mut rows = Vec::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(|source| ProofReviewRepoError::Query {
            operation: OP,
            source,
        })?
    {
        rows.push(decode_excluded_row(&row, OP)?);
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::document_status::STMT_PARTIAL_ADMISSION;

    /// The proof-edges query is built from the shared `schema`/`document_status`
    /// constants, binds the document filter, and returns the aliases the DTO and
    /// builder rely on. This guards regressions a clean compile cannot catch: a
    /// relationship type or label drifting away from the constants (Rule 12), a
    /// dropped/renamed `RETURN` alias breaking row decode, or the optional
    /// document filter being lost (which would make `?document_id=` a silent
    /// no-op). `fetch_proof_edges` itself needs a live `neo4rs::Graph`, so its
    /// row path is exercised by the DEV verification curl — matching
    /// `proof_matrix_repository`, which unit-tests the query string for the same
    /// reason (a `neo4rs::Row` cannot be constructed off a live driver).
    #[test]
    fn proof_edges_query_uses_constants_and_binds_filter() {
        let q = proof_edges_query();
        assert!(q.contains(&format!("-[:{}]->", schema::CORROBORATES)));
        // Labels are parameter-bound, never inline label literals.
        assert!(q.contains("labels(e)[0] = $evidence_label"));
        assert!(q.contains("labels(a)[0] = $allegation_label"));
        // The optional document scope is present.
        assert!(q.contains("$document_id IS NULL OR e.source_document = $document_id"));
        // Every alias the DTO/builder decode by name is produced.
        for alias in [
            "AS statement_type",
            "AS evidence_strength",
            "AS answer",
            "AS question",
            "AS evidence_verbatim_quote",
            "AS paragraph",
            "AS page_number",
            "AS source_document",
            "AS allegation_summary",
            "AS allegation_title",
            "AS allegation_paragraph_number",
            "AS allegation_id",
        ] {
            assert!(q.contains(alias), "proof-edges query missing `{alias}`");
        }
        // No write clause leaked into a read-only endpoint.
        for forbidden in ["MERGE", "CREATE", "DELETE", " SET "] {
            assert!(
                !q.contains(forbidden),
                "read query must not contain `{forbidden}`"
            );
        }
    }

    /// The excluded query binds the non-answer set as a list parameter (never a
    /// spliced literal), tests absence of a `CORROBORATES` edge, and binds the
    /// document filter. A regression here would silently change which Evidence
    /// counts as "excluded".
    #[test]
    fn excluded_query_uses_constants_and_list_param() {
        let q = excluded_query();
        assert!(q.contains("e.statement_type IN $non_answer_types"));
        assert!(q.contains(&format!("NOT (e)-[:{}]->()", schema::CORROBORATES)));
        assert!(q.contains("labels(e)[0] = $evidence_label"));
        assert!(q.contains("$document_id IS NULL OR e.source_document = $document_id"));
        // The borderline literal is never spliced into the string — it is bound
        // (here, the set) — so the partial-admission discriminator stays a
        // builder-side filter, not a hardcoded query fragment.
        assert!(!q.contains(STMT_PARTIAL_ADMISSION));
        for forbidden in ["MERGE", "CREATE", "DELETE", " SET "] {
            assert!(
                !q.contains(forbidden),
                "read query must not contain `{forbidden}`"
            );
        }
    }

    /// The `Query` variant's `Display` must carry the operation tag and name the
    /// failure as a query failure. These error strings are the only signal an
    /// operator gets when a read fails, so a typo or template change in the
    /// `#[error("…")]` attribute is a real (silent) regression — assert the
    /// interpolation actually reaches the formatted output.
    #[test]
    fn query_error_display_includes_operation() {
        let err = ProofReviewRepoError::Query {
            operation: "fetch_proof_edges",
            // Any concrete neo4rs::Error serves as the #[source]; the variant
            // name is irrelevant to what we assert (the operation tag + prefix).
            source: neo4rs::Error::UnsupportedScheme("test".to_string()),
        };
        let shown = err.to_string();
        assert!(
            shown.contains("fetch_proof_edges"),
            "missing operation: {shown}"
        );
        assert!(
            shown.contains("Neo4j query failed"),
            "missing prefix: {shown}"
        );
    }

    /// The `RowDecode` variant's `Display` must carry the operation tag and name
    /// the failure as a decode failure — same operator-signal guarantee as
    /// `query_error_display_includes_operation`.
    #[test]
    fn row_decode_error_display_includes_operation() {
        // `neo4rs::DeError` implements `serde::de::Error`, so `custom` builds one
        // without a live driver.
        use serde::de::Error as _;
        let err = ProofReviewRepoError::RowDecode {
            operation: "fetch_excluded",
            source: neo4rs::DeError::custom("bad row"),
        };
        let shown = err.to_string();
        assert!(
            shown.contains("fetch_excluded"),
            "missing operation: {shown}"
        );
        assert!(shown.contains("decode"), "missing 'decode': {shown}");
    }
}
