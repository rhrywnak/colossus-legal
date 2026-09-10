//! Display tests for the four refusals a human meets while a scan is RUNNING
//! (task SCAN_SERVER_STATE).
//!
//! A sibling of `theme_scan_error_tests.rs` rather than more of it: that file was
//! at the 300-line module limit, and the split is along a real line. Every test
//! there is about a scan that could not start — a missing prompt, a bad model, a
//! scenario with no attack text — and every test here is about a scan that is
//! already going, read by somebody watching it happen.
//!
//! The HTTP status each of these earns is pinned separately, in
//! `api::scenario_theme_scan_tests`. These pin the SENTENCE.

use super::*;

#[test]
fn display_scan_already_running_names_the_run_and_the_two_ways_out() {
    let running = Uuid::from_u128(0x5ca7);
    let s = ThemeScanError::ScanAlreadyRunning {
        scenario_id: Uuid::nil(),
        run_id: running,
    }
    .to_string();
    assert!(
        s.contains(&running.to_string()),
        "the refusal must name the run that IS going, so a log reader can find it: {s}"
    );
    // Both options, because they are genuinely different choices: let it finish,
    // or stop it and start the one you wanted.
    assert!(s.contains("watch that one"), "missing the wait option: {s}");
    assert!(s.contains("stop it"), "missing the stop option: {s}");
}

#[test]
fn display_scan_run_not_running_names_the_status_it_actually_holds() {
    // "Nothing to stop" without saying WHY is the shape that sends a human to the
    // logs. The status is the whole answer: `completed` means it finished on its
    // own; `cancelled` means somebody else stopped it.
    let s = ThemeScanError::ScanRunNotRunning {
        run_id: Uuid::nil(),
        status: "completed".to_string(),
    }
    .to_string();
    assert!(s.contains("completed"), "missing the actual status: {s}");
    assert!(
        s.contains("nothing to stop"),
        "missing the plain answer: {s}"
    );
}

#[test]
fn display_scan_run_not_cancellable_explains_the_state_and_the_next_step() {
    // The narrowest case in the feature, and therefore the one most likely to be
    // met with no idea what it means: a `running` row whose judging task is gone.
    // The message has to carry its own explanation.
    let run = Uuid::from_u128(0xf00d);
    let s = ThemeScanError::ScanRunNotCancellable { run_id: run }.to_string();
    assert!(s.contains(&run.to_string()), "missing the run id: {s}");
    assert!(
        s.contains("no judging task"),
        "must say WHY it cannot be stopped: {s}"
    );
    assert!(
        s.contains("reload the page"),
        "must give the human something to do: {s}"
    );
}

#[test]
fn display_in_flight_check_failed_names_the_scenario_and_the_source() {
    // The mirror of `display_scan_run_write_failed_names_run_and_source`: this one
    // fails BEFORE a run row exists, so the log line is the only record, and the
    // scenario is the only handle it can carry.
    let scenario = Uuid::from_u128(0xbeef);
    let s = ThemeScanError::ScanRunInFlightCheckFailed {
        scenario_id: scenario,
        source: PipelineRepoError::Database("connection reset".to_string()),
    }
    .to_string();
    assert!(
        s.contains(&scenario.to_string()),
        "missing the scenario: {s}"
    );
    assert!(
        s.contains("connection reset"),
        "the underlying cause must survive: {s}"
    );
}
