//! LIVE-DATABASE proofs for the scan's server-owned state (task
//! SCAN_SERVER_STATE, parts A, B, C and D).
//!
//! Every test here is `#[ignore]` because it needs a real `colossus_legal_v2`
//! PostgreSQL database — there is no `#[sqlx::test]` fixture infrastructure in
//! this project, so CI does not run them (the same convention
//! `repositories::pipeline_repository::config_overrides` uses for its live-DB
//! block, and `tests/scan_run_history_integration.rs` for its whole file).
//!
//! They live INSIDE the library rather than in `tests/` for one mechanical
//! reason: `judge_all` is `pub(crate)`, so an integration test — a separate
//! crate — cannot call it. What is being proved is the loop's own behaviour, so
//! the test has to be where the loop is.
//!
//! Run them against a live pipeline database with:
//!
//! ```text
//! cargo test --lib live_tests -- --ignored --test-threads=1
//! ```
//!
//! ## They must not be pointed at DEV's real database
//!
//! Each test writes a scenario, a `scan_runs` row and verdict rows, and deletes
//! the scenario afterwards (the run and its verdicts cascade). That is safe
//! against a scratch copy of the schema and is NOT something to run against
//! `colossus_legal_v2` itself — point `PIPELINE_DATABASE_URL` at a throwaway
//! database schema-copied from it. The slug every test uses is prefixed
//! `__test_` for the same reason the history suite's is.
//!
//! No LLM is called. The provider is a stub that returns canned verdict JSON, so
//! these cost nothing and take milliseconds. The stub, the seeding and the three
//! reads live in the sibling `theme_scan_live_fixture`.

use std::sync::Arc;

use colossus_extract::LlmProvider;
use serde_json::json;
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::repositories::pipeline_repository::{
    cancel_scan_run, insert_scan_run_stub, promote_scan_run_running, PromoteOutcome, ScanRunStart,
    ScanRunStub,
};
use crate::services::theme_scan_live_fixture::{
    cleanup, judge_n, pipeline_pool, run_state, seed_running_run, verdict_count, StubJudge,
    TestResult,
};

/// **B4.** Every verdict is on disk as it lands — BEFORE anything finalizes.
///
/// The measured claim, not an inferred one: after `judge_all` returns and before
/// `persist_and_summarize` or `finalize_scan_run_completed` has been called at
/// all, the database holds one `scan_run_verdicts` row per judged group. That is
/// what makes part D's projection show a run's findings while it is still going.
///
/// The run row is deliberately left `running` for the whole assertion, so the
/// test cannot accidentally be proving something about a completed run.
#[tokio::test]
#[ignore = "needs a live pipeline database — see the module doc; point PIPELINE_DATABASE_URL at a scratch copy"]
async fn verdicts_are_on_disk_before_anything_finalizes() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let (scenario_id, run_id) = seed_running_run(&pool, "b4_live_verdicts", 4).await?;

    // Never cancelled: this is the ordinary path.
    let judged = judge_n(
        &pool,
        run_id,
        "b4-node",
        4,
        2,
        Arc::new(StubJudge::new()),
        CancellationToken::new(),
    )
    .await;

    assert_eq!(judged.results.len(), 4, "all four groups judged");
    assert_eq!(judged.skipped, 0, "nothing was skipped");
    assert!(!judged.cancelled, "the token was never cancelled");

    assert_eq!(
        verdict_count(&pool, run_id).await?,
        4,
        "one verdict row per judged group must exist BEFORE finalize"
    );
    let (status, judged_count, relevant, _error, has_summary) = run_state(&pool, run_id).await?;
    assert_eq!(status, "running", "the run has not been finalized yet");
    assert_eq!(judged_count, 4, "progress was bumped per group");
    assert_eq!(relevant, 4, "the stub admits every candidate");
    assert!(
        !has_summary,
        "no summary is written until the run finalizes"
    );

    cleanup(&pool, scenario_id, "b4_live_verdicts").await?;
    Ok(())
}

/// **C5.** A stop mid-loop: `cancelled`, counts conserved, verdicts kept.
///
/// Concurrency 1 makes the arithmetic exact — the stub cancels the token from
/// inside its second call, so groups 1 and 2 are judged and groups 3–6 are never
/// started. Three separate claims are checked, and each is one of the ways this
/// feature could quietly be wrong:
///
///   * **conservation** — `judged + skipped = total`. A stop that lost a group
///     would show up here and nowhere else.
///   * **the record** — the row reads `cancelled` with NO error (nothing failed),
///     no summary (there is no finished whole to summarize), and the counts the
///     loop actually reached.
///   * **the findings survive** — the two verdicts written before the stop are
///     still on disk afterwards. This is the whole point of stopping rather than
///     deleting: what the scan already found is kept.
#[tokio::test]
#[ignore = "needs a live pipeline database — see the module doc; point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_stopped_run_keeps_what_it_judged_and_records_cancelled() -> TestResult<()> {
    let pool = pipeline_pool().await?;
    let (scenario_id, run_id) = seed_running_run(&pool, "c5_live_cancel", 6).await?;

    let token = CancellationToken::new();
    let provider = Arc::new(StubJudge::cancelling_after(2, token.clone()));
    // Concurrency 1 makes the arithmetic exact: the stub cancels from inside its
    // second call, so groups 1-2 are judged and 3-6 are never started.
    let judged = judge_n(
        &pool,
        run_id,
        "c5-node",
        6,
        1,
        Arc::clone(&provider) as Arc<dyn LlmProvider>,
        token,
    )
    .await;

    assert!(judged.cancelled, "the loop must report that it was stopped");
    assert_eq!(
        judged.results.len() + judged.skipped,
        6,
        "conservation: judged + skipped = total (judged {}, skipped {})",
        judged.results.len(),
        judged.skipped
    );
    assert_eq!(
        judged.results.len(),
        2,
        "two groups were judged before the stop"
    );
    assert_eq!(judged.skipped, 4, "four groups were never started");
    assert_eq!(
        provider.calls(),
        2,
        "a stopped scan stops SPENDING — no call is made for a skipped group"
    );

    // The judging task's own exit write, then the record it left behind.
    assert_eq!(
        cancel_scan_run(&pool, run_id).await?,
        1,
        "the running run was moved to cancelled"
    );
    assert_cancelled_record(&pool, run_id, 2).await?;

    // And a second cancel is a no-op rather than a lie: the run is already
    // terminal, so the guarded UPDATE matches nothing.
    assert_eq!(
        cancel_scan_run(&pool, run_id).await?,
        0,
        "cancelling an already-cancelled run must touch no row"
    );

    cleanup(&pool, scenario_id, "c5_live_cancel").await?;
    Ok(())
}

/// The four things a `cancelled` row must say, and the verdicts it must keep.
///
/// Split out of the test above for the function-size limit, and it earns the
/// split: this is the RECORD contract — what a stopped run looks like on disk —
/// and it is the half a future change is most likely to break by reaching for
/// `finalize_scan_run_completed` on the cancel path.
async fn assert_cancelled_record(pool: &PgPool, run_id: Uuid, judged: i32) -> TestResult<()> {
    let (status, judged_count, relevant, error, has_summary) = run_state(pool, run_id).await?;
    assert_eq!(status, "cancelled");
    assert_eq!(
        judged_count, judged,
        "the counts stand exactly where the loop left them"
    );
    assert_eq!(
        relevant, judged,
        "the stub admits every candidate it judged"
    );
    assert_eq!(error, None, "nothing failed, so no reason is recorded");
    assert!(
        !has_summary,
        "a stopped run has no finished whole to summarize"
    );
    assert_eq!(
        verdict_count(pool, run_id).await?,
        i64::from(judged),
        "the verdicts judged before the stop are KEPT, not discarded"
    );
    Ok(())
}

/// **D7.** A run that is STILL RUNNING projects the two verdicts it has judged.
///
/// The end-to-end claim of part D, measured rather than argued: judge two groups
/// against a `running` run, then ask the projection which run is showing and what
/// it proposes. Before this task the answer was "none" until the run completed.
///
/// The third assertion is the one that keeps a re-scan from blanking the queue: a
/// SECOND running run with no verdicts yet must not take the projecting slot from
/// the first, even though it is newer. (The two runs live under different
/// scenarios because the one-running-per-scenario index forbids them sharing one
/// — the EXISTS clause is what is under test here, not the ordering.)
#[tokio::test]
#[ignore = "needs a live pipeline database — see the module doc; point PIPELINE_DATABASE_URL at a scratch copy"]
async fn a_running_run_projects_the_verdicts_it_has_already_judged() -> TestResult<()> {
    use crate::repositories::pipeline_repository::{
        fetch_projecting_run, list_relevant_verdicts_for_run,
    };

    let pool = pipeline_pool().await?;
    let (scenario_id, run_id) = seed_running_run(&pool, "d7_live_projection", 5).await?;

    let judged = judge_n(
        &pool,
        run_id,
        "d7-node",
        2,
        1,
        Arc::new(StubJudge::new()),
        CancellationToken::new(),
    )
    .await;
    assert_eq!(judged.results.len(), 2);

    // The run is STILL `running` — nothing has finalized it.
    let (status, _, _, _, _) = run_state(&pool, run_id).await?;
    assert_eq!(
        status, "running",
        "the projection must not need a finished run"
    );

    let projecting = fetch_projecting_run(&pool, scenario_id).await?;
    assert_eq!(
        projecting.map(|r| r.run_id),
        Some(run_id),
        "a running run with verdicts must be the projecting run"
    );
    let proposals = list_relevant_verdicts_for_run(&pool, run_id).await?;
    assert_eq!(
        proposals.len(),
        2,
        "both admitted verdicts must reach the queue as proposals"
    );

    // A run that has judged NOTHING yet must not supersede one that has.
    let (empty_scenario, empty_run) = seed_running_run(&pool, "d7_live_no_verdicts", 5).await?;
    assert_eq!(
        fetch_projecting_run(&pool, empty_scenario)
            .await?
            .map(|r| r.run_id),
        None,
        "a run with no verdicts projects nothing (run {empty_run})"
    );

    cleanup(&pool, empty_scenario, "d7_live_no_verdicts").await?;
    cleanup(&pool, scenario_id, "d7_live_projection").await?;
    Ok(())
}

/// **A2, the backstop half.** The index refuses a second running run.
///
/// The pre-check in `theme_scan_start` is what a human normally meets; this is
/// the race it cannot close, and the proof that the collision comes back as
/// `AlreadyRunning` — the input to the same 409 — rather than as a raw database
/// error the route would have to answer with a 500.
#[tokio::test]
#[ignore = "needs a live pipeline database WITH migration 20260910142419 applied — see the module doc"]
async fn a_second_running_run_on_one_scenario_is_refused() -> TestResult<()> {
    use crate::repositories::pipeline_repository::find_running_scan_run;

    let pool = pipeline_pool().await?;
    let (scenario_id, first_run) = seed_running_run(&pool, "a2_live_one_running", 3).await?;

    // The pre-check sees it.
    let in_flight = find_running_scan_run(&pool, scenario_id).await?;
    assert_eq!(
        in_flight.map(|r| r.run_id),
        Some(first_run),
        "the in-flight lookup must find the run the 409 will name"
    );

    // The backstop refuses it.
    let second = Uuid::new_v4();
    insert_scan_run_stub(
        &pool,
        &ScanRunStub {
            run_id: second,
            scenario_id,
            requested_model_id: Some("stub-judge".to_string()),
            started_at: chrono::Utc::now(),
        },
    )
    .await?;
    let outcome = promote_scan_run_running(
        &pool,
        &ScanRunStart {
            run_id: second,
            model_id: "stub-judge".to_string(),
            resolved_params: json!({ "max_tokens": 512 }),
            candidates_total: 3,
            candidates_read: 3,
        },
    )
    .await?;
    assert_eq!(
        outcome,
        PromoteOutcome::AlreadyRunning,
        "the partial unique index must surface as AlreadyRunning, never as an Err"
    );

    // The loser's row is left in its birth state — a `failed` stub — so the
    // history records a scan that did not finish starting, which is the truth.
    let (status, _, _, _, _) = run_state(&pool, second).await?;
    assert_eq!(status, "failed", "the refused start stays a failed stub");

    cleanup(&pool, scenario_id, "a2_live_one_running").await?;
    Ok(())
}
