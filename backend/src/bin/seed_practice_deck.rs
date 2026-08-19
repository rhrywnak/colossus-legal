//! `seed_practice_deck` — write one scenario's practice deck, once.
//!
//! One of the one-shot maintenance family (see `oneshot`): dry run is the
//! default, `--apply` is the only writing path, the count proof is real output,
//! and the exit codes are the family's.
//!
//! ```text
//! # 1. Prove it first — reads the scenario, resolves every source, writes nothing:
//! cargo run --bin seed_practice_deck -- --scenario S-5
//!
//! # 2. Write it:
//! cargo run --bin seed_practice_deck -- --scenario S-5 --apply
//!
//! # 3. Bring an EXISTING deck into line with an edited file (dry run first):
//! cargo run --bin seed_practice_deck -- --scenario S-5 --update
//! cargo run --bin seed_practice_deck -- --scenario S-5 --update --apply
//! ```
//!
//! ## The two modes, and why `--update` is not the default
//!
//! Without `--update` this tool writes a deck ONCE and refuses to touch one that
//! already exists — which is what makes a re-run safe on a scenario nobody meant
//! to change. `--update` is the deliberate second act: it reconciles the stored
//! deck with the file BY KEY, never deletes a row, and never touches an answer.
//! Making it the default would mean every accidental re-run rewrote a deck Chuck
//! had edited on the page.
//!
//! ## What it reads, and what it deliberately does not
//!
//! It reads the scenario row, its ruled accusation instances and its talking
//! points — the scenario's own record, and nothing else. It never opens Neo4j and
//! never reads the case graph: the questions are already written, by a human, in
//! `practice_decks/<code>.yaml`; the only thing this tool decides is which
//! instance or point each one binds to (PRACTICE_SESSION_DESIGN_v1 §5, "nothing
//! reads the whole graph").
//!
//! ## Why v0 does NOT draft questions with a model
//!
//! The task says so, and the reason is worth keeping: a question Marie is drilled
//! on is a sentence opposing counsel might really say, and the first version of
//! those was written by the architect from the record and reviewed by Roman.
//! Drafting is an option for the scenarios nobody has written a deck for yet;
//! it is not how the deck Chuck sees on Tuesday was made.
//!
//! ## Re-running is safe
//!
//! A scenario that already carries the same deck is a NO-OP that says so. One
//! that carries a DIFFERENT deck is a refusal with nothing written — a stored
//! question cannot be replaced while an answer cites it.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use colossus_legal_backend::oneshot::cli::{
    connect_pool, emit_report, init_tracing, pipeline_database_url,
};
use colossus_legal_backend::oneshot::exit::{help_text, EXIT_BAD_INPUT, EXIT_OK, EXIT_UNSAFE_PLAN};
use colossus_legal_backend::practice::seed::{load_deck, render_report, run, SeedError};
use colossus_legal_backend::practice::seed_update::{
    render_update_report, run_update, UpdateError,
};
use tracing::{error, info, warn};

/// CLI arguments. Field doc comments double as `--help` text.
#[derive(Parser, Debug)]
#[command(
    name = "seed_practice_deck",
    about = "Write one scenario's practice deck from its YAML file. Dry run unless --apply.",
    after_help = help_text()
)]
struct Args {
    /// The scenario code the deck is for, e.g. S-5.
    #[arg(long, value_name = "CODE")]
    scenario: String,

    /// Where the deck files live. Defaults to the repo's `practice_decks`.
    ///
    /// A relative default rather than an env var on purpose: this is an
    /// operator's tool run from a checkout or from the container's working
    /// directory, and both have the decks at the same relative place. Nothing
    /// about the path varies per environment, so nothing about it belongs in
    /// Ansible.
    #[arg(long, value_name = "DIR", default_value = "practice_decks")]
    deck_dir: PathBuf,

    /// Read the deck from THIS file instead of `<deck-dir>/<scenario>.yaml`.
    #[arg(long, value_name = "PATH")]
    deck: Option<PathBuf>,

    /// Reconcile an EXISTING deck with the file, matching rows by their key.
    ///
    /// Without it the tool writes a deck once and refuses to touch one that is
    /// already there. With it, rows are updated in place, new keys are inserted,
    /// nothing is ever deleted, and no answer is touched.
    #[arg(long)]
    update: bool,

    /// WRITE. Without this the tool plans, proves and changes nothing.
    #[arg(long)]
    apply: bool,

    /// Where the count proof is written. It is also printed.
    #[arg(
        long,
        value_name = "PATH",
        default_value = "practice_deck_seed_report.txt"
    )]
    report: PathBuf,

    /// The pipeline Postgres URL. Falls back to `PIPELINE_DATABASE_URL`.
    #[arg(long, value_name = "URL")]
    database_url: Option<String>,
}

/// The exit code one [`UpdateError`] earns, and the log line that explains it.
///
/// A sibling of [`exit_for`] rather than more arms on it: these are the refusals
/// of a DIFFERENT act, and the one arm they share defers to that function so the
/// two commands cannot disagree about what a bad file is worth.
fn exit_for_update(error: &UpdateError) -> ExitCode {
    match error {
        // An operator fixes these in an editor: a file with no keys, or a stored
        // row this run cannot honestly match to one.
        UpdateError::FileQuestionHasNoKey { .. } => {
            error!(error = %error, "the deck cannot drive --update; nothing was written");
            ExitCode::from(EXIT_BAD_INPUT)
        }
        // The tool proved, before touching anything, that reconciling would mean
        // guessing which stored question a file question is.
        UpdateError::StoredRowUnmatched { .. } | UpdateError::AmbiguousText { .. } => {
            error!(error = %error, "the plan is unsafe; nothing was written");
            ExitCode::from(EXIT_UNSAFE_PLAN)
        }
        UpdateError::Seed { source } => exit_for(source),
        UpdateError::Database { .. } => {
            error!(error = %error, "the update failed against the database");
            ExitCode::from(EXIT_UNSAFE_PLAN)
        }
    }
}

/// The exit code one [`SeedError`] earns, and the log line that explains it.
///
/// ## Why the mapping is a function and not a `match` inside `main`
///
/// The family's codes are a CONTRACT with the runbook: `1` is bad input, `4` is
/// a plan the tool refuses to execute. Keeping the translation in one place is
/// what stops a later arm being added with whichever number was nearest.
fn exit_for(error: &SeedError) -> ExitCode {
    match error {
        // The file, or what the file says — an operator fixes these in an editor.
        SeedError::Unreadable { .. }
        | SeedError::Unparseable { .. }
        | SeedError::Invalid { .. }
        | SeedError::NoSuchScenario { .. } => {
            error!(error = %error, "the deck cannot be used; nothing was written");
            ExitCode::from(EXIT_BAD_INPUT)
        }
        // The tool proved, before touching anything, that executing would leave a
        // state it cannot justify — a question citing evidence the scenario does
        // not have, or a replacement an answer may already depend on.
        SeedError::SourceOutOfRange { .. }
        | SeedError::PointOutOfRange { .. }
        | SeedError::DeckDiffers { .. } => {
            error!(error = %error, "the plan is unsafe; nothing was written");
            ExitCode::from(EXIT_UNSAFE_PLAN)
        }
        SeedError::Database { .. } => {
            error!(error = %error, "the seed failed against the database");
            ExitCode::from(EXIT_UNSAFE_PLAN)
        }
    }
}

/// Everything that can stop this binary, as the number it exits with.
///
/// ## Rust Learning: two error families, one return type
///
/// The connection helpers in `oneshot::cli` have ALREADY logged their failure and
/// hand back an `ExitCode`; a `SeedError` has not been logged and carries a
/// sentence a human needs to read. Folding the first into the second would have
/// meant inventing a `SeedError` variant to describe "Postgres was unreachable" —
/// and the tempting shortcut (reuse `NoSuchScenario`) would print "no scenario has
/// the code S-5" at an operator whose real problem is a wrong host. So the two
/// stay distinguishable all the way to `main`.
enum RunFailure {
    /// Already logged by the helper that produced it.
    Reported(ExitCode),
    /// Not yet logged; `exit_for` says it and picks the code.
    Seed(SeedError),
    /// Not yet logged; `exit_for_update` says it and picks the code.
    Update(UpdateError),
}

/// Do the work. `main` exists to turn its `Result` into a number.
async fn execute(args: &Args) -> Result<String, RunFailure> {
    let path = args
        .deck
        .clone()
        .unwrap_or_else(|| args.deck_dir.join(format!("{}.yaml", args.scenario.trim())));
    info!(path = %path.display(), scenario = %args.scenario, "reading the deck");

    let deck = load_deck(&path).map_err(RunFailure::Seed)?;
    let url = pipeline_database_url(args.database_url.as_deref()).map_err(RunFailure::Reported)?;
    let pool = connect_pool(&url).await.map_err(RunFailure::Reported)?;

    if args.update {
        let report = run_update(&pool, &deck, args.apply)
            .await
            .map_err(RunFailure::Update)?;
        if !args.apply {
            warn!("DRY RUN — re-run with --apply to update the deck");
        }
        return Ok(render_update_report(&report));
    }

    let report = run(&pool, &deck, args.apply)
        .await
        .map_err(RunFailure::Seed)?;
    if !args.apply && !report.already_seeded {
        warn!("DRY RUN — re-run with --apply to write the deck");
    }
    Ok(render_report(&report))
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
        Err(RunFailure::Seed(error)) => exit_for(&error),
        Err(RunFailure::Update(error)) => exit_for_update(&error),
    }
}
