// Tests for `api::pipeline::ingest_dedupe`.
//
// The decisions are pure, so they are tested here without a graph.
//
// What a test CANNOT show here is that the Collapse arm actually skips the Neo4j
// write — that needs a live transaction, which this repo has no fixture for. An
// earlier draft of this comment claimed `ingest_helpers`'s tests carried that
// assertion; they do not, and the gate caught the claim. What DOES stand behind
// it: the ledger is a required parameter of `create_entity_node`, so the
// compiler forces every write loop to consult it, and the Collapse arm returns
// before any MERGE or SET statement — auditable by reading twenty lines. The
// live proof is P4 on DEV.

use super::*;

const DOC_EV: &str = "doc-george-phillips-response-to-discovery:evidence:042d8287";
const OTHER: &str = "doc-george-phillips-response-to-discovery:evidence:be12ddef";

/// THE RULING, as one assertion: two items on one id → one write, and the
/// FIRST item is the one that wrote.
#[test]
fn two_items_with_the_same_id_produce_one_write_and_the_first_wins() {
    let mut ledger = DuplicateLedger::new();

    // Items arrive in `ORDER BY id`, so 8171 is seen before 8195.
    assert_eq!(
        ledger.observe(DOC_EV, "Evidence", 8171),
        Disposition::Write,
        "the first item must write the node",
    );
    assert_eq!(
        ledger.observe(DOC_EV, "Evidence", 8195),
        Disposition::Collapse {
            first_item_id: 8171
        },
        "the second must collapse onto the first, and name it",
    );

    assert_eq!(ledger.duplicated_nodes(), 1);
    assert_eq!(ledger.collapsed_writes(), 1);
}

/// Different ids are different statements. Nothing collapses.
#[test]
fn items_with_different_ids_each_write_their_own_node() {
    let mut ledger = DuplicateLedger::new();
    assert_eq!(ledger.observe(DOC_EV, "Evidence", 1), Disposition::Write);
    assert_eq!(ledger.observe(OTHER, "Evidence", 2), Disposition::Write);
    assert_eq!(ledger.duplicated_nodes(), 0);
    assert_eq!(
        ledger.collapsed_writes(),
        0,
        "two distinct ids must not be reported as a collapse",
    );
}

/// A third item on the same id collapses too, and the count reflects all three
/// while the write count reflects the two that were skipped.
#[test]
fn a_third_item_on_one_id_collapses_and_the_two_counters_differ() {
    let mut ledger = DuplicateLedger::new();
    ledger.observe(DOC_EV, "Evidence", 10);
    ledger.observe(DOC_EV, "Evidence", 11);
    ledger.observe(DOC_EV, "Evidence", 12);

    assert_eq!(
        ledger.duplicated_nodes(),
        1,
        "one NODE carries duplicates, however many items claimed it",
    );
    assert_eq!(
        ledger.collapsed_writes(),
        2,
        "two writes were skipped — the counters answer different questions",
    );
}

/// The first writer stays the first writer no matter how many follow it, so the
/// node's fields do not drift with the number of duplicates.
#[test]
fn every_later_item_names_the_same_first_writer() {
    let mut ledger = DuplicateLedger::new();
    ledger.observe(DOC_EV, "Evidence", 8171);
    for later in [8195, 8201, 8244] {
        assert_eq!(
            ledger.observe(DOC_EV, "Evidence", later),
            Disposition::Collapse {
                first_item_id: 8171
            },
        );
    }
}

/// IDEMPOTENCE, at the level this module can assert it: a fresh ledger over the
/// same items reaches the same decisions. Ingest is cleanup-then-write, so a
/// re-ingest starts from an empty ledger and an empty graph — the same run
/// produces the same node and the same count, never a ratcheting one.
#[test]
fn a_second_run_over_the_same_items_reaches_the_same_decisions() {
    let items = [(8171, DOC_EV), (8195, DOC_EV), (8300, OTHER)];

    let run = || {
        let mut ledger = DuplicateLedger::new();
        let dispositions: Vec<Disposition> = items
            .iter()
            .map(|(id, node)| ledger.observe(node, "Evidence", *id))
            .collect();
        (
            dispositions,
            ledger.duplicated_nodes(),
            ledger.collapsed_writes(),
        )
    };

    assert_eq!(run(), run(), "two runs over identical input must agree");
    let (dispositions, nodes, writes) = run();
    assert_eq!(
        dispositions,
        vec![
            Disposition::Write,
            Disposition::Collapse {
                first_item_id: 8171
            },
            Disposition::Write,
        ],
    );
    assert_eq!((nodes, writes), (1, 1));
}

/// Ids are scoped per node, not per entity type: two different types cannot
/// collide because the id already carries the document and the type.
#[test]
fn the_ledger_keys_on_the_node_id_alone() {
    let mut ledger = DuplicateLedger::new();
    assert_eq!(ledger.observe(DOC_EV, "Evidence", 1), Disposition::Write);
    // Same id string offered under a different label would be a bug upstream;
    // the ledger still refuses the second write rather than creating a second
    // node, which is the safe direction.
    assert_eq!(
        ledger.observe(DOC_EV, "Harm", 2),
        Disposition::Collapse { first_item_id: 1 },
    );
}

/// An empty run flushes nothing and reports nothing.
#[test]
fn a_run_with_no_duplicates_reports_zero() {
    let ledger = DuplicateLedger::new();
    assert_eq!(ledger.duplicated_nodes(), 0);
    assert_eq!(ledger.collapsed_writes(), 0);
}
