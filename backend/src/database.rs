//! Database pool initialization and migration runner.
//!
//! ## Rust Learning: Extracting infrastructure concerns
//!
//! This module centralizes all PostgreSQL setup — connection pools and migrations —
//! so that `main.rs` stays focused on wiring the application together. Each database
//! gets its own pool and migration strategy:
//!
//! - **Main pool** (`colossus_legal`): uses `sqlx::migrate!()` which embeds `.sql`
//!   files at compile time from `./migrations/`.
//! - **Pipeline pool** (`colossus_legal_v2`): uses `sqlx::migrate::Migrator` which
//!   loads `.sql` files at runtime from `./pipeline_migrations/`. This is necessary
//!   because `sqlx::migrate!()` can only target one database per invocation.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

use crate::config::AppConfig;

/// Both PostgreSQL connection pools, ready for use in AppState.
pub struct DatabasePools {
    /// Pool for the main `colossus_legal` database (ratings, feedback, QA).
    pub main_pool: PgPool,
    /// Pool for the pipeline `colossus_legal_v2` database (extraction, review).
    pub pipeline_pool: PgPool,
}

/// Connect to both PostgreSQL databases and run their migrations.
///
/// ## Rust Learning: `sqlx::migrate!()` vs `sqlx::migrate::Migrator`
///
/// `sqlx::migrate!("./migrations")` is a compile-time macro — it reads the SQL files
/// during `cargo build` and embeds them in the binary. Fast and safe, but it can only
/// target one directory (and implicitly one database).
///
/// `sqlx::migrate::Migrator::new(path)` loads migration files at runtime. We use this
/// for the pipeline database so both databases get their own migration directories
/// without conflicting.
pub async fn init_pools(config: &AppConfig) -> DatabasePools {
    // --- Main database pool (colossus_legal) ---
    let main_pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.postgres_url)
        .await
        .expect("Failed to connect to PostgreSQL (main)");

    rollback_tolerant(sqlx::migrate!("./migrations"))
        .run(&main_pool)
        .await
        .expect("Failed to run main database migrations");

    tracing::info!("PostgreSQL (main) connected and migrations complete");

    // --- Pipeline database pool (colossus_legal_v2) ---
    let pipeline_pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.pipeline_database_url)
        .await
        .expect("Failed to connect to pipeline PostgreSQL database");

    let pipeline_migrator = rollback_tolerant(
        sqlx::migrate::Migrator::new(std::path::Path::new("./pipeline_migrations"))
            .await
            .expect("Failed to load pipeline migrations"),
    );

    pipeline_migrator
        .run(&pipeline_pool)
        .await
        .expect("Failed to run pipeline migrations");

    // The count is the migrator's own view of the directory, so a boot log can be
    // read against `ls pipeline_migrations | wc -l` without a database query. It
    // is how many migrations this BINARY carries — not how many were applied on
    // this boot, which is nearly always zero and tells an operator nothing.
    tracing::info!(
        migrations = pipeline_migrator.iter().count(),
        "PostgreSQL (pipeline / colossus_legal_v2) connected and migrations complete"
    );

    DatabasePools {
        main_pool,
        pipeline_pool,
    }
}

/// Let a binary boot against a store that has migrations it does not carry.
///
/// ## Why (CC_TASK_REVIEW_LOOP_v1 §6)
///
/// By default sqlx refuses to run when the database records an APPLIED migration
/// that is missing from this binary's set — `VersionMissing`, and the `.expect`
/// above turns that into a boot panic. That default is what made a v2.1.9 → v2.1.8
/// rollback impossible: the older image does not carry the newer migration, so it
/// never starts. With `ignore_missing` set, an older image skips the check and
/// runs its own (already applied) set — a version rollback becomes a redeploy
/// rather than a database restore. It only helps images that contain THIS line;
/// every image before it still panics against a newer store.
///
/// What it does NOT relax: a migration whose file CHANGED after it was applied
/// still fails its checksum, and a migration this binary carries but the store
/// lacks is still applied. Only "the store is ahead of me" is tolerated.
///
/// ## Rust Learning: taking `mut self` by value to call a `&mut self` setter
///
/// `Migrator::set_ignore_missing` takes `&mut self`, and `sqlx::migrate!(…)`
/// produces a temporary value with no binding to borrow mutably. Taking the
/// migrator by value as `mut migrator` gives it a mutable home for the duration
/// of the call, and returning it moves it back out — so both call sites stay one
/// expression instead of each growing a `let mut` and a separate statement.
fn rollback_tolerant(mut migrator: sqlx::migrate::Migrator) -> sqlx::migrate::Migrator {
    migrator.set_ignore_missing(true);
    migrator
}
