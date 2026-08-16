//! `remap_evidence` — carry curated rows across a re-extraction.
//!
//! One of the one-shot maintenance family (see `oneshot`). **Built and tested; it
//! runs NOWHERE until the Morris gate test, under supervision.**
//!
//! ## Three subcommands, because approval is a step
//!
//! ```text
//! # BEFORE the reprocess — capture what is there now:
//! cargo run --bin remap_evidence -- snapshot --document doc-sabrina-morris-affidavit \
//!     --out morris_before.json
//!
//! # AFTER the reprocess — match old to new, propose, and queue the rest:
//! cargo run --bin remap_evidence -- propose --snapshot morris_before.json \
//!     --out morris_proposal.txt --queue morris_queue.txt
//!
//! # A human reads the proposal, deletes any MAP line they reject, and
//! # uncomments the APPROVED line. Then:
//! cargo run --bin remap_evidence -- apply --proposal morris_proposal.txt
//! ```
//!
//! `propose` writes nothing to either store. `apply` refuses a proposal with no
//! `APPROVED` line, so a generated file cannot be executed unread.
//!
//! ## What the Morris gate test is checking
//!
//! The `propose` step prints the yield — unchanged plus unambiguous, over all
//! old nodes. The measured floor from before the stable-id arm is **87.8%**, and
//! with the arm live the expectation is far higher, because most ids should not
//! move at all. A yield at or below 87.8% means the arm is not doing its job and
//! the wave should not run.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use colossus_legal_backend::oneshot::cli::{
    connect_graph, connect_pool, emit_report, init_tracing, pipeline_database_url,
};
use colossus_legal_backend::oneshot::exit::{
    help_text, EXIT_BAD_INPUT, EXIT_CONNECTION, EXIT_EXECUTION_FAILED, EXIT_OK,
};
use colossus_legal_backend::remap::execute::{apply, load_document_nodes, take_snapshot};
use colossus_legal_backend::remap::plan::{RemapPlan, Snapshot};
use colossus_legal_backend::remap::proposal;
use tracing::{error, info, warn};

#[derive(Parser, Debug)]
#[command(
    name = "remap_evidence",
    about = "Carry curated rows across a re-extraction. Nothing writes except `apply`.",
    after_help = help_text()
)]
struct Args {
    #[command(subcommand)]
    command: Command,

    /// Postgres URL. Falls back to `PIPELINE_DATABASE_URL`.
    #[arg(long, global = true)]
    database_url: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Capture a document's Evidence state BEFORE a reprocess. Writes nothing.
    Snapshot {
        /// The document id, e.g. `doc-sabrina-morris-affidavit`.
        #[arg(long)]
        document: String,
        /// Where to write the snapshot JSON.
        #[arg(long)]
        out: PathBuf,
        /// A note carried into the file so a stale snapshot reads as stale.
        #[arg(long, default_value = "taken before a reprocess")]
        note: String,
    },
    /// Match a snapshot against the post-reprocess graph. Writes nothing.
    Propose {
        /// The snapshot taken before the reprocess.
        #[arg(long)]
        snapshot: PathBuf,
        /// Where to write the proposal for a human to approve.
        #[arg(long, default_value = "remap_proposal.txt")]
        out: PathBuf,
        /// Where to write the human queue.
        #[arg(long, default_value = "remap_human_queue.txt")]
        queue: PathBuf,
    },
    /// Execute an APPROVED proposal. This is the only writing path.
    Apply {
        /// The proposal, after a human has approved it.
        #[arg(long)]
        proposal: PathBuf,
        /// Where to write the count-proof report.
        #[arg(long, default_value = "remap_report.txt")]
        report: PathBuf,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();
    let _ = dotenvy::dotenv();

    let args = Args::parse();
    match execute(args).await {
        Ok(code) => code,
        Err(code) => code,
    }
}

async fn execute(args: Args) -> Result<ExitCode, ExitCode> {
    let database_url = pipeline_database_url(args.database_url.as_deref())?;
    match args.command {
        Command::Snapshot {
            document,
            out,
            note,
        } => run_snapshot(&database_url, &document, &out, &note).await,
        Command::Propose {
            snapshot,
            out,
            queue,
        } => run_propose(&snapshot, &out, &queue).await,
        Command::Apply { proposal, report } => run_apply(&database_url, &proposal, &report).await,
    }
}

/// Capture the document's state before the reprocess.
async fn run_snapshot(
    database_url: &str,
    document: &str,
    out: &Path,
    note: &str,
) -> Result<ExitCode, ExitCode> {
    let graph = connect_graph().await?;
    let pool = connect_pool(database_url).await?;

    let snapshot = take_snapshot(&graph, &pool, document, note)
        .await
        .map_err(|e| {
            error!(error = %e, document, "could not take the snapshot");
            ExitCode::from(EXIT_CONNECTION)
        })?;

    if snapshot.nodes.is_empty() {
        // Not an error — an unprocessed document legitimately has none — but a
        // silent empty snapshot would later look like "the reprocess deleted
        // everything", so it is called out here where it can still be fixed.
        warn!(
            document,
            "the snapshot is EMPTY: this document has no Evidence nodes. Check the \
             document id before running the reprocess"
        );
    }

    let json = serde_json::to_string_pretty(&snapshot).map_err(|e| {
        error!(error = %e, "could not serialise the snapshot");
        ExitCode::from(EXIT_EXECUTION_FAILED)
    })?;
    std::fs::write(out, json).map_err(|e| {
        error!(error = %e, path = %out.display(), "could not write the snapshot");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;

    info!(
        path = %out.display(),
        nodes = snapshot.nodes.len(),
        curated_nodes = snapshot.curated_nodes(),
        "snapshot written — run the reprocess now, then `propose`"
    );
    Ok(ExitCode::from(EXIT_OK))
}

/// Match the snapshot against the graph as it stands now.
async fn run_propose(snapshot_path: &Path, out: &Path, queue: &Path) -> Result<ExitCode, ExitCode> {
    let text = std::fs::read_to_string(snapshot_path).map_err(|e| {
        error!(error = %e, path = %snapshot_path.display(), "could not read the snapshot");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;
    let snapshot: Snapshot = serde_json::from_str(&text).map_err(|e| {
        error!(error = %e, path = %snapshot_path.display(), "the snapshot is not readable JSON");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;

    let graph = connect_graph().await?;
    let new_nodes = load_document_nodes(&graph, &snapshot.document_id)
        .await
        .map_err(|e| {
            error!(error = %e, "could not read the document's Evidence nodes");
            ExitCode::from(EXIT_CONNECTION)
        })?;

    let plan = RemapPlan::build(&snapshot, &new_nodes);
    let totals = plan.totals();
    info!(
        document = %snapshot.document_id,
        old_nodes = totals.old_nodes,
        unchanged = totals.unchanged,
        unambiguous = totals.unambiguous,
        ambiguous = totals.ambiguous,
        unmatched = totals.unmatched,
        yield_percent = format!("{:.1}", totals.yield_percent()),
        curated_rows_at_risk = totals.curated_rows_at_risk,
        "proposal built — nothing was written to either store"
    );

    std::fs::write(queue, proposal::render_queue(&plan)).map_err(|e| {
        error!(error = %e, path = %queue.display(), "could not write the human queue");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;
    emit_report(&proposal::render(&plan), out)?;
    info!(
        proposal = %out.display(),
        queue = %queue.display(),
        "read the proposal, delete any MAP line you reject, then uncomment APPROVED"
    );
    Ok(ExitCode::from(EXIT_OK))
}

/// Execute an approved proposal.
async fn run_apply(
    database_url: &str,
    proposal_path: &Path,
    report_path: &Path,
) -> Result<ExitCode, ExitCode> {
    let text = std::fs::read_to_string(proposal_path).map_err(|e| {
        error!(error = %e, path = %proposal_path.display(), "could not read the proposal");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;
    let parsed = proposal::parse(&text).map_err(|e| {
        error!(error = %e, path = %proposal_path.display(), "the proposal could not be read");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;
    let moves = parsed.approved_moves().map_err(|e| {
        error!(error = %e, path = %proposal_path.display(), "REFUSING to apply");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;

    let approved_by = parsed.approved_by.clone().unwrap_or_default();
    warn!(
        document = %parsed.document_id,
        moves = moves.len(),
        %approved_by,
        "applying an approved remap: this WILL write. Database backups should \
         have been taken immediately before this step."
    );

    let pool = connect_pool(database_url).await?;
    let report = apply(&pool, &parsed.document_id, &approved_by, moves)
        .await
        .map_err(|e| {
            error!(error = %e, "the remap failed — see the report");
            ExitCode::from(EXIT_EXECUTION_FAILED)
        })?;

    emit_report(&report.render(), report_path)?;

    match &report.aborted {
        Some(reason) => {
            error!(
                reason,
                "the document was rolled back and left unchanged — investigate \
                 the count mismatch before re-running"
            );
            Ok(ExitCode::from(
                colossus_legal_backend::oneshot::exit::EXIT_UNIT_ABORTED,
            ))
        }
        None => Ok(ExitCode::from(EXIT_OK)),
    }
}
