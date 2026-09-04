//! `gate_fixture` — freeze two already-paid-for scans into JSON the reranker
//! gate (G1) can be run against, for nothing, by anyone, forever.
//!
//! **READ-ONLY. There is no `--apply`, no write path, and no subcommand that
//! changes anything.** Every Postgres connection this tool opens is put into
//! `SET SESSION CHARACTERISTICS AS TRANSACTION READ ONLY` before it is used, so
//! the database itself refuses a write nobody intended; every Cypher statement
//! is a bare `MATCH … RETURN`. **It also calls no paid API and embeds nothing —
//! it costs zero dollars to run.**
//!
//! ```text
//! cd backend && \
//! GATE_FIXTURE_DATABASE_URL="$(grep '^PIPELINE_DATABASE_URL=' .env | cut -d= -f2-)" \
//! NEO4J_URI=bolt://10.10.100.200:7687 NEO4J_USER=neo4j NEO4J_PASSWORD=<dev> \
//! cargo run --bin gate_fixture -- \
//!   --expect-database colossus_legal_v2 \
//!   --case-slug awad_v_catholic_family_service \
//!   --out-dir "$HOME/Documents/colossus-legal/GATE" \
//!   --scenario S-11 --run S-11=2026-08-29 --expect S-11=292/44/10/7 \
//!   --file S-11=s11_gate_fixture_v1.json \
//!   --outside S-11=<evidence-id> …
//! ```
//!
//! ## Why the connection comes from ITS OWN environment variable
//!
//! `backend/.env` is read automatically by every other tool in this family, and
//! what it holds is whatever the last local experiment needed. A fixture is a
//! claim about DEV; a fixture silently extracted from a scratch database would
//! be a false one that looks identical. So this binary never calls `dotenvy`,
//! takes its URL from `--database-url` or `GATE_FIXTURE_DATABASE_URL` and from
//! nowhere else, prints the database it actually reached as its FIRST line of
//! output, and refuses to continue unless that name is the one the operator
//! named on the command line.
//!
//! ## Reading the output
//!
//! ```text
//! S-11 : candidates 292 · opus_relevant 44 · included 10 · outside_pool 7
//! ```
//!
//! A number in parentheses beside a count — `292 (EXPECTED 251)` — is the whole
//! point of the run, not a failure of it. **A count is never adjusted to match.**
//! A tuned query that produces the expected number is a fabricated fixture and
//! destroys the gate; a wrong count is information about the scan history, and
//! it is printed so a human can act on it.

//! ## The module split, and why the binary is a directory
//!
//! Cargo builds `src/bin/<name>/main.rs` as the binary `<name>`, and — unlike a
//! flat `src/bin/*.rs` — its SIBLING files are ordinary modules rather than more
//! binaries. That is the only arrangement that lets this tool obey Rule 17 (no
//! module over 300 code lines) without either stripping the teaching comments or
//! pushing one-shot plumbing into the library, where nothing else would use it.
//!
//! - `main` — the command line, the connection, and the order of the work.
//! - `plan` — the repeatable `CODE=VALUE` flags, parsed once.
//! - `build` — the reads, and one fixture assembled from them.
//! - `output` — the JSON, the printed report, and the README.

mod build;
mod output;
mod plan;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use colossus_legal_backend::oneshot::cli::{connect_graph, init_tracing};
use colossus_legal_backend::oneshot::exit::{
    help_text, EXIT_BAD_INPUT, EXIT_CONNECTION, EXIT_OK, EXIT_UNIT_ABORTED,
};
use colossus_legal_backend::services::gate_fixture::audit_fixture;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::error;

use crate::build::build_fixture;
use crate::output::{print_assertion_block, report, write_json, write_readme};
use crate::plan::Plan;

/// How long the pool waits for a connection, in seconds.
///
/// ## Why this is a function and not a `const`
///
/// A pinned `const Duration` is a tunable with nowhere to turn it — and the one
/// time it bites is the one time it matters, on a slow link where the operator
/// needs the run to succeed rather than to fail quickly. So it reads an
/// environment variable and falls back, and there is no named constant for a
/// later reader to mistake for an invariant.
fn pool_acquire_timeout() -> Duration {
    // DEFAULT: 10 seconds, because a one-shot tool is run by a human watching the
    // terminal, and a long hang reads as "it's working" when it means "the host is
    // wrong". Override with GATE_FIXTURE_ACQUIRE_TIMEOUT_SECS=<n> when the link to
    // the database host is slow enough that ten seconds is a false negative.
    // Unparseable or absent falls back to the same 10 — deliberately: a typo in the
    // variable must not silently become a zero-second timeout that fails every run.
    const FALLBACK_SECS: u64 = 10;
    let secs = std::env::var("GATE_FIXTURE_ACQUIRE_TIMEOUT_SECS")
        .ok()
        .and_then(|raw| match raw.trim().parse::<u64>() {
            Ok(n) if n > 0 => Some(n),
            // A set-but-unusable value is the operator trying to do something and
            // failing; saying so is the difference between "ignored" and "silent".
            _ => {
                error!(
                    value = %raw,
                    "GATE_FIXTURE_ACQUIRE_TIMEOUT_SECS is not a positive integer — using the default"
                );
                None
            }
        })
        .unwrap_or(FALLBACK_SECS);
    Duration::from_secs(secs)
}

/// The statement every pooled connection runs before it is used.
///
/// Domain note: this is belt AND braces. The code below issues only `SELECT`s,
/// but "the code only issues SELECTs" is a claim about a file somebody may edit;
/// this is a claim the SERVER enforces. A stray `INSERT` on a connection carrying
/// it fails with `cannot execute INSERT in a read-only transaction` rather than
/// changing DEV.
//
// STRUCTURAL: Postgres wire vocabulary. "Read-only" is not a deployment setting
// — it is the invariant this entire tool exists to enforce, and there is no
// environment in which a legitimate deployment would want it relaxed. A config
// key here would be a switch for turning the guarantee off.
const READ_ONLY_SESSION: &str = "SET SESSION CHARACTERISTICS AS TRANSACTION READ ONLY";

#[derive(Parser, Debug)]
#[command(
    name = "gate_fixture",
    about = "Freeze completed scans into gate fixtures. READ-ONLY, zero cost.",
    after_help = help_text()
)]
pub(crate) struct Args {
    /// Postgres URL for the PIPELINE database. Falls back to
    /// `GATE_FIXTURE_DATABASE_URL`. Deliberately NOT `PIPELINE_DATABASE_URL`
    /// and deliberately not read from `.env` — see the module doc.
    #[arg(long)]
    pub(crate) database_url: Option<String>,

    /// The database name this run must be reading. No default: naming it is how
    /// the operator states which corpus the fixture is a claim about.
    #[arg(long)]
    pub(crate) expect_database: String,

    /// The case whose scenarios are being frozen.
    #[arg(long)]
    pub(crate) case_slug: String,

    /// Where the JSON and the README are written.
    #[arg(long)]
    pub(crate) out_dir: PathBuf,

    /// A scenario code to freeze, e.g. `S-11`. Repeatable; processed in order.
    #[arg(long = "scenario")]
    pub(crate) scenarios: Vec<String>,

    /// `CODE=YYYY-MM-DD` — which completed run of that scenario to freeze.
    #[arg(long = "run")]
    pub(crate) runs: Vec<String>,

    /// `CODE=candidates/relevant/included/outside` — the counts to assert.
    #[arg(long = "expect")]
    pub(crate) expects: Vec<String>,

    /// `CODE=filename.json` — the fixture's file name inside `--out-dir`.
    #[arg(long = "file")]
    pub(crate) files: Vec<String>,

    /// `CODE=<evidence-id>` — one card the scenario's pool cannot see today.
    /// Repeatable. Ids, not a search: the seven $50,000 admissions are named by
    /// their meaning, and a document+page filter that happened to return seven
    /// rows would be a coincidence the fixture then depended on.
    #[arg(long = "outside")]
    pub(crate) outside: Vec<String>,

    /// How a page number is rendered on a card. `{page}` is the only fill.
    #[arg(long, default_value = "p. {page}")]
    pub(crate) pinpoint_template: String,
}

#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();
    // NOTE: no `dotenvy::dotenv()` here, unlike every sibling binary. See the
    // module doc — a fixture read from whatever `.env` holds is a claim nobody
    // can check.
    match execute(Args::parse()).await {
        Ok(code) => code,
        Err(code) => code,
    }
}

async fn execute(args: Args) -> Result<ExitCode, ExitCode> {
    let url = database_url(args.database_url.as_deref())?;
    let pool = open_read_only_pool(&url).await?;
    confirm_database(&pool, &args.expect_database).await?;

    let graph = connect_graph().await?;
    let plan = Plan::from_args(&args)?;

    let mut audits = Vec::new();
    for code in &args.scenarios {
        let fixture = build_fixture(&pool, &graph, &args, &plan, code).await?;
        let audit = audit_fixture(&fixture, plan.expected_counts(code)?);
        write_json(&args.out_dir, plan.file(code)?, &fixture)?;
        report(&fixture, &audit);
        audits.push((fixture, audit));
    }

    write_readme(&args, &audits)?;
    print_assertion_block(&audits);

    // A structural failure is the finding, so it is also the exit code — a
    // runbook step can gate on it without parsing the text. A COUNT mismatch is
    // deliberately NOT a failure: it is history, and it is already printed.
    let sound = audits.iter().all(|(_, a)| a.structurally_sound());
    Ok(ExitCode::from(if sound {
        EXIT_OK
    } else {
        EXIT_UNIT_ABORTED
    }))
}

// ── Connection ────────────────────────────────────────────────────────────────

/// Resolve the URL from the flag or this tool's OWN environment variable.
fn database_url(flag: Option<&str>) -> Result<String, ExitCode> {
    if let Some(url) = flag {
        return Ok(url.to_string());
    }
    std::env::var("GATE_FIXTURE_DATABASE_URL").map_err(|_| {
        error!(
            "no Postgres URL: pass --database-url or set GATE_FIXTURE_DATABASE_URL. \
             This tool does NOT read .env, on purpose — point it at the pipeline \
             database (colossus_legal_v2) explicitly"
        );
        ExitCode::from(EXIT_BAD_INPUT)
    })
}

/// Open the pool with every connection already in a read-only session.
///
/// ## Rust Learning: `after_connect` and the boxed future
///
/// `after_connect` takes a closure returning a boxed future because it runs on
/// EVERY connection the pool opens — including ones opened later, under load, to
/// replace a dropped one. Setting the session once after `connect()` would leave
/// those later connections writable. `Box::pin(async move { … })` is how a
/// closure returns an `async` block from a non-async signature.
async fn open_read_only_pool(url: &str) -> Result<PgPool, ExitCode> {
    PgPoolOptions::new()
        // One connection: this tool is strictly sequential, and a single
        // connection makes "every connection is read-only" a one-line claim.
        .max_connections(1)
        .acquire_timeout(pool_acquire_timeout())
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query(READ_ONLY_SESSION).execute(conn).await?;
                Ok(())
            })
        })
        .connect(url)
        .await
        .map_err(|e| {
            error!(error = %e, "could not connect to Postgres, or could not set the session read-only");
            ExitCode::from(EXIT_CONNECTION)
        })
}

/// Print what we actually reached, and stop unless it is what was asked for.
async fn confirm_database(pool: &PgPool, expected: &str) -> Result<(), ExitCode> {
    let (name, host, read_only): (String, String, String) = sqlx::query_as(
        "SELECT current_database(), \
                coalesce(host(inet_server_addr()), 'local socket'), \
                current_setting('transaction_read_only')",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| {
        error!(error = %e, "could not read current_database() — the connection is unusable");
        ExitCode::from(EXIT_CONNECTION)
    })?;

    // FIRST line of output, before anything is read: the operator's proof of
    // which corpus this fixture is a claim about, and that it cannot be written.
    println!("reading database '{name}' on host {host} · transaction_read_only={read_only}");

    if name != expected {
        error!(
            connected = %name, expected = %expected,
            "connected to the wrong database — nothing was read"
        );
        return Err(ExitCode::from(EXIT_BAD_INPUT));
    }
    Ok(())
}
