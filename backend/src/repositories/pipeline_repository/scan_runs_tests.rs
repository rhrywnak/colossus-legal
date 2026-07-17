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

/// SQL-shape guard for the history list. There is no `#[sqlx::test]`/live-DB
/// harness in this repo, so the behavioural ordering/scoping is asserted by the
/// live-DB integration test in `backend/tests/scan_run_history_integration.rs`
/// (not run in CI). This unit test pins the two properties the panel depends on
/// so a future edit that drops the scenario scope or reverses the order fails
/// here and names the regression.
#[test]
fn list_scan_runs_sql_scopes_by_scenario_and_orders_newest_first() {
    let sql = LIST_SCAN_RUNS_SQL;
    assert!(
        sql.contains("WHERE scenario_id = $1"),
        "history must be scoped to the scenario, got: {sql}"
    );
    assert!(
        sql.contains("ORDER BY started_at DESC"),
        "history must be newest-first, got: {sql}"
    );
    // The cost cast is load-bearing: a bare NUMERIC would fail to decode to f64.
    assert!(
        sql.contains("computed_cost::float8"),
        "computed_cost must be cast to float8 to decode, got: {sql}"
    );
    // Headers only — the heavy summary/verdict payload must NOT ride in the list.
    assert!(
        !sql.contains("summary_json"),
        "the history list must stay light (no summary_json), got: {sql}"
    );
}

/// SQL-shape guard for the delete. The `scenario_id` predicate is the case-fence:
/// dropping it would let any valid `run_id` delete a run in another scenario. Pin
/// both predicates so a future edit that widens the scope fails here and names it.
#[test]
fn delete_scan_run_sql_is_fenced_by_run_and_scenario() {
    let sql = DELETE_SCAN_RUN_SQL;
    assert!(
        sql.contains("DELETE FROM scan_runs"),
        "must delete from scan_runs, got: {sql}"
    );
    assert!(
        sql.contains("run_id = $1"),
        "must target the run by id ($1), got: {sql}"
    );
    assert!(
        sql.contains("scenario_id = $2"),
        "must fence the delete by scenario ($2), got: {sql}"
    );
}
