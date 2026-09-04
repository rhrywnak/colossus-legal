//! Keep `evidence_search` — the lexical mirror — current for ONE document.
//!
//! This is the wiring point both per-document index paths call, and the only
//! place either of them says anything about the mirror. It reads that document's
//! Evidence from the graph and hands it to
//! [`sync_document_evidence_search`](crate::repositories::pipeline_repository::sync_document_evidence_search),
//! which upserts and clears ghosts in one transaction.
//!
//! ## Why TWO callers, and why that is safe
//!
//! Roman's ruling of 2026-09-01 (option (b) on CC_REPORT_GATHER_L1C_v1's R1).
//! There are two independent per-document index implementations:
//!
//! - **Path A** — `pipeline::steps::index::run_index`, the Restate workflow's
//!   step 7.
//! - **Path B** — `api::pipeline::index::run_index_core`, behind both the
//!   `POST /documents/:id/index` route and the delta ingest's inline trigger.
//!
//! Merging them is the correct long-term fix and is an OWED item for after
//! court; doing it now would be surgery on the code that processes every
//! document, weeks before sequencing. Calling this from both is safe because the
//! sync is **idempotent and scoped to one document**: running it twice leaves
//! exactly the rows one run leaves, and it cannot touch another document's rows.
//! That idempotence is the property the ruling rests on, so it is tested rather
//! than assumed — see the L1c integration tests.
//!
//! ## ⚑ Why `services::embedding_pipeline` is NOT wired, and must not be
//!
//! **The mirror mirrors the GRAPH, not Qdrant.** `run_embedding_pipeline` — the
//! full-corpus re-embed behind the CLI, `POST /admin/embed-all` and
//! `POST /admin/reindex` — rebuilds vectors for the whole corpus and changes
//! nothing about what Neo4j holds, so it cannot make the mirror stale. Wiring it
//! would add a third writer that re-derives every row to arrive at the rows
//! already there. The corpus-wide filler is L1b's `backfill_evidence_search`
//! bin, which reads the graph directly. **Do not "fix" this by adding a third
//! call site.**
//!
//! ## Failure is loud, deliberately
//!
//! A failure here fails the index step. It does not log and carry on. A document
//! that is in Qdrant with no mirror row is half-searchable, and the only symptom
//! is a lexical search quietly missing a quote that is sitting in the graph —
//! which nobody notices until it matters in court.

use neo4rs::Graph;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::repositories::evidence_search_repository::{
    read_document_evidence, DocumentEvidence, EvidenceSearchReadError,
};
use crate::repositories::pipeline_repository::{sync_document_evidence_search, PipelineRepoError};

/// What one document's sync came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirrorSync {
    pub rows_written: u64,
    /// Rows removed because the graph no longer has that Evidence. Normally 0;
    /// non-zero means evidence was deleted or re-keyed since the last index.
    pub ghosts_removed: u64,
}

/// Why a mirror sync failed, naming the document and which half broke.
///
/// ## Rust Learning: two variants, because two very different things went wrong
///
/// A collapsed `Mirror { doc_id, message: String }` would read the same in a log
/// whether Neo4j was unreachable or Postgres rejected a row — and those want
/// different responses from an operator. Keeping the halves apart means the
/// message already says which system to look at, and `#[source]` keeps the
/// underlying error attached for the detail.
#[derive(Debug, thiserror::Error)]
pub enum MirrorSyncError {
    #[error("evidence mirror: could not READ document '{doc_id}' from the graph")]
    Read {
        doc_id: String,
        #[source]
        source: EvidenceSearchReadError,
    },
    #[error("evidence mirror: could not WRITE document '{doc_id}' to evidence_search")]
    Write {
        doc_id: String,
        #[source]
        source: PipelineRepoError,
    },
}

/// What the read told us to do about this document.
///
/// The two outcomes look similar and mean opposite things, which is why they are
/// a named type with a pure decision function rather than a `let … else` buried
/// in an async body: `Skip` touches nothing, `Sync` may DELETE every row the
/// document has. Getting them the wrong way round would clear the mirror for a
/// document id the graph has merely not heard of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncPlan {
    /// No `Document` node with this id. Nothing to sync and nothing to clear.
    Skip,
    /// The document exists. Sync it — **including when its evidence list is
    /// empty**, which is the case that clears ghost rows.
    Sync(DocumentEvidence),
}

/// Decide between skipping and syncing. Pure, so the distinction that matters
/// most here is testable without a graph.
///
/// ## Domain note: absent document vs. document with no evidence
///
/// `None` means Neo4j has no `Document` with this `source_document_id` — we know
/// nothing about it, so clearing its mirror rows would be acting on ignorance.
/// `Some` with an empty `rows` means the document exists and its Evidence is
/// gone, which is exactly when the mirror must be cleared. Collapsing the two
/// would either strand ghost rows for ever or delete rows on a typo'd id.
pub fn plan_sync(found: Option<DocumentEvidence>) -> SyncPlan {
    match found {
        None => SyncPlan::Skip,
        Some(document) => SyncPlan::Sync(document),
    }
}

/// Bring the mirror in line with the graph for one document.
///
/// Call this immediately after the document's Qdrant upsert, from every path
/// that performs one. Returns what changed, so the caller can log it.
///
/// ## The empty case is the mechanism, not an edge case
///
/// A document whose Evidence is now empty is still synced, with an empty set:
/// the upsert does nothing and the delete clears every row the mirror still had
/// for it. Skipping the call when the list is empty is exactly how ghost rows
/// would survive for ever, so there is deliberately no early return here.
///
/// A document that does not exist in the graph at all is a different state and
/// IS skipped — with a warning, because an index step running against a document
/// Neo4j has never heard of is worth someone's attention even though there is
/// nothing to sync.
///
/// # Errors
/// Returns [`MirrorSyncError`] if the graph read or the Postgres write fails.
/// The caller must propagate it — see the module doc.
pub async fn sync_document(
    graph: &Graph,
    pool: &PgPool,
    source_document_id: &str,
) -> Result<MirrorSync, MirrorSyncError> {
    let found = read_document_evidence(graph, source_document_id)
        .await
        .map_err(|source| MirrorSyncError::Read {
            doc_id: source_document_id.to_string(),
            source,
        })?;

    let document = match plan_sync(found) {
        SyncPlan::Sync(document) => document,
        SyncPlan::Skip => {
            warn!(
                doc_id = %source_document_id,
                "evidence mirror: no Document node with this source_document_id — nothing to \
                 sync and nothing to clear, so the mirror was left untouched"
            );
            return Ok(MirrorSync {
                rows_written: 0,
                ghosts_removed: 0,
            });
        }
    };

    for id in &document.skipped {
        warn!(
            doc_id = %source_document_id, evidence_id = %id,
            "evidence mirror: node has no document id or no quote — the mirror's columns are \
             NOT NULL, so it is absent from the mirror and will not be searchable"
        );
    }

    // NOT guarded by `rows.is_empty()`. See the doc comment: the empty set is
    // what clears ghost rows.
    let (rows_written, ghosts_removed) =
        sync_document_evidence_search(pool, &document.document_id, &document.rows)
            .await
            .map_err(|source| MirrorSyncError::Write {
                doc_id: source_document_id.to_string(),
                source,
            })?;

    // See the note in backfill_evidence_search::fill: a sync whose projection
    // lost `question` writes NULLs and reports exactly these same numbers, so
    // the count of rows that actually carried one is the only thing in this log
    // that can tell the two apart.
    let with_question = document
        .rows
        .iter()
        .filter(|r| r.question.is_some())
        .count();
    info!(
        doc_id = %source_document_id,
        mirror_document_id = %document.document_id,
        rows_written,
        with_question,
        ghosts_removed,
        skipped = document.skipped.len(),
        "evidence mirror synced"
    );
    Ok(MirrorSync {
        rows_written,
        ghosts_removed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::pipeline_repository::EvidenceSearchRow;

    fn document(rows: Vec<EvidenceSearchRow>) -> DocumentEvidence {
        DocumentEvidence {
            document_id: "doc-x".to_string(),
            rows,
            skipped: Vec::new(),
        }
    }

    /// A document Neo4j has never heard of is SKIPPED — the mirror is not touched.
    ///
    /// The dangerous confusion this guards: treating an unknown id as "no
    /// evidence" would run the ghost delete against it. That deletes nothing
    /// today, because nothing matches the id — but it would the moment an id is
    /// mistyped in a way that collides with a real document.
    #[test]
    fn an_absent_document_is_skipped() {
        assert_eq!(plan_sync(None), SyncPlan::Skip);
    }

    /// A document that EXISTS with no evidence is SYNCED, not skipped.
    ///
    /// This is the case that clears ghost rows, and it is the whole reason the
    /// two outcomes are kept apart. If this ever returns `Skip`, evidence deleted
    /// from the graph stays searchable in the mirror for ever.
    #[test]
    fn a_document_with_no_evidence_is_synced_not_skipped() {
        let plan = plan_sync(Some(document(Vec::new())));
        match plan {
            SyncPlan::Sync(d) => assert!(
                d.rows.is_empty(),
                "an empty row set is what clears the document"
            ),
            SyncPlan::Skip => panic!(
                "a document that exists with no evidence must be SYNCED — skipping it is \
                 how ghost rows survive for ever"
            ),
        }
    }

    /// The two error variants name which system to look at.
    ///
    /// They are separate variants precisely so a log line says "the graph" or
    /// "evidence_search" without the reader decoding a message. `thiserror`
    /// generates those strings from format attributes that nothing else checks.
    #[test]
    fn each_error_variant_names_its_document_and_its_half() {
        let read = MirrorSyncError::Read {
            doc_id: "doc-x".to_string(),
            source: EvidenceSearchReadError::Query {
                operation: "read_document_evidence",
                source: neo4rs::Error::ConnectionError,
            },
        };
        let rendered = read.to_string();
        assert!(rendered.contains("doc-x"), "got: {rendered}");
        assert!(rendered.contains("READ"), "got: {rendered}");
        assert!(rendered.contains("graph"), "got: {rendered}");

        let write = MirrorSyncError::Write {
            doc_id: "doc-y".to_string(),
            source: PipelineRepoError::Database("connection reset".to_string()),
        };
        let rendered = write.to_string();
        assert!(rendered.contains("doc-y"), "got: {rendered}");
        assert!(rendered.contains("WRITE"), "got: {rendered}");
        assert!(rendered.contains("evidence_search"), "got: {rendered}");
    }
}
