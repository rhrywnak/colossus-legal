//! Integration tests for `bias::repository::BiasRepository::all_evidence_about_subject`
//! — the D2a *ungated* candidate read that the Theme Scan (D2b) consumes.
//!
//! Every test touches a live Neo4j, so each is `#[tokio::test] #[ignore]` and
//! gated on the `NEO4J_TEST_URI` env var. These are **read-only** — they never
//! write or delete — so there is no reset helper and they may point at the
//! populated DEV graph. Run:
//!
//! ```text
//! NEO4J_TEST_URI=bolt://10.10.100.200:7687 \
//!   cargo test --test bias_repository_integration -- --ignored --test-threads=1
//! ```
//!
//! Without `NEO4J_TEST_URI` each test prints a skip notice and returns success,
//! so `--ignored` never fails for lack of a database (mirrors the skip-clean
//! pattern in `scenario_traversals_integration.rs`).
//!
//! ## Why the subject id is DISCOVERED, not hardcoded
//!
//! `scenario_traversals_integration.rs` asserts against *observed* baseline ids
//! captured from dated DEV probes. This suite has no such captured baseline for
//! an ABOUT-subject, and inventing one would be a fabricated fixture (the exact
//! thing that file warns against). Instead these tests DISCOVER a real subject
//! at run time — `first_subject_with_evidence` asks the graph for any subject
//! that has at least one Evidence ABOUT it — then assert structural invariants
//! that hold for ANY such subject. This keeps the suite case-agnostic (no Awad
//! literal, so another Colossus case's graph exercises it unchanged) and robust
//! to graph churn.

use colossus_legal_backend::bias::repository::BiasRepository;
use neo4rs::{query, Graph};

/// Connect to the test Neo4j, or `None` (with a skip notice) when
/// `NEO4J_TEST_URI` is unset. Read-only — safe against a populated graph.
///
/// Mirrors `scenario_traversals_integration.rs::test_graph` so the two live-graph
/// suites share one connection convention.
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

/// Discover any subject id that has at least one Evidence node ABOUT it.
///
/// Returns `None` when the graph has no `(:Evidence)-[:ABOUT]->()` edges at all
/// (an empty/unseeded graph) — the caller then skips clean rather than asserting
/// against data that isn't there. We take the subject with the MOST such edges
/// (`ORDER BY cnt DESC`) so the discovered subject is a substantive one, giving
/// the ungated read a non-trivial candidate set to return.
async fn first_subject_with_evidence(graph: &Graph) -> Option<String> {
    let cypher = "
        MATCH (e:Evidence)-[:ABOUT]->(s)
        WITH s, count(e) AS cnt
        RETURN s.id AS subject_id
        ORDER BY cnt DESC, s.id
        LIMIT 1
    ";
    let mut result = graph
        .execute(query(cypher))
        .await
        .expect("discover-subject query executes");
    let row = result.next().await.expect("stream discover-subject row")?;
    row.get::<String>("subject_id").ok()
}

/// A subject id that does not exist in any graph — used to prove the ungated
/// read returns an empty set (not an error) for an unknown subject. The `zzz`
/// tail and uuid-like shape make an accidental real-node collision implausible.
const NONEXISTENT_SUBJECT_ID: &str = "person-nonexistent-zzz-00000000-0000-0000-0000-000000000000";

/// An unknown subject must yield `Ok(vec![])` — an empty candidate set, NOT an
/// error and NOT a panic. This is the "no matches" observable the scan relies on
/// to distinguish "subject has no evidence" from "the query failed" (Standing
/// Rule 1). Fully deterministic: needs no seeded data, only a reachable graph.
#[tokio::test]
#[ignore]
async fn all_evidence_about_unknown_subject_is_empty_not_error() {
    let Some(graph) = test_graph("all_evidence_about_unknown_subject_is_empty_not_error").await
    else {
        return;
    };
    let repo = BiasRepository::new(graph);

    let instances = repo
        .all_evidence_about_subject(NONEXISTENT_SUBJECT_ID)
        .await
        .expect("all_evidence_about_subject must succeed (empty result) for an unknown subject");

    assert!(
        instances.is_empty(),
        "an unknown subject id must return an empty Vec, got {} instance(s)",
        instances.len()
    );
}

/// For a real subject, the ungated read returns a well-formed, in-scope,
/// stably-ordered candidate set. Asserts the invariants that must hold for ANY
/// discovered subject:
///
/// 1. **Non-empty** — the discovered subject has ≥1 Evidence ABOUT it (that is
///    how it was discovered), so the read must surface at least one candidate.
/// 2. **Decoded** — every instance carries a non-empty `evidence_id`, proving
///    the `BiasRow` decode + `AggregationState` collapse ran (not silently
///    dropped rows).
/// 3. **In scope** — every instance's `about` list contains the queried subject,
///    proving the `EXISTS { ... ABOUT ... }` scope filter is correct (we never
///    return evidence that isn't about this subject).
/// 4. **Stable order** — instances come back ordered by `evidence_id`, the
///    reproducibility guarantee D2b's re-scan/diff depends on.
///
/// Note it does NOT assert an exact count or specific ids: those depend on live
/// graph state and would be a fabricated baseline. The invariants above are
/// count-independent and case-agnostic.
#[tokio::test]
#[ignore]
async fn all_evidence_about_subject_returns_ungated_candidates() {
    let Some(graph) = test_graph("all_evidence_about_subject_returns_ungated_candidates").await
    else {
        return;
    };

    let Some(subject_id) = first_subject_with_evidence(&graph).await else {
        eprintln!("SKIP: graph has no (:Evidence)-[:ABOUT]->() edges to exercise");
        return;
    };

    let repo = BiasRepository::new(graph);
    let instances = repo
        .all_evidence_about_subject(&subject_id)
        .await
        .expect("all_evidence_about_subject query for a discovered subject");

    // 1. Non-empty.
    assert!(
        !instances.is_empty(),
        "subject {subject_id} was discovered via an ABOUT edge, so the ungated read must be non-empty"
    );

    // 2. Every row decoded (non-empty evidence_id).
    for inst in &instances {
        assert!(
            !inst.evidence_id.is_empty(),
            "every returned instance must carry a decoded evidence_id"
        );
    }

    // 3. Every row is in scope — its `about` list names the queried subject.
    for inst in &instances {
        assert!(
            inst.about.iter().any(|a| a.id == subject_id),
            "evidence {} must be ABOUT the queried subject {subject_id} (scope filter)",
            inst.evidence_id
        );
    }

    // 4. Stable order by evidence_id (reproducibility for re-scan/diff).
    let ids: Vec<&str> = instances.iter().map(|i| i.evidence_id.as_str()).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(
        ids, sorted,
        "instances must be ordered by evidence_id for reproducible scans"
    );
}
