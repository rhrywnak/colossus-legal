//! Neo4j read layer for `GET /api/cases/:slug/case-health/inventory` (Pane 1).
//!
//! Four fixed, read-only queries. Nothing here writes, and nothing here shapes:
//! rows go to [`super::case_health_builder`], which computes the rates and
//! assembles the wire DTO as a pure function (so the arithmetic is unit-tested
//! without a live graph — the split `services::scenario_dashboard` established).
//!
//! ## Why four queries and not one
//!
//! Each answers a different question at a different grain, and fusing them would
//! force one of the four to be derived from another's rows:
//!
//! 1. [`fetch_label_counts`]    — nodes by populated label.
//! 2. [`fetch_edge_triples`]    — edges by (from-label, rel-type, to-label).
//! 3. [`fetch_corpus`]          — the corpus headline, over ALL Evidence.
//! 4. [`fetch_document_rows`]   — per-Document Evidence yield and connection.
//!
//! In particular (3) is measured over every Evidence node, NOT by summing (4).
//! An Evidence node with no `CONTAINED_IN` edge is invisible to (4) but real, and
//! an Evidence node reachable from two Documents would be double-counted by a
//! sum. The corpus query therefore also returns `evidence_without_document`, so
//! any gap between the headline and the table has a number attached rather than
//! being an unexplained discrepancy (Standing Rule 1).
//!
//! ## Why `labels(x)[0]` and not `db.labels()`
//!
//! Neo4j remembers label NAMES from earlier schema generations forever;
//! `db.labels()` lists twelve labels with zero nodes on this database. Deriving
//! the label set from the nodes themselves means a label appears if and only if
//! something carries it. `labels(x)[0]` (first label) is the established
//! convention across this codebase's reads — every `labels(n)[0] = $param`
//! filter depends on it — so this module matches rather than inventing a second
//! convention. An unlabeled node yields `null` here and is counted separately by
//! the builder, never silently dropped.
//!
//! ## Why the connection queries use pattern comprehensions
//!
//! `size([ (e)-[:CORROBORATES]->(x) WHERE labels(x)[0] = $p | x ])` counts an
//! Evidence node's edges of one class WITHOUT fanning the row out, so several
//! classes can be counted side by side in one `WITH` and then aggregated once.
//! The alternative — an `OPTIONAL MATCH` per class — multiplies rows and forces
//! every downstream aggregate to be `DISTINCT`-guarded, which is exactly the
//! kind of subtlety that silently miscounts.

use neo4rs::{query, Graph, Row};

use crate::domain::connection_tier::{PROBATIVE_EDGE_TYPES, TOPICAL_EDGE_TYPES};
use crate::models::document_status::{ENTITY_ALLEGATION, ENTITY_DOCUMENT, ENTITY_EVIDENCE};
use crate::neo4j::schema;

/// Errors from the Case Health reads. Context (operation + source) is for
/// operator logs, never the HTTP body (Standing Rule 1).
///
/// ## Rust Learning: `thiserror` with `#[source]`
///
/// `#[derive(thiserror::Error)]` generates the `std::error::Error` impl from the
/// `#[error("…")]` format strings. A field tagged `#[source]` becomes the error
/// *cause* in the chain, so the handler's `tracing::error!` can walk and log the
/// underlying neo4rs failure while the variant names which operation failed.
/// Mirrors `proof_matrix_repository::ProofMatrixRepoError`.
#[derive(Debug, thiserror::Error)]
pub enum CaseHealthRepoError {
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
    /// The corpus aggregation returned no row at all.
    ///
    /// A Cypher aggregation with no grouping key yields exactly one row even
    /// over an empty match, so this is unreachable in practice — which is
    /// precisely why it is surfaced rather than defaulted to zeros. A zeroed
    /// corpus would render as "0 Evidence, no connections", indistinguishable
    /// from a genuinely empty graph; an operator seeing that must be able to
    /// tell which it was.
    #[error("corpus aggregation returned no row (expected exactly one)")]
    MissingCorpusRow,
}

/// One populated label and its node count. `label` is `None` for unlabeled nodes.
#[derive(Debug, Clone)]
pub(crate) struct LabelCountRow {
    pub label: Option<String>,
    pub node_count: i64,
}

/// One (from-label, rel-type, to-label) triple and its edge count.
#[derive(Debug, Clone)]
pub(crate) struct EdgeTripleRow {
    pub from_label: Option<String>,
    pub rel_type: String,
    pub to_label: Option<String>,
    pub edge_count: i64,
}

/// The corpus-wide connection aggregation (exactly one row).
#[derive(Debug, Clone)]
pub(crate) struct CorpusRow {
    pub evidence_total: i64,
    pub probative_connected: i64,
    pub topical_connected: i64,
    pub evidence_without_document: i64,
}

/// One Document's Evidence yield and connection breakdown.
///
/// `by_edge_class` is parallel to
/// [`crate::domain::connection_tier::TOPICAL_EDGE_TYPES`] — same length, same
/// order — because the query builds one column per entry of that constant. The
/// builder zips the two, so a class added to the tier definition flows through
/// to the payload with no change here.
#[derive(Debug, Clone)]
pub(crate) struct DocumentRow {
    pub document_id: String,
    pub title: Option<String>,
    pub doc_type: Option<String>,
    pub evidence_total: i64,
    pub probative_connected: i64,
    pub topical_connected: i64,
    pub by_edge_class: Vec<i64>,
}

// ── Cypher construction ───────────────────────────────────────────────────────

/// The result alias holding the per-class edge count for `rel_type`.
///
/// Relationship types are `SCREAMING_SNAKE_CASE` and validated as
/// alphanumeric-plus-underscore at ingest, so lowercasing one yields a legal
/// Cypher identifier. Deriving the alias (rather than hand-listing four of them)
/// is what lets the tier constants be the single source of truth for both the
/// query's columns and the payload's columns.
fn class_alias(rel_type: &str) -> String {
    format!("n_{}", rel_type.to_lowercase())
}

/// `size([...])` expressions counting `node_var`'s edges to an Allegation, one
/// per class in `TOPICAL_EDGE_TYPES`, comma-joined for a `WITH` clause.
///
/// Each comprehension binds its own inner variable (`x_corroborates`, …) because
/// two comprehensions in one `WITH` may not reuse a name.
fn class_count_expressions(node_var: &str) -> String {
    TOPICAL_EDGE_TYPES
        .iter()
        .map(|rel| {
            let alias = class_alias(rel);
            format!(
                "size([ ({node_var})-[:{rel}]->(x_{rel}) \
                 WHERE labels(x_{rel})[0] = $allegation_label | x_{rel} ]) AS {alias}"
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// `a + b + c` over the class aliases for `edge_types` — the expression that is
/// `> 0` exactly when the node is connected at that tier.
fn tier_sum_expression(edge_types: &[&str]) -> String {
    edge_types
        .iter()
        .map(|rel| class_alias(rel))
        .collect::<Vec<_>>()
        .join(" + ")
}

/// `sum(CASE WHEN <guard> AND <expr> > 0 THEN 1 ELSE 0 END) AS <alias>` — count
/// the ROWS (i.e. Evidence items) where the expression is positive.
///
/// `guard` is `None` where the row is known to exist (the corpus query MATCHes
/// Evidence directly) and `Some("e IS NOT NULL")` where an `OPTIONAL MATCH` may
/// have produced a null Evidence for a Document that yielded none.
///
/// Domain note: this counts ITEMS, never edges. An item corroborating three
/// allegations contributes 1, which is what "how much of this document is wired
/// in?" asks.
fn item_tally(guard: Option<&str>, expr: &str, alias: &str) -> String {
    let condition = match guard {
        Some(g) => format!("{g} AND {expr} > 0"),
        None => format!("{expr} > 0"),
    };
    format!("sum(CASE WHEN {condition} THEN 1 ELSE 0 END) AS {alias}")
}

/// Corpus aggregation over EVERY Evidence node — the headline figures.
fn corpus_query() -> String {
    format!(
        "MATCH (e) WHERE labels(e)[0] = $evidence_label \
         WITH e, {classes}, \
              size([ (e)-[:{contained_in}]->(d) \
                     WHERE labels(d)[0] = $document_label | d ]) AS n_documents \
         RETURN count(e) AS evidence_total, \
                {probative}, \
                {topical}, \
                sum(CASE WHEN n_documents = 0 THEN 1 ELSE 0 END) AS evidence_without_document",
        classes = class_count_expressions("e"),
        contained_in = schema::CONTAINED_IN,
        probative = item_tally(
            None,
            &tier_sum_expression(PROBATIVE_EDGE_TYPES),
            "probative_connected"
        ),
        topical = item_tally(
            None,
            &tier_sum_expression(TOPICAL_EDGE_TYPES),
            "topical_connected"
        ),
    )
}

/// Per-Document Evidence yield and connection breakdown.
///
/// `OPTIONAL MATCH` (not `MATCH`) on the Evidence leg is load-bearing: a Document
/// that produced NO Evidence must still appear, with a row of zeros. That row is
/// a finding — a document that cost money and yielded nothing — and dropping it
/// would hide exactly what this pane exists to show.
fn documents_query() -> String {
    let per_class_tallies = TOPICAL_EDGE_TYPES
        .iter()
        .map(|rel| {
            let alias = class_alias(rel);
            item_tally(Some("e IS NOT NULL"), &alias, &format!("{alias}_items"))
        })
        .collect::<Vec<_>>()
        .join(", ");
    let per_class_returns = TOPICAL_EDGE_TYPES
        .iter()
        .map(|rel| format!("{}_items", class_alias(rel)))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "MATCH (doc) WHERE labels(doc)[0] = $document_label \
         OPTIONAL MATCH (e)-[:{contained_in}]->(doc) WHERE labels(e)[0] = $evidence_label \
         WITH doc, e, {classes} \
         WITH doc, count(e) AS evidence_total, {probative}, {topical}, {per_class_tallies} \
         RETURN doc.id AS document_id, doc.title AS title, doc.doc_type AS doc_type, \
                evidence_total, probative_connected, topical_connected, {per_class_returns} \
         ORDER BY evidence_total DESC, document_id ASC",
        contained_in = schema::CONTAINED_IN,
        classes = class_count_expressions("e"),
        probative = item_tally(
            Some("e IS NOT NULL"),
            &tier_sum_expression(PROBATIVE_EDGE_TYPES),
            "probative_connected"
        ),
        topical = item_tally(
            Some("e IS NOT NULL"),
            &tier_sum_expression(TOPICAL_EDGE_TYPES),
            "topical_connected"
        ),
    )
}

/// Node counts by populated label, most populous first.
const LABEL_COUNTS_QUERY: &str = "MATCH (n) \
     RETURN labels(n)[0] AS label, count(*) AS node_count \
     ORDER BY node_count DESC, label ASC";

/// Edge counts by (from-label, rel-type, to-label), most numerous first.
const EDGE_TRIPLES_QUERY: &str = "MATCH (a)-[r]->(b) \
     RETURN labels(a)[0] AS from_label, type(r) AS rel_type, labels(b)[0] AS to_label, \
            count(*) AS edge_count \
     ORDER BY edge_count DESC, from_label ASC, rel_type ASC, to_label ASC";

// ── Execution ─────────────────────────────────────────────────────────────────

/// Map a row-decode failure to the typed error.
fn decode(operation: &'static str) -> impl Fn(neo4rs::DeError) -> CaseHealthRepoError {
    move |source| CaseHealthRepoError::RowDecode { operation, source }
}

/// Map a transport failure to the typed error.
fn transport(operation: &'static str) -> impl Fn(neo4rs::Error) -> CaseHealthRepoError {
    move |source| CaseHealthRepoError::Query { operation, source }
}

/// Run `q` and fold every row through `map`.
///
/// ## Rust Learning: a generic helper taking `impl Fn(&Row) -> Result<T, _>`
///
/// All four reads share the same stream-and-decode shape, differing only in how
/// one row becomes one value. Passing the per-row mapper as a closure keeps that
/// loop — including the `?` propagation of BOTH the transport error and every
/// decode error (no value is ever silently defaulted) — written exactly once.
/// `T` is inferred at each call site, so the helper costs nothing at runtime:
/// the compiler monomorphizes one copy per concrete mapper.
async fn collect_rows<T, F>(
    graph: &Graph,
    operation: &'static str,
    q: neo4rs::Query,
    map: F,
) -> Result<Vec<T>, CaseHealthRepoError>
where
    F: Fn(&Row) -> Result<T, neo4rs::DeError>,
{
    let mut stream = graph.execute(q).await.map_err(transport(operation))?;
    let mut rows = Vec::new();
    while let Some(row) = stream.next().await.map_err(transport(operation))? {
        rows.push(map(&row).map_err(decode(operation))?);
    }
    Ok(rows)
}

/// Fetch node counts by populated label.
pub(crate) async fn fetch_label_counts(
    graph: &Graph,
) -> Result<Vec<LabelCountRow>, CaseHealthRepoError> {
    collect_rows(
        graph,
        "fetch_label_counts",
        query(LABEL_COUNTS_QUERY),
        |row| {
            Ok(LabelCountRow {
                label: row.get("label")?,
                node_count: row.get("node_count")?,
            })
        },
    )
    .await
}

/// Fetch edge counts by (from-label, rel-type, to-label) triple.
pub(crate) async fn fetch_edge_triples(
    graph: &Graph,
) -> Result<Vec<EdgeTripleRow>, CaseHealthRepoError> {
    collect_rows(
        graph,
        "fetch_edge_triples",
        query(EDGE_TRIPLES_QUERY),
        |row| {
            Ok(EdgeTripleRow {
                from_label: row.get("from_label")?,
                rel_type: row.get("rel_type")?,
                to_label: row.get("to_label")?,
                edge_count: row.get("edge_count")?,
            })
        },
    )
    .await
}

/// Fetch the corpus-wide connection aggregation.
///
/// # Errors
/// [`CaseHealthRepoError::MissingCorpusRow`] if the aggregation yields no row —
/// see that variant's note on why this is surfaced instead of zeroed.
pub(crate) async fn fetch_corpus(graph: &Graph) -> Result<CorpusRow, CaseHealthRepoError> {
    let q = query(&corpus_query())
        .param("evidence_label", ENTITY_EVIDENCE)
        .param("allegation_label", ENTITY_ALLEGATION)
        .param("document_label", ENTITY_DOCUMENT);
    let rows = collect_rows(graph, "fetch_corpus", q, |row| {
        Ok(CorpusRow {
            evidence_total: row.get("evidence_total")?,
            probative_connected: row.get("probative_connected")?,
            topical_connected: row.get("topical_connected")?,
            evidence_without_document: row.get("evidence_without_document")?,
        })
    })
    .await?;
    rows.into_iter()
        .next()
        .ok_or(CaseHealthRepoError::MissingCorpusRow)
}

/// Fetch one row per Document node, ordered by Evidence produced (desc).
pub(crate) async fn fetch_document_rows(
    graph: &Graph,
) -> Result<Vec<DocumentRow>, CaseHealthRepoError> {
    let q = query(&documents_query())
        .param("evidence_label", ENTITY_EVIDENCE)
        .param("allegation_label", ENTITY_ALLEGATION)
        .param("document_label", ENTITY_DOCUMENT);
    collect_rows(graph, "fetch_document_rows", q, |row| {
        let mut by_edge_class = Vec::with_capacity(TOPICAL_EDGE_TYPES.len());
        for rel in TOPICAL_EDGE_TYPES {
            by_edge_class.push(row.get(&format!("{}_items", class_alias(rel)))?);
        }
        Ok(DocumentRow {
            document_id: row.get("document_id")?,
            title: row.get("title")?,
            doc_type: row.get("doc_type")?,
            evidence_total: row.get("evidence_total")?,
            probative_connected: row.get("probative_connected")?,
            topical_connected: row.get("topical_connected")?,
            by_edge_class,
        })
    })
    .await
}

#[cfg(test)]
#[path = "case_health_repository_tests.rs"]
mod tests;
