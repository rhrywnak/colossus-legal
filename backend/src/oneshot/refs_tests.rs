//! The registries are the tools' only map of where graph ids live in Postgres.
//! These tests pin their membership and, in particular, pin the KNOWN GAP between
//! the shipped re-key's list and the measured full list — so the discrepancy is a
//! test that has to be edited deliberately, not a footnote in a report.
//!
//! Nothing here touches a database. `count_rows` and `repoint` are pure SQL
//! plumbing over a caller's transaction and are exercised by the tools' own
//! execution paths against DEV, not by unit tests against a fake pool.

use super::*;
use crate::rekey::execute::REFERENCING_COLUMNS as REKEY_COLUMNS;

#[test]
fn no_registry_lists_a_column_twice() {
    for (name, registry) in [
        ("EVIDENCE_REFERENCES", EVIDENCE_REFERENCES),
        ("EVIDENCE_CURATED_REFERENCES", EVIDENCE_CURATED_REFERENCES),
        ("PARTY_REFERENCES", PARTY_REFERENCES),
        ("REKEY_OMITS", REKEY_OMITS),
    ] {
        let mut seen: Vec<String> = registry.iter().map(ReferencingColumn::reference).collect();
        let before = seen.len();
        seen.sort();
        seen.dedup();
        assert_eq!(
            seen.len(),
            before,
            "{name} lists a column twice, so its counts would be doubled in the proof"
        );
    }
}

#[test]
fn curated_references_are_a_subset_of_all_evidence_references() {
    for c in EVIDENCE_CURATED_REFERENCES {
        assert!(
            EVIDENCE_REFERENCES.contains(c),
            "{} is treated as curated but is not in EVIDENCE_REFERENCES, so a \
             merge would test it for rulings and then never repoint it",
            c.reference()
        );
    }
}

#[test]
fn extraction_items_is_the_one_evidence_reference_that_is_not_curated() {
    // Domain note: this asserts the distinction the twin-merge turns on. A twin
    // carrying only an extraction_items row is NOT curated and merges
    // mechanically; a twin carrying a scenario_fact_refs row is Roman's ruling
    // and goes to the human queue. If a future column is added to
    // EVIDENCE_REFERENCES without a deliberate decision about which side it
    // falls on, this test fails and forces the decision.
    let uncurated: Vec<String> = EVIDENCE_REFERENCES
        .iter()
        .filter(|c| !EVIDENCE_CURATED_REFERENCES.contains(c))
        .map(ReferencingColumn::reference)
        .collect();
    assert_eq!(
        uncurated,
        vec!["extraction_items.neo4j_node_id".to_string()],
        "the set of non-curated Evidence references changed; decide explicitly \
         whether the new column is a human ruling before updating this test"
    );
}

#[test]
fn the_shipped_rekey_covers_all_but_the_three_recorded_omissions() {
    // The re-key (39a8ba8) walks eight columns. The full measured list is
    // eleven. This test states the gap in one place so it cannot drift into
    // being forgotten: if someone extends the re-key, this fails and they must
    // shrink REKEY_OMITS to match.
    let rekey: Vec<String> = REKEY_COLUMNS
        .iter()
        .map(|(t, c)| format!("{t}.{c}"))
        .collect();

    let mut missing: Vec<String> = EVIDENCE_REFERENCES
        .iter()
        .map(ReferencingColumn::reference)
        .filter(|r| !rekey.contains(r))
        .collect();
    missing.sort();

    let mut recorded: Vec<String> = REKEY_OMITS
        .iter()
        .map(ReferencingColumn::reference)
        .collect();
    recorded.sort();

    assert_eq!(
        missing, recorded,
        "the columns rekey_evidence does NOT update no longer match REKEY_OMITS. \
         Update REKEY_OMITS (and the runbook's expected row count) deliberately"
    );
}

#[test]
fn every_rekey_column_is_in_the_measured_registry() {
    // The other direction: the re-key must never update a column the merge tools
    // have never heard of, or a merge would leave rows the re-key moved.
    for (table, column) in REKEY_COLUMNS {
        let c = ReferencingColumn { table, column };
        assert!(
            EVIDENCE_REFERENCES.contains(&c),
            "rekey_evidence updates {} but EVIDENCE_REFERENCES does not list it",
            c.reference()
        );
    }
}

#[test]
fn a_reference_renders_as_table_dot_column() {
    // The proof lines and the abort reasons are matched on this exact form.
    assert_eq!(
        col("scenario_fact_refs", "graph_node_id").reference(),
        "scenario_fact_refs.graph_node_id"
    );
}

#[test]
fn party_references_names_the_one_measured_column() {
    // Measured 2026-08-15: no curated Postgres table references a party id. If
    // that changes, this test is the thing that notices.
    let refs: Vec<String> = PARTY_REFERENCES
        .iter()
        .map(ReferencingColumn::reference)
        .collect();
    assert_eq!(refs, vec!["extraction_items.neo4j_node_id".to_string()]);
}

// ── TableProof — the abort gate all four tools share ─────────────────────────

#[test]
fn a_proof_is_sound_only_on_exact_equality() {
    let proof = |expected, updated| TableProof {
        reference: "scenario_fact_refs.graph_node_id".to_string(),
        expected,
        updated,
    };
    assert!(proof(3, 3).is_sound());
    assert!(
        proof(0, 0).is_sound(),
        "a column that moved nothing is still sound"
    );
    assert!(!proof(3, 2).is_sound(), "a short update must abort");
    assert!(
        !proof(3, 4).is_sound(),
        "an update that matched MORE than the plan knew about must abort too — \
         that is why the check is != and not <"
    );
}

#[test]
fn table_proofs_walks_the_registry_not_the_maps() {
    // A column absent from both counts still gets a 0/0 line. An omitted line
    // would be a silence where the proof needs a claim.
    let empty = HashMap::new();
    let proofs = table_proofs(PARTY_REFERENCES, &empty, &empty);

    assert_eq!(proofs.len(), PARTY_REFERENCES.len());
    assert_eq!(proofs[0].reference, "extraction_items.neo4j_node_id");
    assert_eq!((proofs[0].expected, proofs[0].updated), (0, 0));
    assert!(proofs[0].is_sound());
}

#[test]
fn table_proofs_pairs_each_column_with_its_own_counts() {
    let mut expected = HashMap::new();
    expected.insert("extraction_items.neo4j_node_id".to_string(), 4u64);
    let mut updated = HashMap::new();
    updated.insert("extraction_items.neo4j_node_id".to_string(), 3u64);

    let proofs = table_proofs(PARTY_REFERENCES, &expected, &updated);
    assert_eq!((proofs[0].expected, proofs[0].updated), (4, 3));
    assert!(!proofs[0].is_sound());
}
