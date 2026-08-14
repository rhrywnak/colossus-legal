//! `rekey_evidence` — move every Evidence node onto the stable id arm.
//!
//! Deliberately a one-shot binary and NOT an admin endpoint: nothing clickable,
//! nothing an accidental redeploy can re-trigger. Roman runs it from a runbook
//! step after the batch deploys and before the wave.
//!
//! Thin by design — parse args, open the two connections, hand off to
//! `rekey::execute`, write the report, translate errors into exit codes. Every
//! decision lives in `rekey::plan`, which is unit-tested without a database.
//!
//! ## DRY RUN IS THE DEFAULT
//!
//! With no flags it plans everything, prints the count proof it WOULD write, and
//! writes nothing. `--apply` is the only path that mutates, and it executes the
//! same plan the dry run printed.
//!
//! ## Before `--apply`
//!
//! Take database backups. There is no cross-store transaction here and this tool
//! does not pretend there is: each document's Postgres work is one transaction
//! verified against pre-counts before it commits, and the Neo4j property is set
//! in the same unit — but the two stores are not one. The backups are the net,
//! and the runbook step says so.
//!
//! ## Exit codes
//!
//! Defined in `rekey::report` beside the run they describe, so they are pinned by
//! tests rather than decided in an untestable `main`.
//!
//! - `0` ran to completion; every planned re-key applied; nothing aborted. A dry
//!   run also exits 0. The 42 refused twins do NOT affect this — they are a
//!   planned disposition, not a fault.
//! - `1` bad arguments, or the report could not be written
//! - `2` connection failure, Neo4j or Postgres; the log names which
//! - `3` ran to completion, but one or more documents ABORTED on a count
//!   mismatch and were rolled back. Deliberately NOT `2`: this is the tool
//!   refusing to lie about counts, which needs the DATA investigated, not the
//!   tool.
//! - `4` the plan is unsafe — two nodes would share an id; nothing was written
//! - `5` execution failure part-way through; the report names the last good
//!   document
//!
//! ## Usage
//! ```text
//! cargo run --bin rekey_evidence -- [--apply] [--report PATH] [--database-url URL]
//! ```
//! Neo4j details come from `NEO4J_URI` / `NEO4J_USER` / `NEO4J_PASSWORD`; the
//! Postgres URL from `--database-url` or `PIPELINE_DATABASE_URL` — the same
//! sources the rest of the backend reads, loadable from `.env`.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use colossus_legal_backend::rekey::execute::{load_evidence_rows, plan_or_refuse, run, RekeyError};
use colossus_legal_backend::rekey::report::{
    EXIT_BAD_INPUT, EXIT_CONNECTION, EXIT_EXECUTION_FAILED, EXIT_UNSAFE_PLAN,
};
use neo4rs::Graph;
use sqlx::postgres::PgPoolOptions;
use tracing::{error, info, warn};

/// CLI arguments. Field doc comments double as `--help` text.
#[derive(Parser, Debug)]
#[command(
    name = "rekey_evidence",
    about = "Move Evidence nodes onto the stable id arm. Dry run unless --apply."
)]
struct Args {
    /// WRITE. Without this the tool plans and reports but changes nothing.
    ///
    /// Take database backups first — see the module header on why the two-store
    /// seam needs them.
    #[arg(long)]
    apply: bool,

    /// Where to write the count-proof report. Defaults to the working directory.
    #[arg(long, default_value = "rekey_evidence_report.txt")]
    report: PathBuf,

    /// Postgres URL. Falls back to `PIPELINE_DATABASE_URL`.
    #[arg(long)]
    database_url: Option<String>,
}

#[tokio::main]
async fn main() -> ExitCode {
    // Same initialisation the sibling one-shot binary uses, so an operator's
    // RUST_LOG works identically across both.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
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
    let database_url = match args
        .database_url
        .clone()
        .or_else(|| std::env::var("PIPELINE_DATABASE_URL").ok())
    {
        Some(url) => url,
        None => {
            error!(
                "no Postgres URL: pass --database-url or set PIPELINE_DATABASE_URL \
                 (a .env file is read automatically)"
            );
            return Err(ExitCode::from(EXIT_BAD_INPUT));
        }
    };

    let graph = connect_graph().await?;
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&database_url)
        .await
        .map_err(|e| {
            error!(error = %e, "could not connect to Postgres");
            ExitCode::from(EXIT_CONNECTION)
        })?;

    if args.apply {
        warn!(
            "--apply given: this run WILL write. Database backups should have been \
             taken immediately before this step."
        );
    } else {
        info!("dry run (no --apply): nothing will be written");
    }

    let rows = load_evidence_rows(&graph).await.map_err(|e| {
        error!(error = %e, "could not read Evidence nodes");
        ExitCode::from(EXIT_CONNECTION)
    })?;
    info!(evidence_nodes = rows.len(), "read Evidence nodes");

    let plan = plan_or_refuse(rows).map_err(|e| match e {
        RekeyError::UnsafePlan { .. } => {
            error!(error = %e, "REFUSING: the plan would leave two nodes sharing an id");
            ExitCode::from(EXIT_UNSAFE_PLAN)
        }
        other => {
            error!(error = %other, "could not plan the re-key");
            ExitCode::from(EXIT_EXECUTION_FAILED)
        }
    })?;

    let report = run(&graph, &pool, &plan, args.apply).await.map_err(|e| {
        error!(error = %e, "the re-key failed part-way through — see the report");
        ExitCode::from(EXIT_EXECUTION_FAILED)
    })?;

    let rendered = report.render();
    println!("{rendered}");
    std::fs::write(&args.report, &rendered).map_err(|e| {
        error!(
            error = %e,
            path = %args.report.display(),
            "the run finished but its report could not be written — the proof above \
             is the only copy; capture it before this terminal is lost"
        );
        ExitCode::from(EXIT_BAD_INPUT)
    })?;
    info!(path = %args.report.display(), "count proof written");

    // The exit code is the runbook's machine-readable answer to "did everything
    // re-key?", so an abort must not hide under 0 (ruled 2026-08-14). It is
    // decided by the report, not here, so a test can pin it.
    let aborted = report.aborted_documents().len();
    if aborted > 0 {
        warn!(
            aborted,
            exit_code = report.exit_code(),
            "some documents were rolled back and left unchanged — re-run after \
             investigating; completed documents are idempotent and will be skipped"
        );
    }
    Ok(ExitCode::from(report.exit_code()))
}

/// Open Neo4j from the standard env vars.
async fn connect_graph() -> Result<Graph, ExitCode> {
    let uri = std::env::var("NEO4J_URI").map_err(|_| {
        error!("NEO4J_URI is not set");
        ExitCode::from(EXIT_CONNECTION)
    })?;
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let password = std::env::var("NEO4J_PASSWORD").map_err(|_| {
        error!("NEO4J_PASSWORD is not set");
        ExitCode::from(EXIT_CONNECTION)
    })?;

    Graph::new(&uri, &user, &password).await.map_err(|e| {
        error!(error = %e, uri = %uri, "could not connect to Neo4j");
        ExitCode::from(EXIT_CONNECTION)
    })
}
