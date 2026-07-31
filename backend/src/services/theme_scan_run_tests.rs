//! Unit tests for [`crate::services::theme_scan_run`].
//!
//! Split into a sibling file (via `#[path]`) so the lifecycle module stays within
//! the module-size limit — the house pattern (`theme_scan_persist_tests.rs`,
//! `scan_runs_tests.rs`). Pure mapping tests: the repository-row → wire-DTO
//! carry, exercised without a database. (The params-snapshot tests moved with
//! `params_snapshot` itself into `theme_scan_start_tests.rs`.)

use super::*;

/// The repository header row maps 1:1 onto the wire DTO — every column the
/// history row shows is carried across, including the nullable `computed_cost`
/// and the `started_at` that drives the newest-first order. A dropped field
/// here would silently blank a column in the panel.
#[test]
fn scan_run_header_maps_every_row_field() {
    let run_id = Uuid::from_u128(1);
    let started_at = chrono::DateTime::<Utc>::from_timestamp(1_700_000_000, 0)
        .expect("fixed in-range timestamp");
    let row = ScanRunHeaderRow {
        run_id,
        model_id: "qwen-14b".to_string(),
        status: "completed".to_string(),
        candidates_total: Some(94),
        candidates_judged: 94,
        relevant_count: 31,
        irrelevant_count: 60,
        failed_count: 3,
        computed_cost: Some(0.0125),
        duration_ms: 45_000,
        started_at,
    };

    let dto = scan_run_header_from_row(row);

    assert_eq!(dto.run_id, run_id);
    assert_eq!(dto.model_id, "qwen-14b");
    assert_eq!(dto.status, "completed");
    assert_eq!(dto.candidates_total, Some(94));
    assert_eq!(dto.candidates_judged, 94);
    assert_eq!(dto.relevant_count, 31);
    assert_eq!(dto.irrelevant_count, 60);
    assert_eq!(dto.failed_count, 3);
    assert_eq!(dto.computed_cost, Some(0.0125));
    assert_eq!(dto.duration_ms, 45_000);
    assert_eq!(dto.started_at, started_at);
}

/// A null cost (local vLLM model / no token usage) and an absent progress
/// denominator must survive as `None`, not collapse to a fabricated 0
/// (Standing Rule 1 — "no cost" is distinct from "$0.00").
#[test]
fn scan_run_header_preserves_null_cost_and_total() {
    let row = ScanRunHeaderRow {
        run_id: Uuid::from_u128(2),
        model_id: "local-llama".to_string(),
        status: "completed".to_string(),
        candidates_total: None,
        candidates_judged: 0,
        relevant_count: 0,
        irrelevant_count: 0,
        failed_count: 0,
        computed_cost: None,
        duration_ms: 10,
        started_at: chrono::DateTime::<Utc>::from_timestamp(0, 0).expect("epoch is in range"),
    };

    let dto = scan_run_header_from_row(row);

    assert_eq!(dto.computed_cost, None);
    assert_eq!(dto.candidates_total, None);
}
