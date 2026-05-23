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
//!
//! ## Usage
//! ```text
//! cargo run --bin load_canonical_elements -- [--yaml-dir PATH] [--dry-run] [--no-color]
//! ```
//! Connection details come from `NEO4J_URI` / `NEO4J_USER` / `NEO4J_PASSWORD`
//! (loaded from `.env` if present), exactly like the rest of the backend.

use clap::Parser;
use colossus_legal_backend::canonical_elements::report::ChangeReport;
use colossus_legal_backend::canonical_elements::{loader, CanonicalLoaderError, Neo4jConfig};
use colossus_legal_backend::neo4j::check_neo4j;
use neo4rs::Graph;
use std::path::PathBuf;
use std::process::ExitCode;
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
    /// `// Why:` the default is a relative convenience path matching the repo
    /// layout (the loader is normally run from the repo root). It is not an
    /// infrastructure address — it carries no host/port/credential — and every
    /// real invocation can override it with `--yaml-dir`, so it is a CLI
    /// default rather than a hardcoded configuration value.
    #[arg(long, default_value = "backend/canonical_elements/")]
    yaml_dir: PathBuf,

    /// Print what would change without writing anything to Neo4j.
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
    // best-effort: load `.env` so NEO4J_* are available; if they're already
    // exported in the environment this is a no-op, so a missing `.env` is fine.
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

/// Connect to Neo4j and run the loader.
async fn run(args: Args) -> Result<ChangeReport, CanonicalLoaderError> {
    let cfg = Neo4jConfig::from_env()?;
    let graph = Graph::new(cfg.uri.clone(), cfg.user.clone(), cfg.password.clone())
        .await
        .map_err(|source| CanonicalLoaderError::Connection { source })?;
    // Fail fast if the connection is up but the server is unreachable/unauthed.
    check_neo4j(&graph)
        .await
        .map_err(|source| CanonicalLoaderError::Connection { source })?;

    let opts = loader::RunOptions {
        yaml_dir: args.yaml_dir,
        dry_run: args.dry_run,
        no_color: args.no_color,
    };
    loader::run(&graph, opts).await
}

/// Initialize tracing with an env-driven filter (defaults to `info`).
fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}
