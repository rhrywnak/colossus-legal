//! Unit tests for `scan_runs.rs` — kept in a sibling file
//! (`#[cfg(test)] #[path = "..."] mod tests;`) so the parent module stays under
//! the 300-line limit (house pattern, see registry_tests.rs).

use super::*;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

/// A pool aimed at a dead port: any real query fails fast, so a test can prove a
/// code path did NOT touch the database.
fn dead_pool() -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(500))
        .connect_lazy("postgres://127.0.0.1:1/nodb")
        .expect("connect_lazy builds a pool without connecting")
}

#[tokio::test]
async fn insert_scan_run_verdicts_empty_is_ok_without_touching_the_pool() {
    // The empty-slice early return is a legitimate no-op (a subject with no
    // candidate quotes), distinct from a failure. It must return Ok WITHOUT
    // opening a transaction — the dead pool would error on any real connect.
    let result = insert_scan_run_verdicts(&dead_pool(), &[]).await;
    assert!(
        result.is_ok(),
        "empty verdicts must be a no-op Ok, got {result:?}"
    );
}

#[test]
fn bucket_column_maps_each_variant_to_its_count_column() {
    // These column names are interpolated into the progress UPDATE, so pin them:
    // a wrong mapping would advance the wrong live count. Each is a fixed literal
    // (no user input), which is why the format! is injection-safe.
    assert_eq!(bucket_column(ProgressBucket::Relevant), "relevant_count");
    assert_eq!(
        bucket_column(ProgressBucket::Irrelevant),
        "irrelevant_count"
    );
    assert_eq!(bucket_column(ProgressBucket::Failed), "failed_count");
}
