//! Integration tests for the read-only scenario graph traversals
//! (`repositories::scenario_repository::ScenarioRepository`).
//!
//! Scope here is task 0.3c's `anchored_allegation_evidence` — the
//! allegation-anchored evidence pull (Allegation as entry, Evidence as result).
//!
//! Every test touches a live Neo4j, so each is `#[tokio::test] #[ignore]` and
//! gated on the `NEO4J_TEST_URI` env var. Unlike the human-facts suite these
//! are **read-only** — they never write or delete — so there is no reset helper
//! and they may point at the populated DEV graph. Run:
//!
//! ```text
//! NEO4J_TEST_URI=bolt://10.10.100.200:7687 \
//!   cargo test --test scenario_traversals_integration -- --ignored --test-threads=1
//! ```
//!
//! Without `NEO4J_TEST_URI` each test prints a skip notice and returns success,
//! so `--ignored` never fails for lack of a database (mirrors the
//! `human_facts_integration.rs` Tier-2 skip-clean pattern).
//!
//! ## Baseline provenance (NOT an arbitrary fixture)
//!
//! The ¶54 counts and the four named evidence ids asserted below are an
//! **observed live baseline**, captured from the 0.3c shape-verification probes
//! against DEV (beta.334) on **2026-06-27**: the `(:Evidence)-[r]->(:Allegation)`
//! axis for the ¶54 allegation
//! `doc-awad-v-catholic-family-complaint-11-1-13:allegation:cd24fccb`
//! carried **6 CORROBORATES + 4 REBUTS** edges (10 total). If a count differs at
//! run time that means the GRAPH changed, not that a test is wrong — STOP and
//! report, do not "fix" the assertion.

use colossus_legal_backend::repositories::scenario_repository::{
    EvidencePolarity, ScenarioRepository,
};
use neo4rs::Graph;

/// The known-good ¶54 allegation anchor (Awad v. CFS complaint). Stable `id`,
/// not a paragraph number — the method takes the id directly.
const ALLEGATION_54_ID: &str = "doc-awad-v-catholic-family-complaint-11-1-13:allegation:cd24fccb";

/// The four REBUTS evidence ids the 2026-06-27 probes confirmed under ¶54:
/// two George Phillips "No." admissions (page 4) and two CFS "sanctions were
/// never pursued" nodes. Asserted by `id` *suffix* because the composite id's
/// document/hash tail is the stable, human-recognizable part.
const REBUT_ID_SUFFIXES: [&str; 4] = ["3faa8602", "8c7ce875", "370f8fdf", "bf8019ca"];

/// Connect to the test Neo4j, or `None` (with a skip notice) when
/// `NEO4J_TEST_URI` is unset. Read-only — safe against a populated graph.
async fn test_graph(test_name: &str) -> Option<Graph> {
    dotenvy::dotenv().ok();
    let Ok(uri) = std::env::var("NEO4J_TEST_URI") else {
        eprintln!("SKIP {test_name}: NEO4J_TEST_URI not set (live graph required)");
        return None;
    };
    let user = std::env::var("NEO4J_TEST_USER").unwrap_or_else(|_| "neo4j".to_string());
    let password = std::env::var("NEO4J_TEST_PASSWORD").unwrap_or_else(|_| "neo4j".to_string());
    Some(
        Graph::new(uri, user, password)
            .await
            .expect("connect to NEO4J_TEST_URI"),
    )
}

#[tokio::test]
#[ignore]
async fn rebutting_returns_the_four_named_admissions() {
    let Some(graph) = test_graph("rebutting_returns_the_four_named_admissions").await else {
        return;
    };
    let repo = ScenarioRepository::new(graph);

    let resp = repo
        .anchored_allegation_evidence(ALLEGATION_54_ID, EvidencePolarity::Rebutting)
        .await
        .expect("anchored_allegation_evidence (Rebutting) query");

    assert_eq!(resp.allegation_id, ALLEGATION_54_ID);
    assert!(
        resp.facts.len() >= 4,
        "expected ≥4 REBUTS facts for ¶54, got {} — graph may have changed (see baseline note)",
        resp.facts.len()
    );

    // Every returned edge is a REBUTS, sourced from type(r).
    for fact in &resp.facts {
        assert_eq!(fact.polarity, "REBUTS", "evidence {}", fact.evidence_id);
        assert_eq!(fact.allegation_id, ALLEGATION_54_ID);
    }

    // The four 2026-06-27 baseline ids are present...
    for suffix in REBUT_ID_SUFFIXES {
        let row = resp
            .facts
            .iter()
            .find(|f| f.evidence_id.ends_with(suffix))
            .unwrap_or_else(|| {
                panic!("expected a REBUTS evidence id ending '{suffix}' under ¶54 (baseline)")
            });
        // ...and each carries a speaker (George Phillips / Catholic Family
        // Services) and a verbatim quote — the CASE/stated_by + quote columns.
        assert!(
            row.stated_by.is_some(),
            "evidence {} should have a stated_by speaker",
            row.evidence_id
        );
        assert!(
            row.verbatim_quote.is_some(),
            "evidence {} should have a verbatim_quote",
            row.evidence_id
        );
    }
}

#[tokio::test]
#[ignore]
async fn corroborating_returns_the_six_baseline_facts() {
    let Some(graph) = test_graph("corroborating_returns_the_six_baseline_facts").await else {
        return;
    };
    let repo = ScenarioRepository::new(graph);

    let resp = repo
        .anchored_allegation_evidence(ALLEGATION_54_ID, EvidencePolarity::Corroborating)
        .await
        .expect("anchored_allegation_evidence (Corroborating) query");

    assert_eq!(
        resp.facts.len(),
        6,
        "expected exactly 6 CORROBORATES facts for ¶54 (2026-06-27 baseline), got {} — graph may have changed",
        resp.facts.len()
    );
    for fact in &resp.facts {
        assert_eq!(
            fact.polarity, "CORROBORATES",
            "evidence {}",
            fact.evidence_id
        );
        assert_eq!(fact.allegation_id, ALLEGATION_54_ID);
    }
}

#[tokio::test]
#[ignore]
async fn both_returns_all_ten_baseline_facts() {
    let Some(graph) = test_graph("both_returns_all_ten_baseline_facts").await else {
        return;
    };
    let repo = ScenarioRepository::new(graph);

    let resp = repo
        .anchored_allegation_evidence(ALLEGATION_54_ID, EvidencePolarity::Both)
        .await
        .expect("anchored_allegation_evidence (Both) query");

    assert_eq!(
        resp.facts.len(),
        10,
        "expected exactly 10 (6 CORROBORATES + 4 REBUTS) facts for ¶54 (2026-06-27 baseline), got {} — graph may have changed",
        resp.facts.len()
    );
    // Polarity is only ever one of the two selected labels.
    for fact in &resp.facts {
        assert!(
            fact.polarity == "REBUTS" || fact.polarity == "CORROBORATES",
            "unexpected polarity '{}' on evidence {}",
            fact.polarity,
            fact.evidence_id
        );
    }
    let rebuts = resp.facts.iter().filter(|f| f.polarity == "REBUTS").count();
    let corroborates = resp
        .facts
        .iter()
        .filter(|f| f.polarity == "CORROBORATES")
        .count();
    assert_eq!(rebuts, 4, "expected 4 REBUTS within Both");
    assert_eq!(corroborates, 6, "expected 6 CORROBORATES within Both");
}
