//! Integration tests for the human-authored graph fact writer
//! (`neo4j::human_facts`).
//!
//! Every test here touches a live Neo4j, so each is `#[ignore]` and gated on
//! the `NEO4J_TEST_URI` env var. They MUST point at an **isolated** test
//! instance — the reset helper deletes every node this suite marks, and a node
//! write is non-idempotent by design, so re-runs accumulate unless reset. Run:
//!
//! ```text
//! NEO4J_TEST_URI=bolt://localhost:7687 \
//!   cargo test --test human_facts_integration -- --ignored --test-threads=1
//! ```
//!
//! Without `NEO4J_TEST_URI` each test prints a skip notice and returns success,
//! so `--ignored` never fails for lack of a database (mirrors the
//! `canonical_elements_loader_tests.rs` Tier-2 skip-clean pattern).
//!
//! ## What is asserted at which tier
//!
//! The pure validators and Cypher builders are unit-tested in-module (Tier 1,
//! always run). These tests assert the *behavioral graph state* that only a
//! live Neo4j can show: provenance/citation properties on real nodes, the
//! create-vs-merge node/edge counts, fail-loud on a missing endpoint, and the
//! atomicity of a node+edge unit (Err → drop the uncommitted txn → the node
//! never landed).

use colossus_legal_backend::neo4j::human_facts::{
    write_human_edge, write_human_fact, write_human_node, HumanFactError, HumanFactNode,
    HumanFactProperty, HumanFactRequest, OutgoingEdge, ScalarValue,
};
use colossus_legal_backend::neo4j::schema as graph_schema;
use neo4rs::{query, Graph};

// A property every test node carries so `reset` can scope its delete to this
// suite's nodes (never touching anything else in the test DB). Not a reserved
// name, so the writer accepts it.
const TEST_MARKER_KEY: &str = "hf_test_marker";
const TEST_MARKER_VAL: &str = "task-0.2";

/// Connect to the isolated test Neo4j, or `None` (with a skip notice) when
/// `NEO4J_TEST_URI` is unset.
async fn test_graph(test_name: &str) -> Option<Graph> {
    dotenvy::dotenv().ok();
    let Ok(uri) = std::env::var("NEO4J_TEST_URI") else {
        eprintln!("SKIP {test_name}: NEO4J_TEST_URI not set (isolated test DB required)");
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

/// Delete every node this suite created (matched by the test marker), detaching
/// any edges. Run at the start of each test for isolation under
/// `--test-threads=1`.
async fn reset(graph: &Graph) {
    // The property KEY is interpolated (Cypher can't parameterize a property
    // name); the VALUE is bound as a parameter. Both come from the consts so a
    // rename can't silently leave this query matching the wrong nodes.
    graph
        .run(
            query(&format!(
                "MATCH (n {{{TEST_MARKER_KEY}: $m}}) DETACH DELETE n"
            ))
            .param("m", TEST_MARKER_VAL),
        )
        .await
        .expect("reset test nodes");
}

/// Cypher fragment matching a test node of `label` carrying this suite's marker.
/// Both the marker key and value come from the consts so a count query can never
/// drift from what [`marked_node`] / [`reset`] write.
fn marked_match(label: &str) -> String {
    format!("MATCH (n:{label} {{{TEST_MARKER_KEY}: '{TEST_MARKER_VAL}'}})")
}

/// Run a single-`i64`-column query and return that value (column alias `n`).
async fn scalar(graph: &Graph, cypher: &str) -> i64 {
    let mut stream = graph
        .execute(query(cypher))
        .await
        .expect("run scalar query");
    let row = stream
        .next()
        .await
        .expect("scalar stream")
        .expect("one row");
    row.get("n").expect("i64 column 'n'")
}

/// A test node with the suite marker plus the given extra properties.
fn marked_node(label: &str, extra: Vec<HumanFactProperty>) -> HumanFactNode {
    let mut properties = vec![HumanFactProperty {
        name: TEST_MARKER_KEY.to_string(),
        value: ScalarValue::Text(TEST_MARKER_VAL.to_string()),
    }];
    properties.extend(extra);
    HumanFactNode {
        label: label.to_string(),
        properties,
    }
}

/// Write a single marked node in its own committed transaction; return its id.
async fn commit_node(graph: &Graph, node: &HumanFactNode) -> String {
    let mut txn = graph.start_txn().await.expect("start txn");
    let id = write_human_node(&mut txn, node).await.expect("write node");
    txn.commit().await.expect("commit");
    id
}

// ===========================================================================
// Test 1 — one node, provenance set, NO citation trail
// ===========================================================================

#[tokio::test]
#[ignore]
async fn it_writes_one_human_node_with_no_citation_trail() {
    let Some(graph) = test_graph("it_writes_one_human_node_with_no_citation_trail").await else {
        return;
    };
    reset(&graph).await;

    let node = marked_node(
        "HumanFactTestParty",
        vec![HumanFactProperty {
            name: "name".to_string(),
            value: ScalarValue::Text("Marie Awad".to_string()),
        }],
    );
    let id = commit_node(&graph, &node).await;

    // Exactly one node with this id.
    let count = scalar(
        &graph,
        &format!("MATCH (n {{id: '{id}'}}) RETURN count(n) AS n"),
    )
    .await;
    assert_eq!(count, 1, "exactly one node must be created");

    // Provenance + caller property present; citation trail absent.
    let mut stream = graph
        .execute(
            query(
                "MATCH (n {id: $id}) RETURN \
                 n.provenance AS prov, n.name AS name, \
                 n.source_document AS sd, n.verbatim_quote AS vq, \
                 n.grounding_status AS gs, n.page_number AS pn",
            )
            .param("id", id.as_str()),
        )
        .await
        .expect("read back node");
    let row = stream.next().await.expect("stream").expect("one row");

    assert_eq!(
        row.get::<String>("prov").expect("prov"),
        "human-authored",
        "node must carry provenance='human-authored'"
    );
    assert_eq!(row.get::<String>("name").expect("name"), "Marie Awad");
    // Each citation property must be absent (null → Option::None).
    assert!(row.get::<Option<String>>("sd").expect("sd").is_none());
    assert!(row.get::<Option<String>>("vq").expect("vq").is_none());
    assert!(row.get::<Option<String>>("gs").expect("gs").is_none());
    assert!(row.get::<Option<i64>>("pn").expect("pn").is_none());

    reset(&graph).await;
}

// ===========================================================================
// Test 2 — identical input twice ⇒ TWO nodes (intentional non-idempotency)
// ===========================================================================

#[tokio::test]
#[ignore]
async fn it_writes_two_nodes_for_identical_input() {
    let Some(graph) = test_graph("it_writes_two_nodes_for_identical_input").await else {
        return;
    };
    reset(&graph).await;

    let node = marked_node(
        "HumanFactTestParty",
        vec![HumanFactProperty {
            name: "name".to_string(),
            value: ScalarValue::Text("Marie Awad".to_string()),
        }],
    );
    let id_a = commit_node(&graph, &node).await;
    let id_b = commit_node(&graph, &node).await;

    assert_ne!(id_a, id_b, "each write must mint a distinct id");
    let count = scalar(
        &graph,
        &format!(
            "{} RETURN count(n) AS n",
            marked_match("HumanFactTestParty")
        ),
    )
    .await;
    assert_eq!(
        count, 2,
        "identical input written twice must produce two nodes"
    );

    reset(&graph).await;
}

// ===========================================================================
// Test 3 — edge write is idempotent (twice ⇒ one edge)
// ===========================================================================

#[tokio::test]
#[ignore]
async fn it_merges_edge_idempotently() {
    let Some(graph) = test_graph("it_merges_edge_idempotently").await else {
        return;
    };
    reset(&graph).await;

    let from = commit_node(&graph, &marked_node("HumanFactTestStatement", vec![])).await;
    let to = commit_node(&graph, &marked_node("HumanFactTestParty", vec![])).await;

    // Write the same edge twice, each in its own committed txn.
    for _ in 0..2 {
        let mut txn = graph.start_txn().await.expect("start txn");
        write_human_edge(&mut txn, &from, &to, graph_schema::STATED_BY)
            .await
            .expect("write edge");
        txn.commit().await.expect("commit");
    }

    let count = scalar(
        &graph,
        &format!(
            "MATCH (a {{id: '{from}'}})-[r:{rel}]->(b {{id: '{to}'}}) RETURN count(r) AS n",
            rel = graph_schema::STATED_BY
        ),
    )
    .await;
    assert_eq!(count, 1, "edge MERGE must be idempotent — one edge");

    reset(&graph).await;
}

// ===========================================================================
// Test 4 — edge to a non-existent endpoint ⇒ Err, nothing written
// ===========================================================================

#[tokio::test]
#[ignore]
async fn it_errs_and_writes_nothing_on_missing_endpoint() {
    let Some(graph) = test_graph("it_errs_and_writes_nothing_on_missing_endpoint").await else {
        return;
    };
    reset(&graph).await;

    let from = commit_node(&graph, &marked_node("HumanFactTestStatement", vec![])).await;
    let missing = "human-fact-test-nonexistent-id";

    let mut txn = graph.start_txn().await.expect("start txn");
    let result = write_human_edge(&mut txn, &from, missing, graph_schema::STATED_BY).await;
    match result {
        Err(HumanFactError::EndpointNotFound { from_id, to_id }) => {
            assert_eq!(from_id, from);
            assert_eq!(to_id, missing);
        }
        other => panic!("expected EndpointNotFound, got: {other:?}"),
    }
    // Even if we committed, no edge should exist (the MERGE matched nothing).
    txn.commit().await.expect("commit");

    let count = scalar(
        &graph,
        &format!(
            "MATCH (a {{id: '{from}'}})-[r:{rel}]->() RETURN count(r) AS n",
            rel = graph_schema::STATED_BY
        ),
    )
    .await;
    assert_eq!(count, 0, "no edge may be written to a missing endpoint");

    reset(&graph).await;
}

// ===========================================================================
// Test 5 — node+edge unit with a missing endpoint leaves NO node (atomicity)
// ===========================================================================

#[tokio::test]
#[ignore]
async fn it_leaves_no_node_when_unit_edge_endpoint_missing() {
    let Some(graph) = test_graph("it_leaves_no_node_when_unit_edge_endpoint_missing").await else {
        return;
    };
    reset(&graph).await;

    let baseline = scalar(
        &graph,
        &format!(
            "{} RETURN count(n) AS n",
            marked_match("HumanFactTestStatement")
        ),
    )
    .await;

    let request = HumanFactRequest {
        node: marked_node("HumanFactTestStatement", vec![]),
        edges: vec![OutgoingEdge {
            rel_type: graph_schema::STATED_BY.to_string(),
            to_id: "human-fact-test-nonexistent-id".to_string(),
        }],
    };

    let mut txn = graph.start_txn().await.expect("start txn");
    let result = write_human_fact(&mut txn, &request).await;
    assert!(
        matches!(result, Err(HumanFactError::EndpointNotFound { .. })),
        "unit with a missing edge endpoint must fail, got: {result:?}"
    );
    // The caller owns the commit: on Err we DROP the txn without committing,
    // discarding the node CREATE that ran before the failing edge.
    drop(txn);

    let after = scalar(
        &graph,
        &format!(
            "{} RETURN count(n) AS n",
            marked_match("HumanFactTestStatement")
        ),
    )
    .await;
    assert_eq!(
        after, baseline,
        "the freshly-created node must NOT persist when its edge fails (atomicity)"
    );

    reset(&graph).await;
}

// ===========================================================================
// Test 6 — write_human_fact happy path: node + edge committed, returned id real
// ===========================================================================

#[tokio::test]
#[ignore]
async fn it_writes_a_node_and_edge_unit_and_returns_the_real_id() {
    let Some(graph) = test_graph("it_writes_a_node_and_edge_unit_and_returns_the_real_id").await
    else {
        return;
    };
    reset(&graph).await;

    // An existing target the new fact's edge will point at.
    let target = commit_node(&graph, &marked_node("HumanFactTestParty", vec![])).await;

    let request = HumanFactRequest {
        node: marked_node(
            "HumanFactTestStatement",
            vec![HumanFactProperty {
                name: "text".to_string(),
                value: ScalarValue::Text("He admitted it under oath".to_string()),
            }],
        ),
        edges: vec![OutgoingEdge {
            rel_type: graph_schema::STATED_BY.to_string(),
            to_id: target.clone(),
        }],
    };

    let mut txn = graph.start_txn().await.expect("start txn");
    let node_id = write_human_fact(&mut txn, &request)
        .await
        .expect("write fact");
    txn.commit().await.expect("commit");

    // (a) the returned id names exactly one node, with the human-authored mark.
    let node_count = scalar(
        &graph,
        &format!(
            "MATCH (n {{id: '{node_id}'}}) WHERE n.provenance = '{prov}' RETURN count(n) AS n",
            prov = colossus_legal_backend::neo4j::human_facts::PROVENANCE_HUMAN_AUTHORED
        ),
    )
    .await;
    assert_eq!(
        node_count, 1,
        "write_human_fact must commit exactly one human-authored node for the returned id"
    );

    // (b) the edge exists from the new node to the target, human-authored.
    let edge_count = scalar(
        &graph,
        &format!(
            "MATCH (a {{id: '{node_id}'}})-[r:{rel}]->(b {{id: '{target}'}}) \
             WHERE r.provenance = '{prov}' RETURN count(r) AS n",
            rel = graph_schema::STATED_BY,
            prov = colossus_legal_backend::neo4j::human_facts::PROVENANCE_HUMAN_AUTHORED
        ),
    )
    .await;
    assert_eq!(
        edge_count, 1,
        "the unit's edge must be committed and human-authored"
    );

    reset(&graph).await;
}
