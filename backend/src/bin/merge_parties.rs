//! `merge_parties` — collapse duplicate Person / Organization nodes, on Roman's
//! written rulings and nothing else.
//!
//! One of the one-shot maintenance family (see `oneshot`). It is a WAVE
//! PREREQUISITE: the merge pass must precede the wave, or the wave mints another
//! layer of name variants on top of the twelve clusters already there.
//!
//! ## Two modes
//!
//! ```text
//! # 1. Generate the rulings template from the live graph, for Roman's session:
//! cargo run --bin merge_parties -- --emit-template party_merge_rulings.txt
//!
//! # 2. Execute exactly what he wrote — dry run first, always:
//! cargo run --bin merge_parties -- --rulings party_merge_rulings.txt
//! cargo run --bin merge_parties -- --rulings party_merge_rulings.txt --apply
//! ```
//!
//! `--emit-template` reads the graph and writes nothing to it. Every block it
//! generates says `SKIP`, so a template handed straight back merges nothing.
//!
//! ## The tool never guesses who is who
//!
//! There is no fuzzy matching in the execution path and no default survivor.
//! Every merge is a cluster Roman named, with a survivor he chose. The two
//! do-not-auto-merge clusters from the census arrive as `SKIP` and stay that way
//! unless he says otherwise in writing.
//!
//! ## Acceptance (P7 addendum)
//!
//! Statement totals conserved per cluster — Tighe's 39 + 62 becoming one node
//! with 101 — zero dangling references, and the People page dropping by exactly
//! the merged-member count. The tool measures the first two itself and ABORTS any
//! cluster that fails them; the report prints the third as the number to check
//! against the page.
//!
//! ## Before `--apply`
//!
//! Take database backups. This tool DELETES graph nodes.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use colossus_legal_backend::oneshot::cli::{
    connect_graph, connect_pool, emit_report, init_tracing, pipeline_database_url,
};
use colossus_legal_backend::oneshot::exit::{
    help_text, EXIT_BAD_INPUT, EXIT_CONNECTION, EXIT_EXECUTION_FAILED, EXIT_UNSAFE_PLAN,
};
use colossus_legal_backend::partymerge::census::render_template;
use colossus_legal_backend::partymerge::execute::{
    load_census, preflight_edge_types, run, PartyMergeError,
};
use colossus_legal_backend::partymerge::plan::MergePlan;
use colossus_legal_backend::partymerge::rulings;
use tracing::{error, info, warn};

/// CLI arguments. Field doc comments double as `--help` text.
#[derive(Parser, Debug)]
#[command(
    name = "merge_parties",
    about = "Merge duplicate party nodes, from a human rulings file. Dry run unless --apply.",
    after_help = help_text()
)]
struct Args {
    /// Generate the rulings template from the live census and exit.
    ///
    /// Writes no changes. Every generated block says SKIP.
    #[arg(long, value_name = "PATH")]
    emit_template: Option<PathBuf>,

    /// The rulings file to execute. Required unless --emit-template is given.
    #[arg(long, value_name = "PATH")]
    rulings: Option<PathBuf>,

    /// WRITE. Without this the tool plans and reports but changes nothing.
    ///
    /// Take database backups first — this DELETES graph nodes.
    #[arg(long)]
    apply: bool,

    /// Where to write the count-proof report.
    #[arg(long, default_value = "party_merge_report.txt")]
    report: PathBuf,

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

async fn execute(args: Args) -> Result<ExitCode, ExitCode> {
    let graph = connect_graph().await?;
    let census = load_census(&graph).await.map_err(|e| {
        error!(error = %e, "could not read the party census");
        ExitCode::from(EXIT_CONNECTION)
    })?;
    info!(parties = census.len(), "read the party census");

    if let Some(path) = &args.emit_template {
        return emit_template(&render_template(&census), path);
    }

    let Some(rulings_path) = args.rulings.clone() else {
        error!(
            "nothing to do: pass --rulings PATH to execute a rulings file, or \
             --emit-template PATH to generate one. This tool never merges \
             without a written ruling"
        );
        return Err(ExitCode::from(EXIT_BAD_INPUT));
    };

    let plan = build_plan(&rulings_path, &census)?;
    info!(summary = %plan_summary(&plan), "rulings parsed");

    // Before anything is written, and only when something will be: an unknown
    // edge type would be silently deleted by the merge.
    preflight_edge_types(&graph).await.map_err(|e| match e {
        PartyMergeError::UnknownEdgeTypes { .. } => {
            error!(error = %e, "REFUSING: an unmovable relationship type exists");
            ExitCode::from(EXIT_UNSAFE_PLAN)
        }
        other => {
            error!(error = %other, "could not check party relationship types");
            ExitCode::from(EXIT_CONNECTION)
        }
    })?;

    let database_url = pipeline_database_url(args.database_url.as_deref())?;
    let pool = connect_pool(&database_url).await?;

    if args.apply {
        warn!(
            "--apply given: this run WILL write, and it DELETES graph nodes. \
             Database backups should have been taken immediately before this step."
        );
    } else {
        info!("dry run (no --apply): nothing will be written");
    }

    let report = run(&graph, &pool, &plan, args.apply).await.map_err(|e| {
        error!(error = %e, "the merge failed part-way through — see the report");
        ExitCode::from(EXIT_EXECUTION_FAILED)
    })?;

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
    Ok(ExitCode::from(report.exit_code()))
}

/// Read and parse the rulings file, then check it against the live census.
///
/// Both failures are `EXIT_BAD_INPUT` or `EXIT_UNSAFE_PLAN` rather than a
/// generic error: a rulings file the tool cannot read, or one naming a node that
/// is not there, means nothing was written and nothing will be.
fn build_plan(
    path: &Path,
    census: &[colossus_legal_backend::partymerge::census::PartyNode],
) -> Result<MergePlan, ExitCode> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        error!(error = %e, path = %path.display(), "could not read the rulings file");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;

    let parsed = rulings::parse(&text).map_err(|e| {
        error!(error = %e, path = %path.display(), "the rulings file could not be read");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;

    MergePlan::build(&parsed, census).map_err(|e| {
        error!(error = %e, "REFUSING: the rulings do not match the live graph");
        ExitCode::from(EXIT_UNSAFE_PLAN)
    })
}

/// A one-line summary of the plan, for the log line before execution.
fn plan_summary(plan: &MergePlan) -> String {
    let t = plan.totals();
    format!(
        "{} ruled — {} to merge, {} skipped, {} already merged; {} node(s) merging in",
        t.clusters_ruled,
        t.clusters_to_merge,
        t.clusters_skipped,
        t.clusters_already_merged,
        t.nodes_to_merge_in
    )
}

/// Write the generated template and stop.
fn emit_template(rendered: &str, path: &Path) -> Result<ExitCode, ExitCode> {
    std::fs::write(path, rendered).map_err(|e| {
        error!(error = %e, path = %path.display(), "could not write the rulings template");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;
    info!(
        path = %path.display(),
        "rulings template written — every block says SKIP until you edit it"
    );
    Ok(ExitCode::from(
        colossus_legal_backend::oneshot::exit::EXIT_OK,
    ))
}
