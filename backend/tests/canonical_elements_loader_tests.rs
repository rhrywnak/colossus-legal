//! Integration tests for the canonical Element loader.
//!
//! Two tiers:
//! - **Tests 1–3 (parse / validate):** no database; run in a normal
//!   `cargo test`.
//! - **Tests 4–9 (database-touching):** marked `#[ignore]` and gated on the
//!   `NEO4J_TEST_URI` env var. They run the *real* loader, whose orphan wipe is
//!   global — so they must NEVER point at shared DEV Neo4j. Run them against an
//!   isolated instance with:
//!
//!   ```text
//!   NEO4J_TEST_URI=bolt://localhost:7687 \
//!     cargo test --test canonical_elements_loader_tests -- --ignored --test-threads=1
//!   ```
//!
//!   Without `NEO4J_TEST_URI` each ignored test prints a skip notice and
//!   returns success, so `--ignored` never fails for lack of a database.

use colossus_legal_backend::canonical_elements::loader::{self, RunOptions};
use colossus_legal_backend::canonical_elements::plan::ChangeKind;
use colossus_legal_backend::canonical_elements::schema::CountFile;
use colossus_legal_backend::canonical_elements::CanonicalLoaderError;
use colossus_legal_backend::repositories::pipeline_repository::{
    delete_authored_entities_for_case, delete_authored_relationships_by_type,
    list_authored_entities,
};
use neo4rs::{query, Graph};
use sqlx::PgPool;
use std::path::{Path, PathBuf};

// ===========================================================================
// Tier 1 — parse / validate (no database)
// ===========================================================================

fn production_yaml_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("canonical_elements")
}

/// A minimal but schema-valid single-Count YAML, parameterized by count number
/// and one Element id. Optional fields are omitted to prove they default.
fn minimal_count_yaml(count_number: u32, element_id: &str) -> String {
    format!(
        "count:\n  \
           count_number: {count_number}\n  \
           count_name: \"Test Count {count_number}\"\n  \
           template_name: \"test_template\"\n  \
           burden_of_proof: \"preponderance\"\n  \
           controlling_authorities: []\n\
         elements:\n  \
           - id: \"{element_id}\"\n    \
             order_in_count: 1\n    \
             element_name: \"An element\"\n    \
             title: \"An element\"\n    \
             what_plaintiff_must_prove: \"prove the thing\"\n    \
             controlling_authority: \"Some v Authority\"\n"
    )
}

#[test]
fn parses_all_four_production_yaml_files() {
    let dir = production_yaml_dir();
    let empty = std::fs::read_dir(&dir)
        .map(|mut d| d.next().is_none())
        .unwrap_or(true);
    if empty {
        eprintln!(
            "SKIP parses_all_four_production_yaml_files: {} absent or empty \
             (Roman commits the canonical YAML files separately)",
            dir.display()
        );
        return;
    }

    let files = loader::read_count_files(&dir).expect("production YAMLs parse");
    assert_eq!(files.len(), 4, "expected four Count files");

    let count_numbers: Vec<u32> = files.iter().map(|f| f.count.count_number).collect();
    assert_eq!(count_numbers, vec![1, 2, 3, 4], "sorted by count_number");

    let elements_in = |n: u32| {
        files
            .iter()
            .find(|f| f.count.count_number == n)
            .map(|f| f.elements.len())
            .unwrap()
    };
    assert_eq!(elements_in(1), 3, "Count I element count");
    assert_eq!(elements_in(2), 11, "Count II element count");
    assert_eq!(elements_in(3), 4, "Count III element count");
    assert_eq!(elements_in(4), 2, "Count IV element count");

    // Count III carries three declarations (two operative, one historical).
    let c3 = files.iter().find(|f| f.count.count_number == 3).unwrap();
    assert_eq!(c3.declarations_sought.len(), 3);
    assert_eq!(
        c3.declarations_sought
            .iter()
            .filter(|d| d.operative)
            .count(),
        2
    );
    // Count IV carries doctrinal requirements nested under `count`.
    let c4 = files.iter().find(|f| f.count.count_number == 4).unwrap();
    assert_eq!(c4.count.doctrinal_requirements.len(), 3);

    loader::validate(&files).expect("production YAMLs pass cross-file validation");
}

#[test]
fn rejects_unknown_fields() {
    let dir = tempfile::tempdir().unwrap();
    // Schema-valid except for a stray top-level key.
    let body = format!(
        "{}\nbogus_unexpected_field: true\n",
        minimal_count_yaml(1, "element-1-1")
    );
    std::fs::write(dir.path().join("count_1.yaml"), body).unwrap();

    let err = loader::read_count_files(dir.path()).expect_err("unknown field must be rejected");
    assert!(
        matches!(err, CanonicalLoaderError::Parse { .. }),
        "expected Parse error, got {err:?}"
    );
}

#[test]
fn rejects_duplicate_element_ids() {
    // Two distinct Counts that reuse the same Element id.
    let a: CountFile = serde_yaml::from_str(&minimal_count_yaml(1, "element-dup")).unwrap();
    let b: CountFile = serde_yaml::from_str(&minimal_count_yaml(2, "element-dup")).unwrap();

    let err = loader::validate(&[a, b]).expect_err("duplicate Element id must fail validation");
    match err {
        CanonicalLoaderError::Validation(msg) => {
            assert!(msg.contains("Duplicate Element id"), "message was: {msg}");
        }
        other => panic!("expected Validation error, got {other:?}"),
    }
}

#[test]
fn rejects_directory_with_no_yaml_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("readme.txt"), "not yaml").unwrap();
    match loader::read_count_files(dir.path()).expect_err("empty-of-yaml dir must fail") {
        CanonicalLoaderError::Validation(msg) => {
            assert!(msg.contains("No .yaml files found"), "{msg}")
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[test]
fn rejects_duplicate_count_numbers() {
    let a: CountFile = serde_yaml::from_str(&minimal_count_yaml(1, "element-1-1")).unwrap();
    let b: CountFile = serde_yaml::from_str(&minimal_count_yaml(1, "element-9-9")).unwrap();
    match loader::validate(&[a, b]).expect_err("duplicate count_number must fail") {
        CanonicalLoaderError::Validation(msg) => {
            assert!(msg.contains("Duplicate count_number"), "{msg}")
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[test]
fn rejects_duplicate_theory_keys_within_a_count() {
    let yaml = format!(
        "{}breach_theories:\n  \
           - key: \"loyalty\"\n    definition: \"d\"\n    examples: \"e\"\n  \
           - key: \"loyalty\"\n    definition: \"d2\"\n    examples: \"e2\"\n",
        minimal_count_yaml(1, "element-1-1")
    );
    let f: CountFile = serde_yaml::from_str(&yaml).unwrap();
    match loader::validate(&[f]).expect_err("duplicate theory key must fail") {
        CanonicalLoaderError::Validation(msg) => {
            assert!(
                msg.contains("breach theory key") && msg.contains("loyalty"),
                "{msg}"
            );
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[test]
fn rejects_duplicate_declaration_ids_within_a_count() {
    let yaml = format!(
        "{}declarations_sought:\n  \
           - id: \"declaration-3-a\"\n    declaration: \"d\"\n    legal_basis: \"b\"\n    operative: true\n  \
           - id: \"declaration-3-a\"\n    declaration: \"d2\"\n    legal_basis: \"b2\"\n    operative: false\n",
        minimal_count_yaml(3, "element-3-1")
    );
    let f: CountFile = serde_yaml::from_str(&yaml).unwrap();
    match loader::validate(&[f]).expect_err("duplicate declaration id must fail") {
        CanonicalLoaderError::Validation(msg) => assert!(msg.contains("declaration id"), "{msg}"),
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[test]
fn exit_codes_map_error_categories() {
    use CanonicalLoaderError as E;
    // Code 1 — input/parse problems.
    assert_eq!(
        E::MissingEnv {
            key: "NEO4J_URI".into()
        }
        .exit_code(),
        1
    );
    let parse_err = serde_yaml::from_str::<i32>("[not, an, int]").unwrap_err();
    assert_eq!(
        E::Parse {
            path: "x.yaml".into(),
            source: parse_err
        }
        .exit_code(),
        1
    );
    assert_eq!(
        E::FileRead {
            path: "x".into(),
            source: std::io::Error::other("boom")
        }
        .exit_code(),
        1
    );
    // Code 4 — validation / missing prerequisite.
    assert_eq!(E::Validation("dup".into()).exit_code(), 4);
    assert_eq!(E::MissingLegalCount { count_number: 2 }.exit_code(), 4);
    // Code 5 — Postgres write failure.
    assert_eq!(
        E::Postgres {
            operation: "upsert LegalCount count-1".into(),
            message: "connection refused".into(),
        }
        .exit_code(),
        5
    );
    // Codes 2 (Connection) and 3 (Cypher/RowDecode) require a live neo4rs error
    // to construct; they are exercised on the DB-test path.
}

#[test]
fn error_display_messages_are_operator_friendly() {
    let missing = CanonicalLoaderError::MissingLegalCount { count_number: 3 }.to_string();
    assert!(missing.contains("count_number=3"), "{missing}");
    assert!(missing.contains("case-structuring pipeline"), "{missing}");
    assert!(CanonicalLoaderError::MissingEnv {
        key: "NEO4J_URI".into()
    }
    .to_string()
    .contains("NEO4J_URI"));
    assert!(CanonicalLoaderError::Validation("dup id".into())
        .to_string()
        .contains("dup id"));
    // Postgres errors name both the operation (WHERE) and the underlying
    // message (WHY) so the failing step is locatable in the logs.
    let pg = CanonicalLoaderError::Postgres {
        operation: "upsert Element element-2-3".into(),
        message: "duplicate key value violates unique constraint".into(),
    }
    .to_string();
    assert!(pg.contains("upsert Element element-2-3"), "{pg}");
    assert!(pg.contains("duplicate key value"), "{pg}");
}

#[test]
fn neo4j_config_from_env_reads_vars_and_reports_missing() {
    use colossus_legal_backend::canonical_elements::Neo4jConfig;
    // Save originals so this test can't leak env state into others.
    let keys = ["NEO4J_URI", "NEO4J_USER", "NEO4J_PASSWORD"];
    let saved: Vec<(&str, Option<String>)> =
        keys.iter().map(|k| (*k, std::env::var(k).ok())).collect();

    std::env::set_var("NEO4J_URI", "bolt://example:7687");
    std::env::set_var("NEO4J_USER", "neo4j");
    std::env::set_var("NEO4J_PASSWORD", "secret");
    let cfg = Neo4jConfig::from_env().expect("all three present");
    assert_eq!(cfg.uri, "bolt://example:7687");
    assert_eq!(cfg.user, "neo4j");
    assert_eq!(cfg.password, "secret");

    std::env::remove_var("NEO4J_USER");
    match Neo4jConfig::from_env() {
        Err(CanonicalLoaderError::MissingEnv { key }) => assert_eq!(key, "NEO4J_USER"),
        other => panic!("expected MissingEnv(NEO4J_USER), got {other:?}"),
    }

    for (k, v) in saved {
        match v {
            Some(val) => std::env::set_var(k, val),
            None => std::env::remove_var(k),
        }
    }
}

// ===========================================================================
// Tier 2 — database-touching (ignored unless NEO4J_TEST_URI is set)
// ===========================================================================

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

fn opts(dir: &Path, dry_run: bool) -> RunOptions {
    RunOptions {
        yaml_dir: dir.to_path_buf(),
        dry_run,
        no_color: true,
        // These tests exercise the Neo4j path only; the Tier-1 Postgres
        // writes are covered by canonical_loader_postgres_integration.rs.
        pipeline_pool: None,
        case_slug: None,
    }
}

/// Wipe the isolated test DB and seed the four prerequisite LegalCounts.
async fn reset_and_seed(graph: &Graph) {
    graph.run(query("MATCH (n) DETACH DELETE n")).await.unwrap();
    for n in 1..=4i64 {
        graph
            .run(
                query("CREATE (c:LegalCount {count_number: $n, title: $title})")
                    .param("n", n)
                    .param("title", format!("Count {n}")),
            )
            .await
            .unwrap();
    }
}

async fn scalar(graph: &Graph, cypher: &str) -> i64 {
    let mut stream = graph.execute(query(cypher)).await.unwrap();
    let row = stream.next().await.unwrap().expect("one row");
    row.get("n").unwrap()
}

/// Connect to the pipeline test database, or `None` (with a skip notice) when
/// `PIPELINE_DATABASE_URL` is unset.
async fn test_pipeline_pool(test_name: &str) -> Option<PgPool> {
    dotenvy::dotenv().ok();
    let Ok(url) = std::env::var("PIPELINE_DATABASE_URL") else {
        eprintln!("SKIP {test_name}: PIPELINE_DATABASE_URL not set");
        return None;
    };
    Some(
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .expect("connect to PIPELINE_DATABASE_URL"),
    )
}

/// Dry-run must write to neither Postgres nor Neo4j, even when a pool and
/// case-slug are supplied. Requires both isolated test DBs.
#[tokio::test]
#[ignore = "requires NEO4J_TEST_URI + PIPELINE_DATABASE_URL (isolated test DBs)"]
async fn dry_run_writes_neither_neo4j_nor_postgres() {
    let name = "dry_run_writes_neither_neo4j_nor_postgres";
    let Some(graph) = test_graph(name).await else {
        return;
    };
    let Some(pool) = test_pipeline_pool(name).await else {
        return;
    };
    let slug = "awad_v_catholic_family_service__test_loader_dryrun";

    // Clean PG slate, seed the prerequisite Neo4j LegalCounts.
    delete_authored_relationships_by_type(&pool, slug, "HAS_ELEMENT")
        .await
        .unwrap();
    delete_authored_entities_for_case(&pool, slug)
        .await
        .unwrap();
    reset_and_seed(&graph).await;
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());

    let _report = loader::run(
        &graph,
        RunOptions {
            yaml_dir: dir.path().to_path_buf(),
            dry_run: true,
            no_color: true,
            pipeline_pool: Some(pool.clone()),
            case_slug: Some(slug.to_string()),
        },
    )
    .await
    .unwrap();

    assert!(
        list_authored_entities(&pool, slug, None)
            .await
            .unwrap()
            .is_empty(),
        "dry-run must not write authored_entities"
    );
    assert_eq!(
        scalar(&graph, "MATCH (:Element) RETURN count(*) AS n").await,
        0,
        "dry-run must not write Neo4j Elements"
    );

    delete_authored_entities_for_case(&pool, slug)
        .await
        .unwrap();
}

/// `set_legal_count_id` stamps the cross-tier `id` (`count-{N}`) on every
/// LegalCount node, unconditionally and idempotently. Neo4j-only (no PG).
#[tokio::test]
#[ignore = "requires NEO4J_TEST_URI (isolated test DB)"]
async fn legalcount_nodes_get_cross_tier_count_id() {
    let name = "legalcount_nodes_get_cross_tier_count_id";
    let Some(graph) = test_graph(name).await else {
        return;
    };
    reset_and_seed(&graph).await;
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());

    // First run stamps c.id = count-{N} on every LegalCount.
    loader::run(&graph, opts(dir.path(), false)).await.unwrap();
    assert_eq!(
        scalar(
            &graph,
            "MATCH (c:LegalCount {count_number: 1}) WHERE c.id = 'count-1' RETURN count(c) AS n"
        )
        .await,
        1,
        "LegalCount 1 must carry cross-tier id count-1"
    );
    assert_eq!(
        scalar(
            &graph,
            "MATCH (c:LegalCount) WHERE c.id STARTS WITH 'count-' RETURN count(c) AS n"
        )
        .await,
        4,
        "all four LegalCounts carry a count-N id"
    );

    // Second run: the id is set unconditionally, even though no managed
    // property changed (content-hash idempotency skips the property update).
    loader::run(&graph, opts(dir.path(), false)).await.unwrap();
    assert_eq!(
        scalar(
            &graph,
            "MATCH (c:LegalCount {count_number: 4}) WHERE c.id = 'count-4' RETURN count(c) AS n"
        )
        .await,
        1,
        "id persists on the idempotent re-run"
    );
}

/// Write a four-Count fixture exercising every node type and property kind.
fn write_fixtures(dir: &Path) {
    std::fs::write(dir.join("count_1.yaml"), FIXTURE_COUNT_1).unwrap();
    std::fs::write(dir.join("count_2.yaml"), FIXTURE_COUNT_2).unwrap();
    std::fs::write(dir.join("count_3.yaml"), FIXTURE_COUNT_3).unwrap();
    std::fs::write(dir.join("count_4.yaml"), FIXTURE_COUNT_4).unwrap();
}

const FIXTURE_COUNT_1: &str = r#"
count:
  count_number: 1
  count_name: "Breach"
  template_name: "breach_michigan"
  burden_of_proof: "preponderance"
  controlling_authorities:
    - citation: "Some v Case"
      authority_type: "case"
      year: 2020
      role: "test"
elements:
  - id: "element-1-1"
    order_in_count: 1
    element_name: "Duty"
    title: "Duty"
    what_plaintiff_must_prove: "duty existed"
    controlling_authority: "Some v Case"
  - id: "element-1-2"
    order_in_count: 2
    element_name: "Breach"
    title: "Breach"
    what_plaintiff_must_prove: "duty breached"
    controlling_authority: "Some v Case"
  - id: "element-1-3"
    order_in_count: 3
    element_name: "Damages"
    title: "Damages"
    what_plaintiff_must_prove: "damages caused"
    controlling_authority: "Some v Case"
breach_theories:
  - key: "loyalty"
    definition: "self-interest over beneficiary"
    statutory_anchor: "MCL 700.1212(1)"
    examples: "example"
"#;

const FIXTURE_COUNT_2: &str = r#"
count:
  count_number: 2
  count_name: "Fraud"
  template_name: "fraud_michigan"
  burden_of_proof: "clear_and_convincing"
  m_civ_ji_reference: "M Civ JI 128.01"
  chuck_review_required: true
  chuck_review_note: "confirm dual theory"
  controlling_authorities:
    - citation: "M Civ JI 128.01"
      authority_type: "jury_instruction"
      role: "six-element test"
elements:
  - id: "element-2-1"
    order_in_count: 1
    element_name: "Duty to disclose"
    title: "Duty to disclose"
    theory_variant: "silent_fraud"
    what_plaintiff_must_prove: "duty to disclose"
    controlling_authority: "M Civ JI 128.02"
  - id: "element-2-6"
    order_in_count: 6
    element_name: "Material representation"
    title: "Material representation"
    theory_variant: "common_law_fraud"
    what_plaintiff_must_prove: "a representation"
    controlling_authority: "M Civ JI 128.01"
"#;

const FIXTURE_COUNT_3: &str = r#"
count:
  count_number: 3
  count_name: "Declaratory Relief"
  template_name: "declaratory_relief_michigan"
  burden_of_proof: "preponderance"
  special_note: "Elements are jurisdictional prerequisites, not tort elements."
  controlling_authorities:
    - citation: "MCR 2.605(A)(1)"
      authority_type: "court_rule"
      role: "actual controversy"
elements:
  - id: "element-3-1"
    order_in_count: 1
    element_name: "Actual controversy"
    title: "Actual controversy"
    what_plaintiff_must_prove: "an actual controversy"
    controlling_authority: "MCR 2.605(A)(1)"
declarations_sought:
  - id: "declaration-3-a"
    declaration: "CFS acted ultra vires"
    legal_basis: "articles of incorporation"
    operative: true
  - id: "declaration-3-c"
    declaration: "Detroit failed to supervise"
    legal_basis: "supervisory liability"
    operative: false
    inoperative_reason: "Detroit dismissed"
"#;

const FIXTURE_COUNT_4: &str = r#"
count:
  count_number: 4
  count_name: "Abuse of Process"
  template_name: "abuse_of_process_michigan"
  burden_of_proof: "preponderance"
  controlling_authorities:
    - citation: "Friedman v Dozorc"
      authority_type: "case"
      year: 1981
      role: "two-element test"
  doctrinal_requirements:
    - requirement: "specificity"
      description: "must plead specific acts"
      satisfied_in_case: true
      satisfaction_evidence: "complaint paragraphs"
elements:
  - id: "element-4-1"
    order_in_count: 1
    element_name: "Ulterior purpose"
    title: "Ulterior purpose"
    what_plaintiff_must_prove: "ulterior purpose"
    controlling_authority: "Friedman v Dozorc"
improper_act_theories:
  - key: "false_statement_to_court"
    definition: "misleading statement to tribunal"
    examples: "example"
"#;

/// Total Element nodes across the fixture set (3 + 2 + 1 + 1).
const FIXTURE_ELEMENT_TOTAL: i64 = 7;

#[tokio::test]
#[ignore = "requires NEO4J_TEST_URI (isolated test DB)"]
async fn first_run_wipes_orphan_elements_and_their_edges() {
    let Some(graph) = test_graph("first_run_wipes_orphan_elements_and_their_edges").await else {
        return;
    };
    reset_and_seed(&graph).await;

    // Seed 5 wrong Elements on Count 1, each with 2 incoming PROVES_ELEMENT edges.
    for i in 0..5 {
        let eid = format!("element-wrong-{i}");
        graph
            .run(
                query("MATCH (c:LegalCount {count_number: 1}) CREATE (c)-[:HAS_ELEMENT]->(:Element {id: $eid})")
                    .param("eid", eid.clone()),
            )
            .await
            .unwrap();
        for j in 0..2 {
            graph
                .run(
                    query("MATCH (e:Element {id: $eid}) CREATE (:Allegation {id: $aid})-[:PROVES_ELEMENT]->(e)")
                        .param("eid", eid.clone())
                        .param("aid", format!("alleg-{i}-{j}")),
                )
                .await
                .unwrap();
        }
    }

    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());
    let report = loader::run(&graph, opts(dir.path(), false)).await.unwrap();

    // Wrong Elements and their edges are gone; only canonical Elements remain.
    assert_eq!(
        scalar(
            &graph,
            "MATCH (e:Element) WHERE e.id STARTS WITH 'element-wrong' RETURN count(e) AS n"
        )
        .await,
        0
    );
    assert_eq!(
        scalar(&graph, "MATCH (:Element) RETURN count(*) AS n").await,
        FIXTURE_ELEMENT_TOTAL
    );
    assert_eq!(
        scalar(
            &graph,
            "MATCH ()-[r:PROVES_ELEMENT]->() RETURN count(r) AS n"
        )
        .await,
        0
    );

    // The report attributed the 5 orphans (and 10 edges) to Count 1.
    let c1 = report
        .plan()
        .counts
        .iter()
        .find(|c| c.meta.count_number == 1)
        .unwrap();
    assert_eq!(c1.orphan_elements, 5);
    assert_eq!(c1.orphan_proves_edges, 10);
}

#[tokio::test]
#[ignore = "requires NEO4J_TEST_URI (isolated test DB)"]
async fn second_run_is_idempotent() {
    let Some(graph) = test_graph("second_run_is_idempotent").await else {
        return;
    };
    reset_and_seed(&graph).await;
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());

    loader::run(&graph, opts(dir.path(), false)).await.unwrap();
    let second = loader::run(&graph, opts(dir.path(), false)).await.unwrap();

    for c in &second.plan().counts {
        assert!(
            c.changed_legal_count_props.is_empty(),
            "Count {} props changed on second run: {:?}",
            c.meta.count_number,
            c.changed_legal_count_props
        );
        let all_unchanged =
            |kinds: Vec<ChangeKind>| kinds.iter().all(|k| *k == ChangeKind::Unchanged);
        assert!(all_unchanged(c.elements.iter().map(|n| n.kind).collect()));
        assert!(all_unchanged(
            c.breach_theories.iter().map(|n| n.kind).collect()
        ));
        assert!(all_unchanged(
            c.improper_act_theories.iter().map(|n| n.kind).collect()
        ));
        assert!(all_unchanged(
            c.declarations.iter().map(|n| n.kind).collect()
        ));
        assert_eq!(c.orphan_elements, 0);
    }
}

#[tokio::test]
#[ignore = "requires NEO4J_TEST_URI (isolated test DB)"]
async fn dry_run_writes_nothing() {
    let Some(graph) = test_graph("dry_run_writes_nothing").await else {
        return;
    };
    reset_and_seed(&graph).await;
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());

    let report = loader::run(&graph, opts(dir.path(), true)).await.unwrap();

    // Plan still shows the would-be creations...
    let total_created: usize = report
        .plan()
        .counts
        .iter()
        .map(|c| {
            c.elements
                .iter()
                .filter(|e| e.kind == ChangeKind::Created)
                .count()
        })
        .sum();
    assert_eq!(total_created, FIXTURE_ELEMENT_TOTAL as usize);
    // ...but nothing was actually written.
    assert_eq!(
        scalar(&graph, "MATCH (:Element) RETURN count(*) AS n").await,
        0
    );
    assert_eq!(
        scalar(&graph, "MATCH (:DeclarationSought) RETURN count(*) AS n").await,
        0
    );
}

#[tokio::test]
#[ignore = "requires NEO4J_TEST_URI (isolated test DB)"]
async fn count_three_declarations_carry_operative_flag() {
    let Some(graph) = test_graph("count_three_declarations_carry_operative_flag").await else {
        return;
    };
    reset_and_seed(&graph).await;
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());
    loader::run(&graph, opts(dir.path(), false)).await.unwrap();

    assert_eq!(
        scalar(&graph, "MATCH (:DeclarationSought) RETURN count(*) AS n").await,
        2
    );
    assert_eq!(
        scalar(
            &graph,
            "MATCH (d:DeclarationSought) WHERE d.operative = true RETURN count(d) AS n"
        )
        .await,
        1
    );
    assert_eq!(
        scalar(
            &graph,
            "MATCH (d:DeclarationSought) WHERE d.operative = false RETURN count(d) AS n"
        )
        .await,
        1
    );
    // The non-operative one preserves its reason.
    assert_eq!(
        scalar(
            &graph,
            "MATCH (d:DeclarationSought {id:'declaration-3-c'}) \
             WHERE d.inoperative_reason = 'Detroit dismissed' RETURN count(d) AS n"
        )
        .await,
        1
    );
    // And it is attached to LegalCount 3 via SEEKS_DECLARATION.
    assert_eq!(
        scalar(&graph, "MATCH (:LegalCount {count_number:3})-[:SEEKS_DECLARATION]->(:DeclarationSought) RETURN count(*) AS n").await,
        2
    );
}

#[tokio::test]
#[ignore = "requires NEO4J_TEST_URI (isolated test DB)"]
async fn count_four_doctrinal_requirements_stored_as_json() {
    let Some(graph) = test_graph("count_four_doctrinal_requirements_stored_as_json").await else {
        return;
    };
    reset_and_seed(&graph).await;
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());
    loader::run(&graph, opts(dir.path(), false)).await.unwrap();

    let mut stream = graph
        .execute(query(
            "MATCH (c:LegalCount {count_number:4}) RETURN c.doctrinal_requirements_json AS j",
        ))
        .await
        .unwrap();
    let row = stream.next().await.unwrap().unwrap();
    let json: String = row
        .get("j")
        .expect("doctrinal_requirements_json present on Count 4");
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 1);
    assert_eq!(parsed[0]["requirement"], "specificity");

    // Counts without doctrinal requirements have no such property (null).
    let mut s2 = graph
        .execute(query(
            "MATCH (c:LegalCount {count_number:1}) RETURN c.doctrinal_requirements_json AS j",
        ))
        .await
        .unwrap();
    let row2 = s2.next().await.unwrap().unwrap();
    let j1: Option<String> = row2.get("j").unwrap();
    assert!(
        j1.is_none(),
        "Count 1 should have no doctrinal_requirements_json"
    );
}

#[tokio::test]
#[ignore = "requires NEO4J_TEST_URI (isolated test DB)"]
async fn update_flow_touches_only_the_changed_node() {
    let Some(graph) = test_graph("update_flow_touches_only_the_changed_node").await else {
        return;
    };
    reset_and_seed(&graph).await;
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());
    loader::run(&graph, opts(dir.path(), false)).await.unwrap();

    // Change one Element's title in Count 1, leave everything else identical.
    let edited = FIXTURE_COUNT_1.replace("title: \"Duty\"", "title: \"Duty (revised)\"");
    std::fs::write(dir.path().join("count_1.yaml"), edited).unwrap();

    let report = loader::run(&graph, opts(dir.path(), false)).await.unwrap();
    let c1 = report
        .plan()
        .counts
        .iter()
        .find(|c| c.meta.count_number == 1)
        .unwrap();

    let updated: Vec<&str> = c1
        .elements
        .iter()
        .filter(|e| e.kind == ChangeKind::Updated)
        .map(|e| e.def.id.as_str())
        .collect();
    assert_eq!(
        updated,
        vec!["element-1-1"],
        "only the edited Element updates"
    );

    // Every other Count is fully unchanged.
    for c in report
        .plan()
        .counts
        .iter()
        .filter(|c| c.meta.count_number != 1)
    {
        assert!(c.changed_legal_count_props.is_empty());
        assert!(c.elements.iter().all(|e| e.kind == ChangeKind::Unchanged));
    }
    // The new title is persisted.
    assert_eq!(
        scalar(
            &graph,
            "MATCH (e:Element {id:'element-1-1'}) WHERE e.title = 'Duty (revised)' RETURN count(e) AS n"
        )
        .await,
        1
    );
}

#[tokio::test]
#[ignore = "requires NEO4J_TEST_URI (isolated test DB)"]
async fn missing_legal_count_is_a_hard_error() {
    let Some(graph) = test_graph("missing_legal_count_is_a_hard_error").await else {
        return;
    };
    // Seed only Counts 1-3; the Count 4 fixture has no LegalCount to attach to.
    graph.run(query("MATCH (n) DETACH DELETE n")).await.unwrap();
    for n in 1..=3i64 {
        graph
            .run(
                query("CREATE (c:LegalCount {count_number: $n, title: $title})")
                    .param("n", n)
                    .param("title", format!("Count {n}")),
            )
            .await
            .unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());

    match loader::run(&graph, opts(dir.path(), false)).await {
        Err(CanonicalLoaderError::MissingLegalCount { count_number }) => {
            assert_eq!(count_number, 4)
        }
        other => panic!("expected MissingLegalCount(4), got {other:?}"),
    }
}
