//! `seed_chronology` — load the 22 legacy timeline events into the chronology, once.
//!
//! One of the one-shot maintenance family (see `oneshot`): dry run is the
//! default, `--apply` is the only path that commits, the count proof is real
//! output, and the exit codes are the family's.
//!
//! ```text
//! # 1. Prove it, read-only. Prints the re-point map, every row, and the totals:
//! cargo run --bin seed_chronology -- --case-slug <SLUG> --created-by <NAME>
//!
//! # 2. Prove it HARDER: execute every insert and its verification, then roll back:
//! cargo run --bin seed_chronology -- --case-slug <SLUG> --created-by <NAME> --prove
//!
//! # 3. Write it:
//! cargo run --bin seed_chronology -- --case-slug <SLUG> --created-by <NAME> --apply
//! ```
//!
//! ## Why `--created-by` is required and has no default
//!
//! It stamps a real person onto 22 rows. Standing Rule 2 keeps domain-specific
//! names out of code, and a person's name is the most domain-specific value
//! there is — so the runbook step types it and this file never names anybody.
//!
//! ## Why `--prove` exists
//!
//! A read-only dry run proves the PLAN. It cannot prove that 22 INSERTs satisfy
//! every constraint, because it never issues one. `--prove` runs the whole write
//! and its verification inside a transaction and then rolls back — the same
//! technique the migration dry-run uses, and the same reason: every statement
//! executes for real, and nothing survives.
//!
//! ## Re-running is refused, not merged
//!
//! A case that already holds chronology events is refused with nothing written.
//! An event has no natural key — two real events can share a date and a title —
//! so there is nothing to upsert ON, and a second run that "merged" would
//! silently double the chronology.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use colossus_legal_backend::chronology::seed::{build_plan, parse_source, SeedError, SeedPlan};
use colossus_legal_backend::chronology::seed_execute::{run, SeedExecError, SeedMode, SeedOutcome};
use colossus_legal_backend::chronology::seed_report::{render_outcome, render_report};
use colossus_legal_backend::oneshot::cli::{
    connect_pool, emit_report, init_tracing, pipeline_database_url,
};
use colossus_legal_backend::oneshot::exit::{
    help_text, EXIT_BAD_INPUT, EXIT_EXECUTION_FAILED, EXIT_OK, EXIT_UNIT_ABORTED, EXIT_UNSAFE_PLAN,
};
use tracing::{error, info, warn};

/// CLI arguments. Field doc comments double as `--help` text.
#[derive(Parser, Debug)]
#[command(
    name = "seed_chronology",
    about = "Load an archived legacy timeline document into the chronology tables. Dry run unless --apply.",
    after_help = help_text()
)]
struct Args {
    /// The case the events belong to, e.g. the slug `scenarios.case_slug` holds.
    #[arg(long, value_name = "SLUG")]
    case_slug: String,

    /// Who the 22 rows are attributed to. Required — see the module header.
    #[arg(long, value_name = "NAME")]
    created_by: String,

    /// The timeline document to read. REQUIRED — there is no default any more.
    ///
    /// ## ⚑ The file this used to default to has been deleted
    ///
    /// `frontend/public/data/timeline.json` retired with Phase B (ruling R15):
    /// the phases and tags are tables now, the 22 events are rows, and nothing
    /// in the product reads it. This tool has RUN. Keeping a default that points
    /// at a deleted path would turn "the seed is finished" into a file-not-found
    /// an operator has to interpret; making the argument required means anyone
    /// re-running it must deliberately supply an archived copy, which is the
    /// only circumstance in which running it again makes sense. Against a case
    /// that already holds events it refuses anyway.
    #[arg(long, value_name = "PATH")]
    source: PathBuf,

    /// Execute every insert and its verification inside a transaction, then ROLL
    /// BACK. Proves the writes without keeping them.
    #[arg(long)]
    prove: bool,

    /// WRITE AND COMMIT. Without this the tool plans, proves and keeps nothing.
    #[arg(long)]
    apply: bool,

    /// Where the count proof is written. It is also printed.
    #[arg(
        long,
        value_name = "PATH",
        default_value = "chronology_seed_report.txt"
    )]
    report: PathBuf,

    /// The pipeline Postgres URL. Falls back to `PIPELINE_DATABASE_URL`.
    #[arg(long, value_name = "URL")]
    database_url: Option<String>,
}

/// Which mode the flags ask for, or a refusal if they ask for two.
fn mode_for(args: &Args) -> Result<SeedMode, ExitCode> {
    match (args.prove, args.apply) {
        (true, true) => {
            error!("--prove and --apply are different acts; pass one or the other");
            Err(ExitCode::from(EXIT_BAD_INPUT))
        }
        (true, false) => Ok(SeedMode::ProveInTransaction),
        (false, true) => Ok(SeedMode::Apply),
        (false, false) => Ok(SeedMode::DryRun),
    }
}

/// Read and plan the source file, or refuse with the family's input code.
fn plan_from(source: &PathBuf) -> Result<SeedPlan, ExitCode> {
    let path = source.to_string_lossy().to_string();
    let raw = std::fs::read_to_string(source).map_err(|e| {
        let err = SeedError::Unreadable {
            path: path.clone(),
            cause: e.to_string(),
        };
        error!(error = %err, "the chronology seed file could not be read");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;

    let parsed = parse_source(&path, &raw).map_err(|err| {
        error!(error = %err, "the chronology seed file is not the document this tool expects");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;

    build_plan(&parsed).map_err(|err| {
        error!(error = %err, "the chronology seed refused to plan; nothing was written");
        ExitCode::from(EXIT_BAD_INPUT)
    })
}

/// The exit code one execution failure earns, and the log line that explains it.
fn exit_for(error: &SeedExecError) -> ExitCode {
    match error {
        // The tool proved, before writing, that the plan cannot be justified.
        SeedExecError::AlreadySeeded { .. } | SeedExecError::MissingTargets { .. } => {
            error!(error = %error, "the plan is unsafe; nothing was written");
            ExitCode::from(EXIT_UNSAFE_PLAN)
        }
        // Ran, counted, and refused to agree with itself. The DATA needs a look.
        SeedExecError::Verification { .. } => {
            error!(error = %error, "verification failed; the transaction was rolled back");
            ExitCode::from(EXIT_UNIT_ABORTED)
        }
        SeedExecError::Database(_) => {
            // The error's own Display already names the database and the cause;
            // this line adds the only fact it does not carry — what survived.
            error!(error = %error, "nothing was written");
            ExitCode::from(EXIT_EXECUTION_FAILED)
        }
    }
}

/// Log the outcome at the level its mode deserves.
fn announce(outcome: &SeedOutcome, mode: SeedMode) {
    match mode {
        SeedMode::Apply => info!(
            events = outcome.events_written,
            links = outcome.links_written,
            phases = outcome.phases_present,
            "chronology seeded and committed"
        ),
        SeedMode::ProveInTransaction => warn!(
            events = outcome.events_written,
            links = outcome.links_written,
            phases = outcome.phases_present,
            rolled_back = outcome.rolled_back,
            "chronology seed PROVED and rolled back; nothing was kept"
        ),
        SeedMode::DryRun => info!(
            events = outcome.events_written,
            links = outcome.links_written,
            phases = outcome.phases_present,
            "chronology seed dry run; nothing was written"
        ),
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    // best-effort: a `.env` is how a developer supplies PIPELINE_DATABASE_URL from
    // a checkout; in the container the same variable arrives from the runtime and
    // there is no file to read. Its absence is therefore normal and not an error —
    // and if the variable is missing too, `pipeline_database_url` refuses by name
    // in `execute`, which is where that failure belongs.
    dotenvy::dotenv().ok();
    init_tracing();
    let args = Args::parse();
    match execute(&args).await {
        Ok(code) | Err(code) => code,
    }
}

/// The whole run. Split out of `main` so every step can use `?` instead of a
/// six-deep `match`, and so `main` stays short enough to read at a glance.
///
/// ## Rust Learning: `Result<ExitCode, ExitCode>`
///
/// Both arms are the same type because every path here ends in a code, and the
/// distinction the `Result` carries is "kept going" vs "stopped early" — which
/// is exactly what `?` is for. `main` then collapses the two arms, because to
/// the operating system there is no difference.
async fn execute(args: &Args) -> Result<ExitCode, ExitCode> {
    let mode = mode_for(args)?;
    let plan = plan_from(&args.source)?;
    let rendered = render_report(
        &plan,
        &args.case_slug,
        &args.created_by,
        mode == SeedMode::Apply,
    );

    // THE PLAN IS PRINTED BEFORE THE DATABASE IS TOUCHED, on a dry run.
    //
    // The re-point map and the 22 rows are facts about the FILE, and an operator
    // eyeballing them should not need a reachable Postgres to do it. It also
    // means that when the tables are not deployed yet, the dry run still hands
    // back the thing it was run for before it reports what it could not check.
    if mode == SeedMode::DryRun {
        println!("{rendered}");
    }

    let url = pipeline_database_url(args.database_url.as_deref())?;
    let pool = connect_pool(&url).await?;
    let expected_phases = plan_expected_phases(&args.source)?;

    let outcome = run(
        &pool,
        &plan,
        &args.case_slug,
        &args.created_by,
        expected_phases,
        mode,
    )
    .await
    .map_err(|e| exit_for(&e))?;

    // THE FILE IS WRITTEN ONLY ONCE THE OUTCOME IS KNOWN, and it carries that
    // outcome. Writing it beside the early print would have produced a report
    // saying "DRY RUN — nothing written" even when the target check then failed
    // — a file that reads as a clean run when it was not. A failed run leaves no
    // new file, and its exit code and log say why.
    let outcome_section = render_outcome(&outcome, mode);
    if mode == SeedMode::DryRun {
        println!("{outcome_section}");
    } else {
        println!("{rendered}{outcome_section}");
    }
    emit_report(&format!("{rendered}{outcome_section}"), &args.report)?;
    announce(&outcome, mode);
    Ok(ExitCode::from(EXIT_OK))
}

/// How many phase rows the source file says there should be.
///
/// Re-reads the file rather than threading the parsed document through the plan:
/// the plan is deliberately about EVENTS, and giving it a phase count it never
/// uses would be a field carried for one caller.
fn plan_expected_phases(source: &PathBuf) -> Result<i64, ExitCode> {
    let path = source.to_string_lossy().to_string();
    let raw = std::fs::read_to_string(source).map_err(|e| {
        error!(path = %path, error = %e, "the chronology seed file could not be re-read");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;
    let parsed = parse_source(&path, &raw).map_err(|err| {
        error!(error = %err, "the chronology seed file is not the document this tool expects");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;
    Ok(parsed.phases.len() as i64)
}
