//! Unit tests for `theme_scan_persist.rs` — kept in a sibling file
//! (`#[cfg(test)] #[path = "..."] mod tests;`) so the parent module
//! stays under the 300-line limit (house pattern, see registry_tests.rs).

use super::*;

#[test]
fn add_tokens_stays_none_until_first_report() {
    let mut sum = None;
    add_tokens(&mut sum, None);
    assert_eq!(sum, None, "no report keeps the sum absent, not zero");
    add_tokens(&mut sum, Some(10));
    add_tokens(&mut sum, Some(5));
    assert_eq!(sum, Some(15));
    add_tokens(&mut sum, None);
    assert_eq!(
        sum,
        Some(15),
        "a later absent report does not reset the sum"
    );
}

#[test]
fn compute_cost_none_when_costs_absent() {
    // vLLM: no per-token cost → no computed cost even with token counts.
    assert_eq!(compute_cost(Some(1000), Some(500), None, None), None);
    assert_eq!(compute_cost(Some(1000), Some(500), Some(0.001), None), None);
}

#[test]
fn compute_cost_none_when_tokens_absent() {
    assert_eq!(compute_cost(None, None, Some(0.001), Some(0.002)), None);
}

#[test]
fn compute_cost_multiplies_when_all_known() {
    let cost =
        compute_cost(Some(1000), Some(500), Some(0.001), Some(0.002)).expect("all known → Some");
    // 1000*0.001 + 500*0.002 = 1.0 + 1.0 = 2.0
    assert!((cost - 2.0).abs() < 1e-9, "got {cost}");
}

fn rejected(id: &str) -> ThemeScanRejected {
    ThemeScanRejected {
        graph_node_id: id.to_string(),
        reason: "r".to_string(),
        confidence: 0.1,
        content: BiasInstance {
            evidence_id: id.to_string(),
            title: String::new(),
            verbatim_quote: None,
            question: None,
            page_number: None,
            pattern_tags: Vec::new(),
            stated_by: None,
            about: Vec::new(),
            document: None,
        },
    }
}

#[test]
fn sample_returns_all_when_under_max() {
    let out = sample_rejected(vec![rejected("a"), rejected("b")], 10);
    assert_eq!(out.len(), 2);
}

#[test]
fn sample_caps_and_spreads_when_over_max() {
    let set: Vec<_> = (0..100).map(|i| rejected(&format!("e{i:03}"))).collect();
    let out = sample_rejected(set, 5);
    assert_eq!(out.len(), 5);
    assert_eq!(out[0].graph_node_id, "e000");
    assert_eq!(out[1].graph_node_id, "e020");
    assert_eq!(out[4].graph_node_id, "e080");
}

// ── dry_run suppression (A4) — behavioral, using a dead pool ──────────────
//
// A pool aimed at a dead port never connects, so ANY real query fails fast.
// That lets a test assert WHETHER persist attempted a `scenario_fact_refs`
// write without a live database: dry-run must not attempt it (relevant is
// recorded, nothing fails); a normal run must (the write fails and is
// counted). The scan_runs audit write also fails here and is logged — it does
// not affect the classification counts these tests assert.

use crate::domain::fact_role::FactRole;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

fn bias_instance(id: &str) -> BiasInstance {
    BiasInstance {
        evidence_id: id.to_string(),
        title: String::new(),
        verbatim_quote: None,
        question: None,
        page_number: None,
        pattern_tags: Vec::new(),
        stated_by: None,
        about: Vec::new(),
        document: None,
    }
}

fn relevant_outcome() -> JudgeOutcome {
    JudgeOutcome {
        verdict: Ok(Verdict {
            relevant: true,
            proposed_role: FactRole::Supports,
            reason: "backs the accusation".to_string(),
            confidence: 0.9,
        }),
        raw_reply: Some("{\"relevant\":true}".to_string()),
        input_tokens: Some(100),
        output_tokens: Some(20),
    }
}

fn dead_pool() -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(500))
        .connect_lazy("postgres://127.0.0.1:1/nodb")
        .expect("connect_lazy builds a pool without connecting")
}

fn meta(dry_run: bool) -> ScanRunMeta {
    ScanRunMeta {
        run_id: Uuid::nil(),
        scenario_id: Uuid::nil(),
        model_id: "m".to_string(),
        dry_run,
        cost_per_input_token: None,
        cost_per_output_token: None,
        duration_ms: 0,
    }
}

#[tokio::test]
async fn dry_run_records_relevant_without_writing_fact_refs() {
    let summary = persist_and_summarize(
        &dead_pool(),
        meta(true),
        vec![(bias_instance("ev-1"), relevant_outcome())],
    )
    .await;
    assert_eq!(
        summary.relevant_written, 1,
        "a dry-run relevant verdict is recorded, not written"
    );
    assert_eq!(
        summary.failed, 0,
        "no scenario_fact_refs write was attempted, so nothing failed"
    );
    assert!(summary.dry_run);
}

#[tokio::test]
async fn non_dry_run_attempts_the_write_and_counts_its_failure() {
    let summary = persist_and_summarize(
        &dead_pool(),
        meta(false),
        vec![(bias_instance("ev-1"), relevant_outcome())],
    )
    .await;
    assert_eq!(
        summary.relevant_written, 0,
        "the fact_refs write was attempted and failed, so nothing was written"
    );
    assert_eq!(
        summary.failed, 1,
        "a real write attempt failed and was counted (the write path IS taken)"
    );
}

#[test]
fn count_to_i32_clamps_impossible_overflow_without_panic() {
    // A scan never has this many candidates; the guard must cap (and log), not
    // panic or wrap (Standing Rule 1). The happy path is exercised everywhere else.
    assert_eq!(count_to_i32(usize::MAX, "test"), i32::MAX);
    assert_eq!(count_to_i32(0, "test"), 0);
    assert_eq!(count_to_i32(94, "test"), 94);
}
