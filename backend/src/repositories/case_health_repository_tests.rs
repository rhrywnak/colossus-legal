//! Query-shape tests for [`super`] — the Case Health Neo4j reads.
//!
//! The `fetch_*` functions need a live `neo4rs::Graph` and are exercised by DEV
//! verification, exactly as `proof_matrix_repository` and
//! `causes_of_action_repository` are (a `neo4rs::Row` cannot be constructed off a
//! live driver). What CAN be pinned here — and is — is the Cypher itself: a
//! clean compile says nothing about whether the headline still excludes ABOUT,
//! whether the label list still avoids `db.labels()`, or whether a property was
//! renamed to one that never existed. Those are the regressions this file
//! catches.

use super::*;

// ── Cypher fragment helpers ───────────────────────────────────────────────────

#[test]
fn class_alias_lowercases_the_relationship_type() {
    assert_eq!(class_alias("CORROBORATES"), "n_corroborates");
    assert_eq!(class_alias(schema::ABOUT), "n_about");
}

#[test]
fn tier_sum_expression_joins_the_class_aliases() {
    assert_eq!(
        tier_sum_expression(ConnectionTier::Probative.edge_types()),
        "n_corroborates + n_rebuts + n_characterizes"
    );
    assert_eq!(
        tier_sum_expression(ConnectionTier::Topical.edge_types()),
        "n_corroborates + n_rebuts + n_characterizes + n_about"
    );
}

/// The guard is omitted entirely when there is none, rather than emitting a
/// `true AND …` filler that a future reader would have to decode.
#[test]
fn item_tally_emits_the_guard_only_when_given() {
    assert_eq!(
        item_tally(None, "n_about", "topical_connected"),
        "sum(CASE WHEN n_about > 0 THEN 1 ELSE 0 END) AS topical_connected"
    );
    assert_eq!(
        item_tally(Some("e IS NOT NULL"), "n_about", "about_items"),
        "sum(CASE WHEN e IS NOT NULL AND n_about > 0 THEN 1 ELSE 0 END) AS about_items"
    );
}

/// One `size([...])` per topical class, each with its own inner variable — two
/// comprehensions in one `WITH` may not reuse a name, and a collision would be a
/// runtime Cypher error rather than a compile error.
#[test]
fn class_count_expressions_bind_a_distinct_inner_variable_per_class() {
    let expressions = class_count_expressions("e");
    for rel in ConnectionTier::Topical.edge_types() {
        assert!(
            expressions.contains(&format!("(e)-[:{rel}]->(x_{rel})")),
            "missing comprehension for {rel}"
        );
        assert!(expressions.contains(&format!("AS {}", class_alias(rel))));
    }
    assert_eq!(
        expressions.matches("size([").count(),
        ConnectionTier::Topical.edge_types().len()
    );
}

// ── Corpus query ──────────────────────────────────────────────────────────────

/// The ratified headline basis, pinned in the generated Cypher. If someone adds
/// ABOUT to the probative tier, the headline silently inflates and this pane
/// stops doing its job — so assert it at the query level too, not only at the
/// vocabulary level.
#[test]
fn corpus_query_excludes_about_from_the_probative_tally() {
    let q = corpus_query();
    assert!(q.contains(&format!(
        "sum(CASE WHEN {} > 0 THEN 1 ELSE 0 END) AS probative_connected",
        tier_sum_expression(ConnectionTier::Probative.edge_types())
    )));
    assert!(q.contains(&format!(
        "sum(CASE WHEN {} > 0 THEN 1 ELSE 0 END) AS topical_connected",
        tier_sum_expression(ConnectionTier::Topical.edge_types())
    )));
    // …and the probative tally's own expression must not name the ABOUT alias.
    assert!(
        !tier_sum_expression(ConnectionTier::Probative.edge_types())
            .contains(&class_alias(schema::ABOUT)),
        "ABOUT must not appear in the probative tally"
    );
}

/// Node labels are parameter-bound, relationship types interpolated from
/// `neo4j::schema` (Cypher cannot parameterize a relationship type). Guards both
/// halves of Rule 16 in one assertion set.
#[test]
fn corpus_query_parameterizes_labels_and_uses_schema_relationships() {
    let q = corpus_query();
    assert!(q.contains("labels(e)[0] = $evidence_label"));
    assert!(q.contains("labels(x_CORROBORATES)[0] = $allegation_label"));
    assert!(q.contains("labels(d)[0] = $document_label"));
    assert!(q.contains(&format!("-[:{}]->", schema::CONTAINED_IN)));
    assert!(q.contains("count(e) AS evidence_total"));
}

/// The orphan tally is present: the corpus is measured over ALL Evidence, so the
/// payload must be able to explain any gap against the per-document table.
#[test]
fn corpus_query_counts_evidence_with_no_document() {
    let q = corpus_query();
    assert!(
        q.contains("sum(CASE WHEN n_documents = 0 THEN 1 ELSE 0 END) AS evidence_without_document")
    );
}

// ── Documents query ───────────────────────────────────────────────────────────

/// `OPTIONAL MATCH` on the Evidence leg is load-bearing: a Document that
/// produced no Evidence must still appear as a row of zeros. A plain `MATCH`
/// would drop it — hiding precisely the document worth looking at.
#[test]
fn documents_query_keeps_documents_that_produced_no_evidence() {
    let q = documents_query();
    assert!(
        q.contains(&format!(
            "OPTIONAL MATCH (e)-[:{}]->(doc)",
            schema::CONTAINED_IN
        )),
        "the Evidence leg must be OPTIONAL so zero-yield documents survive"
    );
    assert!(q.contains("e IS NOT NULL AND"));
}

/// Property-name discipline: the Document node's type is `doc_type` and its
/// display name is `title`. `document_type` and `file_name` have never existed
/// on this node — two of the 2026-07-26 alarm findings were queries against
/// exactly such invented names, so the correct spellings are pinned here.
#[test]
fn documents_query_reads_the_properties_the_write_path_actually_sets() {
    let q = documents_query();
    assert!(q.contains("doc.doc_type AS doc_type"));
    assert!(q.contains("doc.title AS title"));
    assert!(q.contains("doc.id AS document_id"));
    assert!(
        !q.contains("document_type"),
        "d.document_type is not a real property on a Document node"
    );
    assert!(
        !q.contains("file_name"),
        "d.file_name is not a real property on a Document node"
    );
}

/// One per-class column per topical edge class, aliased so the row decoder can
/// rebuild the positionally-parallel `by_edge_class` vector.
#[test]
fn documents_query_returns_one_column_per_edge_class() {
    let q = documents_query();
    for rel in ConnectionTier::Topical.edge_types() {
        let alias = format!("{}_items", class_alias(rel));
        assert!(
            q.contains(&format!("AS {alias}")),
            "missing per-class tally {alias}"
        );
    }
}

/// Deterministic ordering — worst-first by yield, tie-broken by id — so the
/// frontend sorts nothing and two loads of the same graph render identically.
#[test]
fn documents_query_orders_deterministically() {
    assert!(documents_query().contains("ORDER BY evidence_total DESC, document_id ASC"));
}

// ── Inventory queries ─────────────────────────────────────────────────────────

/// The label list is derived from the nodes themselves. `db.labels()` returns
/// label names Neo4j retains from earlier schema generations — twelve of them
/// carry zero nodes on this database — so using it would report a model that
/// does not exist.
#[test]
fn label_counts_query_derives_populated_labels_and_never_calls_db_labels() {
    assert!(LABEL_COUNTS_QUERY.contains("MATCH (n)"));
    assert!(LABEL_COUNTS_QUERY.contains("labels(n)[0] AS label"));
    assert!(LABEL_COUNTS_QUERY.contains("count(*) AS node_count"));
    assert!(
        !LABEL_COUNTS_QUERY.contains("db.labels"),
        "db.labels() reports vestigial empty labels and must never be used"
    );
    assert!(LABEL_COUNTS_QUERY.contains("ORDER BY node_count DESC, label ASC"));
}

/// The edge query returns the full (from, rel, to) triple. A count by
/// relationship type alone cannot distinguish `Evidence ABOUT Person` from
/// `Evidence ABOUT Allegation`, and those mean entirely different things — the
/// second is a connection, the first is not.
#[test]
fn edge_triples_query_returns_both_endpoint_labels() {
    assert!(EDGE_TRIPLES_QUERY.contains("labels(a)[0] AS from_label"));
    assert!(EDGE_TRIPLES_QUERY.contains("type(r) AS rel_type"));
    assert!(EDGE_TRIPLES_QUERY.contains("labels(b)[0] AS to_label"));
    assert!(EDGE_TRIPLES_QUERY.contains("count(*) AS edge_count"));
    assert!(EDGE_TRIPLES_QUERY.contains("ORDER BY edge_count DESC"));
}

/// Every query in this module is read-only. A write reaching the graph from the
/// Case Health surface would violate the pane's defining constraint, so scan the
/// generated Cypher for mutating clauses rather than trusting review.
#[test]
fn every_query_is_read_only() {
    let queries = [
        LABEL_COUNTS_QUERY.to_string(),
        EDGE_TRIPLES_QUERY.to_string(),
        corpus_query(),
        documents_query(),
    ];
    for q in &queries {
        let upper = q.to_uppercase();
        for forbidden in ["CREATE", "MERGE", " SET ", "DELETE", "REMOVE", "DETACH"] {
            assert!(
                !upper.contains(forbidden),
                "read-only violation: `{forbidden}` found in `{q}`"
            );
        }
    }
}

// ── Error surface ─────────────────────────────────────────────────────────────

/// `MissingCorpusRow` is the one variant that carries no wrapped external error,
/// so its message is the ONLY thing an operator would see. It must name what was
/// expected and what happened — "returned no row (expected exactly one)" — rather
/// than the state being papered over with a zeroed corpus, which would render
/// identically to a genuinely empty graph.
#[test]
fn missing_corpus_row_names_what_was_expected() {
    assert_eq!(
        CaseHealthRepoError::MissingCorpusRow.to_string(),
        "corpus aggregation returned no row (expected exactly one)"
    );
}

/// The `operation` field is what tells an operator WHICH of the four reads
/// failed. Pin that it reaches the rendered message for both wrapping variants —
/// a `Display` impl that dropped it would leave a log line saying only that
/// "a Neo4j query failed".
#[test]
fn repo_error_messages_name_the_failing_operation() {
    let decode_err = CaseHealthRepoError::RowDecode {
        operation: "fetch_document_rows",
        source: serde::de::Error::custom("bad column"),
    };
    let rendered = decode_err.to_string();
    assert!(
        rendered.contains("fetch_document_rows"),
        "the failing operation must appear in the message, got: {rendered}"
    );
    assert!(rendered.contains("bad column"), "got: {rendered}");
}
