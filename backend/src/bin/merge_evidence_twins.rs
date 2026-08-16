//! `merge_evidence_twins` — collapse the duplicate-extraction class in Evidence.
//!
//! One of the one-shot maintenance family (see `oneshot`): not an admin
//! endpoint, nothing clickable, nothing an accidental redeploy can re-trigger.
//! Roman runs it from a runbook step.
//!
//! ## DRY RUN IS THE DEFAULT
//!
//! With no flags it plans everything, prints the count proof it WOULD write and
//! the human queue it WOULD refuse, and writes nothing. `--apply` is the only
//! path that mutates, and it executes the same plan the dry run printed.
//!
//! ## Where it sits in the Saturday sequence
//!
//! AFTER `rekey_evidence --apply`. The re-key refuses the 42 twin nodes, so they
//! keep their old blob-hash ids; this tool collapses each pair onto one node
//! carrying the stable-arm key. Running it first would work but would leave the
//! re-key's count proof harder to check against the measured 483/42/525.
//!
//! ## Expected on the first live dry run (measured on DEV 2026-08-15)
//!
//! ```text
//! Same-key clusters seen   : 21
//! Nodes in those clusters  : 42
//! Clusters to merge        : 14
//! Nodes to delete          : 14
//! Refused, curated on 2+   : 7
//! Refused, edges diverge   : 0
//! ```
//!
//! Different numbers mean the corpus moved since the measurement. Stop and find
//! out why before `--apply`.
//!
//! ## Before `--apply`
//!
//! Take database backups. There is no cross-store transaction here and this tool
//! does not pretend there is — each cluster's Postgres work is one transaction
//! verified against pre-counts before it commits, and the graph is only touched
//! after that verification passes, but the two stores are not one.
//!
//! ## Usage
//! ```text
//! cargo run --bin merge_evidence_twins -- [--apply] [--report PATH] [--queue PATH]
//! ```

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use colossus_legal_backend::oneshot::cli::{
    connect_graph, connect_pool, emit_report, init_tracing, pipeline_database_url,
};
use colossus_legal_backend::oneshot::exit::{
    help_text, EXIT_CONNECTION, EXIT_EXECUTION_FAILED, EXIT_UNSAFE_PLAN,
};
use colossus_legal_backend::twinmerge::execute::{
    load_twin_nodes, plan_or_refuse, run, TwinMergeError,
};
use tracing::{error, info, warn};

/// CLI arguments. Field doc comments double as `--help` text.
#[derive(Parser, Debug)]
#[command(
    name = "merge_evidence_twins",
    about = "Collapse same-key Evidence duplicates onto one node. Dry run unless --apply.",
    // Built from `oneshot::exit`, so `--help` and the ruled scheme cannot drift
    // apart the way two hand-written copies would.
    after_help = help_text()
)]
struct Args {
    /// WRITE. Without this the tool plans and reports but changes nothing.
    ///
    /// Take database backups first.
    #[arg(long)]
    apply: bool,

    /// Where to write the count-proof report.
    #[arg(long, default_value = "twin_merge_report.txt")]
    report: PathBuf,

    /// Where to write the human queue — the clusters this tool refuses to merge.
    #[arg(long, default_value = "twin_merge_human_queue.txt")]
    queue: PathBuf,

    /// Postgres URL. Falls back to `PIPELINE_DATABASE_URL`.
    #[arg(long)]
    database_url: Option<String>,
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

/// The body, split out so every failure is one `?` and the exit codes are read
/// off a single `match` rather than scattered through `main`.
async fn execute(args: Args) -> Result<ExitCode, ExitCode> {
    let database_url = pipeline_database_url(args.database_url.as_deref())?;
    let graph = connect_graph().await?;
    let pool = connect_pool(&database_url).await?;

    if args.apply {
        warn!(
            "--apply given: this run WILL write, and it DELETES graph nodes. \
             Database backups should have been taken immediately before this step."
        );
    } else {
        info!("dry run (no --apply): nothing will be written");
    }

    let nodes = load_twin_nodes(&graph, &pool).await.map_err(|e| {
        error!(error = %e, "could not read Evidence nodes and their curated rows");
        ExitCode::from(EXIT_CONNECTION)
    })?;
    info!(evidence_nodes = nodes.len(), "read Evidence nodes");

    let plan = plan_or_refuse(nodes).map_err(|e| match e {
        TwinMergeError::UnsafePlan { .. } => {
            error!(error = %e, "REFUSING: a merge target is already held elsewhere");
            ExitCode::from(EXIT_UNSAFE_PLAN)
        }
        other => {
            error!(error = %other, "could not plan the merge");
            ExitCode::from(EXIT_EXECUTION_FAILED)
        }
    })?;

    let report = run(&graph, &pool, &plan, args.apply).await.map_err(|e| {
        error!(error = %e, "the merge failed part-way through — see the report");
        ExitCode::from(EXIT_EXECUTION_FAILED)
    })?;

    // The queue is written FIRST and always, even when empty. It is the agenda
    // for a session that happens hours later, and an absent file cannot say
    // whether nothing was refused or the tool never got this far.
    emit_queue(&report.render_queue(), &args.queue)?;
    emit_report(&report.render(), &args.report)?;

    let aborted = report.aborted_clusters().len();
    if aborted > 0 {
        warn!(
            aborted,
            exit_code = report.exit_code(),
            "some clusters were rolled back and left unchanged — investigate, \
             then re-run; merged clusters are idempotent and will be skipped"
        );
    }
    if !report.queue.is_empty() {
        info!(
            refused = report.queue.len(),
            queue = %args.queue.display(),
            "clusters needing a ruling were written to the queue file"
        );
    }
    Ok(ExitCode::from(report.exit_code()))
}

/// Write the human queue, printing nothing — the proof gets the terminal.
///
///
/// Kept separate from `emit_report` on purpose: printing both to stdout would
/// bury the count proof under the queue, and the queue's readers want a file
/// they can open in the merge session, not scrollback.
fn emit_queue(rendered: &str, path: &Path) -> Result<(), ExitCode> {
    std::fs::write(path, rendered).map_err(|e| {
        error!(
            error = %e,
            path = %path.display(),
            "could not write the human queue — refusing to continue, because a \
             run whose refusals are unrecorded looks exactly like a run with none"
        );
        ExitCode::from(colossus_legal_backend::oneshot::exit::EXIT_BAD_INPUT)
    })?;
    info!(path = %path.display(), "human queue written");
    Ok(())
}
