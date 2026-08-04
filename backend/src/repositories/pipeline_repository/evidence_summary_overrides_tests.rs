//! Tests for `evidence_summary_overrides` (task 1.7F Part B).
//!
//! These pin the SQL's SHAPE rather than its behaviour against a live database,
//! matching `scenario_human_facts_tests`: the properties that matter here —
//! upsert rather than duplicate, `created_at` frozen across a re-edit, the key
//! being the node alone, and revert being a DELETE — are all readable from the
//! statements, and reading them needs no Postgres.
//!
//! The behavioural half (a real upsert against a real table) belongs to the DEV
//! click-through, which task 1.7F closes with.

use super::*;

/// Tokens of the `SET` clause on the upsert's conflict branch.
///
/// Parsed rather than matched literally, for the reason `scenario_human_facts`
/// records: an assertion against the exact string breaks the day a column is
/// added, and breaks by reporting the wrong invariant as violated.
fn conflict_set_columns() -> Vec<String> {
    let start = UPSERT_OVERRIDE_SQL
        .find("DO UPDATE SET")
        .expect("the upsert has a conflict branch");
    UPSERT_OVERRIDE_SQL[start + "DO UPDATE SET".len()..]
        .split(',')
        .filter_map(|clause| clause.split('=').next())
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect()
}

// ─── The key is the node, and nothing else (ruling R1) ───────────────────────

/// THE CASE-WIDE TEST. One summary per piece of evidence, keyed by the node.
///
/// A `scenario_id` reaching this table would make the override per-scenario,
/// which is the two-meanings defect ruling R1 exists to prevent: the same C-code
/// reading differently depending on which scenario page you opened it from. It
/// would also compile and pass every other test here, which is why this one is
/// written down.
#[test]
fn the_override_is_keyed_by_the_evidence_node_alone() {
    assert!(
        UPSERT_OVERRIDE_SQL.contains("ON CONFLICT (graph_node_id)"),
        "the upsert's conflict target must be the node alone (ruling R1): \
         {UPSERT_OVERRIDE_SQL}"
    );
    assert!(
        !UPSERT_OVERRIDE_SQL.contains("scenario_id"),
        "a scenario column here re-creates the per-scenario summary ruling R1 \
         rejected — the same quote must read the same in every scenario: \
         {UPSERT_OVERRIDE_SQL}"
    );
    assert!(
        !OVERRIDE_COLUMNS.contains("scenario_id"),
        "the projection names a scenario column, so the table has grown one: \
         {OVERRIDE_COLUMNS}"
    );
}

// ─── The upsert's shape ──────────────────────────────────────────────────────

/// Correcting a question twice replaces the text; it does not accumulate rows.
#[test]
fn a_second_correction_replaces_the_first() {
    let set = conflict_set_columns();
    assert!(
        set.iter().any(|c| c == "summary_text"),
        "the conflict branch must replace the text, or a re-edit would silently \
         keep the older correction: {UPSERT_OVERRIDE_SQL}"
    );
    assert!(
        set.iter().any(|c| c == "authored_by"),
        "the conflict branch must replace the author: the badge names whoever \
         wrote the CURRENT text, and leaving a stale name credits the wrong \
         person: {UPSERT_OVERRIDE_SQL}"
    );
}

/// `created_at` survives a re-edit; only `updated_at` moves.
///
/// So "corrected once, months ago" and "corrected again this morning" stay
/// distinguishable — the badge shows `updated_at`, and a reader who wants to know
/// how long this text has stood has `created_at` to consult.
#[test]
fn a_re_edit_moves_updated_at_and_leaves_created_at_alone() {
    let set = conflict_set_columns();
    assert!(
        set.iter().any(|c| c == "updated_at"),
        "a re-edit must move updated_at, or the badge would report the first \
         correction's date forever: {UPSERT_OVERRIDE_SQL}"
    );
    assert!(
        !set.iter().any(|c| c == "created_at"),
        "created_at must NOT be touched on conflict — it is what makes an old \
         correction distinguishable from a fresh one: {UPSERT_OVERRIDE_SQL}"
    );
}

/// A brand-new override has equal stamps, the same way a fresh human fact does.
#[test]
fn a_new_override_has_equal_created_and_updated_stamps() {
    let start = UPSERT_OVERRIDE_SQL
        .find("VALUES (")
        .expect("the upsert has a VALUES list");
    let rest = &UPSERT_OVERRIDE_SQL[start + "VALUES (".len()..];
    let end = rest.find(')').expect("the VALUES list is closed");
    let placeholders: Vec<&str> = rest[..end].split(',').map(str::trim).collect();
    let last = placeholders.last().expect("at least one placeholder");
    let second_last = &placeholders[placeholders.len() - 2];
    assert_eq!(
        last, second_last,
        "created_at and updated_at must bind the SAME parameter on insert: \
         {UPSERT_OVERRIDE_SQL}"
    );
}

// ─── The machine original is untouchable (ruling R2) ─────────────────────────

/// THE ORIGINAL-UNTOUCHABLE TEST.
///
/// The machine's words are `Evidence.question` in Neo4j. This module must not be
/// able to reach them — not by writing the graph, and not by copying the original
/// into its own table on revert. Both would defeat ruling R2, and both would look
/// entirely reasonable in a diff, which is why the property is asserted against
/// the module's own source rather than left to review.
///
/// Standing Rule 21: an invariant that must hold across a file is asserted by
/// scanning the file.
#[test]
fn nothing_here_can_reach_the_machines_words() {
    let source = include_str!("evidence_summary_overrides.rs");

    // The graph is Cypher and a `Graph` handle. Neither belongs in a Postgres
    // repository, and their absence is what makes "never overwritten" structural.
    for forbidden in ["MATCH (", "MERGE (", "SET e.", "neo4rs", "Graph"] {
        assert!(
            !source.contains(forbidden),
            "this module names `{forbidden}` — the machine's question lives in \
             Neo4j and ruling R2 says it is untouchable. An override lives BESIDE \
             the original, never over it."
        );
    }

    // Revert is a DELETE. A revert that instead wrote the machine's text into
    // this table would leave a human-authored row holding machine words, and the
    // next reader could not tell whose sentence they were looking at.
    assert!(
        source.contains("DELETE FROM evidence_summary_overrides"),
        "revert must DELETE the override row rather than copy the system text \
         back (ruling R2)"
    );
}

/// Revert reports whether it did anything.
///
/// A revert on an item nobody had overridden is not an error, but it is not a
/// revert either, and a handler that cannot tell them apart would report a
/// restoration that never happened (Standing Rule 1).
#[test]
fn revert_returns_whether_a_row_was_actually_removed() {
    let source = include_str!("evidence_summary_overrides.rs");
    assert!(
        source.contains("Result<bool, PipelineRepoError>"),
        "delete_summary_override must return whether a row was removed, so the \
         caller can distinguish 'reverted' from 'there was nothing to revert'"
    );
    assert!(
        source.contains("rows_affected() > 0"),
        "the boolean must come from the delete's own row count, not from a \
         preceding read that could race it"
    );
}

// ─── The batch read ──────────────────────────────────────────────────────────

/// An empty pool asks the database nothing.
///
/// A scenario nobody has scanned is a real state, and `= ANY('{}')` is a round
/// trip whose only possible answer is "no rows".
#[test]
fn an_empty_node_list_short_circuits() {
    let source = include_str!("evidence_summary_overrides.rs");
    assert!(
        source.contains("if graph_node_ids.is_empty() {"),
        "list_summary_overrides must short-circuit an empty id list rather than \
         issue a query that cannot match anything"
    );
}

/// The batch read is one query, not one per card.
#[test]
fn overrides_are_read_in_one_query_for_the_whole_pool() {
    assert!(
        OVERRIDE_COLUMNS.contains("graph_node_id"),
        "the projection must carry the key, or the caller cannot map rows back \
         to cards: {OVERRIDE_COLUMNS}"
    );
    let source = include_str!("evidence_summary_overrides.rs");
    assert!(
        source.contains("graph_node_id = ANY($1)"),
        "148 cards per request means the pool's overrides are read with one \
         `= ANY`, the same shape as the sibling fact-ref and ordinal reads"
    );
}
