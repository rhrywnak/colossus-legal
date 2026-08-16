//! The registries are the tools' only map of where graph ids live in Postgres.
//! These tests pin their membership and, in particular, pin the KNOWN GAP between
//! the shipped re-key's list and the measured full list — so the discrepancy is a
//! test that has to be edited deliberately, not a footnote in a report.
//!
//! Nothing here touches a database. `count_rows` and `repoint` are pure SQL
//! plumbing over a caller's transaction and are exercised by the tools' own
//! execution paths against DEV, not by unit tests against a fake pool.

use super::*;

#[test]
fn no_registry_lists_a_column_twice() {
    for (name, registry) in [
        ("EVIDENCE_REFERENCES", EVIDENCE_REFERENCES),
        ("EVIDENCE_CURATED_REFERENCES", EVIDENCE_CURATED_REFERENCES),
        ("PARTY_REFERENCES", PARTY_REFERENCES),
        ("SWEPT_AND_EXCLUDED", SWEPT_AND_EXCLUDED),
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

/// The eleven columns the 2026-08-15 `information_schema` sweep found, verbatim.
///
/// The registry is a MEASUREMENT, so the test compares it against the recorded
/// result of that measurement rather than against itself. A column added to the
/// schema without a re-sweep fails nothing here — nothing can catch that from
/// inside the process — but a column silently REMOVED from the registry does,
/// which is the direction the damage runs: the tools' proofs only see what the
/// registry lists.
const SWEEP_2026_08_15: &[(&str, &str)] = &[
    ("scenario_candidate_ordinals", "graph_node_id"),
    ("scan_run_verdicts", "graph_node_id"),
    ("scenario_ruling_anchors", "graph_node_id"),
    ("evidence_allegation_link_events", "graph_node_id"),
    ("scenario_fact_refs", "graph_node_id"),
    ("scenario_human_facts", "anchor_graph_node_id"),
    ("evidence_allegation_links", "graph_node_id"),
    ("scenario_human_facts", "answers_graph_node_id"),
    ("evidence_summary_overrides", "graph_node_id"),
    ("response_item_fact_refs", "graph_node_id"),
    ("extraction_items", "neo4j_node_id"),
];

#[test]
fn the_registry_is_exactly_the_measured_information_schema_sweep() {
    let mut registry: Vec<String> = EVIDENCE_REFERENCES
        .iter()
        .map(ReferencingColumn::reference)
        .collect();
    let mut swept: Vec<String> = SWEEP_2026_08_15
        .iter()
        .map(|(t, c)| format!("{t}.{c}"))
        .collect();
    registry.sort();
    swept.sort();

    assert_eq!(
        registry, swept,
        "EVIDENCE_REFERENCES no longer matches the recorded 2026-08-15 sweep. \
         Re-run the query in the module header, update SWEEP_2026_08_15 with \
         what it returns, and date it"
    );
    assert_eq!(EVIDENCE_REFERENCES.len(), 11);
}

#[test]
fn the_rekey_walks_the_entire_registry_and_has_no_exceptions() {
    // Ruled 2026-08-16, replacing a REKEY_OMITS list of three columns. There is
    // one list now, and `rekey::execute::apply_document` reads
    // EVIDENCE_REFERENCES directly — so "the re-key's list" and "the registry"
    // are the same object and cannot drift apart. This asserts the ruling stayed
    // ruled.
    assert!(
        REKEY_UPDATES_EVERYTHING,
        "the re-key must update every column in EVIDENCE_REFERENCES; \
         re-introducing an exception list needs a ruling, not a constant"
    );
}

#[test]
fn the_three_columns_the_rekey_used_to_miss_are_in_the_registry() {
    // Named individually rather than counted, because these three are the whole
    // point of the 2026-08-16 correction and a regression would most likely drop
    // exactly them.
    for (table, column) in [
        ("extraction_items", "neo4j_node_id"),
        ("evidence_summary_overrides", "graph_node_id"),
        ("response_item_fact_refs", "graph_node_id"),
    ] {
        let c = ReferencingColumn { table, column };
        assert!(
            EVIDENCE_REFERENCES.contains(&c),
            "{} was added to the re-key on 2026-08-16 and is missing again",
            c.reference()
        );
    }
}

#[test]
fn the_two_empty_curated_columns_are_treated_as_curated_not_as_provenance() {
    // They hold zero rows today. If a future edit decides "empty means it does
    // not matter" and moves them out of the curated set, the twin merge would
    // stop counting them when deciding whether a twin carries a ruling — and the
    // first summary override Roman writes would become mergeable without him.
    for (table, column) in [
        ("evidence_summary_overrides", "graph_node_id"),
        ("response_item_fact_refs", "graph_node_id"),
    ] {
        let c = ReferencingColumn { table, column };
        assert!(
            EVIDENCE_CURATED_REFERENCES.contains(&c),
            "{} must stay in the curated set even while it is empty",
            c.reference()
        );
    }
}

#[test]
fn the_swept_and_excluded_columns_are_in_no_registry() {
    // These matched the sweep by NAME and were excluded by CONTENT. If one is
    // ever added to a registry without being removed from here, the two records
    // disagree about what it holds.
    for c in SWEPT_AND_EXCLUDED {
        assert!(
            !EVIDENCE_REFERENCES.contains(c) && !PARTY_REFERENCES.contains(c),
            "{} is recorded as holding neither id family, yet a registry lists it",
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
