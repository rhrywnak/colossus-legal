//! backend/tests/scan_run_history_integration.rs
//!
//! Integration tests for `list_scan_runs`
//! (`repositories::pipeline_repository::scan_runs`) — the scan-run HISTORY
//! reader that the Theme Scan panel hydrates from.
//!
//! Every test is `#[ignore]` because it requires a live `colossus_legal_v2`
//! PostgreSQL database — the project has no `#[sqlx::test]` fixture infra, so CI
//! does NOT run them (same convention as `scenarios_integration.rs`). The
//! CI-runnable coverage is the SQL-shape + row→DTO mapping unit tests; THIS test
//! is the behavioural proof that the query orders newest-first and stays scoped
//! to one scenario.
//!
//! Run manually against live DEV infra:
//!   `cargo test -p colossus-legal-backend --test scan_run_history_integration -- \
//!      --ignored --test-threads=1`
//!
//! ## Case-slug safety
//!
//! These tests DELETE scenarios (which cascade to `scan_runs`) by `case_slug`.
//! They therefore must NOT use the bare production slug: each derives a per-test
//! `__test_<tag>` slug so cleanup only ever touches the test's own rows and the
//! suite is re-runnable.

use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use colossus_legal_backend::config::AppConfig;
use colossus_legal_backend::repositories::pipeline_repository::{
    delete_scenario, insert_scan_run_running, insert_scenario, list_scan_runs, ScanRunStart,
};

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Documented base slug (the real matter). Tests append `__test_<tag>` so
/// destructive cleanup never touches production rows. See the module doc.
const CASE_SLUG_BASE: &str = "awad_v_catholic_family_service";

fn test_slug(tag: &str) -> String {
    format!("{CASE_SLUG_BASE}__test_{tag}")
}

/// Connect to the live pipeline database from env (`.env` honored).
async fn pipeline_pool() -> TestResult<PgPool> {
    // best-effort: a missing .env is normal when the live URL comes from the
    // shell env in CI / live-infra runs; the connect below fails loudly if unset.
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&config.pipeline_database_url)
        .await?;
    Ok(pool)
}

/// Insert a minimal scenario and return its id (the FK owner of the scan runs).
async fn insert_test_scenario(pool: &PgPool, slug: &str, name: &str) -> TestResult<Uuid> {
    let id = insert_scenario(
        pool,
        name,
        "offense",
        "draft",
        slug,
        None,
        None,
        &json!({ "schema_v": 1 }),
    )
    .await?;
    Ok(id)
}

/// Insert one `running` scan_runs row with a chosen `started_at` (the column the
/// history orders by). Content beyond the timestamp is irrelevant to ordering.
async fn insert_run_at(
    pool: &PgPool,
    scenario_id: Uuid,
    started_at: DateTime<Utc>,
) -> TestResult<Uuid> {
    let run_id = Uuid::new_v4();
    insert_scan_run_running(
        pool,
        &ScanRunStart {
            run_id,
            scenario_id,
            model_id: "qwen-14b".to_string(),
            resolved_params: json!({ "temperature": 0.0, "timeout_secs": 90, "max_tokens": 512 }),
            dry_run: true,
            candidates_total: 10,
            started_at,
        },
    )
    .await?;
    Ok(run_id)
}

fn at(secs: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(secs, 0).single().expect("in-range ts")
}

/// The history is ordered newest-first and scoped to the requested scenario:
/// rows from a sibling scenario in the same case never leak in.
#[tokio::test]
#[ignore = "requires a live colossus_legal_v2 database"]
async fn list_scan_runs_is_newest_first_and_scenario_scoped() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("scan_history");

    // Two scenarios in the SAME case, to prove the scope excludes siblings.
    let scenario_a = insert_test_scenario(&pool, &slug, "History A").await?;
    let scenario_b = insert_test_scenario(&pool, &slug, "History B").await?;

    // Three runs for A inserted OUT of chronological order, one run for B.
    let mid = insert_run_at(&pool, scenario_a, at(1_700_000_060)).await?;
    let newest = insert_run_at(&pool, scenario_a, at(1_700_000_120)).await?;
    let oldest = insert_run_at(&pool, scenario_a, at(1_700_000_000)).await?;
    let _b_run = insert_run_at(&pool, scenario_b, at(1_700_000_999)).await?;

    let a_runs = list_scan_runs(&pool, scenario_a).await?;

    // Scoped: exactly A's three runs, none of B's (even though B's is the most
    // recent of all, it must not appear in A's history).
    assert_eq!(a_runs.len(), 3, "A's history must hold exactly its 3 runs");
    let ids: Vec<Uuid> = a_runs.iter().map(|r| r.run_id).collect();
    assert_eq!(
        ids,
        vec![newest, mid, oldest],
        "history must be ordered started_at DESC (newest first)"
    );

    // B sees only its own single run.
    let b_runs = list_scan_runs(&pool, scenario_b).await?;
    assert_eq!(b_runs.len(), 1, "B's history is independent of A's");

    // Cleanup — deleting the scenarios cascades to their scan_runs.
    delete_scenario(&pool, scenario_a, &slug).await?;
    delete_scenario(&pool, scenario_b, &slug).await?;
    Ok(())
}

/// A scenario that was never scanned returns an empty history (a legitimate
/// empty `Vec`, distinct from an error — Standing Rule 1).
#[tokio::test]
#[ignore = "requires a live colossus_legal_v2 database"]
async fn list_scan_runs_returns_empty_for_unscanned_scenario() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let slug = test_slug("scan_history_empty");
    let scenario = insert_test_scenario(&pool, &slug, "Never scanned").await?;

    let runs = list_scan_runs(&pool, scenario).await?;
    assert!(runs.is_empty(), "an unscanned scenario has no history");

    delete_scenario(&pool, scenario, &slug).await?;
    Ok(())
}
