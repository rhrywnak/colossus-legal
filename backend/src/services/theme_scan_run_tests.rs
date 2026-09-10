//! Unit tests for [`crate::services::theme_scan_run`].
//!
//! Split into a sibling file (via `#[path]`) so the lifecycle module stays within
//! the module-size limit — the house pattern (`theme_scan_persist_tests.rs`,
//! `scan_runs_tests.rs`). Pure mapping tests: the repository-row → wire-DTO
//! carry, exercised without a database. (The params-snapshot tests moved with
//! `params_snapshot` itself into `theme_scan_start_tests.rs`.)

use super::*;
// `Utc` is only needed by these fixtures' timestamps: the module under test
// stopped importing chrono when the merge path (which bound `Utc::now()` for the
// merge event) was removed.
use chrono::Utc;

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
        // Added by task 1.7C (ruling R2) so the history table can render the
        // Candidates and New columns and an honest failure reason.
        candidates_read: 94,
        error: None,
        dry_run: true,
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
    assert_eq!(dto.candidates_read, 94);
    assert_eq!(dto.error, None);
    assert!(dto.dry_run);
    // The delta is position-derived, so a single-row map cannot know it. It is
    // filled by `scan_run_delta::with_pool_deltas` once the whole history is in
    // hand — pinned here so nobody "helpfully" computes it in this function.
    assert_eq!(dto.pool_delta, None);
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
        candidates_read: 0,
        error: Some("vLLM offline".to_string()),
        dry_run: false,
    };

    let dto = scan_run_header_from_row(row);

    assert_eq!(dto.computed_cost, None);
    assert_eq!(dto.candidates_total, None);
    // The failure reason survives the map: "Failed" with no reason sends the reader
    // to the logs, which is the silent-failure shape Rule 1 exists to prevent.
    assert_eq!(dto.error.as_deref(), Some("vLLM offline"));
    // `candidates_read: 0` means "never got to read the pool" and must NOT be
    // rewritten into a number the history table would display as a real count.
    assert_eq!(dto.candidates_read, 0);
}

// ─── The STOP path's guard order, read off the shipped source ────────────────
//
// `cancel_scenario_scan_run` is a sequence of four fences, and three of its six
// exits are invisible to the compiler: a mis-ordered check compiles, passes every
// mapping test, and is wrong. So they are pinned textually — the same discipline
// `theme_scan_start_tests.rs` applies to the start sequence, and the SQL-shape
// tests apply to a rule that lives in a query.
//
// These read the SOURCE. They cannot prove the function runs correctly against a
// database; they prove it cannot be edited into the wrong shape without a test
// going red, which is the mistake that would actually be made.

/// The body of `cancel_scenario_scan_run`, signature to closing brace.
fn cancel_fn_body() -> String {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services/theme_scan_run.rs");
    let text = std::fs::read_to_string(path).expect("theme_scan_run.rs is readable");
    let start = text
        .find("pub async fn cancel_scenario_scan_run")
        .expect("cancel_scenario_scan_run is present");
    let rest = &text[start..];
    let end = rest.find("\n}").map(|i| i + 2).unwrap_or(rest.len());
    rest[..end].to_string()
}

/// The position of a needle in that body, or a failing assertion naming it.
fn at(body: &str, needle: &str) -> usize {
    body.find(needle)
        .unwrap_or_else(|| panic!("`{needle}` is no longer in cancel_scenario_scan_run"))
}

/// The four fences, in order: case → scenario → state → liveness.
///
/// Each ordering matters for its own reason. The case fence must come FIRST or a
/// caller learns that a run exists in another case by the shape of the refusal.
/// The scenario match must precede the state check, or a cross-scenario run's
/// STATUS leaks in a 409 that should have been a 404. And the state check must
/// precede the token lookup, so a run that already finished is answered with the
/// status it holds rather than with "no live task owns it" — two different
/// sentences for two different situations.
#[test]
fn the_stop_checks_its_four_fences_in_order() {
    let body = cancel_fn_body();

    let case_fence = at(&body, "load_scenario_fenced(");
    let read = at(&body, "get_scan_run(");
    let scenario_match = at(&body, "row.scenario_id != scenario_id");
    let state = at(&body, "row.status != SCAN_STATUS_RUNNING");
    let token = at(&body, "scan_cancel");
    let fire = at(&body, "token.cancel()");

    assert!(
        case_fence < read,
        "the case fence must run BEFORE the run is read — a caller must not learn \
         that a run exists in another case"
    );
    assert!(
        read < scenario_match && scenario_match < state,
        "the scenario match must run BEFORE the state check, or a cross-scenario \
         run's status leaks in a 409 that should have been a 404"
    );
    assert!(
        state < token,
        "the state check must run BEFORE the token lookup: a run that already \
         settled is answered with the status it holds, not with 'no live task \
         owns it'"
    );
    assert!(
        token < fire,
        "the token is looked up before it is cancelled"
    );
}

/// A run in ANOTHER scenario is reported exactly like an absent one.
///
/// `get_scan_run` is keyed by `run_id` alone, so this compare is the whole
/// scenario fence. It must yield `ScanRunNotFound` — the same observable as a
/// truly-absent id — and NOT a distinct variant, which would let a caller probe
/// another scenario's runs by the difference between two error messages.
#[test]
fn a_run_in_another_scenario_is_indistinguishable_from_an_absent_one() {
    let body = cancel_fn_body();
    let idx = at(&body, "row.scenario_id != scenario_id");
    let arm = &body[idx..(idx + 200).min(body.len())];
    assert!(
        arm.contains("ScanRunNotFound"),
        "a cross-scenario run must be reported as not-found, not as its own \
         variant: {arm}"
    );
}

/// A `running` row with no live token is REFUSED, never accepted.
///
/// The Standing Rule 1 case this feature leans on hardest. A 202 here would
/// promise a stop that nothing in this process is going to perform, and the human
/// would watch a progress bar that never becomes `cancelled`. The absent entry is
/// a real answer and says so.
#[test]
fn a_running_run_with_no_live_task_is_refused_rather_than_silently_accepted() {
    let body = cancel_fn_body();
    let idx = at(&body, "let Some(token) = token else");
    let arm = &body[idx..(idx + 500).min(body.len())];
    assert!(
        arm.contains("ScanRunNotCancellable"),
        "a missing token must return the refusal, not fall through to Ok: {arm}"
    );
    assert!(
        arm.contains("tracing::warn!"),
        "and it must be observable in the log, not only to the caller: {arm}"
    );
}

/// The stop writes NOTHING to `scan_runs`. One writer, and it is the judging task.
///
/// If this function ever wrote the row itself, its write and the loop's last
/// progress bump would race: a stopped run could end up `cancelled` with counts
/// from before its final in-flight calls, or `running` again after them. The
/// absence is the design, so the absence is what is asserted.
#[test]
fn the_stop_route_writes_no_row_of_its_own() {
    let body = cancel_fn_body();
    for writer in [
        "cancel_scan_run(",
        "fail_scan_run(",
        "finalize_scan_run_completed(",
        "bump_scan_run_progress(",
    ] {
        assert!(
            !body.contains(writer),
            "the cancel route must not write the run row — `{writer}` belongs to \
             the judging task, which is the one writer that knows what it judged"
        );
    }
}
