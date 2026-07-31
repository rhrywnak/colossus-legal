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

// ── "scanning is scoring" — behavioral, using a dead pool ─────────────────
//
// A pool aimed at a dead port never connects, so ANY real query fails fast.
// That is what lets these tests assert WHETHER persist attempted a
// `scenario_fact_refs` write, with no live database: if the write path still
// existed, a dead pool would make it fail and the failure would surface in the
// counts. Silence in the counts is therefore positive evidence that no
// per-candidate write is attempted at all.
//
// (The batched `scan_run_verdicts` audit write also fails against this pool. It
// is logged and deliberately does NOT touch the classification counts these
// tests assert — the scan still owes the caller the summary it paid for.)

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

fn meta() -> ScanRunMeta {
    ScanRunMeta {
        run_id: Uuid::nil(),
        scenario_id: Uuid::nil(),
        model_id: "m".to_string(),
        cost_per_input_token: None,
        cost_per_output_token: None,
        duration_ms: 0,
    }
}

/// An irrelevant verdict — never suggested, only sampled for the honesty check.
fn irrelevant_outcome() -> JudgeOutcome {
    JudgeOutcome {
        verdict: Ok(Verdict {
            relevant: false,
            proposed_role: FactRole::Supports,
            reason: "unrelated to the accusation".to_string(),
            confidence: 0.4,
        }),
        raw_reply: Some("{\"relevant\":false}".to_string()),
        input_tokens: Some(10),
        output_tokens: Some(5),
    }
}

/// The scan must NEVER touch `scenario_fact_refs` — merge is the only write path
/// into a scenario's candidate facts.
///
/// The dead pool is the instrument: if any per-candidate write were still
/// attempted it would fail here and be counted as a per-item failure (that is
/// precisely what the retired non-dry test asserted). A clean `failed: 0` proves
/// the write path is gone rather than merely disabled by a flag.
#[tokio::test]
async fn scan_never_attempts_a_fact_ref_write_even_against_a_dead_database() {
    let summary = persist_and_summarize(
        &dead_pool(),
        meta(),
        vec![(bias_instance("ev-1"), relevant_outcome())],
    )
    .await;

    assert_eq!(
        summary.relevant, 1,
        "the relevant verdict is scored and recorded"
    );
    assert_eq!(
        summary.failed, 0,
        "no scenario_fact_refs write is attempted, so a dead database cannot fail one"
    );
}

/// Every relevant verdict reaches the human as a checkable suggestion.
///
/// This is the behavior the old write path could silently break: a database
/// hiccup used to suppress a suggestion the scan had already paid an LLM call
/// for. With scoring decoupled from writing, `relevant` and `suggestions.len()`
/// are the same number by construction — and the human gets to decide on every
/// pick the model flagged.
#[tokio::test]
async fn every_relevant_verdict_becomes_a_suggestion_and_irrelevant_ones_do_not() {
    let summary = persist_and_summarize(
        &dead_pool(),
        meta(),
        vec![
            (bias_instance("ev-1"), relevant_outcome()),
            (bias_instance("ev-2"), irrelevant_outcome()),
            (bias_instance("ev-3"), relevant_outcome()),
        ],
    )
    .await;

    assert_eq!(summary.relevant, 2, "two relevant verdicts");
    assert_eq!(
        summary.suggestions.len(),
        summary.relevant,
        "every relevant verdict must be offered to the human as a suggestion"
    );
    assert_eq!(summary.irrelevant, 1, "the irrelevant one is not suggested");
    assert_eq!(summary.failed, 0);
    // The suggestions are the relevant candidates, not the rejected one.
    let ids: Vec<&str> = summary
        .suggestions
        .iter()
        .map(|s| s.graph_node_id.as_str())
        .collect();
    assert_eq!(ids, vec!["ev-1", "ev-3"]);
    // The exhaustive-recall identity still holds with the write path removed.
    assert_eq!(
        summary.candidates_read,
        summary.relevant + summary.irrelevant + summary.failed,
        "every candidate read must land in exactly one bucket"
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
