//! `backfill_evidence_search` — fill the lexical mirror from the graph, once.
//!
//! Neo4j is the source of truth for Evidence; `evidence_search` is a derived
//! copy of its text so Postgres full-text and trigram search can reach it. This
//! binary is the ONE-TIME fill. Steady state is L1c: the pipeline's index step
//! writing the same rows beside its Qdrant upsert, through the same
//! [`upsert_evidence_search_rows`] this calls.
//!
//! **The graph read is READ-ONLY** — one `MATCH … RETURN`, asserted by a shape
//! test in `evidence_search_repository`. The only writes are to
//! `evidence_search` in the Postgres database the operator names.
//!
//! ```text
//! cd backend && \
//! EVIDENCE_SEARCH_DATABASE_URL="postgres://…/colossus_legal_v2" \
//! NEO4J_URI=bolt://10.10.100.200:7687 NEO4J_USER=neo4j NEO4J_PASSWORD=<dev> \
//! QDRANT_URL=http://10.10.100.200:6333 \
//! cargo run --bin backfill_evidence_search -- --expect-database colossus_legal_v2
//! ```
//!
//! ## Why its own environment variable
//!
//! Same reasoning as `gate_fixture`: this binary WRITES, and what it writes is a
//! search corpus. Falling back to whatever `.env` holds would make "which
//! database did I just fill?" a question nobody can answer after the fact. So it
//! reads `EVIDENCE_SEARCH_DATABASE_URL` (or `--database-url`), never `.env`,
//! prints the database it reached as its first line, and refuses to continue
//! unless that name is the one the operator named on the command line.
//!
//! ## Reading the output
//!
//! ```text
//! graph Evidence nodes      : 1209
//! rows in evidence_search   : 1209
//! Qdrant colossus_evidence  : 1209
//! ```
//!
//! The first two MUST agree; if they do not, the run exits non-zero and nothing
//! is deleted or retried to make them agree — a short mirror is a fact about the
//! read, and papering over it would hide the one defect this tool exists to
//! prevent. The third is informational: a mismatch there is a finding about the
//! VECTOR index (the other half of the gather), not about this backfill.
//!
//! ## The module split, and why the binary is a directory
//!
//! Cargo builds `src/bin/<name>/main.rs` as the binary `<name>`, and its SIBLING
//! files are ordinary modules rather than more binaries. That is what lets this
//! tool obey Rule 17 (no module over 300 code lines) without stripping the
//! teaching comments or pushing one-shot plumbing into the library. Same
//! arrangement as `bin/gate_fixture/`.
//!
//! - `main` — the command line, the connection, and the order of the work.
//! - `counts` — the three numbers and the pure verdict over them.
//! - `fill` — the paged read-and-upsert loop, and the Qdrant count.

mod counts;
mod fill;

use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use colossus_legal_backend::oneshot::cli::{connect_graph, init_tracing};
use colossus_legal_backend::oneshot::exit::{
    help_text, EXIT_BAD_INPUT, EXIT_CONNECTION, EXIT_OK, EXIT_UNIT_ABORTED,
};
use colossus_legal_backend::repositories::evidence_search_repository::{
    count_evidence_nodes, read_batch_size,
};
use colossus_legal_backend::repositories::pipeline_repository::count_evidence_search_rows;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::{error, info, warn};

use crate::counts::{judge_counts, CountVerdict, Counts};
use crate::fill::{fill, qdrant_evidence_points};

#[derive(Parser, Debug)]
#[command(
    name = "backfill_evidence_search",
    about = "Fill evidence_search from the graph, once. Graph read is read-only.",
    after_help = help_text()
)]
pub(crate) struct Args {
    /// Postgres URL for the database holding `evidence_search`. Falls back to
    /// `EVIDENCE_SEARCH_DATABASE_URL`. Never read from `.env` — see the module doc.
    #[arg(long)]
    database_url: Option<String>,

    /// The database name this run must be writing. No default: naming it is how
    /// the operator states which corpus they are filling.
    #[arg(long)]
    expect_database: String,

    /// Read and count, then stop without writing. The rehearsal.
    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();
    // No `dotenvy::dotenv()`, deliberately: this binary writes, and a write
    // aimed by whatever `.env` happened to hold is a write nobody can audit.
    match execute(Args::parse()).await {
        Ok(code) => code,
        Err(code) => code,
    }
}

async fn execute(args: Args) -> Result<ExitCode, ExitCode> {
    let url = database_url(args.database_url.as_deref())?;
    let pool = connect_pool(&url).await?;
    confirm_database(&pool, &args.expect_database).await?;

    let graph = connect_graph().await?;
    let graph_nodes = read(count_evidence_nodes(&graph).await, "graph Evidence count")?;
    info!(
        graph_nodes,
        batch = read_batch_size(),
        dry_run = args.dry_run,
        "starting the mirror backfill"
    );

    let written = if args.dry_run {
        warn!("--dry-run: reading and counting only, nothing will be written");
        0
    } else {
        fill(&pool, &graph).await?
    };

    let counts = Counts {
        graph_nodes,
        mirror_rows: read(count_evidence_search_rows(&pool).await, "mirror row count")?,
        qdrant_points: qdrant_evidence_points().await,
    };
    let verdict = judge_counts(counts);
    info!(written, "rows upserted");
    Ok(report(&verdict, counts, args.dry_run))
}

/// Print the counts and turn them into an exit code.
///
/// ## Why the exit code is decided here and not by the caller
///
/// A runbook step gates on the number, so the rule that produces it belongs
/// beside the words that explain it — one place a reader can check that "the
/// mirror is short" and "exit non-zero" mean the same thing. The COUNT is never
/// adjusted to make the code zero: a short mirror is a fact about the read.
fn report(verdict: &CountVerdict, counts: Counts, dry_run: bool) -> ExitCode {
    println!("\n=== COUNTS ===");
    for line in &verdict.lines {
        println!("{line}");
    }
    if let Some(finding) = &verdict.vector_index_finding {
        println!("\nFINDING: {finding}");
    }

    if dry_run {
        println!("\n--dry-run: nothing was written.");
        return ExitCode::from(EXIT_OK);
    }
    if !verdict.mirror_complete {
        error!(
            graph_nodes = counts.graph_nodes,
            mirror_rows = counts.mirror_rows,
            "the mirror does not match the graph — NOT adjusted; re-run to finish, \
             and if it stays short the read is dropping nodes"
        );
        return ExitCode::from(EXIT_UNIT_ABORTED);
    }
    println!("\nThe mirror matches the graph.");
    ExitCode::from(EXIT_OK)
}

// ── Connection ────────────────────────────────────────────────────────────────

fn database_url(flag: Option<&str>) -> Result<String, ExitCode> {
    if let Some(url) = flag {
        return Ok(url.to_string());
    }
    std::env::var("EVIDENCE_SEARCH_DATABASE_URL").map_err(|_| {
        error!(
            "no Postgres URL: pass --database-url or set EVIDENCE_SEARCH_DATABASE_URL. \
             This tool does NOT read .env, on purpose — it WRITES, and a write nobody \
             can audit is worse than no write"
        );
        ExitCode::from(EXIT_BAD_INPUT)
    })
}

async fn connect_pool(url: &str) -> Result<PgPool, ExitCode> {
    PgPoolOptions::new()
        .max_connections(1)
        // DEFAULT: 10 seconds to get a connection, because this tool is run by a
        // human watching a terminal and a long hang reads as "it's working"
        // when it means "the host is wrong". Override with
        // EVIDENCE_SEARCH_ACQUIRE_TIMEOUT_SECS=<n> on a slow link.
        .acquire_timeout(env_secs("EVIDENCE_SEARCH_ACQUIRE_TIMEOUT_SECS", 10))
        .connect(url)
        .await
        .map_err(|e| {
            error!(error = %e, "could not connect to Postgres");
            ExitCode::from(EXIT_CONNECTION)
        })
}

/// A duration read from an environment variable, in seconds, with a fallback.
///
/// One helper for every timeout this tool has, so the fallback is a PARAMETER
/// rather than a named constant and each call site names its own override
/// variable. That is what keeps these values genuinely configurable instead of
/// pinned behind a comment claiming they could be.
///
/// ## Rust Learning: `match` on the `Result`, not `.ok()`
///
/// `.ok()` would throw the "why" away. A variable that is SET but unparseable is
/// an operator trying to do something and failing, and that is a different state
/// from an unset variable — one deserves a complaint, the other is the normal
/// case. So the two arms are written out: absent is silent, present-and-bad is
/// an error, and neither silently becomes a zero-second timeout that would fail
/// every run.
pub(crate) fn env_secs(var: &str, fallback_secs: u64) -> Duration {
    let secs = match std::env::var(var) {
        Err(_) => fallback_secs,
        Ok(raw) => match raw.trim().parse::<u64>() {
            Ok(n) if n > 0 => n,
            _ => {
                error!(
                    variable = %var, value = %raw,
                    "not a positive integer — using the default instead"
                );
                fallback_secs
            }
        },
    };
    Duration::from_secs(secs)
}

/// Print which database we reached, and stop unless it is the one named.
async fn confirm_database(pool: &PgPool, expected: &str) -> Result<(), ExitCode> {
    let (name, host): (String, String) = sqlx::query_as(
        "SELECT current_database(), coalesce(host(inet_server_addr()), 'local socket')",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| {
        error!(error = %e, "could not read current_database() — the connection is unusable");
        ExitCode::from(EXIT_CONNECTION)
    })?;

    println!("writing database '{name}' on host {host}");

    if name != expected {
        error!(
            connected = %name, expected = %expected,
            "connected to the wrong database — nothing was written"
        );
        return Err(ExitCode::from(EXIT_BAD_INPUT));
    }
    Ok(())
}

/// Turn a repository error into a logged exit code, naming what was being read.
pub(crate) fn read<T, E: std::fmt::Display>(
    result: Result<T, E>,
    what: &str,
) -> Result<T, ExitCode> {
    result.map_err(|e| {
        error!(error = %e, operation = %what, "failed — the backfill stopped here");
        ExitCode::from(EXIT_CONNECTION)
    })
}
