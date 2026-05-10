//! Pure unit tests for bias DTO serialization and pattern-tag parsing.
//!
//! These do not touch Neo4j. Live-Neo4j integration testing is out of
//! scope for this feature per the project's test policy (no live-fixture
//! coverage in `cargo test --workspace`).

use super::aggregation::parse_pattern_tags;
use super::dto::ActorOption;
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
