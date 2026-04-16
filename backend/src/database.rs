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

    sqlx::migrate!("./migrations")
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

    let pipeline_migrator =
        sqlx::migrate::Migrator::new(std::path::Path::new("./pipeline_migrations"))
            .await
            .expect("Failed to load pipeline migrations");

    pipeline_migrator
        .run(&pipeline_pool)
        .await
        .expect("Failed to run pipeline migrations");

    tracing::info!("PostgreSQL (pipeline / colossus_legal_v2) connected and migrations complete");

    DatabasePools {
        main_pool,
        pipeline_pool,
    }
}
