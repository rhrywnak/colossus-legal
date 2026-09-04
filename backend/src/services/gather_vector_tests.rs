//! Tests for the vector half: the prefix pairing and the filter's shape.

use super::*;
use crate::services::embedding_text::{build_embedding_text, DOCUMENT_PREFIX};
use std::collections::HashMap;

/// ⚑ THE PAIRING. The query prefix is the mirror of the document prefix.
///
/// nomic-embed-text is asymmetric: what is indexed under `search_document:`
/// must be searched with `search_query:`. Getting it wrong returns vectors,
/// returns hits, and renders a page — it is just quietly worse, with no error
/// and no empty list to notice. So it is asserted here against the real
/// indexing function rather than trusted to a comment.
#[test]
fn the_query_prefix_is_the_mirror_of_the_document_prefix() {
    let mut props = HashMap::new();
    props.insert("title".to_string(), "A deposit slip".to_string());
    props.insert("verbatim_quote".to_string(), "$50,000.00".to_string());
    let indexed = build_embedding_text("Evidence", &props);

    assert!(
        indexed.starts_with(DOCUMENT_PREFIX),
        "the index side must still write the document prefix: {indexed}"
    );
    assert!(
        query_text("the money he deposited").starts_with(QUERY_PREFIX),
        "and the query side must write the query prefix"
    );
    assert_ne!(
        DOCUMENT_PREFIX, QUERY_PREFIX,
        "an asymmetric model with one prefix on both sides is the silent failure"
    );
    // The pair differs in exactly the one word, which is what makes them a pair
    // rather than two unrelated strings that happen to be prefixes.
    assert_eq!(
        DOCUMENT_PREFIX.replace("document", "query"),
        QUERY_PREFIX,
        "if these stop mirroring, one side was edited without the other"
    );
}

/// The query text is the prefix plus the composed query, unaltered.
#[test]
fn the_composed_query_reaches_the_model_unaltered() {
    let composed = "Everything downstream flows from one choice.\nAllegations of theft arose.";
    let text = query_text(composed);

    assert_eq!(text, format!("search_query: {composed}"));
    assert!(
        text.ends_with("Allegations of theft arose."),
        "nothing may be trimmed off the end — that is where the last allegation lives"
    );
    assert!(text.contains('\n'), "the composer's newlines survive");
}

/// ⚑ The party filter is a `must` over `node_id`, and it is really there.
///
/// A vector search that silently loses its filter returns the whole corpus
/// ranked by similarity, which looks like a working gather and is not the
/// scenario's pool at all.
#[test]
fn the_filter_binds_the_search_to_the_allowed_ids() {
    let ids = vec![
        "doc:evidence:41068bce".to_string(),
        "doc:evidence:7bf6759b".to_string(),
    ];
    let body = search_body(&[0.1, 0.2], Some(&ids), 200);

    assert_eq!(body["limit"], 200);
    assert_eq!(body["with_payload"], true);
    let clause = &body["filter"]["must"][0];
    assert_eq!(clause["key"], "node_id");
    assert_eq!(
        clause["match"]["any"],
        serde_json::json!(["doc:evidence:41068bce", "doc:evidence:7bf6759b"])
    );
}

/// `None` sends no filter; `Some(empty)` sends a filter that matches nothing.
///
/// Same distinction the lexical half draws. Collapsing them would turn "this
/// scenario reaches nobody" into "search the entire corpus", which is the
/// loudest possible wrong answer delivered silently.
#[test]
fn no_filter_and_an_empty_filter_are_different_bodies() {
    let unfiltered = search_body(&[0.1], None, 10);
    assert!(
        unfiltered.get("filter").is_none(),
        "None must send NO filter key at all"
    );

    let empty = search_body(&[0.1], Some(&[]), 10);
    assert_eq!(
        empty["filter"]["must"][0]["match"]["any"],
        serde_json::json!([]),
        "Some(empty) must send a filter that admits nothing"
    );
}

fn hits(value: serde_json::Value) -> Vec<serde_json::Value> {
    value["result"].as_array().expect("a result array").clone()
}

/// Hits come back in Qdrant's order, which is the rank fusion relies on.
#[test]
fn the_ids_come_back_in_rank_order() {
    let response = serde_json::json!({"result": [
        {"score": 0.91, "payload": {"node_id": "first"}},
        {"score": 0.72, "payload": {"node_id": "second"}},
        {"score": 0.51, "payload": {"node_id": "third"}}
    ]});
    assert_eq!(
        node_ids_of(&hits(response)),
        vec!["first", "second", "third"]
    );
}

/// A point with no `node_id` is skipped, not defaulted to an empty string.
///
/// An empty id would fuse, rank, and render as a card nobody can open. Skipping
/// it makes the loss visible as a length difference the caller can report.
#[test]
fn a_point_with_no_node_id_is_skipped_rather_than_blanked() {
    let response = serde_json::json!({"result": [
        {"payload": {"node_id": "good"}},
        {"payload": {"title": "no node_id here"}},
        {"payload": {"node_id": "also-good"}}
    ]});
    let raw = hits(response);
    let ids = node_ids_of(&raw);

    assert_eq!(ids, vec!["good", "also-good"]);
    assert_eq!(
        raw.len() - ids.len(),
        1,
        "the caller compares these two lengths and warns — that difference IS the \
         mechanism, and it must remain computable"
    );
    assert!(
        !ids.iter().any(|id| id.is_empty()),
        "an empty id would render as a card that opens nothing"
    );
}

/// An empty result is an empty list — a legitimate zero-hit search.
#[test]
fn a_legitimately_empty_search_yields_nothing() {
    assert!(node_ids_of(&hits(serde_json::json!({"result": []}))).is_empty());
}

/// ⚑ A 2xx carrying no `result` array is quoted, bounded, into the error.
///
/// It is a schema change, a proxy, or a feature flag — never a real empty
/// search, which answers `"result": []`. Collapsing it to an empty list would
/// make a broken Qdrant look exactly like a query that matched nothing, and a
/// gather would come back empty with nothing anywhere to explain it.
#[test]
fn an_unexpected_response_body_is_quoted_but_bounded() {
    let quoted = excerpt_of(&serde_json::json!({"status": "ok", "note": "no result key"}));
    assert!(quoted.contains("no result key"), "{quoted}");

    let huge = serde_json::json!({"blob": "x".repeat(10_000)});
    assert_eq!(
        excerpt_of(&huge).chars().count(),
        BODY_EXCERPT_CHARS,
        "a large body must not flood the error field"
    );
}
