//! backend/tests/app_settings_decode_integration.rs
//!
//! The live round-trip the configuration store never had: migrate a scratch
//! database from ZERO, then read it back through the REAL production read path
//! and assert every seeded row decodes.
//!
//! ## Why this test exists — the defect it is the answer to
//!
//! v2.0.0-beta.364 applied all 51 migrations, seeded `app_settings`, and then
//! refused to start on the first live read:
//!
//! ```text
//! error occurred while decoding column "min_value": mismatched types;
//! Rust type `core::option::Option<f64>` (as SQL type `FLOAT8`) is not
//! compatible with SQL type `NUMERIC`
//! ```
//!
//! Nothing was blind to that seam by accident — nothing LOOKED at it. The seed
//! test parses the migration as TEXT (values, not types); the SQL-shape tests
//! read the `const` statements without a server; the 1240 unit tests build
//! `AppSettingRecord` from hand-written fixtures. Every one of them was green
//! while the migration and the struct disagreed about what `min_value` IS.
//!
//! A type seam is only observable where the two sides MEET: a real Postgres
//! applying the real migrations, decoded by the real struct. That is this file.
//!
//! ## Why a scratch database, created and dropped per run
//!
//! "From zero" has to be literal. Running against DEV's `colossus_legal_v2`
//! would prove the migrations work on a database that already has them —
//! precisely the case that CANNOT fail — and it would say nothing about a fresh
//! PROD deploy, which is where a broken migration actually bites. The scratch
//! database is created, migrated from the empty state, read, and dropped.
//!
//! ## Why the store's OWN two migrations, and not all 52
//!
//! This test was written to run the whole `pipeline_migrations` directory, and
//! doing so exposed a SEPARATE, pre-existing defect: **the pipeline migration
//! chain cannot be applied from zero at all.** `sqlx` orders migrations by the
//! numeric filename prefix parsed as `i64`, and 16 of the 52 files carry the old
//! 8-digit `YYYYMMDD` prefix. `20260513` is twenty million; `20260422214842` is
//! twenty TRILLION — so every 8-digit migration sorts before every 14-digit one,
//! regardless of when it was written. On a fresh database
//! `20260513_consolidate_model_columns_and_add_overrides` therefore runs before
//! the April migration that adds the column it edits, and fails with
//! `column "pass2_extraction_model" does not exist`.
//!
//! DEV and PROD never saw it because they were migrated INCREMENTALLY: each
//! boot's pending set held one new migration whose dependencies were already
//! applied. The chain's CONTENT is sound — applying all 52 files in filename
//! order to an empty database succeeds and produces the full 29-table schema.
//! Only the ORDER sqlx derives from the filenames is wrong.
//!
//! That defect is not this hotfix's to fix (renaming an applied migration is a
//! checksum-breaking change to DEV and PROD, and it is a project, not a line).
//! It is filed separately. What this test does instead is apply the
//! configuration store's own two migrations — the CREATE and this hotfix's
//! ALTER — with the REAL `Migrator` over real files copied into a scratch
//! directory. Both carry 14-digit prefixes, so their relative order is the same
//! one the boot path uses, and neither depends on any other table.
//!
//! That is the whole seam this test exists to cover: migration text → real
//! Postgres → real struct. The one thing it deliberately no longer covers is
//! whole-chain ordering, which is now a known defect with its own tracker entry
//! rather than something this test could quietly have been believed to check.
//!
//! ## Why `#[ignore]`
//!
//! It needs a live PostgreSQL server with CREATEDB rights; the project has no
//! `#[sqlx::test]` fixture infrastructure, so CI cannot run it (the same
//! convention as `scan_run_history_integration.rs` and `scenarios_integration.rs`).
//! The CI-runnable half of this defence lives in the lib target:
//! `the_migration_declares_only_types_this_build_can_decode` in
//! `src/repositories/pipeline_repository/app_settings_tests.rs` parses the
//! declared SQL types with no database at all. THIS test is the behavioural
//! proof that the two sides actually meet.
//!
//! Run manually against live infra (the DEV server; a scratch database, never
//! `colossus_legal_v2` itself):
//!   `cargo test -p colossus-legal-backend --test app_settings_decode_integration -- \
//!      --ignored --test-threads=1 --nocapture`

use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool};

use colossus_legal_backend::config::AppConfig;
use colossus_legal_backend::repositories::pipeline_repository::{
    insert_setting_change, list_setting_changes, list_settings,
};
use colossus_legal_backend::services::settings_store::{load_settings, REQUIRED_KEYS};

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// How many parameters the 1.6 migration seeds. Pinned here so a seed that
/// silently shrank would fail this test rather than pass it vacuously.
const SEEDED_PARAMETERS: usize = 7;

/// Split a Postgres URL into (everything before the database name, db name).
///
/// ## Why string surgery and not a URL crate
///
/// The connection string is already a `String` in `AppConfig`, and the only edit
/// needed is the last path segment. Pulling in a URL parser to swap one segment
/// would be more machinery than the job, and this returns `None` rather than
/// guessing when the shape is not what it expects — so a malformed URL fails
/// with a message naming the problem instead of silently targeting the wrong
/// database, which on this path would mean DROPping something real.
fn split_database_url(url: &str) -> Option<(&str, &str)> {
    let cut = url.rfind('/')?;
    let name = &url[cut + 1..];
    // A trailing slash, a query string, or an empty name all mean "this is not
    // the shape I know how to edit".
    if name.is_empty() || name.contains('?') {
        return None;
    }
    Some((&url[..cut], name))
}

/// A per-run scratch database name.
///
/// The nanosecond stamp keeps two runs (or a run against a server someone else
/// is testing on) from colliding, and the `__test_` marker makes it obvious in
/// `\l` that the database is disposable — the same discipline the case-slug
/// suffix provides in `scan_run_history_integration.rs`.
fn scratch_database_name() -> TestResult<String> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(format!("colossus_legal_v2__test_decode_{nanos}"))
}

/// Connect to the server's `postgres` maintenance database.
///
/// CREATE DATABASE cannot run from inside the database being created, so the
/// create and the drop both need a connection that is not to the scratch.
async fn admin_pool(base: &str) -> TestResult<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&format!("{base}/postgres"))
        .await?;
    Ok(pool)
}

/// The store's own migrations, in the order their prefixes give them.
///
/// Named rather than globbed so that a THIRD migration touching these tables
/// cannot join the set silently: whoever writes it adds the filename here, and
/// in doing so re-runs this test against their change. A glob would have swept
/// it in and reported nothing.
const STORE_MIGRATIONS: &[&str] = &[
    "20260801225147_create_app_settings_store.sql",
    "20260802092813_app_settings_bounds_to_double_precision.sql",
];

/// Copy the store's migrations into a scratch directory the `Migrator` can read.
///
/// ## Why a directory of copies and not the real one
///
/// `Migrator::new` takes a DIRECTORY and applies everything in it; there is no
/// "apply this subset" API. Copying the two files into their own directory is
/// what lets the real `Migrator` — with its checksums, its `_sqlx_migrations`
/// bookkeeping and its per-migration transaction — run over exactly the two
/// files under test. The files are read from the repository at run time, so a
/// change to either is picked up; nothing here is a transcription of the SQL.
///
/// `CARGO_TARGET_TMPDIR` is cargo's own per-test-binary scratch space under
/// `target/`, so the copies never land in the source tree.
fn stage_store_migrations() -> TestResult<std::path::PathBuf> {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("pipeline_migrations");
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let staged = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("settings_migrations_{nanos}"));
    std::fs::create_dir_all(&staged)?;

    for name in STORE_MIGRATIONS {
        let from = source.join(name);
        let sql = std::fs::read_to_string(&from)
            .map_err(|e| format!("{} is not readable: {e}", from.display()))?;
        std::fs::write(staged.join(name), sql)?;
    }
    Ok(staged)
}

/// Apply the configuration store's migrations to a freshly created database.
///
/// Uses the SAME runtime `Migrator` type that `database.rs:64-72` uses at boot,
/// over the real migration files — not a copy of the SQL and not a
/// reimplementation. A test that applied migrations its own way could pass while
/// the boot path failed, which is the exact failure mode this file exists to
/// close. See the module header for why the set is the store's own two files.
async fn migrate_from_zero(url: &str) -> TestResult<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(url)
        .await?;

    let staged = stage_store_migrations()?;
    sqlx::migrate::Migrator::new(staged.as_path())
        .await?
        .run(&pool)
        .await?;

    // Removed on success only: a failed run leaves the staged copies on disk,
    // where they are the first thing to look at when diagnosing why.
    std::fs::remove_dir_all(&staged)?;
    Ok(pool)
}

/// The whole round trip: create → migrate → read → drop.
///
/// ## Why the drop is not in a `Drop` impl
///
/// Dropping a database is async, and `Drop` is not — the usual workaround is to
/// block on a runtime inside `drop`, which deadlocks under `#[tokio::test]`'s
/// current-thread scheduler. Instead the assertions are collected into a
/// `Result` FIRST, the database is dropped unconditionally, and only then is the
/// result unwrapped: a failed assertion still cleans up after itself, and the
/// failure message survives.
#[tokio::test]
#[ignore = "requires a live PostgreSQL server with CREATEDB rights"]
async fn every_seeded_parameter_decodes_through_the_real_read_path() -> TestResult<()> {
    // best-effort: a missing .env is normal when the URL comes from the shell
    // environment; the connect below fails loudly if it is unset either way.
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;

    let (base, _) = split_database_url(&config.pipeline_database_url)
        .ok_or("PIPELINE_DATABASE_URL does not end in /<database>; refusing to guess")?;
    let scratch = scratch_database_name()?;
    let scratch_url = format!("{base}/{scratch}");

    let admin = admin_pool(base).await?;
    // Identifiers cannot be bound as parameters, and the name is built above
    // from a constant prefix plus a numeric stamp — no external input reaches
    // this string.
    admin
        .execute(format!(r#"CREATE DATABASE "{scratch}""#).as_str())
        .await?;
    println!("scratch database created: {scratch}");

    let outcome = run_assertions(&scratch_url).await;

    // Unconditional cleanup, before the `?` on the outcome. WITH (FORCE)
    // terminates any connection the pool has not finished closing, so a failed
    // assertion cannot leave a database behind on the server.
    admin
        .execute(format!(r#"DROP DATABASE IF EXISTS "{scratch}" WITH (FORCE)"#).as_str())
        .await?;
    println!("scratch database dropped: {scratch}");
    admin.close().await;

    outcome
}

/// Every assertion, so the caller can clean up whether these pass or fail.
async fn run_assertions(scratch_url: &str) -> TestResult<()> {
    let pool = migrate_from_zero(scratch_url).await?;
    println!("migrations applied from zero");

    assert_store_decodes(&pool).await?;
    assert_ledger_decodes(&pool).await?;

    // Closed before the caller drops the database: an open pool would make
    // DROP DATABASE wait on it.
    pool.close().await;
    Ok(())
}

/// The seeded parameters read back through the production path.
async fn assert_store_decodes(pool: &PgPool) -> TestResult<()> {
    // 1. THE DEFECT'S OWN TEST. `load_settings` is the production read path —
    //    the function `load_at_boot_or_exit` calls at `main.rs:189`. On
    //    beta.364 this returned the NUMERIC decode error and the process exited.
    let settings = load_settings(pool).await?;
    println!("configuration store loaded: {settings:?}");

    // 2. Every ROW decodes, not just the seven the snapshot needs. `load_settings`
    //    only touches keys in REQUIRED_KEYS; a future dormant row seeded ahead of
    //    its code would be decoded by the Settings page and by nothing else, so
    //    the page's own read path is exercised here too.
    let rows = list_settings(pool).await?;
    assert_eq!(
        rows.len(),
        SEEDED_PARAMETERS,
        "the migration seeds {SEEDED_PARAMETERS} parameters; the store returned {}",
        rows.len()
    );

    // 3. The bounds specifically — the columns that failed. Asserting they are
    //    READ correctly, not merely that decoding did not error: a cast that
    //    silently produced NULL would satisfy `Ok` and break every bounds check.
    let high = rows
        .iter()
        .find(|r| r.key == "confidence_band_high")
        .ok_or("confidence_band_high is not seeded")?;
    assert_eq!(
        (high.min_value, high.max_value),
        (Some(0.0), Some(1.0)),
        "the band cutoff's declared bounds must survive the round trip"
    );

    let cap = rows
        .iter()
        .find(|r| r.key == "talking_points_cap")
        .ok_or("talking_points_cap is not seeded")?;
    assert_eq!(
        (cap.min_value, cap.max_value),
        (Some(1.0), None),
        "an unbounded side must arrive as None, not as a zero"
    );

    // 4. Anti-vacuity: the snapshot the store built carries the seeded numbers.
    //    Without this, a store that decoded into all-default values would pass.
    assert_eq!(settings.confidence_band_high, 0.80);
    assert_eq!(settings.talking_points_cap, 3);
    assert_eq!(REQUIRED_KEYS.len(), SEEDED_PARAMETERS);
    Ok(())
}

/// The change ledger's own read path.
///
/// DEV's `app_setting_changes` is empty, so no live read has ever decoded it
/// either — and an empty result proves nothing about a type seam. One row is
/// inserted here so the decode actually runs.
async fn assert_ledger_decodes(pool: &PgPool) -> TestResult<()> {
    let now = Utc::now();
    insert_setting_change(pool, "talking_points_cap", "3", "4", "test", now).await?;

    let changes = list_setting_changes(pool, "talking_points_cap").await?;
    assert_eq!(changes.len(), 1, "the ledger row must read back");
    assert_eq!(changes[0].old_value, "3");
    assert_eq!(changes[0].new_value, "4");
    assert_eq!(changes[0].actor, "test");
    println!("ledger row decoded: {:?}", changes[0]);
    Ok(())
}
