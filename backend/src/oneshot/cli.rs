//! Connection and report-file plumbing shared by every one-shot binary.
//!
//! Each binary is meant to be a thin shell — parse args, open two connections,
//! hand off to its own `execute`, write the report, translate errors into exit
//! codes. Three of those five steps are identical across the family, so they live
//! here and the binaries keep only what is genuinely theirs.
//!
//! ## Why these return `ExitCode` and not a rich error
//!
//! They are called from `main`, where the only thing that can be done with a
//! failure is to log it and exit with the right number. Returning an error type
//! would mean every binary writing the same `match` to turn it back into the same
//! number — so the log happens here, beside the code it earns, and the caller
//! gets the number.

use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use neo4rs::Graph;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::error;

use super::exit::{EXIT_BAD_INPUT, EXIT_CONNECTION};

/// How long a one-shot tool waits for a Postgres connection before giving up.
///
// STRUCTURAL: not a per-deployment value. These tools run interactively from a
// runbook step with an operator watching, and the value is about that human, not
// about the deployment: a long hang reads as "it's working" when it means "the
// host is wrong". The family's exit code `2` exists to say so quickly, and an
// env var that let someone raise this to five minutes would defeat it. Nothing
// varies per environment here — the same operator runs it against DEV and PROD.
const POOL_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(10);

/// How many Postgres connections a one-shot tool opens.
///
// STRUCTURAL: not a per-deployment value; nothing about it varies per environment. These
// tools are single-threaded over a handful of documents or clusters and issue one
// statement at a time; four is headroom, not a tuning knob, and no deployment has
// a reason to want a different number. Contrast `PIPELINE_DATABASE_URL`, which is
// read at runtime precisely because it DOES differ between DEV and PROD.
const POOL_MAX_CONNECTIONS: u32 = 4;

/// Open Neo4j from the standard env vars.
///
/// `NEO4J_URI` and `NEO4J_PASSWORD` are required; `NEO4J_USER` defaults to
/// `neo4j`, which is the account name Neo4j itself ships with and the one every
/// deployment in this project uses. The two required vars have no default on
/// purpose: guessing a URI would produce a confusing connection error instead of
/// the clear "this is not configured" the operator needs.
pub async fn connect_graph() -> Result<Graph, ExitCode> {
    let uri = std::env::var("NEO4J_URI").map_err(|_| {
        error!("NEO4J_URI is not set (a .env file is read automatically)");
        ExitCode::from(EXIT_CONNECTION)
    })?;
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let password = std::env::var("NEO4J_PASSWORD").map_err(|_| {
        error!("NEO4J_PASSWORD is not set (a .env file is read automatically)");
        ExitCode::from(EXIT_CONNECTION)
    })?;

    Graph::new(&uri, &user, &password).await.map_err(|e| {
        error!(error = %e, uri = %uri, "could not connect to Neo4j");
        ExitCode::from(EXIT_CONNECTION)
    })
}

/// Resolve the pipeline Postgres URL from a CLI flag or the environment.
///
/// Domain note: it is the PIPELINE database (`colossus_legal_v2`) every one of
/// these tools wants — that is where the curated scenario tables live. Reading
/// `DATABASE_URL` as a fallback would silently point a merge at the main
/// database, where none of the tables exist, and the tool would report a very
/// confident zero. So the only env fallback is `PIPELINE_DATABASE_URL`.
pub fn pipeline_database_url(flag: Option<&str>) -> Result<String, ExitCode> {
    if let Some(url) = flag {
        return Ok(url.to_string());
    }
    std::env::var("PIPELINE_DATABASE_URL").map_err(|_| {
        error!(
            "no Postgres URL: pass --database-url or set PIPELINE_DATABASE_URL \
             (a .env file is read automatically). Note this is the PIPELINE \
             database, colossus_legal_v2 — not DATABASE_URL"
        );
        ExitCode::from(EXIT_BAD_INPUT)
    })
}

/// Open the pipeline Postgres pool.
pub async fn connect_pool(database_url: &str) -> Result<PgPool, ExitCode> {
    PgPoolOptions::new()
        .max_connections(POOL_MAX_CONNECTIONS)
        .acquire_timeout(POOL_ACQUIRE_TIMEOUT)
        .connect(database_url)
        .await
        .map_err(|e| {
            error!(error = %e, "could not connect to Postgres");
            ExitCode::from(EXIT_CONNECTION)
        })
}

/// Print the count proof and write it to disk.
///
/// Both, always, and printing FIRST. If the write fails, the proof has already
/// reached the operator's terminal — which is the whole reason the order is this
/// way round and not the other. The error message says so, because an operator
/// who loses that terminal loses the only record of what a `--apply` run did.
pub fn emit_report(rendered: &str, path: &Path) -> Result<(), ExitCode> {
    println!("{rendered}");
    std::fs::write(path, rendered).map_err(|e| {
        error!(
            error = %e,
            path = %path.display(),
            "the run finished but its report could not be written — the proof \
             printed above is the only copy; capture it before this terminal is lost"
        );
        ExitCode::from(EXIT_BAD_INPUT)
    })?;
    tracing::info!(path = %path.display(), "count proof written");
    Ok(())
}

/// Initialise logging the same way for every tool in the family.
///
/// One `RUST_LOG` works identically across all four binaries, which matters when
/// a runbook step tells an operator to re-run "with more logging".
pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
