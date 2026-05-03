//! Pure unit tests for bias DTO serialization and pattern-tag parsing.
//!
//! These do not touch Neo4j. Live-Neo4j integration testing is out of
//! scope for this feature per the project's test policy (no live-fixture
//! coverage in `cargo test --workspace`).

use serde_json::json;

use super::aggregation::parse_pattern_tags;
use super::dto::{
    ActorOption, AvailableFilters, BiasInstance, BiasQueryFilters, BiasQueryResult, DocumentRef,
};
use super::repository::resolve_default_subject_id;

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
        subject_id: None,
    };
    let value = serde_json::to_value(&filters).unwrap();
    // actor_id and subject_id absent (skip_serializing_if), pattern_tag present.
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
        subjects: vec![],
        default_subject_id: None,
    };

    let value = serde_json::to_value(&filters).unwrap();
    let back: AvailableFilters = serde_json::from_value(value).unwrap();

    assert_eq!(back.actors.len(), 2);
    assert_eq!(back.actors[0].actor_type, "Person");
    assert_eq!(back.actors[1].actor_type, "Organization");
    assert_eq!(back.pattern_tags, vec!["disparagement", "secrecy"]);
    assert!(back.subjects.is_empty());
    assert!(back.default_subject_id.is_none());
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
        total_unfiltered: 248,
        instances: Vec::new(),
        applied_filters: BiasQueryFilters {
            actor_id: Some("person-jeffrey".into()),
            pattern_tag: Some("lies_under_oath".into()),
            subject_id: None,
        },
    };
    let value = serde_json::to_value(&result).unwrap();
    assert_eq!(value["applied_filters"]["actor_id"], "person-jeffrey");
    assert_eq!(value["applied_filters"]["pattern_tag"], "lies_under_oath");
    assert_eq!(value["total_count"], 3);
    assert_eq!(value["total_unfiltered"], 248);
    assert_eq!(value["instances"], json!([]));
}

// ─── v2: subject_id, subjects, total_unfiltered ─────────────────────────────

#[test]
fn bias_query_filters_serializes_subject_id_when_set() {
    let filters = BiasQueryFilters {
        actor_id: None,
        pattern_tag: None,
        subject_id: Some("person-marie".to_string()),
    };
    let value = serde_json::to_value(&filters).unwrap();
    // actor_id and pattern_tag are absent (skip_serializing_if), subject_id present.
    assert_eq!(value, json!({ "subject_id": "person-marie" }));
}

#[test]
fn bias_query_filters_omits_subject_id_when_none() {
    // Standing Rule 1: distinct states are observable in JSON.
    // None must be omitted (not serialized as `null`) so a future
    // multi-select form factor that genuinely sends `subject_id: null`
    // remains distinguishable.
    let filters = BiasQueryFilters::default();
    let value = serde_json::to_value(&filters).unwrap();
    assert!(value.get("subject_id").is_none());
}

#[test]
fn bias_query_filters_deserializes_with_only_subject_id() {
    let parsed: BiasQueryFilters =
        serde_json::from_value(json!({ "subject_id": "person-marie" })).unwrap();
    assert!(parsed.actor_id.is_none());
    assert!(parsed.pattern_tag.is_none());
    assert_eq!(parsed.subject_id.as_deref(), Some("person-marie"));
}

#[test]
fn available_filters_response_includes_subjects_field() {
    let filters = AvailableFilters {
        actors: vec![],
        pattern_tags: vec![],
        subjects: vec![ActorOption {
            id: "person-marie".into(),
            name: "Marie Awad".into(),
            actor_type: "Person".into(),
            tagged_statement_count: 47,
        }],
        default_subject_id: Some("person-marie".into()),
    };
    let value = serde_json::to_value(&filters).unwrap();
    assert!(value["subjects"].is_array());
    assert_eq!(value["subjects"][0]["id"], "person-marie");
    assert_eq!(value["subjects"][0]["name"], "Marie Awad");
    assert_eq!(value["default_subject_id"], "person-marie");
}

#[test]
fn available_filters_omits_default_subject_id_when_none() {
    let filters = AvailableFilters {
        actors: vec![],
        pattern_tags: vec![],
        subjects: vec![],
        default_subject_id: None,
    };
    let value = serde_json::to_value(&filters).unwrap();
    assert!(
        value.get("default_subject_id").is_none(),
        "default_subject_id must be skip_serializing_if when None"
    );
}

#[test]
fn bias_query_result_serializes_total_unfiltered() {
    let result = BiasQueryResult {
        total_count: 47,
        total_unfiltered: 231,
        instances: Vec::new(),
        applied_filters: BiasQueryFilters::default(),
    };
    let value = serde_json::to_value(&result).unwrap();
    assert_eq!(value["total_count"], 47);
    assert_eq!(value["total_unfiltered"], 231);
}

// ─── resolve_default_subject_id helper ──────────────────────────────────────

fn make_subject(id: &str, name: &str, count: i64) -> ActorOption {
    ActorOption {
        id: id.to_string(),
        name: name.to_string(),
        actor_type: "Person".to_string(),
        tagged_statement_count: count,
    }
}

#[test]
fn resolve_default_subject_id_returns_id_on_exact_match() {
    let subjects = vec![
        make_subject("person-george", "George Phillips", 114),
        make_subject("person-marie", "Marie Awad", 47),
    ];
    let resolved = resolve_default_subject_id(&subjects, Some("Marie Awad"));
    assert_eq!(resolved.as_deref(), Some("person-marie"));
}

#[test]
fn resolve_default_subject_id_returns_none_when_name_unset() {
    let subjects = vec![make_subject("person-marie", "Marie Awad", 47)];
    let resolved = resolve_default_subject_id(&subjects, None);
    assert!(resolved.is_none());
}

#[test]
fn resolve_default_subject_id_treats_blank_name_as_unset() {
    // Empty / whitespace-only env value behaves like "unset" so an
    // accidentally-empty Ansible variable does not silently "match"
    // anything in the subjects list.
    let subjects = vec![make_subject("person-marie", "Marie Awad", 47)];
    assert!(resolve_default_subject_id(&subjects, Some("")).is_none());
    assert!(resolve_default_subject_id(&subjects, Some("   ")).is_none());
}

#[test]
fn resolve_default_subject_id_returns_none_when_no_match() {
    let subjects = vec![make_subject("person-george", "George Phillips", 114)];
    let resolved = resolve_default_subject_id(&subjects, Some("Marie Awad"));
    assert!(resolved.is_none());
}

#[test]
fn resolve_default_subject_id_is_case_sensitive() {
    let subjects = vec![make_subject("person-marie", "Marie Awad", 47)];
    // Lowercase variant must not match.
    assert!(resolve_default_subject_id(&subjects, Some("marie awad")).is_none());
}

#[test]
fn resolve_default_subject_id_picks_first_when_multiple_match() {
    // Subjects are pre-sorted by tagged_count DESC, so "first" means
    // highest-count match. We assert the helper picks that one.
    let subjects = vec![
        make_subject("person-marie-a", "Marie Awad", 47),
        make_subject("person-marie-b", "Marie Awad", 12),
    ];
    let resolved = resolve_default_subject_id(&subjects, Some("Marie Awad"));
    assert_eq!(resolved.as_deref(), Some("person-marie-a"));
}
