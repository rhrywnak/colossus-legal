//! Pure unit tests for bias DTO serialization and pattern-tag parsing.
//!
//! These do not touch Neo4j. Live-Neo4j integration testing is out of
//! scope for this feature per the project's test policy (no live-fixture
//! coverage in `cargo test --workspace`).

use serde_json::json;

use super::dto::{
    ActorOption, AvailableFilters, BiasInstance, BiasQueryFilters, BiasQueryResult, DocumentRef,
};
use super::repository::parse_pattern_tags;

// ─── parse_pattern_tags ─────────────────────────────────────────────────────

#[test]
fn parses_pattern_tags_csv_trimming_whitespace_and_empties() {
    let parsed = parse_pattern_tags("disparagement, secrecy ,, evasive_responses ");
    assert_eq!(
        parsed,
        vec!["disparagement", "secrecy", "evasive_responses"]
    );
}

#[test]
fn parses_pattern_tags_empty_string_returns_empty_vec() {
    assert!(parse_pattern_tags("").is_empty());
}

#[test]
fn parses_pattern_tags_only_commas_returns_empty_vec() {
    assert!(parse_pattern_tags(",,, ,").is_empty());
}

#[test]
fn parses_pattern_tags_preserves_authoring_order() {
    let parsed = parse_pattern_tags("c,a,b");
    assert_eq!(parsed, vec!["c", "a", "b"]);
}

// ─── BiasQueryFilters serialization ─────────────────────────────────────────

#[test]
fn bias_query_filters_serializes_omitting_none_fields() {
    let filters = BiasQueryFilters {
        actor_id: None,
        pattern_tag: Some("disparagement".to_string()),
    };
    let value = serde_json::to_value(&filters).unwrap();
    // actor_id absent (skip_serializing_if), pattern_tag present.
    assert_eq!(value, json!({ "pattern_tag": "disparagement" }));
}

#[test]
fn bias_query_filters_serializes_empty_when_all_none() {
    let filters = BiasQueryFilters::default();
    let value = serde_json::to_value(&filters).unwrap();
    assert_eq!(value, json!({}));
}

#[test]
fn bias_query_filters_deserialises_with_all_fields_absent() {
    // Empty object is a valid request — it means "no filters".
    let parsed: BiasQueryFilters = serde_json::from_value(json!({})).unwrap();
    assert!(parsed.actor_id.is_none());
    assert!(parsed.pattern_tag.is_none());
}

#[test]
fn bias_query_filters_deserialises_with_unknown_fields_ignored() {
    // Forward-compat: a future client sending a not-yet-supported field
    // (e.g., date_from) should not break this older server.
    let parsed: BiasQueryFilters =
        serde_json::from_value(json!({ "pattern_tag": "secrecy", "date_from": "2026-01-01" }))
            .unwrap();
    assert_eq!(parsed.pattern_tag.as_deref(), Some("secrecy"));
}

// ─── ActorOption / AvailableFilters round-trip ──────────────────────────────

#[test]
fn available_filters_round_trips_with_actor_types() {
    let filters = AvailableFilters {
        actors: vec![
            ActorOption {
                id: "person-george".into(),
                name: "George Phillips".into(),
                actor_type: "Person".into(),
                tagged_statement_count: 114,
            },
            ActorOption {
                id: "org-cfs".into(),
                name: "Catholic Family Services".into(),
                actor_type: "Organization".into(),
                tagged_statement_count: 101,
            },
        ],
        pattern_tags: vec!["disparagement".into(), "secrecy".into()],
    };

    let value = serde_json::to_value(&filters).unwrap();
    let back: AvailableFilters = serde_json::from_value(value).unwrap();

    assert_eq!(back.actors.len(), 2);
    assert_eq!(back.actors[0].actor_type, "Person");
    assert_eq!(back.actors[1].actor_type, "Organization");
    assert_eq!(back.pattern_tags, vec!["disparagement", "secrecy"]);
}

// ─── BiasInstance / DocumentRef field-skipping ──────────────────────────────

#[test]
fn bias_instance_round_trips_with_empty_about_as_array() {
    let instance = BiasInstance {
        evidence_id: "evidence-q97".into(),
        title: "100% costs argument".into(),
        verbatim_quote: Some("Phillips argued 100% costs against Marie".into()),
        page_number: Some(12),
        pattern_tags: vec!["selective_enforcement".into()],
        stated_by: Some(ActorOption {
            id: "org-cfs".into(),
            name: "Catholic Family Services".into(),
            actor_type: "Organization".into(),
            tagged_statement_count: 0,
        }),
        about: Vec::new(),
        document: Some(DocumentRef {
            id: "doc-cfs-int".into(),
            title: "CFS Interrogatory Response".into(),
            document_type: Some("interrogatory_response".into()),
        }),
    };
    let value = serde_json::to_value(&instance).unwrap();

    // Empty `about` must serialize as `[]`, not be omitted — the frontend
    // shape contract is that `about` always exists.
    assert_eq!(value["about"], json!([]));
}

#[test]
fn bias_instance_omits_document_when_none() {
    let instance = BiasInstance {
        evidence_id: "evidence-orphan".into(),
        title: "Statement without document link".into(),
        verbatim_quote: None,
        page_number: None,
        pattern_tags: vec![],
        stated_by: None,
        about: vec![],
        document: None,
    };
    let value = serde_json::to_value(&instance).unwrap();
    assert!(
        value.get("document").is_none(),
        "document should be skipped when None"
    );
    assert!(value.get("verbatim_quote").is_none());
    assert!(value.get("page_number").is_none());
    assert!(value.get("stated_by").is_none());
}

#[test]
fn document_ref_omits_document_type_when_none() {
    // Standing Rule 1: missing document_type must remain distinguishable
    // from an empty-string document_type. Skip-serializing-if-None is how
    // we keep that boundary visible in the JSON contract.
    let doc = DocumentRef {
        id: "doc-1".into(),
        title: "Untyped doc".into(),
        document_type: None,
    };
    let value = serde_json::to_value(&doc).unwrap();
    assert!(value.get("document_type").is_none());
}

// ─── BiasQueryResult shape ──────────────────────────────────────────────────

#[test]
fn bias_query_result_echoes_applied_filters() {
    let result = BiasQueryResult {
        total_count: 3,
        instances: Vec::new(),
        applied_filters: BiasQueryFilters {
            actor_id: Some("person-jeffrey".into()),
            pattern_tag: Some("lies_under_oath".into()),
        },
    };
    let value = serde_json::to_value(&result).unwrap();
    assert_eq!(value["applied_filters"]["actor_id"], "person-jeffrey");
    assert_eq!(value["applied_filters"]["pattern_tag"], "lies_under_oath");
    assert_eq!(value["total_count"], 3);
    assert_eq!(value["instances"], json!([]));
}
