//! The paged read-and-upsert loop, and the third count.
//!
//! Every batch is written before the next is read, so peak memory is one page
//! and an interrupted run leaves a partially-refreshed mirror that is still
//! internally consistent — re-running finishes it.

use std::process::ExitCode;

use colossus_legal_backend::repositories::evidence_search_repository::{
    read_batch_size, read_evidence_page,
};
use colossus_legal_backend::repositories::pipeline_repository::upsert_evidence_search_rows;
use colossus_legal_backend::services::qdrant_service;
use sqlx::PgPool;
use tracing::{error, info, warn};

use crate::{env_secs, read};

// STRUCTURAL: Qdrant payload vocabulary. `node_type` is declared as a keyword
// payload index on the collection by `services::qdrant_service::ensure_collection`
// — which is the canonical statement that this key exists — and written on every
// point by all three embed paths (`pipeline::steps::index`,
// `api::pipeline::index` and `services::embedding_pipeline`). The value is the
// Neo4j node label, so it comes from `ENTITY_EVIDENCE` at the call site rather
// than being spelled here. Neither varies by deployment: they are the shape of
// the data, not a setting.
const QDRANT_TYPE_KEY: &str = "node_type";

/// Page the graph and upsert each batch as it arrives.
///
/// ## Why each batch is written before the next is read
///
/// Peak memory stays at one batch rather than the whole corpus, and — the part
/// that matters more — an interrupted run leaves a partially filled mirror that
/// is still internally consistent, because the upsert is keyed by
/// `evidence_id`. Re-running finishes it. Reading everything first and writing
/// once at the end would make an interruption cost the entire read.
/// Has the corpus run out?
///
/// ## Why this counts what the GRAPH returned, not what survived the filter
///
/// The subtlest invariant in the paging loop, and the reason it is a named
/// function with tests rather than an inline comparison. `read_count` is
/// `rows + skipped`, so a page of 200 nodes of which 200 were unmirrorable
/// still reads as a FULL page and the loop continues to `skip = 400`. Comparing
/// `rows.len()` instead would stop the backfill dead at the first page that
/// happened to be all-skips, silently leaving every later node out of the
/// mirror — and the only symptom would be a count that came up short with no
/// error to explain it.
fn is_last_page(read_count: usize, batch_size: usize) -> bool {
    read_count < batch_size
}

pub(crate) async fn fill(pool: &PgPool, graph: &neo4rs::Graph) -> Result<u64, ExitCode> {
    let mut skip: i64 = 0;
    let mut written = 0u64;
    let mut skipped_total = 0usize;

    loop {
        let (rows, skipped) = read(read_evidence_page(graph, skip).await, "an Evidence page")?;
        let read_count = rows.len() + skipped.len();

        for id in &skipped {
            warn!(
                evidence_id = %id,
                "node skipped: no document id, or no verbatim quote — the mirror's columns are \
                 NOT NULL and this row cannot be represented"
            );
        }
        skipped_total += skipped.len();

        // Counted BEFORE the move into the upsert. This is the number that
        // separates a real backfill from one that silently wrote NULLs: if a
        // Cypher edit dropped `e.question AS question`, every node here still
        // has a quote, still passes `mirror_row`, still lands in `rows`, and
        // `batch_rows` and `written` come out identical to a correct run. Both
        // generated columns would then be exactly what they were before the
        // feature existed, with nothing in any log to say so.
        let with_question = rows.iter().filter(|r| r.question.is_some()).count();
        written += read(
            upsert_evidence_search_rows(pool, &rows).await,
            "an evidence_search batch",
        )?;
        info!(
            skip,
            batch_rows = rows.len(),
            with_question,
            written,
            "batch upserted"
        );

        if is_last_page(read_count, read_batch_size()) {
            break;
        }
        skip += read_batch_size() as i64;
    }

    if skipped_total > 0 {
        warn!(
            skipped_total,
            "nodes were skipped and are NOT in the mirror — the count assertion below will \
             show the gap rather than hide it"
        );
    }
    Ok(written)
}

/// How many Evidence points Qdrant holds, or `None` if it could not be asked.
///
/// Read-only, through the existing `qdrant_service`. A failure here is logged
/// and folded into `None` rather than aborting the run: the vector store is the
/// OTHER half of the gather, and a backfill of the lexical half that already
/// succeeded must not be reported as failed because a third system was down.
/// `None` is rendered as "unreachable", never as a zero.
pub(crate) async fn qdrant_evidence_points() -> Option<i64> {
    let url = match std::env::var("QDRANT_URL") {
        Ok(url) => url,
        Err(_) => {
            warn!("QDRANT_URL is not set — the third count cannot be taken");
            return None;
        }
    };
    let client = match reqwest::Client::builder()
        // DEFAULT: 30 seconds overall and 5 to connect — the shape CLAUDE.md
        // rule 13 mandates for every HTTP client in this project. Override with
        // EVIDENCE_SEARCH_QDRANT_TIMEOUT_SECS / _CONNECT_SECS. This call is
        // informational (its failure folds to "unreachable" and never fails the
        // backfill), so the timeouts exist to stop a hung vector store holding
        // the terminal, not to make the count succeed.
        .timeout(env_secs("EVIDENCE_SEARCH_QDRANT_TIMEOUT_SECS", 30))
        .connect_timeout(env_secs("EVIDENCE_SEARCH_QDRANT_CONNECT_SECS", 5))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            error!(error = %e, "could not build an HTTP client for Qdrant");
            return None;
        }
    };

    match qdrant_service::count_points_by_filter(
        &client,
        &url,
        QDRANT_TYPE_KEY,
        colossus_legal_backend::models::document_status::ENTITY_EVIDENCE,
    )
    .await
    {
        Ok(count) => Some(count as i64),
        Err(e) => {
            error!(error = %e, %url, "could not count Qdrant Evidence points");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A full page means there may be more.
    #[test]
    fn a_full_page_continues() {
        assert!(!is_last_page(200, 200));
    }

    /// One short of a full page is the end.
    #[test]
    fn a_short_page_is_the_last_page() {
        assert!(is_last_page(199, 200));
    }

    /// An empty page is the end — the corpus divided exactly by the batch size,
    /// so the previous page was full and this one returned nothing.
    #[test]
    fn an_empty_page_is_the_last_page() {
        assert!(is_last_page(0, 200));
    }

    /// THE ONE THAT MATTERS: a full page whose nodes were ALL skipped is still a
    /// full page. `read_count` is rows + skipped, so this is `is_last_page(200,
    /// 200)` and the loop continues. Were the loop to compare the surviving row
    /// count instead — zero here — the backfill would stop at this page and
    /// every later node would be silently absent from the mirror.
    #[test]
    fn a_full_page_of_skipped_nodes_does_not_end_the_backfill() {
        let rows_that_survived = 0usize;
        let skipped = 200usize;
        assert!(
            !is_last_page(rows_that_survived + skipped, 200),
            "termination must count what the graph returned, not what the filter kept"
        );
        assert!(
            is_last_page(rows_that_survived, 200),
            "and this is the bug that mistake would be — pinned so the comparison \
             cannot be quietly changed to rows.len()"
        );
    }
}
