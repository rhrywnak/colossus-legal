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
            statement_type: None,
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
        statement_type: None,
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

/// A judge call that came back with no verdict — the 400 of 2026-08-09.
fn failed_outcome() -> JudgeOutcome {
    JudgeOutcome {
        verdict: Err("temperature is deprecated for this model".to_string()),
        raw_reply: None,
        input_tokens: None,
        output_tokens: None,
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
        // These tests are about classification, not about the pre-filter; the
        // counts ride through untouched and each test that cares sets its own.
        conservation: conservation(0, 0),
    }
}

/// `meta()` with a conservation block that says how many groups were judged.
///
/// The default fixture reports `judged: 0` because the classification tests do
/// not look at it. The reconciliation tests do — `judged` is the LEFT side of
/// R4's identity, and a fixture claiming zero would make the law trivially false.
fn meta_judging(judged: usize) -> ScanRunMeta {
    ScanRunMeta {
        conservation: conservation(judged, judged),
        ..meta()
    }
}

/// A conservation block for a pool of `pool` rows that judged `judged` groups.
fn conservation(pool: usize, judged: usize) -> ScanConservation {
    ScanConservation {
        pool,
        excluded_empty: 0,
        excluded_statement_type: 0,
        excluded_too_short: 0,
        duplicates_collapsed: pool.saturating_sub(judged),
        judged,
        // 0 is what the PRE-FILTER writes — it runs before any call and cannot
        // know. `persist_and_summarize` overwrites it from the fan-out's tally,
        // which is exactly what the failure tests below check.
        failed: 0,
    }
}

/// One candidate, judged alone — the ordinary (non-duplicate) case.
fn group(id: &str) -> CandidateGroup {
    CandidateGroup {
        representative: bias_instance(id),
        members: vec![id.to_string()],
    }
}

/// One candidate speaking for itself AND a byte-identical twin.
fn collapsed_group(id: &str, twin: &str) -> CandidateGroup {
    CandidateGroup {
        representative: bias_instance(id),
        members: vec![id.to_string(), twin.to_string()],
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

/// The scan must NEVER touch `scenario_fact_refs` — the human's ruling is the
/// only write path into a scenario's candidate facts.
///
/// ## RE-SCOPED, not retired (2026-08-08)
///
/// This test used to pin "merge is the only writer". Merge is gone, and what it
/// pinned is now MORE load-bearing rather than less: a completed run's admitted
/// verdicts reach the queue as a read-time PROJECTION, and the whole safety of
/// that design — junk scans cost nothing, a re-scan cannot touch a ruling, a run
/// can be deleted and its unruled proposals simply vanish — rests on scan
/// COMPLETION writing no candidate fact at all. The day something here starts
/// writing one, every one of those properties quietly stops being true.
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
        vec![(group("ev-1"), relevant_outcome())],
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
            (group("ev-1"), relevant_outcome()),
            (group("ev-2"), irrelevant_outcome()),
            (group("ev-3"), relevant_outcome()),
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

/// A collapsed pair is judged once and RULED as a set (task 2.15 Tier 2, R2).
///
/// Two things must be true at once, and they pull in opposite directions:
/// the run's tallies count the group ONCE (it cost one call, and a scan that
/// judged 124 quotes must not report 138), while the pick the human sees carries
/// BOTH node ids — because merging it has to rule the twin too, or the identical
/// sentence returns tomorrow as an unruled candidate.
#[tokio::test]
async fn a_collapsed_duplicate_is_counted_once_and_merged_as_a_set() {
    let summary = persist_and_summarize(
        &dead_pool(),
        meta(),
        vec![(collapsed_group("ev-45", "ev-46"), relevant_outcome())],
    )
    .await;

    assert_eq!(
        summary.relevant, 1,
        "one quote was judged, so the run reports one relevant verdict"
    );
    assert_eq!(summary.suggestions.len(), 1, "the human sees ONE card");

    let pick = &summary.suggestions[0];
    assert_eq!(
        pick.covers_node_ids,
        vec!["ev-45".to_string(), "ev-46".to_string()],
        "merging this pick must write the judgment onto the twin as well"
    );
    assert_eq!(
        pick.duplicate_count, 2,
        "the card says how many pool rows this one ruling settles"
    );
}

/// Every member of a collapsed group gets its OWN verdict row.
///
/// The scorecard joins Roman's ledger to `scan_run_verdicts` on `graph_node_id`.
/// A twin with no row would be scored as a statement the scan LOST, when in fact
/// the scan judged its text and folded the row on purpose — so the audit trail
/// carries one row per node even though only one call was made.
#[test]
fn every_member_of_a_collapsed_group_gets_its_own_verdict_row() {
    let mut acc = Accumulator::default();
    process_one(
        &meta(),
        collapsed_group("ev-45", "ev-46"),
        relevant_outcome(),
        &mut acc,
    );

    let ids: Vec<&str> = acc
        .verdicts
        .iter()
        .map(|v| v.graph_node_id.as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["ev-45", "ev-46"],
        "both nodes are answerable in the audit table"
    );
    assert!(
        acc.verdicts.iter().all(|v| v.relevant == Some(true)),
        "the twin carries the SAME verdict — it is the same sentence"
    );
    assert_eq!(
        acc.relevant, 1,
        "the run's tally still counts the judged group once"
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

/// The failed count reaches the block the REPORT is built from (ruling R4).
///
/// The defect: `scan_runs.failed_count` recorded 104 and `ThemeScanSummary.failed`
/// carried 104, but the tiles and the reconciliation sentence are built from
/// `ScanConservation` — which had no field for it. The number existed everywhere
/// except the one place the screen reads.
#[tokio::test]
async fn a_partially_failed_run_reports_its_failed_count_in_conservation() {
    let summary = persist_and_summarize(
        &dead_pool(),
        meta_judging(3),
        vec![
            (group("ev-1"), relevant_outcome()),
            (group("ev-2"), irrelevant_outcome()),
            (group("ev-3"), failed_outcome()),
        ],
    )
    .await;

    assert_eq!(summary.failed, 1, "the summary counts the dead call");
    assert_eq!(
        summary.conservation.failed, 1,
        "and so does the block the report's tiles and sentence are built from — \
         `prepare_pool` wrote 0 there because it runs before any call"
    );

    // R4's law, on the object that has to satisfy it. Asserted through the
    // production predicate rather than by re-adding the numbers here, so the test
    // and the runtime check cannot disagree about what reconciling means.
    assert!(
        summary
            .conservation
            .reconciles(summary.relevant, summary.irrelevant),
        "judged={} but relevant={} + irrelevant={} + failed={}",
        summary.conservation.judged,
        summary.relevant,
        summary.irrelevant,
        summary.conservation.failed
    );
}

/// …and the same block on a run where EVERY call died.
#[tokio::test]
async fn a_fully_failed_run_still_reconciles_with_nothing_relevant() {
    let summary = persist_and_summarize(
        &dead_pool(),
        meta_judging(2),
        vec![
            (group("ev-1"), failed_outcome()),
            (group("ev-2"), failed_outcome()),
        ],
    )
    .await;

    assert_eq!(summary.relevant, 0);
    assert_eq!(summary.conservation.failed, 2);
    assert!(
        summary
            .conservation
            .reconciles(summary.relevant, summary.irrelevant),
        "a run that judged nothing must still add up — this is the shape that \
         shipped as 'Complete · 104 judged · 0 relevant'"
    );
    assert!(
        summary.suggestions.is_empty(),
        "a call that never returned a verdict cannot propose anything"
    );
}
