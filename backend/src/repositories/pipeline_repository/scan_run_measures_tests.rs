//! Unit tests for `scan_run_measures.rs` — kept in a sibling file (house
//! pattern) so the parent module stays well under the 300-line limit.
//!
//! SQL-shape guards. There is no live-DB harness in this crate, so what can be
//! asserted here is that the query text keeps the four properties the estimate
//! depends on. Each one, if dropped, produces a NUMBER rather than an error —
//! which is the reason they are pinned: a silently wrong estimate is read by a
//! human as a promise about the next hour of their afternoon.

use super::*;

#[test]
fn the_rate_query_counts_only_completed_runs_that_judged_something() {
    let sql = SECONDS_PER_CANDIDATE_SQL;
    assert!(
        sql.contains("WHERE status = $1"),
        "the status must be bound, not interpolated, got: {sql}"
    );
    assert!(
        sql.contains("candidates_judged > 0"),
        "a run that judged nothing measured nothing, and dividing by its zero \
         would panic the query rather than omit the row, got: {sql}"
    );
}

#[test]
fn the_rate_query_casts_to_float8_because_numeric_does_not_decode_here() {
    let sql = SECONDS_PER_CANDIDATE_SQL;
    // `sum()` over BIGINT/INTEGER returns NUMERIC, which has no `f64` decoder in
    // this tree. Without both casts the read fails at runtime with a decode
    // error, on a page that would otherwise have worked.
    assert_eq!(
        sql.matches("::float8").count(),
        2,
        "both sums must be cast before the division, got: {sql}"
    );
}

#[test]
fn the_rate_query_divides_the_sums_rather_than_averaging_the_rates() {
    let sql = SECONDS_PER_CANDIDATE_SQL;
    assert!(
        sql.contains("sum(duration_ms)") && sql.contains("sum(candidates_judged)"),
        "the estimate is work-weighted: a nine-card smoke test must not have the \
         same vote as a 313-card scan, got: {sql}"
    );
    assert!(
        !sql.contains("avg("),
        "averaging per-run rates is the estimator this query rejects, got: {sql}"
    );
}

#[test]
fn the_rate_is_reported_in_seconds_not_milliseconds() {
    // The column is milliseconds and the caller multiplies by a candidate count
    // to get minutes. A missing /1000 would report a 10-second card as nearly
    // three hours per card — large enough to notice, and small enough to ship.
    assert!(
        SECONDS_PER_CANDIDATE_SQL.contains("/ 1000.0"),
        "duration_ms must be divided into seconds, got: {SECONDS_PER_CANDIDATE_SQL}"
    );
}

#[test]
fn the_rate_query_groups_by_model_so_one_slow_model_does_not_price_another() {
    assert!(
        SECONDS_PER_CANDIDATE_SQL.contains("GROUP BY model_id"),
        "the estimate is per MODEL — a 27B local model and a billed API model \
         share no rate, got: {SECONDS_PER_CANDIDATE_SQL}"
    );
}
