//! Unit tests for [`crate::pipeline::steps::cleanup`].
//!
//! Split into a sibling file (via `#[path]`) so the cleanup module stays within
//! the module-size limit — the house pattern (`theme_scan_persist_tests.rs`,
//! `scan_runs_tests.rs`). Pure tests: the error-display contracts, the FK-safe
//! Postgres delete order, and the Cypher-shape guard that stops a shared Party
//! node being deleted out from under another document.

use super::*;

/// Distinctive payload used to prove that the outer Display of
/// [`CleanupError::Neo4j`] does NOT duplicate the inner source text.
const UNIQUE_INNER: &str = "UNIQUE_INNER_ERROR_MESSAGE";

/// Build a [`neo4rs::Error`] whose Display equals [`UNIQUE_INNER`] so
/// the "source not in outer" assertion in
/// `cleanup_error_neo4j_display_includes_doc_id_not_source` is exact.
/// `AuthenticationError(String)` formats with `"{0}"`, giving the raw
/// payload string verbatim in the inner Display.
fn dummy_neo4j_err() -> neo4rs::Error {
    neo4rs::Error::AuthenticationError(UNIQUE_INNER.to_string())
}

#[test]
fn cleanup_error_neo4j_display_includes_doc_id_not_source() {
    let err = CleanupError::Neo4j {
        doc_id: "doc-42".to_string(),
        source: dummy_neo4j_err(),
    };
    let display = format!("{err}");

    // Sanity check: the inner error really does carry UNIQUE_INNER.
    let inner_display = format!("{}", dummy_neo4j_err());
    assert_eq!(
        inner_display, UNIQUE_INNER,
        "dummy inner Display should equal the sentinel; got {inner_display}"
    );

    assert!(
        display.contains("doc-42"),
        "outer Display must include doc_id, got: {display}"
    );
    assert!(
        !display.contains(UNIQUE_INNER),
        "outer Display must NOT duplicate inner source text (Kazlauskas 6), got: {display}"
    );
}

#[test]
fn cleanup_error_partial_display_names_subsystems() {
    let inner = CleanupError::Neo4j {
        doc_id: "doc-7".to_string(),
        source: dummy_neo4j_err(),
    };
    let err = CleanupError::Partial {
        doc_id: "doc-7".to_string(),
        neo4j_error: Some(Box::new(inner)),
        qdrant_error: None,
        postgres_error: None,
        partial_report: CleanupReport::default(),
    };
    let display = format!("{err}");
    assert!(
        display.contains("doc-7"),
        "Partial Display must include doc_id, got: {display}"
    );
}

/// DELETE-FK-FIX guard for the saga path: the relationships clear must
/// match BOTH item-endpoint FKs, not just the owning `document_id`. If this
/// regresses, a single-document teardown rolls back whenever another
/// document's relationship points at this document's items.
// ── R5: the shared-node deletion guard ────────────────────────────────────

/// A node owned by ONE document still deletes — the common case is unchanged.
#[test]
fn party_delete_cypher_still_deletes_a_last_owner() {
    let cypher = build_party_delete_cypher("source_document");
    assert!(
        cypher.contains("DETACH DELETE n"),
        "must still delete: {cypher}"
    );
    assert!(
        cypher.contains("size([x IN n.source_documents WHERE x <> $doc_id]) = 0"),
        "last-owner test must be an emptiness check on the remaining owners: {cypher}"
    );
}

/// Nodes with NO source_documents array keep the old behavior.
///
/// Only Party nodes carry the array; Evidence, Allegation and the rest are
/// single-document by construction, and adding the guard must not strand them.
#[test]
fn party_delete_cypher_leaves_non_party_nodes_unguarded() {
    let cypher = build_party_delete_cypher("source_document");
    assert!(
        cypher.contains("n.source_documents IS NULL"),
        "a node with no ownership array must still delete on the scalar: {cypher}"
    );
}

/// The guard is what stops silent edge destruction across documents.
///
/// Without it, cleaning doc A deletes a Party shared with doc B — and DETACH
/// takes doc B's edges with it, with nothing logged and nothing failing. This
/// test is the reason the predicate exists; deleting it should be loud.
#[test]
fn party_delete_cypher_spares_a_node_another_document_still_owns() {
    let cypher = build_party_delete_cypher("source_document");
    // The predicate must be a conjunction: matching the property is necessary
    // but NOT sufficient. A bare `WHERE n.prop = $doc_id DETACH DELETE` is the
    // regression this guards against.
    assert!(
        cypher.contains("AND ("),
        "the property match alone must not authorize deletion: {cypher}"
    );
    let where_clause = cypher
        .split_once("WHERE")
        .expect("has a WHERE")
        .1
        .split_once("DETACH")
        .expect("has a DETACH")
        .0;
    assert!(
        where_clause.contains("source_documents"),
        "the ownership array must participate in the delete decision: {where_clause}"
    );
}

/// The guard is applied to BOTH scalar properties cleanup sweeps.
#[test]
fn party_delete_cypher_guards_every_property_it_is_built_for() {
    for property in ["source_document", "source_document_id"] {
        let cypher = build_party_delete_cypher(property);
        assert!(
            cypher.contains(&format!("n.{property} = $doc_id")),
            "must scope to {property}: {cypher}"
        );
        assert!(
            cypher.contains("n.source_documents IS NULL"),
            "{property} sweep must carry the same shared-node guard: {cypher}"
        );
    }
}

#[test]
fn postgres_delete_order_relationships_covers_both_fk_endpoints() {
    let (_, sql) = POSTGRES_DELETE_ORDER
        .iter()
        .find(|(table, _)| *table == "extraction_relationships")
        .expect("extraction_relationships step must exist in POSTGRES_DELETE_ORDER");
    assert!(
        sql.contains("document_id = $1"),
        "must still clear rows this document owns"
    );
    assert!(
        sql.contains("from_item_id IN (SELECT id FROM extraction_items WHERE document_id = $1)"),
        "must clear rows pointing FROM this document's items"
    );
    assert!(
        sql.contains("to_item_id IN (SELECT id FROM extraction_items WHERE document_id = $1)"),
        "must clear rows pointing TO this document's items (the RESTRICT endpoint)"
    );
}
