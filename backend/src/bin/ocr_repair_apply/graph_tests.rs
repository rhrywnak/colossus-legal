//! Rule 21 in miniature, applied to a WRITE: the statement text is an invariant,
//! so a test reads it. No connection is opened here.

use super::*;

/// The exact five properties the instruction authorises this bin to change.
const AUTHORISED: [&str; 5] = [
    "verbatim_quote_ocr_original",
    "verbatim_quote",
    "grounding_status",
    "ocr_repaired_at",
    "ocr_repair_source",
];

#[test]
fn the_write_sets_only_the_five_authorised_properties() {
    for property in AUTHORISED {
        assert!(
            Q_WRITE.contains(&format!("e.{property} =")),
            "the write statement no longer sets {property}"
        );
    }
    // Count the assignments: `SET a = x, b = y` has one `e.` per assignment on
    // the left-hand side, and the RETURN clause has none. Five, and only five.
    let assignments = Q_WRITE.matches("e.").count();
    assert_eq!(
        assignments,
        AUTHORISED.len(),
        "the write statement touches {assignments} properties, not {}: {Q_WRITE}",
        AUTHORISED.len()
    );
}

#[test]
fn the_write_is_a_match_and_never_a_create_or_a_delete() {
    let upper = Q_WRITE.to_uppercase();
    for forbidden in [
        "CREATE ", "MERGE ", "DELETE ", "DETACH ", "REMOVE ", "DROP ",
    ] {
        assert!(
            !upper.contains(forbidden),
            "the write statement contains {forbidden:?} — this bin only SETs"
        );
    }
    assert!(Q_WRITE.starts_with("MATCH (e:Evidence {id: $id})"));
}

#[test]
fn every_other_statement_is_a_read() {
    for (name, text) in [
        ("Q_READ", Q_READ),
        ("Q_MANUAL_PROBE", Q_MANUAL_PROBE),
        ("Q_COUNT_BY_SOURCE", Q_COUNT_BY_SOURCE),
        ("Q_COUNT_ORIGINALS", Q_COUNT_ORIGINALS),
        ("Q_ALL_QUOTES", Q_ALL_QUOTES),
    ] {
        let upper = text.to_uppercase();
        for forbidden in [
            "CREATE ", "MERGE ", "DELETE ", "DETACH ", " SET ", "REMOVE ", "DROP ",
        ] {
            assert!(
                !upper.contains(forbidden),
                "{name} contains {forbidden:?} — only Q_WRITE may write"
            );
        }
    }
}

#[test]
fn the_node_is_addressed_by_id_because_evidence_id_does_not_exist() {
    // Domain note: the instruction wrote `MATCH (e:Evidence {evidence_id: $id})`,
    // but STOP 0 of EVIDENCE_CORPUS_READ_v1 lists the key on all 1,209 nodes as
    // `id` and no node anywhere carries `evidence_id`. Matching on the key name the
    // instruction used would have found zero nodes and STOPped on all 76.
    assert!(Q_READ.contains("{id: $id}"));
    assert!(!Q_READ.contains("evidence_id"));
    assert!(!Q_WRITE.contains("evidence_id"));
}
