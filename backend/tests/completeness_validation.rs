//! Unit tests for completeness validation (pure logic, no database needed).
//!
//! Tests the `validate_completeness` function with synthetic schemas (built
//! from inline YAML) and synthetic JSON extraction output.

use colossus_extract::ExtractionSchema;
use colossus_legal_backend::api::pipeline::completeness_validation::validate_completeness;

/// Helper: build an ExtractionSchema from inline YAML.
fn schema_from_yaml(yaml: &str) -> ExtractionSchema {
    ExtractionSchema::from_yaml_str(yaml).expect("test YAML should parse")
}

/// Schema with Party (required, min_count: 2) and LegalCount (required, min_count: 1).
fn schema_with_required_types() -> ExtractionSchema {
    schema_from_yaml(
        r#"
document_type: complaint
document_category: foundation
entity_types:
  - name: Party
    category: foundation
    required: true
    min_count: 2
    grounding_mode: name_match
    description: "A party in the case"
  - name: LegalCount
    category: structural
    required: true
    min_count: 1
    grounding_mode: verbatim
    description: "A legal count or cause of action"
relationship_types:
  - name: FILED_BY
    description: "Party filed the count"
valid_patterns:
  - from: LegalCount
    relationship: FILED_BY
    to: Party
"#,
    )
}

#[test]
fn test_completeness_passes_with_sufficient_entities() {
    let schema = schema_with_required_types();
    let parsed = serde_json::json!({
        "entities": [
            { "id": "p1", "entity_type": "Party", "name": "Alice" },
            { "id": "p2", "entity_type": "Party", "name": "Bob" },
            { "id": "p3", "entity_type": "Party", "name": "Carol" },
            { "id": "lc1", "entity_type": "LegalCount", "name": "Count 1" },
            { "id": "lc2", "entity_type": "LegalCount", "name": "Count 2" },
        ],
        "relationships": []
    });

    let result = validate_completeness(&schema, &parsed);

    assert!(result.passed, "Expected passed=true, got errors: {:?}", result.errors);
    assert!(result.errors.is_empty());
}

#[test]
fn test_completeness_fails_missing_required_type() {
    let schema = schema_with_required_types();
    // Has Parties but zero LegalCounts
    let parsed = serde_json::json!({
        "entities": [
            { "id": "p1", "entity_type": "Party", "name": "Alice" },
            { "id": "p2", "entity_type": "Party", "name": "Bob" },
        ],
        "relationships": []
    });

    let result = validate_completeness(&schema, &parsed);

    assert!(!result.passed);
    assert!(
        result.errors.iter().any(|e| e.contains("LegalCount")),
        "Expected error mentioning LegalCount, got: {:?}",
        result.errors
    );
}

#[test]
fn test_completeness_fails_below_min_count() {
    let schema = schema_with_required_types();
    // Party min_count is 2, but only 1 Party present
    let parsed = serde_json::json!({
        "entities": [
            { "id": "p1", "entity_type": "Party", "name": "Alice" },
            { "id": "lc1", "entity_type": "LegalCount", "name": "Count 1" },
        ],
        "relationships": []
    });

    let result = validate_completeness(&schema, &parsed);

    assert!(!result.passed);
    let party_error = result.errors.iter().find(|e| e.contains("Party"));
    assert!(party_error.is_some(), "Expected error for Party, got: {:?}", result.errors);
    assert!(
        party_error.unwrap().contains("need at least 2"),
        "Expected 'need at least 2' in error, got: {}",
        party_error.unwrap()
    );
}

#[test]
fn test_completeness_warns_low_relationship_percentage() {
    // Schema with a RelationshipExists completeness rule
    let schema = schema_from_yaml(
        r#"
document_type: complaint
document_category: foundation
entity_types:
  - name: ComplaintAllegation
    category: foundation
    required: true
    min_count: 1
    grounding_mode: verbatim
    description: "An allegation"
  - name: LegalCount
    category: structural
    required: true
    min_count: 1
    grounding_mode: verbatim
    description: "A legal count"
relationship_types:
  - name: SUPPORTS
    description: "Allegation supports a count"
valid_patterns:
  - from: ComplaintAllegation
    relationship: SUPPORTS
    to: LegalCount
completeness_rules:
  - type: relationship_exists
    from: ComplaintAllegation
    relationship: SUPPORTS
    to: LegalCount
    min_percentage: 50
    message: "Allegations should support at least one count"
"#,
    );

    // 10 allegations, but only 3 have SUPPORTS relationships to LegalCounts
    let parsed = serde_json::json!({
        "entities": [
            { "id": "a1", "entity_type": "ComplaintAllegation" },
            { "id": "a2", "entity_type": "ComplaintAllegation" },
            { "id": "a3", "entity_type": "ComplaintAllegation" },
            { "id": "a4", "entity_type": "ComplaintAllegation" },
            { "id": "a5", "entity_type": "ComplaintAllegation" },
            { "id": "a6", "entity_type": "ComplaintAllegation" },
            { "id": "a7", "entity_type": "ComplaintAllegation" },
            { "id": "a8", "entity_type": "ComplaintAllegation" },
            { "id": "a9", "entity_type": "ComplaintAllegation" },
            { "id": "a10", "entity_type": "ComplaintAllegation" },
            { "id": "lc1", "entity_type": "LegalCount" },
        ],
        "relationships": [
            { "relationship_type": "SUPPORTS", "from_entity": "a1", "to_entity": "lc1" },
            { "relationship_type": "SUPPORTS", "from_entity": "a2", "to_entity": "lc1" },
            { "relationship_type": "SUPPORTS", "from_entity": "a3", "to_entity": "lc1" },
        ]
    });

    let result = validate_completeness(&schema, &parsed);

    // Warnings don't block — should still pass
    assert!(result.passed, "Expected passed=true (warnings don't block), got errors: {:?}", result.errors);
    assert!(
        result.warnings.iter().any(|w| w.contains("30%")),
        "Expected warning with '30%', got: {:?}",
        result.warnings
    );
}

#[test]
fn test_completeness_passes_no_rules() {
    // Schema with no completeness_rules and no required entities
    let schema = schema_from_yaml(
        r#"
document_type: evidence_doc
entity_types:
  - name: Exhibit
    description: "An exhibit"
relationship_types:
  - name: REFERENCES
    description: "References another entity"
valid_patterns:
  - from: Exhibit
    relationship: REFERENCES
    to: Exhibit
"#,
    );

    let parsed = serde_json::json!({ "entities": [], "relationships": [] });

    let result = validate_completeness(&schema, &parsed);

    assert!(result.passed, "Expected passed=true with no rules, got errors: {:?}", result.errors);
    assert!(result.errors.is_empty());
}

#[test]
fn test_completeness_handles_empty_entities() {
    let schema = schema_with_required_types();
    // Empty entities array — required types will fail
    let parsed = serde_json::json!({ "entities": [], "relationships": [] });

    let result = validate_completeness(&schema, &parsed);

    assert!(!result.passed);
    assert!(
        result.errors.iter().any(|e| e.contains("Party")),
        "Expected error for missing Party, got: {:?}",
        result.errors
    );
    assert!(
        result.errors.iter().any(|e| e.contains("LegalCount")),
        "Expected error for missing LegalCount, got: {:?}",
        result.errors
    );
}

#[test]
fn test_completeness_entity_counts_reported() {
    let schema = schema_with_required_types();
    let parsed = serde_json::json!({
        "entities": [
            { "id": "p1", "entity_type": "Party" },
            { "id": "p2", "entity_type": "Party" },
            { "id": "p3", "entity_type": "Party" },
            { "id": "lc1", "entity_type": "LegalCount" },
        ],
        "relationships": []
    });

    let result = validate_completeness(&schema, &parsed);

    // entity_counts should have one entry per schema entity type
    assert_eq!(result.entity_counts.len(), 2, "Expected 2 entity type counts");

    let party_count = result.entity_counts.iter().find(|(name, _)| name == "Party");
    assert_eq!(
        party_count,
        Some(&("Party".to_string(), 3)),
        "Expected Party count = 3"
    );

    let lc_count = result.entity_counts.iter().find(|(name, _)| name == "LegalCount");
    assert_eq!(
        lc_count,
        Some(&("LegalCount".to_string(), 1)),
        "Expected LegalCount count = 1"
    );
}
