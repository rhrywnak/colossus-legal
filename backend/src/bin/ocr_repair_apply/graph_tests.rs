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

/// Split every `e.<name>` in a statement into the ones being ASSIGNED (the name
/// is followed by ` =`) and the ones merely READ.
///
/// ## Rust Learning: `split` and `char::is_alphanumeric` instead of a regex crate
///
/// The property names are `[a-z_]+`, so taking characters while they are
/// alphanumeric-or-underscore is a complete parser for this one job. Pulling in a
/// regex dependency for a test that reads six fixed strings would cost a compile
/// unit to express the same thing less clearly.
fn properties(statement: &str) -> (Vec<String>, Vec<String>) {
    let (mut assigned, mut read) = (Vec::new(), Vec::new());
    for piece in statement.split("e.").skip(1) {
        let name: String = piece
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if name.is_empty() {
            continue;
        }
        let rest = piece[name.len()..].trim_start();
        if rest.starts_with('=') {
            assigned.push(name);
        } else {
            read.push(name);
        }
    }
    (assigned, read)
}

#[test]
fn the_write_sets_only_the_five_authorised_properties() {
    let (mut assigned, read) = properties(Q_WRITE);
    assigned.sort();
    let mut expected: Vec<String> = AUTHORISED.iter().map(|p| p.to_string()).collect();
    expected.sort();
    assert_eq!(
        assigned, expected,
        "the write statement assigns {assigned:?}, not the five authorised properties"
    );
    // The ONE property the statement is allowed to read is the original it must
    // not overwrite — that read IS the `coalesce` guard. Anything else appearing
    // on the read side would mean the statement grew a condition nobody reviewed.
    assert_eq!(
        read,
        vec!["verbatim_quote_ocr_original".to_string()],
        "the write statement reads {read:?}; only the coalesce guard may read"
    );
}

#[test]
fn the_write_never_overwrites_an_earlier_rounds_original() {
    // Domain note: v1a corrects sixteen cards v1 already corrected. Without the
    // `coalesce` the second write would store v1's OUTPUT as the "OCR original"
    // and the real pre-repair text — the only copy of what Surya produced —
    // would be gone. The other four properties must stay unconditional.
    assert!(Q_WRITE.contains("coalesce(e.verbatim_quote_ocr_original, $original)"));
    // And it is the ONLY conditional write. A prefix check would not do the job —
    // `coalesce(e.verbatim_quote_ocr_original` starts with `coalesce(e.verbatim_quote`
    // — so the statement is required to contain exactly one `coalesce` at all,
    // which together with the line above pins which property it guards.
    assert_eq!(
        Q_WRITE.matches("coalesce(").count(),
        1,
        "only the pre-repair original may be written conditionally: {Q_WRITE}"
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
        ("Q_VERIFY_THIS_RUN", Q_VERIFY_THIS_RUN),
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
