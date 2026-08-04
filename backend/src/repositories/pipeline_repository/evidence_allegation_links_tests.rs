//! Tests for `evidence_allegation_links` (task 2.10).
//!
//! These pin the SQL's SHAPE rather than its behaviour against a live database,
//! matching `evidence_summary_overrides_tests` and `scenario_human_facts_tests`:
//! the properties that matter here — case-wide keying, upsert rather than
//! duplicate, `created_at` frozen across a re-cut, the ledger's cut being NULL on
//! an unlink and only there — are all readable from the statements, and reading
//! them needs no Postgres.
//!
//! The behavioural half (a real upsert against a real table) belongs to the DEV
//! click-through this task closes with.

use super::*;

/// Tokens of the `SET` clause on the upsert's conflict branch.
///
/// Parsed rather than matched literally, for the reason `scenario_human_facts`
/// records: an assertion against the exact string breaks the day a column is
/// added, and breaks by reporting the wrong invariant as violated.
fn conflict_set_columns() -> Vec<String> {
    let start = UPSERT_LINK_SQL
        .find("DO UPDATE SET")
        .expect("the upsert has a conflict branch");
    UPSERT_LINK_SQL[start + "DO UPDATE SET".len()..]
        .split(',')
        .filter_map(|clause| clause.split('=').next())
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect()
}

// ─── The key is the pair, and the scope is the case ──────────────────────────

/// THE CASE-WIDE TEST. A link belongs to the statement, not to a scenario.
///
/// A `scenario_id` reaching this table would make a statement bear on ¶41 in one
/// scenario and not in another — the two-meanings defect ruling R1 rejected for
/// summaries, in the place it would do more damage, because this one decides what
/// a human is allowed to rule. It would also compile and pass every other test
/// here, which is why this one is written down.
#[test]
fn a_link_is_keyed_by_the_statement_and_the_accusation_alone() {
    assert!(
        UPSERT_LINK_SQL.contains("ON CONFLICT (graph_node_id, allegation_id)"),
        "the conflict target must be the pair: {UPSERT_LINK_SQL}"
    );
    assert!(
        !UPSERT_LINK_SQL.contains("scenario_id"),
        "a scenario column here makes a link mean different things on different \
         pages: {UPSERT_LINK_SQL}"
    );
    assert!(
        !LINK_COLUMNS.contains("scenario_id"),
        "the projection names a scenario column, so the table has grown one: \
         {LINK_COLUMNS}"
    );
    assert!(
        !INSERT_LINK_EVENT_SQL.contains("scenario_id"),
        "the ledger must be case-wide too: {INSERT_LINK_EVENT_SQL}"
    );
}

// ─── The upsert's shape ──────────────────────────────────────────────────────

/// Linking the same pair again re-cuts it; it never stores a second row.
///
/// Two rows for one pair would be a statement that both supports us and is used
/// against us on the same accusation — a contradiction the card would then have
/// to render.
#[test]
fn re_linking_a_pair_updates_it_rather_than_duplicating_it() {
    assert!(UPSERT_LINK_SQL.contains("INSERT INTO evidence_allegation_links"));
    assert!(UPSERT_LINK_SQL.contains("DO UPDATE SET"));

    let updated = conflict_set_columns();
    assert!(
        updated.iter().any(|c| c == "cut"),
        "a re-link must change the cut: {updated:?}"
    );
    assert!(
        updated.iter().any(|c| c == "updated_at"),
        "a re-link must move updated_at: {updated:?}"
    );
    assert!(
        updated.iter().any(|c| c == "authored_by"),
        "the person who re-cut it is the person the row now names: {updated:?}"
    );
}

/// A re-cut keeps the date the link was first made.
///
/// "Linked in June" and "re-cut this morning" are different facts about the same
/// pair, and touching `created_at` on conflict would collapse them.
#[test]
fn a_re_cut_does_not_move_the_original_date() {
    let updated = conflict_set_columns();
    assert!(
        !updated.iter().any(|c| c == "created_at"),
        "created_at must survive a re-cut: {updated:?}"
    );
}

/// The upsert reports which of the two things it did, without a second query.
///
/// `xmax <> 0` is 0 on a freshly inserted row and non-zero on one this statement
/// updated. Reading it in the RETURNING clause is what lets the ledger record a
/// `link` or a `recut` correctly — a preceding SELECT would be a race another
/// request could interleave with, and the ledger would then record the wrong act.
#[test]
fn the_upsert_reports_whether_the_pair_was_already_linked() {
    assert!(
        UPSERT_LINK_SQL.contains("RETURNING (xmax <> 0)"),
        "the insert/update distinction must come from the statement itself: \
         {UPSERT_LINK_SQL}"
    );
}

// ─── The ledger ──────────────────────────────────────────────────────────────

/// Every act records who did it and when.
#[test]
fn the_ledger_records_the_actor_and_the_moment() {
    assert!(INSERT_LINK_EVENT_SQL.contains("actor"));
    assert!(INSERT_LINK_EVENT_SQL.contains("at"));
    assert!(INSERT_LINK_EVENT_SQL.contains("action"));
}

/// The ledger is append-only: it is never updated and never deleted from.
///
/// The point of the record is that it survives the decision. An UPDATE reaching
/// this table would let a later act rewrite the history of an earlier one.
#[test]
fn nothing_in_this_module_updates_or_deletes_the_ledger() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/repositories/pipeline_repository/evidence_allegation_links.rs"),
    )
    .expect("this module is on disk");

    // Strip comment lines first: the prose above discusses updates and deletes,
    // and a raw scan would report the doc comments as violations (the lesson
    // `write_targets` in scenario_human_facts_tests records).
    let code: String = source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with('*'))
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !code.contains("UPDATE evidence_allegation_link_events"),
        "the ledger is append-only"
    );
    assert!(
        !code.contains("DELETE FROM evidence_allegation_link_events"),
        "the ledger is append-only"
    );
}

/// The unlink removes the state row and leaves the ledger holding the story.
///
/// This is the whole reason the ledger exists: after an unlink there is nothing
/// in the state table, so a link made and withdrawn would otherwise be
/// indistinguishable from one never made.
#[test]
fn the_unlink_deletes_the_pair_and_appends_an_event() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/repositories/pipeline_repository/evidence_allegation_links.rs"),
    )
    .expect("this module is on disk");

    let body = source
        .split_once("pub async fn delete_link(")
        .map(|(_, rest)| rest)
        .and_then(|rest| rest.split_once("\n}"))
        .map(|(body, _)| body)
        .expect("delete_link exists");

    assert!(
        body.contains("DELETE FROM evidence_allegation_links"),
        "the state row must go: {body}"
    );
    assert!(
        body.contains("INSERT_LINK_EVENT_SQL"),
        "the withdrawal must be recorded: {body}"
    );
    assert!(
        body.contains("LinkAction::Unlink"),
        "and recorded AS a withdrawal: {body}"
    );
}

/// Both writes commit together, or neither does.
///
/// A state row with no event is a link nobody is recorded as having made; an
/// event with no row is a record of a decision that did not take effect.
#[test]
fn both_writers_are_transactional() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/repositories/pipeline_repository/evidence_allegation_links.rs"),
    )
    .expect("this module is on disk");

    for name in ["pub async fn save_link(", "pub async fn delete_link("] {
        let body = source
            .split_once(name)
            .map(|(_, rest)| rest)
            .and_then(|rest| rest.split_once("\n}"))
            .map(|(body, _)| body)
            .unwrap_or_else(|| panic!("{name} exists"));
        assert!(
            body.contains("pool.begin()"),
            "{name} must open a transaction"
        );
        assert!(body.contains("tx.commit()"), "{name} must commit it");
    }
}

// ─── The batch read ──────────────────────────────────────────────────────────

/// An empty pool costs no round trip and returns no rows.
#[tokio::test]
async fn an_empty_id_list_short_circuits() {
    // No pool is touched: the guard returns before the query is built. Passing a
    // pool that would panic if used is not possible here, so the assertion is on
    // the outcome — the function is total for the empty case.
    let ids: Vec<String> = Vec::new();
    assert!(ids.is_empty(), "the fixture is the empty case");
    // The guard is the first statement of the function; pin it textually, since
    // calling it needs a live pool.
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/repositories/pipeline_repository/evidence_allegation_links.rs"),
    )
    .expect("this module is on disk");
    let body = source
        .split_once("pub async fn list_links_for_nodes(")
        .and_then(|(_, rest)| rest.split_once("\n}"))
        .map(|(body, _)| body)
        .expect("list_links_for_nodes exists");
    assert!(
        body.contains("graph_node_ids.is_empty()"),
        "an empty pool must not make a round trip: {body}"
    );
}

/// The batch read is deterministically ordered.
///
/// Without an ORDER BY, two reads of the same unchanged data could return a
/// statement's accusations in different orders — and the card's chips and its
/// composed sentence would shuffle between page loads for no reason a reader
/// could explain.
#[test]
fn the_batch_read_is_ordered() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/repositories/pipeline_repository/evidence_allegation_links.rs"),
    )
    .expect("this module is on disk");
    assert!(
        source.contains("ORDER BY graph_node_id, created_at, allegation_id"),
        "the pool read must be deterministically ordered"
    );
}
