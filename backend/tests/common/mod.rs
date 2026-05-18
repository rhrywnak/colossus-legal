//! Shared test setup helpers.
//!
//! Lives under `tests/common/mod.rs` (not `tests/common.rs`) so cargo
//! treats it as a submodule rather than its own test binary. Each
//! integration test that needs env-backed config pulls it in via
//! `mod common;`.

use std::sync::Once;

static INIT: Once = Once::new();

/// Populate every env var that `AppConfig::from_env()` treats as required
/// with a placeholder if the developer hasn't already set it.
///
/// Order: load `.env` first (so a developer's real values win), then
/// backfill anything still missing. Idempotent — every test's setup
/// function can call this freely.
///
/// Tests build PostgreSQL pools with `connect_lazy`, so the placeholder
/// URLs are never actually dialed. Neo4j is the exception: setup paths
/// in `setup_app()` do `create_neo4j_graph(&config).await`, which means
/// `NEO4J_URI` etc. must point at a reachable instance — that comes from
/// the developer's `.env`, which dotenvy loads here.
pub fn init_test_env() {
    INIT.call_once(|| {
        dotenvy::dotenv().ok();

        // SAFETY: `std::env::set_var` became `unsafe` in Rust 2024 because
        // concurrent env mutation across threads is UB. We're inside
        // `Once::call_once`, which serializes our writes, and no test
        // threads have spawned yet at first-call time.
        unsafe {
            for (key, value) in REQUIRED_TEST_ENV {
                if std::env::var(key).is_err() {
                    std::env::set_var(key, value);
                }
            }
        }
    });
}

/// Required-by-`AppConfig::from_env()` env vars and their test placeholders.
/// Real values from `.env` always win — these only fill gaps.
const REQUIRED_TEST_ENV: &[(&str, &str)] = &[
    ("NEO4J_URI", "bolt://localhost:7687"),
    ("NEO4J_USER", "neo4j"),
    ("NEO4J_PASSWORD", "test-password"),
    ("ANTHROPIC_MODEL", "claude-sonnet-4-6"),
    ("DECOMPOSER_MODEL", "claude-sonnet-4-6"),
    ("DATABASE_URL", "postgres://test:test@localhost/test"),
    (
        "PIPELINE_DATABASE_URL",
        "postgres://test:test@localhost/test_pipeline",
    ),
    ("PROMPTS_DIR", "/tmp/colossus-legal-test-prompts"),
];
