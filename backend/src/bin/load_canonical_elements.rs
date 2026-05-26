//! `load_canonical_elements` — sync canonical Element YAML into Neo4j.
//!
//! This binary is intentionally thin: parse CLI args, open a Neo4j connection
//! from the standard env vars, hand off to
//! [`canonical_elements::loader::run`], print the report, and translate any
//! error into a documented exit code. All real logic lives in the
//! `canonical_elements` library module so the integration tests can drive it
//! directly.
//!
//! ## Exit codes
//! - `0` success
//! - `1` input/parse problem (bad dir, unreadable file, invalid YAML)
//! - `2` Neo4j connection failure
//! - `3` Cypher execution / row-decode failure
//! - `4` validation failure or missing prerequisite `LegalCount`
//! - `5` Postgres write failure (authored-entity tables)
//!
//! ## Usage
//! ```text
//! cargo run --bin load_canonical_elements -- \
//!   --case-slug SLUG [--yaml-dir PATH] [--database-url URL] [--dry-run] [--no-color]
//! ```
//! Neo4j connection details come from `NEO4J_URI` / `NEO4J_USER` /
//! `NEO4J_PASSWORD`; the Postgres URL from `--database-url` or
//! `PIPELINE_DATABASE_URL`; the YAML dir from `--yaml-dir` or
//! `CANONICAL_ELEMENTS_YAML_DIR` (all loadable from `.env`), exactly like the
//! rest of the backend.

use clap::Parser;
use colossus_legal_backend::canonical_elements::report::ChangeReport;
use colossus_legal_backend::canonical_elements::{loader, CanonicalLoaderError, Neo4jConfig};
use colossus_legal_backend::neo4j::check_neo4j;
use neo4rs::Graph;
use sqlx::postgres::PgPoolOptions;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;
use tracing::error;

/// CLI arguments. Field doc comments double as `--help` text.
#[derive(Parser, Debug)]
#[command(
    name = "load_canonical_elements",
    about = "Idempotently sync canonical Element YAML files into Neo4j."
)]
struct Args {
    /// Directory containing the `count_N_*.yaml` files.
    ///
    /// Required: pass `--yaml-dir PATH`, or omit it and set
    /// `CANONICAL_ELEMENTS_YAML_DIR` (resolved in `run`). No compiled default
    /// — the path is deployment/workstation-specific, so it comes from the
    /// operator, never from code (Standing Rule 2).
    #[arg(long)]
    yaml_dir: Option<PathBuf>,

    /// Pipeline-database connection URL for the Tier-1 authored-entity
    /// writes. Required: pass `--database-url URL`, or omit it and set
    /// `PIPELINE_DATABASE_URL` (the same var the backend uses). No hardcoded
    /// connection string (Standing Rule 2).
    #[arg(long)]
    database_url: Option<String>,

    /// Case slug written to `authored_entities` / `authored_relationships`
    /// (e.g. `awad_v_catholic_family_service`). Required — case-specific data
    /// comes from the operator, never from code (Standing Rule 2).
    #[arg(long)]
    case_slug: String,

    /// Print what would change without writing anything to Postgres or Neo4j.
    #[arg(long)]
    dry_run: bool,

    /// Disable ANSI color codes in the report output.
    #[arg(long)]
    no_color: bool,
}

/// ## Rust Learning: returning `ExitCode` from `main`
///
/// Returning [`std::process::ExitCode`] (rather than calling
/// `std::process::exit`) lets `main` set a custom exit status while still
/// running normal destructors. We map each error variant to a documented code
/// via [`CanonicalLoaderError::exit_code`].
#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();
    // best-effort: load `.env` (no-op if the vars are already exported; a missing `.env` is fine).
    dotenvy::dotenv().ok();

    let args = Args::parse();
    match run(args).await {
        Ok(report) => {
            // The report is the primary artifact — print to stdout.
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            // Log with full context for operators, and echo a concise line to
            // stderr for interactive use.
            error!(error = %err, "canonical Element load failed");
            eprintln!("error: {err}");
            ExitCode::from(err.exit_code())
        }
    }
}

/// Connect to Neo4j and Postgres, then run the loader.
async fn run(args: Args) -> Result<ChangeReport, CanonicalLoaderError> {
    let cfg = Neo4jConfig::from_env()?;
    let graph = Graph::new(cfg.uri.clone(), cfg.user.clone(), cfg.password.clone())
        .await
        .map_err(|source| CanonicalLoaderError::Connection { source })?;
    // Fail fast if the connection is up but the server is unreachable/unauthed.
    check_neo4j(&graph)
        .await
        .map_err(|source| CanonicalLoaderError::Connection { source })?;

    // Resolve the two flag-or-env inputs (no compiled defaults; case-specific
    // / deployment-specific values come from the operator — Standing Rule 2).
    let yaml_dir = match args.yaml_dir {
        Some(p) => p,
        None => PathBuf::from(env_or_err("CANONICAL_ELEMENTS_YAML_DIR", "--yaml-dir")?),
    };
    let database_url = match args.database_url {
        Some(u) => u,
        None => env_or_err("PIPELINE_DATABASE_URL", "--database-url")?,
    };

    // Pipeline-DB pool for the Tier-1 authored-entity writes. CLI tool, so a
    // small fixed pool is ample.
    let pipeline_pool = PgPoolOptions::new()
        .max_connections(2)
        // DEFAULT: 5s acquire timeout, mirroring backend/src/database.rs. A
        // compiled default for this CLI tool (no per-deployment tuning), so a
        // misconfigured URL fails fast rather than hanging.
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await
        .map_err(|e| CanonicalLoaderError::Postgres {
            operation: "connect to pipeline database".to_string(),
            message: e.to_string(),
        })?;

    let opts = loader::RunOptions {
        yaml_dir,
        dry_run: args.dry_run,
        no_color: args.no_color,
        pipeline_pool: Some(pipeline_pool),
        case_slug: Some(args.case_slug),
    };
    loader::run(&graph, opts).await
}

/// Resolve a required input from its env-var fallback when the CLI flag was
/// omitted. A missing value is a hard "required input not provided" error.
fn env_or_err(env_key: &str, flag_name: &str) -> Result<String, CanonicalLoaderError> {
    // best-effort: the env var is the fallback when the flag is absent; a
    // missing value here means the required input was not provided at all,
    // which the `map_err` turns into a startup error naming both sources.
    std::env::var(env_key).map_err(|_| CanonicalLoaderError::MissingEnv {
        key: format!("{env_key} (or pass {flag_name})"),
    })
}

/// Initialize tracing with an env-driven filter (defaults to `info`).
fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}
