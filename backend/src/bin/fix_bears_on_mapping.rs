//! `fix_bears_on_mapping` — durable add/delete/promote of `BEARS_ON`
//! Allegation→Element mappings, for B2 review corrections.
//!
//! A twin of `load_canonical_elements`: a thin CLI that parses args, opens the
//! standard Neo4j + Postgres connections from env, hands off to
//! [`mapping_correction::apply`], prints a summary, and maps any error to a
//! documented exit code. All logic lives in the `mapping_correction` library
//! module so it is testable without a live database.
//!
//! ## Exit codes
//! - `0` success (all requested corrections applied, or no-op)
//! - `1` input/argument/CSV-parse problem
//! - `2` Neo4j connection failure
//! - `3` store write failure / partial write (re-run the identical command)
//! - `4` validation failure (missing node, already-extracted, mapping-not-found)
//! - `5` Postgres connection failure
//!
//! ## Usage
//! ```text
//! # single pair
//! cargo run --bin fix_bears_on_mapping -- \
//!   --case-slug SLUG --op add|delete|promote --from ALLEGATION_ID --to ELEMENT_ID \
//!   [--database-url URL] [--dry-run] [--no-color]
//!
//! # batch (op,from,to per line; '#' comments and a header line are ignored)
//! cargo run --bin fix_bears_on_mapping -- \
//!   --case-slug SLUG --from-file corrections.csv [--database-url URL] [--dry-run]
//! ```
//! Neo4j connection from `NEO4J_URI` / `NEO4J_USER` / `NEO4J_PASSWORD`; Postgres
//! from `--database-url` or `PIPELINE_DATABASE_URL` — the same env vars (and
//! `.env`) the rest of the backend uses. `relationship_type` is fixed to
//! `BEARS_ON` in code and is not an argument.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use colossus_legal_backend::canonical_elements::Neo4jConfig;
use colossus_legal_backend::mapping_correction::{apply, csv, OpRequest, Operation, Summary};
use colossus_legal_backend::neo4j::check_neo4j;
use neo4rs::Graph;
use sqlx::postgres::PgPoolOptions;
use tracing::error;

/// CLI arguments. Field doc comments double as `--help` text.
#[derive(Parser, Debug)]
#[command(
    name = "fix_bears_on_mapping",
    about = "Durably add / delete / promote BEARS_ON Allegation→Element mappings."
)]
struct Args {
    /// Case slug written to `authored_relationships` (e.g.
    /// `awad_v_catholic_family_service`). Required — case-specific data comes
    /// from the operator, never from code (Standing Rule 2).
    #[arg(long)]
    case_slug: String,

    /// Operation for single-pair mode. Mutually exclusive with `--from-file`.
    #[arg(long, value_enum, requires = "from", requires = "to")]
    op: Option<Operation>,

    /// Source Allegation node id (single-pair mode).
    #[arg(long)]
    from: Option<String>,

    /// Target Element id, e.g. `element-1-1` (single-pair mode).
    #[arg(long)]
    to: Option<String>,

    /// Batch file: `op,from,to` per line. Mutually exclusive with `--op`.
    #[arg(long, conflicts_with = "op")]
    from_file: Option<PathBuf>,

    /// Pipeline-database URL. Required: pass `--database-url URL`, or set
    /// `PIPELINE_DATABASE_URL` (the same var the backend uses). No hardcoded
    /// connection string (Standing Rule 2).
    #[arg(long)]
    database_url: Option<String>,

    /// Print what WOULD change (per-row and summary) without writing anything.
    #[arg(long)]
    dry_run: bool,

    /// Reserved for parity with the loader CLI; report output is plain text.
    #[arg(long)]
    no_color: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();
    // best-effort: load `.env` (no-op if vars are exported; a missing file is fine).
    dotenvy::dotenv().ok();

    let args = Args::parse();
    match run(args).await {
        Ok(summary) => {
            print!("{summary}");
            if summary.has_failures() {
                // Some rows failed in a batch — surface non-zero so a script
                // notices, even though other rows succeeded.
                ExitCode::from(3)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(err) => {
            error!(error = %err, exit_code = err.code, "mapping correction failed");
            eprintln!("error: {}", err.message);
            ExitCode::from(err.code)
        }
    }
}

/// A startup/connection error carrying its own exit code and operator message.
/// Operation-level errors are folded into the [`Summary`] instead.
struct CliError {
    code: u8,
    message: String,
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Resolve inputs, open both connections, build the request list, and apply it.
async fn run(args: Args) -> Result<Summary, CliError> {
    let requests = build_requests(&args)?;

    let graph = connect_neo4j().await?;
    let pool = connect_postgres(args.database_url.clone()).await?;

    Ok(apply::apply_batch(&graph, &pool, &args.case_slug, &requests, args.dry_run).await)
}

/// Build the request list from either single-pair flags or the batch file.
/// Exactly one mode must be supplied.
fn build_requests(args: &Args) -> Result<Vec<OpRequest>, CliError> {
    match (&args.from_file, args.op) {
        (Some(path), None) => {
            let contents = std::fs::read_to_string(path).map_err(|e| CliError {
                code: 1,
                message: format!("cannot read --from-file '{}': {e}", path.display()),
            })?;
            csv::parse(&contents).map_err(|e| CliError {
                code: e.exit_code(),
                message: e.to_string(),
            })
        }
        (None, Some(op)) => {
            // clap's `requires` guarantees from/to are present when op is.
            let from = args.from.clone().unwrap_or_default();
            let to = args.to.clone().unwrap_or_default();
            Ok(vec![OpRequest { op, from, to }])
        }
        _ => Err(CliError {
            code: 1,
            message: "supply EITHER --op with --from/--to, OR --from-file (not both, not neither)"
                .to_string(),
        }),
    }
}

/// Open Neo4j from the standard env vars and ping it (fail fast on a bad
/// connection rather than mid-run).
async fn connect_neo4j() -> Result<Graph, CliError> {
    let cfg = Neo4jConfig::from_env().map_err(|e| CliError {
        code: 2,
        message: e.to_string(),
    })?;
    let graph = Graph::new(cfg.uri.clone(), cfg.user.clone(), cfg.password.clone())
        .await
        .map_err(|e| CliError {
            code: 2,
            message: format!("Neo4j connection failed: {e}"),
        })?;
    check_neo4j(&graph).await.map_err(|e| CliError {
        code: 2,
        message: format!("Neo4j ping failed: {e}"),
    })?;
    Ok(graph)
}

/// Open the pipeline Postgres pool from `--database-url` or
/// `PIPELINE_DATABASE_URL`. Small fixed pool + 5s acquire timeout, mirroring
/// `load_canonical_elements` (a CLI tool, so no per-deployment tuning).
async fn connect_postgres(database_url: Option<String>) -> Result<sqlx::PgPool, CliError> {
    let url = match database_url {
        Some(u) => u,
        None => std::env::var("PIPELINE_DATABASE_URL").map_err(|_| CliError {
            code: 5,
            message: "missing PIPELINE_DATABASE_URL (or pass --database-url)".to_string(),
        })?,
    };
    PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .map_err(|e| CliError {
            code: 5,
            message: format!("Postgres connection failed: {e}"),
        })
}

/// Initialize tracing with an env-driven filter (defaults to `info`).
fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}
