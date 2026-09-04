//! The per-document half of the mirror's graph read (L1c).
//!
//! **READ-ONLY**, like its sibling in `mod.rs`. That one pages the whole corpus
//! for L1b's one-time backfill; this one reads a single document, because that
//! is the unit the pipeline indexes and therefore the unit the mirror re-syncs.
//!
//! Split into its own file purely for Rule 17: the two reads together put the
//! module over 300 code lines. They share [`MirrorCandidate`] and
//! [`mirror_row`] from the parent, so the rule that decides whether a node can
//! be mirrored is still written exactly once.

use neo4rs::{query, Graph};

use super::{mirror_row, EvidenceSearchReadError, MirrorCandidate};
use crate::models::document_status::ENTITY_EVIDENCE;
use crate::neo4j::schema;
use crate::repositories::pipeline_repository::EvidenceSearchRow;

/// One document's Evidence, plus the key the mirror files it under.
///
/// ## Why the document id comes back rather than being assumed
///
/// The pipeline addresses a document by `d.source_document_id`; the mirror files
/// rows under `d.id`. Measured on DEV 2026-09-01 those are equal for all 20
/// documents — but "equal today" is not "the same field", and if they ever
/// diverge, a sync that deleted by the pipeline's id while the rows carried the
/// node's id would delete nothing and leave every ghost row in place, silently.
/// So the read returns the id the ROWS carry and the caller deletes by that.
/// The two keys cannot disagree because only one of them is ever used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentEvidence {
    /// `d.id` — the value written into `evidence_search.document_id`, and the
    /// value the ghost-row delete must be scoped by.
    pub document_id: String,
    pub rows: Vec<EvidenceSearchRow>,
    /// Nodes the mirror cannot hold, named. See [`mirror_row`].
    pub skipped: Vec<String>,
}

/// Every `Evidence` node contained in ONE document, ready to sync.
///
/// `Ok(None)` means no `Document` node matches this id at all — there is nothing
/// to sync and nothing to clear, which is a different state from a document that
/// exists and has no Evidence. That second state comes back as `Ok(Some(..))`
/// with an EMPTY `rows`, and it is the case that clears ghost rows; collapsing
/// the two would make the clearing case unreachable.
///
/// ## Rust Learning: `OPTIONAL MATCH` and the row that is all nulls
///
/// The Evidence match is OPTIONAL, so a document with no Evidence still returns
/// one row — carrying `d.id` and a null `evidence_id`. That is what lets the
/// empty case know which document to clear. A plain MATCH would return zero rows
/// and the caller could not tell "no such document" from "no evidence".
///
/// # Errors
/// Returns [`EvidenceSearchReadError`] if the query or a row decode fails.
pub async fn read_document_evidence(
    graph: &Graph,
    source_document_id: &str,
) -> Result<Option<DocumentEvidence>, EvidenceSearchReadError> {
    const OP: &str = "read_document_evidence";

    let mut stream = graph
        .execute(query(&document_cypher()).param("doc_id", source_document_id))
        .await
        .map_err(|source| EvidenceSearchReadError::Query {
            operation: OP,
            source,
        })?;

    let mut found: Option<DocumentEvidence> = None;
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
        let document_id: String = row.get("document_id").map_err(decode)?;
        let entry = found.get_or_insert_with(|| DocumentEvidence {
            document_id,
            rows: Vec::new(),
            skipped: Vec::new(),
        });

        // The all-nulls row of a document with no Evidence: it told us the
        // document exists, which is all we needed from it.
        let evidence_id: Option<String> = row.get("evidence_id").map_err(decode)?;
        let Some(evidence_id) = evidence_id else {
            continue;
        };

        let candidate = MirrorCandidate {
            evidence_id,
            document_id: Some(entry.document_id.clone()),
            title: row.get("title").map_err(decode)?,
            quote: row.get("quote").map_err(decode)?,
            question: row.get("question").map_err(decode)?,
            significance: row.get("significance").map_err(decode)?,
            page: row.get("page").map_err(decode)?,
            about: row.get("about").map_err(decode)?,
        };
        match mirror_row(candidate) {
            Ok(mirrored) => entry.rows.push(mirrored),
            Err(unmirrorable_id) => entry.skipped.push(unmirrorable_id),
        }
    }
    Ok(found)
}

/// The per-document read, filtered the way the PIPELINE addresses a document.
///
/// `d.source_document_id = $doc_id` is the same predicate
/// `embedding_repository::fetch_nodes_for_document` uses, so this read sees
/// exactly the document the index step is indexing. It projects `d.id` as the
/// mirror's key — see [`DocumentEvidence`] for why those are kept distinct.
///
/// Labels and relationship types come from the shared constants, so a rename
/// reaches this query with no edit here (Rule 16). `$doc_id` is a real parameter.
pub(super) fn document_cypher() -> String {
    format!(
        "MATCH (d:Document) WHERE d.source_document_id = $doc_id \
         OPTIONAL MATCH (e:{evidence})-[:{contained_in}]->(d) \
         OPTIONAL MATCH (e)-[:{about}]->(s) \
         WITH d, e, collect(DISTINCT s.id) AS about_ids \
         RETURN d.id AS document_id, \
                e.id AS evidence_id, \
                e.title AS title, \
                e.verbatim_quote AS quote, \
                e.question AS question, \
                e.significance AS significance, \
                e.page_number AS page, \
                about_ids AS about \
         ORDER BY evidence_id",
        evidence = ENTITY_EVIDENCE,
        contained_in = schema::CONTAINED_IN,
        about = schema::ABOUT,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The per-document read is filtered the way the PIPELINE addresses a
    /// document, and projects the key the MIRROR files rows under. Those are two
    /// different fields and this is the one place both appear.
    #[test]
    fn the_document_read_filters_on_source_document_id_and_projects_the_node_id() {
        let cypher = document_cypher();
        assert!(
            cypher.contains("d.source_document_id = $doc_id"),
            "must select the document the same way fetch_nodes_for_document does"
        );
        assert!(
            cypher.contains("d.id AS document_id"),
            "must project the id the mirror stores, so the ghost delete is scoped by it"
        );
    }

    /// The Evidence match is OPTIONAL, which is what lets a document with no
    /// Evidence still report that it exists — the case that clears ghost rows.

    #[test]
    fn a_document_with_no_evidence_still_returns_its_id() {
        let cypher = document_cypher();
        let optional = cypher
            .find("OPTIONAL MATCH (e:")
            .expect("the Evidence match must be OPTIONAL");
        let doc_match = cypher
            .find("MATCH (d:Document)")
            .expect("the document match is not optional");
        assert!(
            doc_match < optional,
            "the Document is matched first and unconditionally; the Evidence optionally"
        );
    }

    /// The per-document read must project every column the mirror row holds.
    ///
    /// `question` is the one that would go missing silently: the column exists,
    /// the upsert writes it, and a sync whose projection omits it would rewrite
    /// every row for the document with a NULL question — turning both generated
    /// columns back into what they were before the migration, on a code path
    /// that reports success.
    #[test]
    fn the_document_read_projects_every_mirror_column() {
        let cypher = document_cypher();
        for column in [
            "AS evidence_id",
            "AS title",
            "AS quote",
            "AS question",
            "AS significance",
            "AS page",
            "AS about",
        ] {
            assert!(
                cypher.contains(column),
                "the sync read must project {column}"
            );
        }
    }
}
