// =============================================================================
// scan_run_delta_tests.rs — the "+N since the previous scan" rules (R2)
// =============================================================================

use super::*;
use chrono::{TimeZone, Utc};
use uuid::Uuid;

/// A history row carrying only what the delta cares about.
///
/// `started_at` is bound from a fixed instant rather than `Utc::now()` so the
/// fixture is deterministic; the delta never reads it (ordering comes from the
/// slice position, which is what `list_scan_runs`'s `ORDER BY started_at DESC`
/// already established).
fn run(candidates_read: i32, status: &str, dry_run: bool) -> ScanRunHeader {
    ScanRunHeader {
        run_id: Uuid::nil(),
        model_id: "claude-opus-4-8".to_string(),
        status: status.to_string(),
        candidates_total: Some(candidates_read),
        candidates_judged: candidates_read,
        relevant_count: 0,
        irrelevant_count: 0,
        failed_count: 0,
        computed_cost: None,
        duration_ms: 1000,
        started_at: Utc.timestamp_opt(1_780_000_000, 0).single().expect("fixed"),
        candidates_read,
        error: None,
        dry_run,
        pool_delta: None,
    }
}

#[test]
fn the_delta_is_the_difference_against_the_previous_measurable_run() {
    // The live DEV history, newest first: 148 then 94 — measured 2026-08-03.
    let runs = with_pool_deltas(vec![
        run(148, "completed", false),
        run(94, "completed", true),
    ]);
    assert_eq!(runs[0].pool_delta, Some(54), "148 - 94 = +54");
}

#[test]
fn the_first_measurable_run_has_no_delta_rather_than_zero() {
    // R2, explicitly: never 0. "The pool did not change" and "there is nothing to
    // compare against" are different facts, and only one of them is reassuring.
    let runs = with_pool_deltas(vec![run(94, "completed", false)]);
    assert_eq!(runs[0].pool_delta, None);
}

#[test]
fn an_unchanged_pool_reads_as_zero_not_as_absent() {
    // The other side of the same coin: a real comparison that came out even IS a
    // zero, and must not be collapsed into "no delta".
    let runs = with_pool_deltas(vec![
        run(94, "completed", false),
        run(94, "completed", false),
    ]);
    assert_eq!(runs[0].pool_delta, Some(0));
}

#[test]
fn a_run_that_never_read_the_pool_is_skipped_as_a_baseline() {
    // `candidates_read` is NOT NULL, so a run that failed before the read stores
    // 0 — and 0 there means "read nothing". Using it as a baseline would report a
    // fictitious +148.
    let runs = with_pool_deltas(vec![
        run(148, "completed", false),
        run(0, "failed", false),
        run(94, "completed", false),
    ]);
    assert_eq!(
        runs[0].pool_delta,
        Some(54),
        "the baseline must skip the unmeasurable run and use 94, not 0"
    );
}

#[test]
fn a_run_that_never_read_the_pool_gets_no_delta_of_its_own() {
    let runs = with_pool_deltas(vec![run(0, "failed", false), run(94, "completed", false)]);
    assert_eq!(runs[0].pool_delta, None);
}

#[test]
fn a_dry_run_is_a_valid_baseline() {
    // R2, explicitly: dry runs are NOT skipped — a dry run's pool read is a real
    // measurement of the pool, whatever it did (or did not) merge afterwards.
    // Measured on DEV, 3 of the 4 stored runs are dry runs, so skipping them would
    // leave the one real run with no baseline at all.
    let runs = with_pool_deltas(vec![
        run(148, "completed", false),
        run(94, "completed", true), // dry
    ]);
    assert_eq!(runs[0].pool_delta, Some(54));
}

#[test]
fn the_delta_is_signed_so_a_shrinking_pool_is_visible() {
    // Not clamped. After task 2.5's re-anchoring the pool can legitimately shrink,
    // and a -12 that says so is worth more than a 0 that hides it.
    let runs = with_pool_deltas(vec![
        run(136, "completed", false),
        run(148, "completed", false),
    ]);
    assert_eq!(runs[0].pool_delta, Some(-12));
}

#[test]
fn an_empty_history_produces_an_empty_result() {
    // A scenario that has never been scanned. Not an error (Standing Rule 1).
    assert!(with_pool_deltas(Vec::new()).is_empty());
}

#[test]
fn every_run_gets_exactly_one_delta_slot() {
    // The output is parallel to the input — a length mismatch would silently
    // misattribute a delta to the wrong row in the history table.
    let runs = with_pool_deltas(vec![
        run(148, "completed", false),
        run(0, "failed", false),
        run(94, "completed", true),
        run(94, "completed", true),
    ]);
    assert_eq!(runs.len(), 4);
    assert_eq!(runs[0].pool_delta, Some(54));
    assert_eq!(runs[1].pool_delta, None, "unmeasurable");
    assert_eq!(runs[2].pool_delta, Some(0), "94 against 94");
    assert_eq!(runs[3].pool_delta, None, "oldest measurable run");
}
