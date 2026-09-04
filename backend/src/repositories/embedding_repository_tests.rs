//! Shape tests for the two graph reads that feed `build_embedding_text`.
//!
//! ## Why a repository has query-text tests and no database
//!
//! These reads have one failure mode that no compiler and no integration test on
//! an empty corpus will catch: a property the BUILDER reads but the QUERY does
//! not return. `run_node_query` drops empty values before they reach the
//! properties map, and `get_prop` answers `""` for a missing key — so the
//! builder quietly produces its no-question text, the pipeline succeeds, the
//! vectors are written, and nothing anywhere says the composition never ran.
//!
//! That is exactly how `question` was already absent while the card DTO and the
//! scan judge both showed it. These tests make the query text and the key list
//! agree by construction, so the next property to be added cannot ship dead.

use super::*;

/// Every key in a list must be a column the query actually returns.
fn assert_keys_are_returned(cypher: &str, keys: &[&str], name: &str) {
    for key in keys {
        assert!(
            cypher.contains(&format!("AS {key}")),
            "{name}: `{key}` is in the property list but the query never returns it — \
             the builder would silently see an empty string"
        );
    }
}

#[test]
fn the_embed_all_evidence_read_returns_every_property_it_lists() {
    assert_keys_are_returned(&q_all_evidence(), &EVIDENCE_PROP_KEYS, "q_all_evidence");
}

#[test]
fn the_per_document_read_returns_every_property_it_lists() {
    assert_keys_are_returned(
        &q_document_entities(),
        &ENTITY_PROP_KEYS,
        "q_document_entities",
    );
}

/// The property this task exists to carry, pinned in both reads.
///
/// Domain note: 367 Evidence cards carry the interrogatory or request for
/// admission they answer, and 99 of those have an answer-only `verbatim_quote`
/// ("Admitted.", "Denied as untrue."). `build_embedding_text` composes
/// `Request: … Answer: …` when it sees a `question` — and can only see one if
/// these two queries return it.
#[test]
fn both_reads_carry_question_to_the_embedding_builder() {
    assert!(q_all_evidence().contains("e.question AS question"));
    assert!(EVIDENCE_PROP_KEYS.contains(&"question"));
    assert!(q_document_entities().contains("n.question AS question"));
    assert!(ENTITY_PROP_KEYS.contains(&"question"));
}

#[test]
fn both_reads_are_reads() {
    for (name, cypher) in [
        ("q_all_evidence", q_all_evidence()),
        ("q_document_entities", q_document_entities()),
    ] {
        let upper = cypher.to_uppercase();
        for forbidden in [
            "CREATE ", "MERGE ", "DELETE ", "DETACH ", " SET ", "REMOVE ",
        ] {
            assert!(
                !upper.contains(forbidden),
                "{name} contains {forbidden:?} — the embedding reads must never write"
            );
        }
    }
}

/// A key list with a duplicate would overwrite one property with another in the
/// map, silently. Cheap to rule out, impossible to see by eye in an 11-entry list.
#[test]
fn no_property_key_is_listed_twice() {
    for (name, keys) in [
        ("EVIDENCE_PROP_KEYS", EVIDENCE_PROP_KEYS.as_slice()),
        ("ENTITY_PROP_KEYS", ENTITY_PROP_KEYS.as_slice()),
    ] {
        let mut sorted = keys.to_vec();
        sorted.sort_unstable();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), before, "{name} lists a key twice");
    }
}
