//! Canonical Element loader — reads the four `count_N_*.yaml` files and
//! idempotently syncs them to Neo4j.
//!
//! This module is consumed by the `load_canonical_elements` binary
//! (`backend/src/bin/load_canonical_elements.rs`). The binary is thin: it
//! parses CLI args, opens a Neo4j connection, calls [`loader::run`], and
//! prints the returned report. All the real work lives here.
//!
//! ## Why a module (not just the binary)?
//!
//! Keeping the schema types, Cypher builders, diff logic, and report in a
//! library module means the integration tests can drive the loader directly
//! against a test Neo4j instance without shelling out to the binary. The
//! binary becomes a trivial wiring layer.
//!
//! Sub-modules:
//! - [`schema`]   — serde types for the YAML schema (`deny_unknown_fields`)
//! - [`cypher`]   — `neo4rs::Query` builders, one per operation
//! - [`plan`]     — reads current graph state and diffs it against the YAML
//! - [`authored`] — Tier-1 authored-entity writes to Postgres (Option A)
//! - [`loader`]   — orchestration: read → validate → Postgres → plan → Neo4j → report
//! - [`report`]   — the change report struct and its `Display` impl

pub mod authored;
pub mod cypher;
pub mod diff;
pub mod loader;
pub mod plan;
pub mod report;
pub mod schema;
pub mod state;

use std::path::PathBuf;

/// Provenance marker stamped on everything the canonical loader writes, in
/// both tiers: the Neo4j `provenance` node property ([`cypher`]) and the
/// `authored_entities`/`authored_relationships.provenance` column
/// ([`authored`]). Defined once here — accessible to the child modules via
/// `super::PROVENANCE_CANONICAL` — so the two tiers cannot silently drift to
/// different markers. A fixed data-model identifier, not configuration
/// (Standing Rule 2 does not apply to schema identifiers).
const PROVENANCE_CANONICAL: &str = "canonical";

/// Minimal Neo4j connection config for the loader binary.
///
/// ## Why not reuse `AppConfig::from_env()`?
///
/// `AppConfig::from_env()` requires a dozen unrelated env vars
/// (`ANTHROPIC_MODEL`, `DATABASE_URL`, `PROMPTS_DIR`, …) that this
/// Neo4j-only tool never touches. Forcing an operator to set all of them
/// just to load Elements is a hidden coupling and a silent-failure trap.
/// We read only the three values we actually use, with the **same env var
/// names** the rest of the backend uses (`config.rs`), so a working `.env`
/// drives both. (Standing Rule 2 — configuration, not hardcoding.)
#[derive(Debug, Clone)]
pub struct Neo4jConfig {
    pub uri: String,
    pub user: String,
    pub password: String,
}

impl Neo4jConfig {
    /// Read the three required Neo4j env vars.
    ///
    /// Returns [`CanonicalLoaderError::MissingEnv`] naming the first missing
    /// key — a startup error, never a runtime surprise (Standing Rule 1).
    pub fn from_env() -> Result<Self, CanonicalLoaderError> {
        let read = |key: &str| {
            std::env::var(key).map_err(|_| CanonicalLoaderError::MissingEnv {
                key: key.to_string(),
            })
        };
        Ok(Self {
            uri: read("NEO4J_URI")?,
            user: read("NEO4J_USER")?,
            password: read("NEO4J_PASSWORD")?,
        })
    }
}

/// Errors raised by the canonical Element loader library.
///
/// ## Rust Learning: `thiserror` + `#[source]`
///
/// `#[derive(thiserror::Error)]` generates the `Display` and
/// `std::error::Error` impls from the `#[error("…")]` strings. A field
/// tagged `#[source]` is reported as the error's cause, so the binary can
/// print the full chain. We keep the operation/context in the message so a
/// reader of the logs can tell *what* failed and *why* (Standing Rule 1).
///
/// The binary maps each variant to a documented process exit code — see
/// [`CanonicalLoaderError::exit_code`].
#[derive(Debug, thiserror::Error)]
pub enum CanonicalLoaderError {
    /// A required env var was absent at startup.
    #[error("Missing env var: {key}")]
    MissingEnv { key: String },

    /// The `--yaml-dir` directory could not be listed.
    #[error("Failed to read YAML directory '{path}': {source}")]
    YamlDirRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A `count_N_*.yaml` file could not be read from disk.
    #[error("Failed to read YAML file '{path}': {source}")]
    FileRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A YAML file failed to deserialize. `serde_yaml` includes the
    /// line/column of the offending token in `source`.
    #[error("Failed to parse YAML file '{path}': {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    /// Cross-file consistency failed (duplicate ids, missing counts, …).
    #[error("Validation error: {0}")]
    Validation(String),

    /// A `LegalCount` the YAML targets does not exist in the graph.
    #[error(
        "LegalCount(count_number={count_number}) not found in graph. Ensure \
         case-structuring pipeline has run on the complaint document before \
         loading canonical Elements."
    )]
    MissingLegalCount { count_number: u32 },

    /// The Neo4j connection (or startup ping) failed.
    #[error(
        "Neo4j connection failed: {source} — verify NEO4J_URI, NEO4J_USER, and \
         NEO4J_PASSWORD are correct and that the Neo4j server is reachable"
    )]
    Connection {
        #[source]
        source: neo4rs::Error,
    },

    /// A Cypher query failed during plan-building or execution. `operation`
    /// names the step so the failure is locatable in the logs.
    #[error("Neo4j query failed during {operation}: {source}")]
    Cypher {
        operation: String,
        #[source]
        source: neo4rs::Error,
    },

    /// A returned row could not be decoded into the expected type.
    /// `neo4rs` uses a distinct error type (`DeError`) for row decoding than
    /// for query execution, so this is its own variant.
    #[error("Failed to decode Neo4j row during {operation}: {source}")]
    RowDecode {
        operation: &'static str,
        #[source]
        source: neo4rs::DeError,
    },

    /// JSON encoding of a LegalCount list property failed.
    #[error("Failed to JSON-encode {field}: {source}")]
    JsonEncode {
        field: String,
        #[source]
        source: serde_json::Error,
    },

    /// A Postgres operation against the authored-entity tables failed.
    /// `operation` names the step (connect / delete / upsert / commit) so
    /// the failure is locatable in the logs (Standing Rule 1). The source
    /// `message` carries the underlying sqlx / repository error text.
    #[error("Postgres operation '{operation}' failed: {message}")]
    Postgres { operation: String, message: String },
}

impl CanonicalLoaderError {
    /// Closure mapping a query-execution error to [`Self::Cypher`], tagged with
    /// the operation name. `op.map_err(CanonicalLoaderError::exec("step"))`.
    pub(crate) fn exec(operation: &'static str) -> impl Fn(neo4rs::Error) -> Self {
        move |source| Self::Cypher {
            operation: operation.to_string(),
            source,
        }
    }

    /// Closure mapping a row-decode error to [`Self::RowDecode`].
    pub(crate) fn decode(operation: &'static str) -> impl Fn(neo4rs::DeError) -> Self {
        move |source| Self::RowDecode { operation, source }
    }

    /// Process exit code for this error, per the instruction spec.
    ///
    /// - `1` — input/parse problems (bad dir, unreadable file, bad YAML, JSON encode)
    /// - `2` — Neo4j connection failure
    /// - `3` — Cypher execution failure
    /// - `4` — validation failure / missing prerequisite `LegalCount`
    /// - `5` — Postgres write failure (authored-entity tables)
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::MissingEnv { .. }
            | Self::YamlDirRead { .. }
            | Self::FileRead { .. }
            | Self::Parse { .. }
            | Self::JsonEncode { .. } => 1,
            Self::Connection { .. } => 2,
            Self::Cypher { .. } | Self::RowDecode { .. } => 3,
            Self::Validation(_) | Self::MissingLegalCount { .. } => 4,
            Self::Postgres { .. } => 5,
        }
    }
}
