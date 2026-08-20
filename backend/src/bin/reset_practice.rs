//! `reset_practice` — put one scenario's drill back to "never practised".
//!
//! One of the one-shot maintenance family (see `oneshot`): dry run is the
//! default, `--apply` is the only writing path, the count proof is real output,
//! and the exit codes are the family's.
//!
//! ```text
//! # 1. Prove it first — counts what would go, writes nothing:
//! cargo run --bin reset_practice -- --scenario S-6
//!
//! # 2. Clear it:
//! cargo run --bin reset_practice -- --scenario S-6 --apply
//! ```
//!
//! ## What it clears, and what it deliberately keeps
//!
//! Clears every sitting, every answer and every note for that scenario. Keeps
//! `practice_questions` (the deck: text, order, flags, hidden state) and
//! `practice_deck_changes` (who edited the deck, and when) — see
//! `practice::reset`'s header for why those two are not a rehearsal's to throw
//! away.
//!
//! ## Why the dry run is the default on THIS tool especially
//!
//! It deletes a witness's own record of her preparation, and there is no undo.
//! An operator should have to read the counts and type a second command before
//! that happens.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use colossus_legal_backend::oneshot::cli::{
    connect_pool, emit_report, init_tracing, pipeline_database_url,
};
use colossus_legal_backend::oneshot::exit::{help_text, EXIT_BAD_INPUT, EXIT_OK, EXIT_UNSAFE_PLAN};
use colossus_legal_backend::practice::reset::{
    apply, count, render_report, scenario_id, ResetError,
};
use tracing::{error, info, warn};

/// CLI arguments. Field doc comments double as `--help` text.
#[derive(Parser, Debug)]
#[command(
    name = "reset_practice",
    about = "Clear one scenario's sittings, answers and notes. The deck is kept. Dry run unless --apply.",
    after_help = help_text()
)]
struct Args {
    /// The scenario code to reset, e.g. S-6.
    #[arg(long, value_name = "CODE")]
    scenario: String,

    /// WRITE. Without this the tool counts, proves and changes nothing.
    #[arg(long)]
    apply: bool,

    /// Where the count proof is written. It is also printed.
    #[arg(long, value_name = "PATH", default_value = "practice_reset_report.txt")]
    report: PathBuf,

    /// The pipeline Postgres URL. Falls back to `PIPELINE_DATABASE_URL`.
    #[arg(long, value_name = "URL")]
    database_url: Option<String>,
}

/// The exit code one [`ResetError`] earns, and the log line that explains it.
///
/// The family's codes are a CONTRACT with the runbook: `1` is bad input, `4` is
/// a plan the tool refuses to execute or a write that failed. Keeping the
/// translation in one function is what stops a later arm being added with
/// whichever number was nearest.
fn exit_for(error: &ResetError) -> ExitCode {
    match error {
        // An operator fixes this by typing a code that exists.
        ResetError::UnknownScenario { .. } => {
            error!(error = %error, "no such scenario; nothing was written");
            ExitCode::from(EXIT_BAD_INPUT)
        }
        // The transaction rolled back. Nothing was written, by construction.
        ResetError::Database { .. } => {
            error!(error = %error, "the reset failed against the database; nothing was written");
            ExitCode::from(EXIT_UNSAFE_PLAN)
        }
    }
}

/// Everything that can stop this binary, as the number it exits with.
///
/// ## Rust Learning: two error families, one return type
///
/// The connection helpers in `oneshot::cli` have ALREADY logged their failure and
/// chosen a code; re-reporting them here would print a second, vaguer sentence
/// over the specific one. So they arrive pre-decided as `Reported`, and only the
/// reset's own errors go through `exit_for`.
enum RunFailure {
    /// Already logged by the helper that produced it.
    Reported(ExitCode),
    /// Not yet logged; `exit_for` says it and picks the code.
    Reset(ResetError),
}

/// Do the work. `main` exists to turn its `Result` into a number.
async fn execute(args: &Args) -> Result<String, RunFailure> {
    let code = args.scenario.trim();
    let url = pipeline_database_url(args.database_url.as_deref()).map_err(RunFailure::Reported)?;
    let pool = connect_pool(&url).await.map_err(RunFailure::Reported)?;

    // Resolve the code BEFORE counting anything. A typo must be a refusal, not a
    // row of zeroes an operator reads as "already clean".
    let id = scenario_id(&pool, code).await.map_err(RunFailure::Reset)?;
    info!(scenario = %code, scenario_id = %id, "resolved the scenario");

    if !args.apply {
        let before = count(&pool, id, code).await.map_err(RunFailure::Reset)?;
        if before.is_empty() {
            info!(scenario = %code, "nothing to clear — this scenario has never been practised");
        }
        warn!("DRY RUN — re-run with --apply to clear this scenario's practice record");
        return Ok(render_report(code, &before, None));
    }

    let (before, after) = apply(&pool, id, code).await.map_err(RunFailure::Reset)?;
    info!(
        scenario = %code,
        sessions = before.sessions,
        answers = before.answers,
        notes = before.notes,
        "practice record cleared"
    );
    // Said out loud rather than left to the reader of a report file: an --apply
    // that left rows behind is the one outcome this tool must not report quietly.
    if !after.is_empty() {
        error!(
            scenario = %code,
            sessions = after.sessions,
            answers = after.answers,
            notes = after.notes,
            "rows survived the reset — the transaction committed but the tables are not empty"
        );
    }
    Ok(render_report(code, &before, Some(&after)))
}

#[tokio::main]
async fn main() -> ExitCode {
    // A `.env` beside the checkout is how an operator points this at DEV without
    // exporting a URL into their shell history.
    // best-effort: an absent .env is the normal case in the container, where the
    // variables arrive from the unit file.
    let _ = dotenvy::dotenv();
    init_tracing();

    let args = Args::parse();
    match execute(&args).await {
        Ok(rendered) => match emit_report(&rendered, &args.report) {
            Ok(()) => ExitCode::from(EXIT_OK),
            Err(code) => code,
        },
        Err(RunFailure::Reported(code)) => code,
        Err(RunFailure::Reset(error)) => exit_for(&error),
    }
}
