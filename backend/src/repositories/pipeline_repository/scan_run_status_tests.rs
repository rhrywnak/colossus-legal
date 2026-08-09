// The FAILED-status predicate, pinned on its own (ruling R3, 2026-08-09).
//
// A THIRD sibling test module for `scan_runs.rs`, for the same mechanical reason
// the first two exist: the parent reached the 300-line ceiling. The subject is
// distinct too — everything in `scan_runs_tests.rs` reads the SQL as source text
// because it needs a database, while `final_status` is the one pure function
// this repository owns and can simply be called.

use super::*;

fn tallies(relevant: i32, irrelevant: i32, failed: i32) -> ScanRunFinal {
    ScanRunFinal {
        run_id: Uuid::nil(),
        relevant_count: relevant,
        irrelevant_count: irrelevant,
        failed_count: failed,
        input_tokens: None,
        output_tokens: None,
        computed_cost: None,
        duration_ms: 0,
        summary_json: serde_json::json!({}),
    }
}

#[test]
fn a_fully_failed_run_records_failed_status_not_completed() {
    // Run 6a9fad89's own shape: 104 attempted, 104 dead, nothing judged.
    assert_eq!(final_status(&tallies(0, 0, 104)), SCAN_STATUS_FAILED);

    // ANTI-VACUITY. A predicate that returned `failed` for everything would
    // satisfy the line above while breaking every real run — and it would do
    // it silently, because a `failed` run simply stops projecting. The three
    // shapes that must still record `completed`:
    assert_eq!(
        final_status(&tallies(30, 117, 1)),
        SCAN_STATUS_COMPLETED,
        "a run with ONE dead call did real work — the Aug 7 Opus 4.8 run"
    );
    assert_eq!(
        final_status(&tallies(0, 124, 0)),
        SCAN_STATUS_COMPLETED,
        "judging 124 quotes and finding none relevant is a finding, not a failure"
    );
    assert_eq!(
        final_status(&tallies(0, 0, 0)),
        SCAN_STATUS_COMPLETED,
        "a scenario whose pool was empty had nothing to do; it did not fail"
    );
}

#[test]
fn a_failed_run_can_never_project_or_supersede() {
    // The recovery half of R3, and the reason the status change matters
    // beyond honesty: the projection query binds COMPLETED, so a run that
    // records FAILED is invisible to it — the previous completed run keeps
    // the projecting slot and its proposals stay in the queue. On 2026-08-09
    // the dead run recorded `completed`, took that slot, and projected
    // nothing; S-4 lost 30 proposals with no message anywhere.
    //
    // Asserted as a token INEQUALITY rather than by running the query,
    // because the query is bound to this exact constant (see
    // `scan_run_projection`) and the failure mode being guarded against is
    // the two spellings drifting into agreement.
    assert_ne!(
        final_status(&tallies(0, 0, 104)),
        SCAN_STATUS_COMPLETED,
        "the projection binds SCAN_STATUS_COMPLETED; a dead run matching it \
         is a dead run projecting nothing over a good run's proposals"
    );
}
