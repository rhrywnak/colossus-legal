// Tests for `services::last_run` — board 5's numbers from PROD's store.

use super::*;
use crate::repositories::pipeline_repository::last_run::DocumentCounts;
use chrono::TimeZone;

// STRUCTURAL (test): the case's timezone as PROD's settings hold it.
const TZ: &str = "America/New_York";

fn at(month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, month, day, hour, minute, 14)
        .single()
        .expect("a valid moment")
}

fn step(
    name: &str,
    runs: i64,
    avg: f64,
    failed: i64,
    failed_at: Option<DateTime<Utc>>,
) -> StepStats {
    StepStats {
        step_name: name.into(),
        runs,
        avg_secs: Some(avg),
        failed,
        last_failed_at: failed_at,
    }
}

/// PROD's `pipeline_steps` on 2026-09-24, grouped as the repository returns them
/// (in no particular order).
fn prod_facts() -> LastRunFacts {
    LastRunFacts {
        counts: DocumentCounts {
            total: 20,
            finished: 20,
        },
        quote_pct: Some(97.66),
        last_activity: Some(at(9, 5, 14, 6)),
        steps: vec![
            step("verify", 20, 0.389, 0, None),
            step("auto_approve", 20, 0.018, 0, None),
            step("completeness", 20, 0.1, 0, None),
            step("extract_text", 28, 15.5, 0, None),
            step("index", 21, 49.9, 0, None),
            step("ingest", 20, 2.9, 0, None),
            step("ingest_delta", 1, 51.8, 0, None),
            step("llm_extract_pass1", 20, 323.5, 0, None),
            step("llm_extract_pass2", 21, 150.7, 1, Some(at(8, 17, 18, 41))),
            step("upload", 20, 0.342, 0, None),
        ],
        running: 0,
        avg_doc_secs: Some(600.0),
    }
}

#[test]
fn prods_store_draws_board_five() {
    let w = LastRunWording::for_test();
    let got = compose_last_run(&prod_facts(), &w, TZ);
    let cards: Vec<(&str, &str)> = got
        .cards
        .iter()
        .map(|c| (c.value.as_str(), c.label.as_str()))
        .collect();
    assert_eq!(
        cards,
        [
            ("20 of 20", "documents finished"),
            ("97.7%", "of quotes found in their source"),
            ("Sat 5 Sep", "last run, 10:06 am"),
        ]
    );
    assert_eq!(
        got.progress,
        "No document run in progress. Time estimates appear here while one is running."
    );
    let labels: Vec<&str> = got.steps.iter().map(|s| s.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Upload",
            "Read the text",
            "First extraction pass",
            "Second extraction pass",
            "Check the quotes",
            "Approve automatically",
            "Load into the case graph",
            "Graph update",
            "Search index",
            "Completeness check",
        ]
    );
    assert_eq!(got.steps[0].avg, "342 ms");
    assert_eq!(got.steps[2].avg, "5 m 24 s");
    assert_eq!(got.steps[3].failed, "1");
    assert_eq!(
        got.summary,
        "191 step runs, 1 failed (Second extraction pass, 17 Aug)."
    );
    assert!(got.warnings.is_empty());
}

#[test]
fn a_run_in_progress_gives_the_estimate() {
    let w = LastRunWording::for_test();
    let mut facts = prod_facts();
    facts.running = 1;
    facts.counts.finished = 18;
    let got = compose_last_run(&facts, &w, TZ);
    assert_eq!(
        got.progress,
        "A document run is in progress: 2 documents left, about 20 m 0 s to go."
    );
    facts.avg_doc_secs = None;
    let got = compose_last_run(&facts, &w, TZ);
    assert!(
        got.progress.contains("No finished document yet"),
        "{}",
        got.progress
    );
}

#[test]
fn a_step_the_page_cannot_name_is_shown_last_and_said_out_loud() {
    let w = LastRunWording::for_test();
    let mut facts = prod_facts();
    facts.steps.push(step("reindex_all", 2, 3.0, 0, None));
    let got = compose_last_run(&facts, &w, TZ);
    assert_eq!(
        got.steps.last().map(|s| s.label.as_str()),
        Some("reindex_all")
    );
    assert_eq!(got.warnings.len(), 1);
    assert!(got.warnings[0].contains("reindex_all"));
}

#[test]
fn no_documents_says_so_and_nothing_else() {
    let w = LastRunWording::for_test();
    let mut facts = prod_facts();
    facts.counts = DocumentCounts {
        total: 0,
        finished: 0,
    };
    let got = compose_last_run(&facts, &w, TZ);
    assert_eq!(got.empty.as_deref(), Some("No document has been run yet."));
    assert!(got.cards.is_empty() && got.steps.is_empty());
}

#[test]
fn no_quote_check_yet_shows_a_dash_not_zero() {
    let w = LastRunWording::for_test();
    let mut facts = prod_facts();
    facts.quote_pct = None;
    let got = compose_last_run(&facts, &w, TZ);
    assert_eq!(got.cards[1].value, "—");
}

#[test]
fn a_clean_history_says_none_failed() {
    let w = LastRunWording::for_test();
    let mut facts = prod_facts();
    facts.steps.retain(|s| s.failed == 0);
    let got = compose_last_run(&facts, &w, TZ);
    assert_eq!(got.summary, "170 step runs, none failed.");
}

#[test]
fn every_step_in_running_order_has_a_plain_name() {
    let w = LastRunWording::for_test();
    for name in STEP_ORDER {
        assert!(step_label(name, &w).is_some(), "{name} has no plain name");
    }
}

#[test]
fn durations_read_as_board_five_writes_them() {
    assert_eq!(duration(0.342), "342 ms");
    assert_eq!(duration(15.5), "15.5 s");
    assert_eq!(duration(323.5), "5 m 24 s");
    assert_eq!(duration(119.6), "2 m 0 s");
    assert_eq!(duration(4320.0), "1 h 12 m");
}
