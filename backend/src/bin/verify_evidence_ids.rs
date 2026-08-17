//! `verify_evidence_ids` — prove the Evidence id arm against the live corpus.
//!
//! **READ-ONLY. There is no `--apply`, no write path, and no subcommand that
//! changes anything.** It issues one `SELECT` per document and nothing else.
//!
//! ## Why this exists rather than a test
//!
//! The arm it checks shipped inert for eleven days with a fully green suite,
//! because every fixture had been hand-built in the shape the code assumed. A
//! test can only ever assert against a fixture somebody wrote; this asserts
//! against the rows the pipeline actually wrote, through the REAL
//! [`stable_entity_id`] — not a re-implementation of it — and compares to the
//! ids the graph carries in `extraction_items.neo4j_node_id`.
//!
//! ```text
//! cargo run --bin verify_evidence_ids -- \
//!     --document doc-sabrina-morris-affidavit \
//!     --document doc-jeffrey-humphrey-affidavit
//! ```
//!
//! Pass no `--document` and it checks every document that has Evidence items.
//!
//! ## Reading the output
//!
//! ```text
//! Humphrey : 26 rows · 26 equal · 0 differ
//! ```
//!
//! `equal` means the arm recomputed exactly the id that row already carries — so
//! a re-extraction of unchanged text would MERGE onto the same node and every
//! curated row pointing at it survives. `differ` is the number that matters: it
//! is the count of curated references a reprocess of that document would strand.
//!
//! One caveat the operator must hold, and the tool prints it: this compares
//! against whatever `neo4j_node_id` holds NOW. If a document has been reprocessed
//! since the re-key, that column holds post-reprocess ids, and `differ` counts
//! the drift from those — not from the re-keyed ids the graph had before.

use std::collections::BTreeMap;
use std::process::ExitCode;

use clap::Parser;
use colossus_legal_backend::api::pipeline::stable_entity_id;
use colossus_legal_backend::models::document_status::ENTITY_EVIDENCE;
use colossus_legal_backend::oneshot::cli::{connect_pool, init_tracing, pipeline_database_url};
use colossus_legal_backend::oneshot::exit::{
    help_text, EXIT_CONNECTION, EXIT_OK, EXIT_UNIT_ABORTED,
};
use colossus_legal_backend::repositories::pipeline_repository::ExtractionItemRecord;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(
    name = "verify_evidence_ids",
    about = "Recompute every Evidence id through the real arm and compare to the graph. Read-only.",
    after_help = help_text()
)]
struct Args {
    /// A document to check. Repeatable. Omit to check every document.
    #[arg(long = "document")]
    documents: Vec<String>,

    /// Postgres URL. Falls back to `PIPELINE_DATABASE_URL`.
    #[arg(long)]
    database_url: Option<String>,
}

/// What one document's rows came to.
#[derive(Debug, Default, Clone, Copy)]
struct DocTotals {
    rows: usize,
    equal: usize,
    differ: usize,
    /// Rows with no `neo4j_node_id` — never ingested, so there is nothing to
    /// compare against. Counted separately rather than scored as a difference,
    /// because calling them "differ" would report a queue state as a defect.
    unlinked: usize,
}

#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();
    let _ = dotenvy::dotenv();

    match execute(Args::parse()).await {
        Ok(code) => code,
        Err(code) => code,
    }
}

async fn execute(args: Args) -> Result<ExitCode, ExitCode> {
    let database_url = pipeline_database_url(args.database_url.as_deref())?;
    let pool = connect_pool(&database_url).await?;

    let items = load_evidence_items(&pool, &args.documents).await?;
    info!(
        rows = items.len(),
        documents = args.documents.len(),
        "loaded Evidence items — nothing was written"
    );

    // BTreeMap so the report is in document order and two runs diff cleanly.
    let mut totals: BTreeMap<String, DocTotals> = BTreeMap::new();
    let mut differences: Vec<(i32, String, String, String)> = Vec::new();

    for item in &items {
        let entry = totals.entry(item.document_id.clone()).or_default();
        entry.rows += 1;

        // THE POINT: the real function, on a real row, with the same `doc_id` the
        // ingest path passes it.
        let computed = stable_entity_id(item, &item.document_id);

        match item.neo4j_node_id.as_deref() {
            None => entry.unlinked += 1,
            Some(live) if live == computed => entry.equal += 1,
            Some(live) => {
                entry.differ += 1;
                differences.push((
                    item.id,
                    item.document_id.clone(),
                    live.to_string(),
                    computed,
                ));
            }
        }
    }

    print_report(&totals, &differences);

    // A non-zero `differ` is the finding, so it is also the exit code — a runbook
    // step can gate on it without parsing the text.
    let any_differ = totals.values().any(|t| t.differ > 0);
    Ok(ExitCode::from(if any_differ {
        EXIT_UNIT_ABORTED
    } else {
        EXIT_OK
    }))
}

/// Read the Evidence rows, optionally narrowed to named documents.
///
/// ## Rust Learning: one query, two shapes, via `= ANY($2)`
///
/// Postgres's `ANY` takes an array parameter, so an empty `--document` list and a
/// three-document list are the SAME statement with a different binding — no
/// string-built `IN (...)` clause, and therefore no way for a document id to be
/// concatenated into SQL. The `cardinality($2) = 0` disjunct is what makes the
/// empty case mean "every document" rather than "no documents".
async fn load_evidence_items(
    pool: &sqlx::PgPool,
    documents: &[String],
) -> Result<Vec<ExtractionItemRecord>, ExitCode> {
    // The projection is spelled out rather than reusing the repository's
    // `ITEM_SELECT_COLUMNS`, which is `pub(super)` to its module. Column drift
    // would surface here as a `FromRow` decode failure on the first run, which is
    // loud and immediate — acceptable for a read-only proof tool, and cheaper
    // than widening a repository constant for one binary.
    const SQL: &str = "SELECT id, run_id, document_id, entity_type, item_data, verbatim_quote, \
                grounding_status, grounded_page, review_status, reviewed_by, reviewed_at, \
                review_notes, graph_status, neo4j_node_id, resolved_entity_type \
         FROM extraction_items \
         WHERE entity_type = $1 AND (cardinality($2::text[]) = 0 OR document_id = ANY($2)) \
         ORDER BY document_id, id";

    sqlx::query_as::<_, ExtractionItemRecord>(SQL)
        .bind(ENTITY_EVIDENCE)
        .bind(documents)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            error!(
                error = %e,
                "could not read extraction_items — check PIPELINE_DATABASE_URL points at \
                 the pipeline database (colossus_legal_v2)"
            );
            ExitCode::from(EXIT_CONNECTION)
        })
}

/// Print the count proof. One line per document, then every difference.
fn print_report(
    totals: &BTreeMap<String, DocTotals>,
    differences: &[(i32, String, String, String)],
) {
    println!("\n=== EVIDENCE ID VERIFICATION — READ-ONLY ===\n");
    println!(
        "Recomputed through stable_entity_id() and compared to extraction_items.neo4j_node_id."
    );
    println!("A document reprocessed since the re-key holds post-reprocess ids in that column.\n");

    for (doc, t) in totals {
        let unlinked = if t.unlinked > 0 {
            format!("  ({} not ingested)", t.unlinked)
        } else {
            String::new()
        };
        println!(
            "{doc:<60} {:>4} rows · {:>4} equal · {:>4} differ{unlinked}",
            t.rows, t.equal, t.differ
        );
    }

    if differences.is_empty() {
        println!("\nNo differences. Every ingested Evidence row recomputes to the id it carries.");
        return;
    }

    println!("\n--- DIFFERENCES ({}) ---", differences.len());
    for (item_id, doc, live, computed) in differences {
        println!("  item {item_id} in {doc}\n    live     {live}\n    computed {computed}");
    }
}
