//! backend/tests/evidence_search_l1c_integration.rs
//!
//! ⚑ L1c — the behavioural proof that the mirror stays in step with the graph.
//!
//! `#[ignore]`d and run by hand against a throwaway database carrying L1a's
//! migration, the same convention as the L1a and L1b suites.
//!
//! ```text
//! EVIDENCE_SEARCH_TEST_DATABASE_URL=postgres://…/colossus_l1c_proof \
//!   cargo test -p colossus-legal-backend \
//!     --test evidence_search_l1c_integration -- --ignored --test-threads=1
//! ```
//!
//! ## What these prove, and why it is the sync and not the paths
//!
//! Roman's ruling (option (b)) wires the SAME sync function into both
//! per-document index paths. Its safety therefore rests on one property:
//! **the sync is idempotent and scoped to one document**. If that holds, being
//! called from two paths — or twice from one — is harmless; if it does not, the
//! ruling is unsafe. So these tests hammer that property directly rather than
//! standing up two pipelines to observe it indirectly.
//!
//! The claim that each PATH calls it is proved separately and more cheaply, by
//! source assertions in `wiring` below: driving Path A needs a Restate runtime
//! and Path B needs an `AppState` carrying two pools, a graph and a Qdrant
//! client, and this project has no tier that builds either.
//!
//! ## The same refusal the L1a and L1b suites carry
//!
//! These tests WRITE, through the very function the pipeline now calls on every
//! document. [`guarded_pool`] refuses by database name, before connecting.

use sqlx::PgPool;

use colossus_legal_backend::repositories::pipeline_repository::{
    count_evidence_search_rows, sync_document_evidence_search, EvidenceSearchRow,
};

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const FORBIDDEN_DATABASES: &[&str] = &["colossus_legal", "colossus_legal_v2"];
const TEST_ID_PREFIX: &str = "__l1c_test:evidence:";
const DOC_A: &str = "__l1c_test_doc_a";
const DOC_B: &str = "__l1c_test_doc_b";

async fn guarded_pool() -> TestResult<PgPool> {
    let url = std::env::var("EVIDENCE_SEARCH_TEST_DATABASE_URL").map_err(|_| {
        "EVIDENCE_SEARCH_TEST_DATABASE_URL is not set. These tests WRITE through the same \
         sync the pipeline calls on every document — point them at a throwaway database."
    })?;
    let name = url
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .split('?')
        .next()
        .unwrap_or_default();
    if FORBIDDEN_DATABASES.contains(&name) {
        return Err(format!(
            "refusing to run: '{name}' is a real database. These tests write through the \
             sync the pipeline uses; their rows would be indistinguishable from evidence."
        )
        .into());
    }
    Ok(sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await?)
}

async fn clear(pool: &PgPool) -> TestResult<()> {
    sqlx::query("DELETE FROM evidence_search WHERE evidence_id LIKE $1")
        .bind(format!("{TEST_ID_PREFIX}%"))
        .execute(pool)
        .await?;
    Ok(())
}

fn row(document_id: &str, suffix: &str, quote: &str) -> EvidenceSearchRow {
    EvidenceSearchRow {
        evidence_id: format!("{TEST_ID_PREFIX}{suffix}"),
        document_id: document_id.to_string(),
        title: Some("Phillips admissions extract".to_string()),
        quote: quote.to_string(),
        significance: Some("why it matters".to_string()),
        page: Some(22),
        about: vec!["org-catholic-family-services".to_string()],
    }
}

/// The ids currently in the mirror for one document, sorted.
async fn ids_for(pool: &PgPool, document_id: &str) -> TestResult<Vec<String>> {
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT evidence_id FROM evidence_search WHERE document_id = $1 ORDER BY evidence_id",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await?;
    Ok(ids)
}

/// Syncing the same document twice leaves exactly the same rows.
///
/// The property Roman's ruling rests on. If this fails, calling the sync from
/// two index paths is not safe and option (b) has to be revisited.
#[tokio::test]
#[ignore = "requires EVIDENCE_SEARCH_TEST_DATABASE_URL pointing at a throwaway database"]
async fn the_sync_is_idempotent() -> TestResult<()> {
    let pool = guarded_pool().await?;
    clear(&pool).await?;

    let rows = vec![
        row(DOC_A, "a1", "CFS took custody of the $50,000 check."),
        row(DOC_A, "a2", "Richard Milster prepared the pleadings."),
    ];

    let (written, deleted) = sync_document_evidence_search(&pool, DOC_A, &rows).await?;
    assert_eq!(
        (written, deleted),
        (2, 0),
        "first sync inserts, deletes nothing"
    );
    let after_first = ids_for(&pool, DOC_A).await?;
    assert_eq!(after_first.len(), 2);

    let (written, deleted) = sync_document_evidence_search(&pool, DOC_A, &rows).await?;
    assert_eq!(
        (written, deleted),
        (2, 0),
        "a second identical sync refreshes the same two rows and deletes nothing"
    );
    assert_eq!(
        ids_for(&pool, DOC_A).await?,
        after_first,
        "the mirror must hold exactly the same rows after the second sync"
    );

    clear(&pool).await?;
    Ok(())
}

/// Path A then Path B leaves the same rows as either alone.
///
/// This is the ruling's safety condition stated in the ruling's own terms: the
/// two paths call one function, so "A then B" is just "the sync, twice" — and
/// the assertion is that the mirror cannot tell the difference.
#[tokio::test]
#[ignore = "requires EVIDENCE_SEARCH_TEST_DATABASE_URL pointing at a throwaway database"]
async fn syncing_from_both_paths_leaves_the_same_rows() -> TestResult<()> {
    let pool = guarded_pool().await?;
    clear(&pool).await?;

    let rows = vec![row(DOC_A, "a1", "CFS took custody of the $50,000 check.")];

    // Path A indexes the document…
    sync_document_evidence_search(&pool, DOC_A, &rows).await?;
    let after_path_a = ids_for(&pool, DOC_A).await?;

    // …and then an operator re-indexes it by hand through Path B.
    sync_document_evidence_search(&pool, DOC_A, &rows).await?;
    let after_path_b = ids_for(&pool, DOC_A).await?;

    assert_eq!(after_path_a, after_path_b);
    assert_eq!(after_path_b.len(), 1);

    clear(&pool).await?;
    Ok(())
}

/// Evidence deleted from the graph disappears from the mirror on the next sync.
#[tokio::test]
#[ignore = "requires EVIDENCE_SEARCH_TEST_DATABASE_URL pointing at a throwaway database"]
async fn evidence_removed_from_the_graph_disappears_from_the_mirror() -> TestResult<()> {
    let pool = guarded_pool().await?;
    clear(&pool).await?;

    let before = vec![
        row(DOC_A, "a1", "CFS took custody of the $50,000 check."),
        row(DOC_A, "a2", "Richard Milster prepared the pleadings."),
        row(DOC_A, "a3", "A quote that a re-extraction will drop."),
    ];
    sync_document_evidence_search(&pool, DOC_A, &before).await?;
    assert_eq!(ids_for(&pool, DOC_A).await?.len(), 3);

    // Re-extraction dropped a3. The next index of this document carries two rows.
    let after = vec![
        row(DOC_A, "a1", "CFS took custody of the $50,000 check."),
        row(DOC_A, "a2", "Richard Milster prepared the pleadings."),
    ];
    let (written, deleted) = sync_document_evidence_search(&pool, DOC_A, &after).await?;
    assert_eq!(written, 2);
    assert_eq!(
        deleted, 1,
        "the dropped quote must be removed, not left as a ghost"
    );

    let ids = ids_for(&pool, DOC_A).await?;
    assert_eq!(ids.len(), 2);
    assert!(
        !ids.contains(&format!("{TEST_ID_PREFIX}a3")),
        "a ghost row would keep answering lexical searches for a quote the graph no longer has"
    );

    clear(&pool).await?;
    Ok(())
}

/// An empty set CLEARS the document and is not skipped.
///
/// The case the whole delete half exists for: a document whose Evidence was all
/// removed. Skipping the call when the list is empty — the obvious
/// "optimisation" — is exactly how those rows would survive for ever.
#[tokio::test]
#[ignore = "requires EVIDENCE_SEARCH_TEST_DATABASE_URL pointing at a throwaway database"]
async fn an_empty_set_clears_the_document_rather_than_skipping() -> TestResult<()> {
    let pool = guarded_pool().await?;
    clear(&pool).await?;

    let rows = vec![
        row(DOC_A, "a1", "CFS took custody of the $50,000 check."),
        row(DOC_A, "a2", "Richard Milster prepared the pleadings."),
    ];
    sync_document_evidence_search(&pool, DOC_A, &rows).await?;
    assert_eq!(ids_for(&pool, DOC_A).await?.len(), 2);

    let (written, deleted) = sync_document_evidence_search(&pool, DOC_A, &[]).await?;
    assert_eq!(written, 0, "nothing to upsert");
    assert_eq!(deleted, 2, "everything the mirror still had must go");
    assert!(
        ids_for(&pool, DOC_A).await?.is_empty(),
        "the document's rows must be gone, not merely un-refreshed"
    );

    clear(&pool).await?;
    Ok(())
}

/// One document's sync does not touch another document's rows.
///
/// The property that makes it safe to call this from two paths concurrently, and
/// the one whose failure would be worst: an unscoped delete would empty the
/// mirror for the whole corpus on the first document indexed.
#[tokio::test]
#[ignore = "requires EVIDENCE_SEARCH_TEST_DATABASE_URL pointing at a throwaway database"]
async fn a_documents_sync_leaves_other_documents_alone() -> TestResult<()> {
    let pool = guarded_pool().await?;
    clear(&pool).await?;

    sync_document_evidence_search(
        &pool,
        DOC_A,
        &[row(DOC_A, "a1", "Document A's only quote.")],
    )
    .await?;
    sync_document_evidence_search(
        &pool,
        DOC_B,
        &[
            row(DOC_B, "b1", "Document B's first quote."),
            row(DOC_B, "b2", "Document B's second quote."),
        ],
    )
    .await?;
    let total_before = count_evidence_search_rows(&pool).await?;

    // The most destructive thing that can happen to document A: everything gone.
    let (_, deleted) = sync_document_evidence_search(&pool, DOC_A, &[]).await?;
    assert_eq!(deleted, 1);

    assert!(
        ids_for(&pool, DOC_A).await?.is_empty(),
        "document A must be cleared"
    );
    assert_eq!(
        ids_for(&pool, DOC_B).await?.len(),
        2,
        "document B must be untouched — the delete is scoped by document_id"
    );
    assert_eq!(
        count_evidence_search_rows(&pool).await?,
        total_before - 1,
        "exactly one row left the table"
    );

    clear(&pool).await?;
    Ok(())
}

/// A failing write leaves the mirror EXACTLY as it was — the pair is atomic.
///
/// The upsert and the ghost delete are one transaction. If the second statement
/// fails, the first must not stand: a mirror that had been upserted but not
/// cleared would carry ghost rows with no error to say so, which is the silent
/// half-state the transaction exists to prevent.
///
/// The failure is induced by pointing the sync at a pool whose connection cannot
/// be made, so `begin()` fails and nothing is applied. That is a blunter failure
/// than a mid-transaction one, but it is the honest one available: the row type
/// makes a NOT NULL violation unrepresentable, which is itself the point of
/// L1b's design.
#[tokio::test]
#[ignore = "requires EVIDENCE_SEARCH_TEST_DATABASE_URL pointing at a throwaway database"]
async fn a_failed_sync_leaves_the_mirror_untouched() -> TestResult<()> {
    let pool = guarded_pool().await?;
    clear(&pool).await?;

    let rows = vec![row(DOC_A, "a1", "CFS took custody of the $50,000 check.")];
    sync_document_evidence_search(&pool, DOC_A, &rows).await?;
    let before = ids_for(&pool, DOC_A).await?;

    // A pool aimed at a dead port: any statement fails, so the sync cannot
    // commit. Same instrument `scenario_candidate_ordinals`' tests use.
    let dead: PgPool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .connect_lazy("postgres://127.0.0.1:1/nodb")?;
    let outcome = sync_document_evidence_search(&dead, DOC_A, &[]).await;
    assert!(
        outcome.is_err(),
        "a sync that cannot reach the database must fail, not report success"
    );

    assert_eq!(
        ids_for(&pool, DOC_A).await?,
        before,
        "the real mirror must be exactly as it was — nothing half-applied"
    );

    clear(&pool).await?;
    Ok(())
}

/// Source assertions that each of the two index paths calls the sync.
///
/// Not a substitute for a behavioural test — a substitute for a test this
/// project has no tier to write. Driving Path A needs a Restate runtime and Path
/// B needs an `AppState` carrying two pools, a graph and a Qdrant client. The
/// same discipline `api::timeline_subsets::writes::tests` uses for its
/// write-guard proof: assert against the source that the call is present, at the
/// place it has to be.
mod wiring {
    /// Path A — the Restate workflow's step 7.
    #[test]
    fn path_a_syncs_the_mirror_after_its_qdrant_upsert() {
        let src = std::fs::read_to_string("src/pipeline/steps/index.rs")
            .expect("Path A's source is readable");
        let qdrant = src
            .find("qdrant_service::upsert_points")
            .expect("Path A upserts to Qdrant");
        let mirror = src
            .find("evidence_mirror::sync_document")
            .expect("Path A must sync the lexical mirror — without it the mirror goes stale");
        assert!(
            qdrant < mirror,
            "the mirror sync must come AFTER the Qdrant upsert, so a Qdrant failure \
             short-circuits before Postgres is touched"
        );
    }

    /// Path B — `run_index_core`, behind both the route and the delta ingest.
    #[test]
    fn path_b_syncs_the_mirror_after_its_qdrant_upsert() {
        let src = std::fs::read_to_string("src/api/pipeline/index.rs")
            .expect("Path B's source is readable");
        let qdrant = src
            .find("qdrant_service::upsert_points")
            .expect("Path B upserts to Qdrant");
        let mirror = src.find("evidence_mirror::sync_document").expect(
            "Path B must sync the lexical mirror — without it a hand re-index or a \
                     delta ingest leaves the mirror stale",
        );
        assert!(
            qdrant < mirror,
            "the mirror sync must come after the Qdrant upsert"
        );
    }

    /// Neither path propagates the mirror failure by logging it.
    ///
    /// The L1c requirement in one assertion: if the mirror write fails, the step
    /// fails. A `warn!`-and-continue would leave a half-searchable document with
    /// nothing but a log line to say so.
    #[test]
    fn neither_path_swallows_a_mirror_failure() {
        for path in ["src/pipeline/steps/index.rs", "src/api/pipeline/index.rs"] {
            let src = std::fs::read_to_string(path).expect("source is readable");
            let call = src
                .find("evidence_mirror::sync_document")
                .expect("the path syncs the mirror");
            // The 400 characters after the call carry its error handling.
            let tail = &src[call..usize::min(call + 400, src.len())];
            assert!(
                tail.contains('?'),
                "{path} must PROPAGATE a mirror failure, not log and carry on"
            );
        }
    }

    /// The full-corpus re-embed is deliberately NOT a third writer, and the
    /// reason is recorded where someone would go to add one.
    #[test]
    fn the_corpus_wide_re_embed_is_not_wired() {
        let src = std::fs::read_to_string("src/services/embedding_pipeline.rs")
            .expect("source is readable");
        assert!(
            !src.contains("evidence_mirror"),
            "the mirror mirrors the GRAPH, not Qdrant: a corpus-wide vector rebuild cannot \
             make it stale, and wiring it here would add a third writer"
        );
        let mirror = std::fs::read_to_string("src/services/evidence_mirror.rs")
            .expect("the service is readable");
        assert!(
            mirror.contains("embedding_pipeline"),
            "the decision must be recorded where the next reader would try to 'fix' it"
        );
    }
}
