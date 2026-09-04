//! The graph side of the lexical mirror: read every `Evidence` node, in batches.
//!
//! **READ-ONLY.** One `MATCH … RETURN`, and a shape test asserts it can never
//! be anything else.
//!
//! ## Why this is a new read and not one of the two G0 already has
//!
//! `gate_fixture_repository` was the first place to look — note it does not
//! exist on this branch, it arrives with G0 (`chore/gate-fixtures-g0`, unmerged
//! at the time of writing) — and neither of its reads fits:
//!
//! - `significance_by_ids` returns exactly two columns — id and significance —
//!   because the gate fixture got everything else from `BiasInstance`. The
//!   mirror needs seven fields per node.
//! - `allegations_by_ids` is about `Allegation` nodes, not `Evidence`.
//!
//! The candidate that DOES exist here is
//! `BiasRepository::all_evidence_about_subject`, which
//! projects almost the right columns — but it is **scoped to one subject** and
//! it does not return `significance` at all. Widening it would change the wire
//! shape of three live endpoints (the Bias Explorer's query, the candidate
//! gather and the theme scan) to serve a backfill that runs once, which is the
//! trade the G0 report already declined for the same reason.
//!
//! So: a new read, narrow, with the seven fields the mirror stores and nothing
//! else. What was missing from every existing projection was the combination of
//! `significance` and being unscoped by subject.
//!
//! ## Why batched, and why 200
//!
//! The corpus is 1209 nodes today, and every one carries a verbatim quote that
//! can run to a paragraph. Reading them all in one statement means the whole
//! corpus's text is materialised twice at once — once in the driver's response
//! buffer and once in the `Vec` — for no benefit, and it gets worse with every
//! document Roman scans. Batching bounds the peak.
//!
//! 200 is chosen so the whole corpus is seven round trips rather than one or
//! six hundred: large enough that per-round-trip latency is irrelevant, small
//! enough that one batch is a few hundred kilobytes rather than several
//! megabytes. It is a paging size, not a tuning knob — see the constant.

mod document;

pub mod lexical;
pub use document::{read_document_evidence, DocumentEvidence};
pub use lexical::{lexical_search, party_membership, probe_counts, LexicalHits, LexicalReadError};

use neo4rs::{query, Graph};

use crate::models::document_status::ENTITY_EVIDENCE;
use crate::neo4j::schema;
use crate::repositories::pipeline_repository::EvidenceSearchRow;

/// How many Evidence nodes one graph round trip carries.
///
// STRUCTURAL: a paging size, not a per-deployment setting. It exists to bound
// peak memory while reading a corpus of unbounded size, and no environment wants
// a different number — DEV and PROD run the same tool over the same shape of
// data. Contrast the connection URL, which IS read at runtime because it varies.
const READ_BATCH: usize = 200;

/// Errors this read can raise, each naming the operation that raised it.
#[derive(Debug, thiserror::Error)]
pub enum EvidenceSearchReadError {
    #[error("evidence mirror read '{operation}' failed: {source}")]
    Query {
        operation: &'static str,
        #[source]
        source: neo4rs::Error,
    },
    #[error("evidence mirror read '{operation}' could not decode a row: {source}")]
    RowDecode {
        operation: &'static str,
        #[source]
        source: neo4rs::DeError,
    },
}

/// The paged read.
///
/// ## Why `ORDER BY e.id` before `SKIP`/`LIMIT`
///
/// Paging without a total order is not paging — Cypher makes no promise about
/// row order without `ORDER BY`, so two batches could overlap or skip nodes
/// entirely and the mirror would come out short with nothing to show for it.
/// `e.id` is unique and stable, so the pages tile the corpus exactly.
///
/// Nothing user-supplied is interpolated: the node label comes from
/// [`ENTITY_EVIDENCE`] and the relationship types from `neo4j::schema`, so a
/// rename in either reaches this query with no edit here (Rule 16). `$skip` and
/// `$limit` are real parameters.
fn page_cypher() -> String {
    format!(
        "MATCH (e:{evidence}) \
         OPTIONAL MATCH (e)-[:{contained_in}]->(d:Document) \
         OPTIONAL MATCH (e)-[:{about}]->(s) \
         WITH e, d, collect(DISTINCT s.id) AS about_ids \
         RETURN e.id AS evidence_id, \
                d.id AS document_id, \
                e.title AS title, \
                e.verbatim_quote AS quote, \
                e.question AS question, \
                e.significance AS significance, \
                e.page_number AS page, \
                about_ids AS about \
         ORDER BY evidence_id \
         SKIP $skip LIMIT $limit",
        evidence = ENTITY_EVIDENCE,
        contained_in = schema::CONTAINED_IN,
        about = schema::ABOUT,
    )
}

/// How many `Evidence` nodes the graph holds. The first of L1b's three counts.
///
/// # Errors
/// Returns [`EvidenceSearchReadError`] if the query or the decode fails.
pub async fn count_evidence_nodes(graph: &Graph) -> Result<i64, EvidenceSearchReadError> {
    const OP: &str = "count_evidence_nodes";
    let cypher = format!("MATCH (e:{ENTITY_EVIDENCE}) RETURN count(e) AS total");

    let mut stream =
        graph
            .execute(query(&cypher))
            .await
            .map_err(|source| EvidenceSearchReadError::Query {
                operation: OP,
                source,
            })?;

    // `count()` always returns exactly one row, so a missing row is a broken
    // driver rather than an empty corpus — and it is reported as such instead of
    // being folded into a comfortable zero.
    let row = stream
        .next()
        .await
        .map_err(|source| EvidenceSearchReadError::Query {
            operation: OP,
            source,
        })?
        .ok_or(EvidenceSearchReadError::Query {
            operation: OP,
            source: neo4rs::Error::UnexpectedMessage("count(e) returned no row at all".to_string()),
        })?;

    row.get("total")
        .map_err(|source| EvidenceSearchReadError::RowDecode {
            operation: OP,
            source,
        })
}

/// One graph row, before it is known whether the mirror can hold it.
///
/// A separate type from [`EvidenceSearchRow`] on purpose: this one admits the
/// two absences the mirror's `NOT NULL` columns forbid, so the decode step never
/// has to decide anything and [`mirror_row`] is a pure function over data rather
/// than a branch buried in a stream loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorCandidate {
    pub evidence_id: String,
    pub document_id: Option<String>,
    pub title: Option<String>,
    pub quote: Option<String>,
    /// The request this Evidence answers, if the node carried one. It does NOT
    /// participate in the mirrorable/unmirrorable decision below: a card with a
    /// question and no quote is still unmirrorable, because `quote` is NOT NULL
    /// and the reranker cannot score a row that has nothing to score.
    pub question: Option<String>,
    pub significance: Option<String>,
    pub page: Option<i64>,
    pub about: Vec<String>,
}

/// Decide whether a graph row can become a mirror row.
///
/// `Ok(row)` when it can; `Err(evidence_id)` when it cannot, carrying the id so
/// the caller can name it. Pure, so all three rejection cases are testable
/// without a graph — which matters precisely because none of them occurs on
/// today's corpus and so none would ever be exercised by a live test.
///
/// ## Domain note: the three ways a node cannot be mirrored
///
/// The mirror's `document_id` and `quote` columns are `NOT NULL`, and a quote of
/// nothing but whitespace is the same as no quote for every purpose this table
/// serves — the reranker cannot score it and neither index can match it.
/// Inventing an empty string for any of the three would put a row in the mirror
/// that can never be found, which is worse than a row that is honestly absent
/// and named in the log.
pub fn mirror_row(candidate: MirrorCandidate) -> Result<EvidenceSearchRow, String> {
    match (candidate.document_id, candidate.quote) {
        (Some(document_id), Some(quote)) if !quote.trim().is_empty() => Ok(EvidenceSearchRow {
            evidence_id: candidate.evidence_id,
            document_id,
            title: candidate.title,
            quote,
            question: candidate.question,
            significance: candidate.significance,
            page: candidate.page,
            about: candidate.about,
        }),
        _ => Err(candidate.evidence_id),
    }
}

/// One page of Evidence nodes, ready to hand straight to the upsert.
///
/// Returns fewer than [`READ_BATCH`] rows exactly once — on the last page — so
/// the caller's loop terminates on a short read.
///
/// ## Domain note: a node with no quote is SKIPPED, loudly
///
/// The mirror's `quote` column is `NOT NULL` by design. Rather than fail the
/// whole backfill on one bad node, or invent an empty string for it, such a node
/// is left out of the batch and named by the caller. Measured 2026-09-01: zero
/// of 1209 nodes have a null or empty `verbatim_quote`, so this path is
/// currently unreachable — it exists so that the day it becomes reachable, the
/// operator is told which node rather than handed a mirror one row short.
///
/// # Errors
/// Returns [`EvidenceSearchReadError`] if the query or a row decode fails.
pub async fn read_evidence_page(
    graph: &Graph,
    skip: i64,
) -> Result<(Vec<EvidenceSearchRow>, Vec<String>), EvidenceSearchReadError> {
    const OP: &str = "read_evidence_page";

    let mut stream = graph
        .execute(
            query(&page_cypher())
                .param("skip", skip)
                .param("limit", READ_BATCH as i64),
        )
        .await
        .map_err(|source| EvidenceSearchReadError::Query {
            operation: OP,
            source,
        })?;

    let mut rows = Vec::new();
    let mut skipped = Vec::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(|source| EvidenceSearchReadError::Query {
            operation: OP,
            source,
        })?
    {
        let decode = |source: neo4rs::DeError| EvidenceSearchReadError::RowDecode {
            operation: OP,
            source,
        };
        let evidence_id: String = row.get("evidence_id").map_err(decode)?;
        let candidate = MirrorCandidate {
            evidence_id,
            document_id: row.get("document_id").map_err(decode)?,
            title: row.get("title").map_err(decode)?,
            quote: row.get("quote").map_err(decode)?,
            question: row.get("question").map_err(decode)?,
            significance: row.get("significance").map_err(decode)?,
            page: row.get("page").map_err(decode)?,
            about: row.get("about").map_err(decode)?,
        };
        match mirror_row(candidate) {
            Ok(mirrored) => rows.push(mirrored),
            Err(unmirrorable_id) => skipped.push(unmirrorable_id),
        }
    }
    Ok((rows, skipped))
}

/// The page size, exposed so the caller can report what it used.
pub fn read_batch_size() -> usize {
    READ_BATCH
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The read cannot write. Same guard as `gate_fixture_repository`, for the
    /// same reason: "it only reads" is a claim about a file somebody may edit.
    #[test]
    fn the_page_read_cannot_write() {
        for cypher in [page_cypher(), document::document_cypher()] {
            let cypher = cypher.to_uppercase();
            for forbidden in ["CREATE", "MERGE", " SET ", "REMOVE", "DETACH"] {
                assert!(
                    !cypher.contains(forbidden),
                    "the mirror's source reads must not contain {forbidden}"
                );
            }
            // "DELETE" is checked separately: it must not appear as a Cypher
            // clause, and neither query contains the word at all.
            assert!(!cypher.contains("DELETE"));
        }
    }

    /// The projection carries exactly the eight fields the mirror stores.
    ///
    /// `question` joined them on 2026-09-04. It is the one the backfill would
    /// silently leave NULL if this test did not name it: the column exists, the
    /// upsert writes it, and a projection that never returns it would fill the
    /// mirror with rows whose generated `probe_text` and `search_vector` are the
    /// same as before — a correct-looking backfill of a blind index.
    #[test]
    fn the_projection_matches_the_mirror_columns() {
        let cypher = page_cypher();
        for column in [
            "AS evidence_id",
            "AS document_id",
            "AS title",
            "AS quote",
            "AS question",
            "AS significance",
            "AS page",
            "AS about",
        ] {
            assert!(cypher.contains(column), "the read must project {column}");
        }
    }

    /// Paging is ordered, or it is not paging.
    ///
    /// Without `ORDER BY` before `SKIP`/`LIMIT`, Cypher may return rows in any
    /// order per call, so pages could overlap or miss nodes — and the only
    /// symptom would be a mirror that is short by an unpredictable number.
    #[test]
    fn the_pages_tile_the_corpus_because_the_read_is_ordered() {
        let cypher = page_cypher();
        let order = cypher
            .find("ORDER BY evidence_id")
            .expect("the read is ordered");
        let skip = cypher.find("SKIP $skip").expect("the read is paged");
        assert!(order < skip, "ORDER BY must precede SKIP/LIMIT");
    }

    /// Label and relationship types come from the shared constants, so a schema
    /// rename reaches this query without an edit here.
    #[test]
    fn the_labels_come_from_the_shared_constants() {
        let cypher = page_cypher();
        assert!(cypher.contains(&format!("(e:{ENTITY_EVIDENCE})")));
        assert!(cypher.contains(schema::CONTAINED_IN));
        assert!(cypher.contains(schema::ABOUT));
    }

    fn candidate(document_id: Option<&str>, quote: Option<&str>) -> MirrorCandidate {
        MirrorCandidate {
            evidence_id: "doc-x:evidence:1".to_string(),
            document_id: document_id.map(str::to_string),
            title: Some("a title".to_string()),
            quote: quote.map(str::to_string),
            question: Some("Admit that it matters.".to_string()),
            significance: Some("why it matters".to_string()),
            page: Some(22),
            about: vec!["org-catholic-family-services".to_string()],
        }
    }

    /// A complete node becomes a row, with every field carried across.
    #[test]
    fn a_complete_node_becomes_a_mirror_row() {
        let row = mirror_row(candidate(
            Some("doc-x"),
            Some("the check was never deposited"),
        ))
        .expect("a complete node is mirrorable");
        assert_eq!(row.evidence_id, "doc-x:evidence:1");
        assert_eq!(row.document_id, "doc-x");
        assert_eq!(row.quote, "the check was never deposited");
        // i64 straight through: BIGINT column, no narrowing (ruling R1).
        assert_eq!(row.page, Some(22));
        assert_eq!(row.about.len(), 1);
        // The pass-through this whole change exists for. `question:
        // candidate.question` and `question: None` are indistinguishable to
        // every other test here — the projections still name the column, the
        // INSERT still lists it, and the mirror fills with NULLs.
        assert_eq!(row.question, Some("Admit that it matters.".to_string()));
    }

    /// No document id: the mirror's column is NOT NULL and the node is named.
    #[test]
    fn a_node_with_no_document_is_skipped_by_id() {
        assert_eq!(
            mirror_row(candidate(None, Some("a real quote"))),
            Err("doc-x:evidence:1".to_string())
        );
    }

    /// No quote at all: nothing to score, nothing to index.
    #[test]
    fn a_node_with_no_quote_is_skipped_by_id() {
        assert_eq!(
            mirror_row(candidate(Some("doc-x"), None)),
            Err("doc-x:evidence:1".to_string())
        );
    }

    /// A whitespace-only quote is the same as no quote, and must not slip
    /// through as a row that can never be found.
    #[test]
    fn a_whitespace_only_quote_is_skipped_by_id() {
        assert_eq!(
            mirror_row(candidate(Some("doc-x"), Some("   \n\t "))),
            Err("doc-x:evidence:1".to_string())
        );
    }

    /// A node with no ABOUT edges IS mirrorable — an empty list is a real state,
    /// unlike the two absences above.
    #[test]
    fn a_node_about_nobody_is_still_mirrorable() {
        let mut c = candidate(Some("doc-x"), Some("a quote about nobody"));
        c.about = Vec::new();
        let row = mirror_row(c).expect("no ABOUT edges is not a reason to skip");
        assert!(row.about.is_empty());
    }
}
