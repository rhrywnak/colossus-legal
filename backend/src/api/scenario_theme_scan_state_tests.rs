//! HTTP-mapping tests for the four refusals a human meets while a scan is RUNNING
//! (task SCAN_SERVER_STATE).
//!
//! A sibling of `scenario_theme_scan_tests.rs` rather than more of it: that file
//! was at the 300-line module limit, and the split follows the one the error
//! taxonomy's tests already make (`theme_scan_state_error_tests.rs`). Those pin
//! the SENTENCE each variant produces; these pin the STATUS and the `details`
//! payload the browser branches on.
//!
//! `details` is where the substance is, and it is why these are tests rather than
//! a comment: `AppError`'s envelope writes `"conflict"` in the top-level `error`
//! field for every 409 this API returns, so the code that says WHICH conflict —
//! and the run id the panel adopts — live one level down. A refactor that dropped
//! either would still return a correct-looking 409.

use super::*;
use uuid::Uuid;

/// **A2.** A second start is a 409 that hands the browser the running run's id.
///
/// The id in `details` is the load-bearing half of this mapping: without it the
/// client can only show a message, and with it the client ADOPTS the run that is
/// actually happening — which is the difference between "you did something
/// wrong" and "here is the scan you asked about". Pinned so a refactor cannot
/// quietly drop the field or demote the arm into the catch-all 500.
#[test]
fn a_second_scan_start_maps_to_409_naming_the_run_already_going() {
    let running = Uuid::from_u128(0x5ca7);
    let e = ThemeScanError::ScanAlreadyRunning {
        scenario_id: Uuid::nil(),
        run_id: running,
    };
    match map_scan_error(e) {
        AppError::Conflict { message, details } => {
            assert_eq!(
                details["reason"], "scan_already_running",
                "the client branches on this code, not on the prose"
            );
            assert_eq!(
                details["run_id"],
                serde_json::json!(running),
                "the 409 must carry the run the browser is meant to adopt"
            );
            // Plain words, because a human meeting this is not being told off.
            assert!(
                message.contains("already running") && message.contains("stop it"),
                "409 must say what is happening and what the human can do: {message}"
            );
        }
        other => panic!("expected 409 Conflict, got {other:?}"),
    }
}

/// A stop for a run that is not running is a 409 carrying the status it holds.
///
/// Not a 404 (the run exists), not a 400 (the request is well-formed): the run
/// already settled, which is an answer. The status rides in `details` so the
/// client can settle its own view instead of polling for a transition that has
/// already happened.
#[test]
fn stopping_a_settled_run_maps_to_409_naming_its_status() {
    let e = ThemeScanError::ScanRunNotRunning {
        run_id: Uuid::nil(),
        status: "completed".to_string(),
    };
    match map_scan_error(e) {
        AppError::Conflict { message, details } => {
            assert_eq!(details["reason"], "scan_not_running");
            assert_eq!(details["status"], "completed");
            assert!(
                message.contains("completed"),
                "the message must name the status it actually holds: {message}"
            );
        }
        other => panic!("expected 409 Conflict, got {other:?}"),
    }
}

/// A `running` row with no live task is a 409, never a cheerful 202.
///
/// Standing Rule 1 in the shape this feature could most easily get wrong: a 202
/// would promise a stop that nothing in this process is going to perform, and
/// the human would watch a progress bar that never becomes `cancelled`.
#[test]
fn a_run_with_no_live_task_is_refused_rather_than_accepted() {
    let e = ThemeScanError::ScanRunNotCancellable {
        run_id: Uuid::nil(),
    };
    match map_scan_error(e) {
        AppError::Conflict { details, .. } => {
            assert_eq!(details["reason"], "scan_not_cancellable");
        }
        other => panic!("expected 409 Conflict, got {other:?}"),
    }
}

/// The in-flight CHECK failing is a 500 that KEEPS ITS MESSAGE.
///
/// Two claims, and the second is the one a refactor loses first.
///
/// A 500, never a permissive start — the mirror of
/// `provenance_check_failure_is_a_500_not_a_permissive_delete`, in the expensive
/// direction: an unreadable check must not be mistaken for "nothing is running,
/// go ahead", which would launch a second metered scan over a pool the first one
/// is already paying for.
///
/// And the message survives, because this failure happens at step 3b — BEFORE
/// `write_stub` — so there is no run row for an operator to open and the toast is
/// its only surface. That is the same reasoning `DefinitionInvalid` and
/// `ModelLookupFailed` are in that arm for, and this test is what keeps the
/// variant from drifting into the generic catch-all, where the scenario id, the
/// cause and the recovery action would all be thrown away.
#[test]
fn in_flight_check_failure_is_a_500_that_keeps_its_message() {
    let scenario = Uuid::from_u128(0xbeef);
    let e = ThemeScanError::ScanRunInFlightCheckFailed {
        scenario_id: scenario,
        source: crate::repositories::pipeline_repository::PipelineRepoError::Database(
            "connection reset".to_string(),
        ),
    };
    match map_scan_error(e) {
        AppError::Internal { message } => {
            assert!(
                message.contains(&scenario.to_string()),
                "the toast is this failure's only surface — it must name the scenario: {message}"
            );
            assert!(
                message.contains("connection reset"),
                "the underlying cause must reach the operator: {message}"
            );
            assert!(
                message.contains("try again"),
                "a pre-row failure must carry its own recovery action: {message}"
            );
            assert_ne!(
                message, "theme scan failed",
                "this variant must NOT fall into the generic catch-all"
            );
        }
        other => panic!("expected a 500, got {other:?}"),
    }
}
