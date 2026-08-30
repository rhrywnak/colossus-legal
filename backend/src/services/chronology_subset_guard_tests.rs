//! Tests for `services::chronology_subset_guard`.
//!
//! The seal itself needs a live transaction and so is proved by
//! `tests/timeline_subsets_integration.rs`. What is reachable here is the half
//! that would fail SILENTLY: the action vocabulary drifting from the migration's
//! CHECK, and a snapshot that quietly stopped carrying the ordered event list —
//! which would leave every reorder recorded as a history row saying nothing
//! changed.

use super::*;

use chrono::{TimeZone, Utc};

/// The migration whose CHECK constrains `chronology_subset_history.action`.
// STRUCTURAL: a repo-internal pointer to one immutable, version-controlled
// migration. Identical in every environment; nothing here can vary by deployment.
const ACTION_MIGRATION: &str = "pipeline_migrations/20260830122249_timeline_subsets.sql";

fn subset_row() -> ChronologySubsetRow {
    ChronologySubsetRow {
        id: Uuid::from_bytes([7; 16]),
        case_slug: "awad_v_catholic_family_service".to_string(),
        name: "The $50,000".to_string(),
        description: "What the money did.".to_string(),
        created_by: "roman".to_string(),
        created_at: Utc.with_ymd_and_hms(2026, 8, 30, 9, 0, 0).unwrap(),
        updated_by: "marie".to_string(),
        updated_at: Utc.with_ymd_and_hms(2026, 8, 30, 10, 0, 0).unwrap(),
        deleted_at: None,
    }
}

fn reference(event: u8, position: i32, note: &str) -> SubsetEventRefRow {
    SubsetEventRefRow {
        subset_id: Uuid::from_bytes([7; 16]),
        event_id: Uuid::from_bytes([event; 16]),
        position,
        note: note.to_string(),
    }
}

#[test]
fn the_action_words_match_the_migrations_check() {
    // ⚑ THE DRIFT THIS FILE EXISTS FOR. A variant renamed in Rust and not in the
    // CHECK compiles, serves, and fails at the database on the first write that
    // uses it — a 500 for a word a test could have caught. Read off disk (Rule
    // 21) rather than restated, so the two lists cannot be edited apart.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(ACTION_MIGRATION);
    let sql = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} is not on disk: {e}", path.display()));

    for action in SubsetHistoryAction::ALL {
        let quoted = format!("'{}'", action.as_str());
        assert!(
            sql.contains(&quoted),
            "{quoted} is spellable in Rust and is not in the migration's CHECK"
        );
    }

    // And the other direction, which the loop above cannot see: a word in the
    // CHECK that no variant spells would be a state the database allows and this
    // build can never write — dead vocabulary that reads as a feature.
    let check = sql
        .split("chronology_subset_history_action_valid")
        .nth(2)
        .expect("the ADD CONSTRAINT statement is in the file");
    let clause = &check[..check.find(");").expect("the CHECK list closes")];
    let in_sql = clause.matches('\'').count() / 2;
    assert_eq!(
        in_sql,
        SubsetHistoryAction::ALL.len(),
        "the CHECK lists {in_sql} actions and Rust spells {}",
        SubsetHistoryAction::ALL.len()
    );
}

#[test]
fn every_action_word_is_distinct() {
    // Two variants sharing a token would make two different acts indistinguishable
    // in history — a delete that read as an edit.
    let mut words: Vec<&str> = SubsetHistoryAction::ALL
        .iter()
        .map(|a| a.as_str())
        .collect();
    words.sort_unstable();
    let before = words.len();
    words.dedup();
    assert_eq!(before, words.len(), "two actions share a stored word");
}

#[test]
fn the_snapshot_carries_the_ordered_event_list() {
    // ⚑ The quietest failure this feature has. A snapshot holding only the
    // subset's columns records every rename perfectly and records NOTHING about
    // the reorder that is the whole point of the feature — and it would look
    // completely healthy: history rows appearing, one per write, each with an
    // author and a time.
    let value = snapshot_of(
        &subset_row(),
        &[
            reference(1, 1, "the transfer"),
            reference(2, 2, ""),
            reference(3, 3, "the check"),
        ],
    );
    let events = value["events"].as_array().expect("an events array");
    assert_eq!(events.len(), 3);
    assert_eq!(events[0]["position"], 1);
    assert_eq!(events[0]["note"], "the transfer");
    assert_eq!(
        events[2]["event_id"],
        serde_json::json!(Uuid::from_bytes([3; 16]))
    );
}

#[test]
fn the_snapshot_copies_no_event_content() {
    // ⚑ Design §4 arriving through the back door. A snapshot that carried titles
    // and dates would be a copy of the chronology frozen in a history row, and
    // an edit to the event would leave the two disagreeing forever — with the
    // history looking authoritative, because history is what people trust.
    let value = snapshot_of(&subset_row(), &[reference(1, 1, "the transfer")]);
    let event = &value["events"][0];
    for forbidden in ["title", "event_date", "fact", "phase", "date_precision"] {
        assert!(
            event.get(forbidden).is_none(),
            "the snapshot carries a copied event field: {forbidden}"
        );
    }
    // Exactly the three reference columns, so a field added later is a visible
    // change here rather than a quiet one.
    let keys: Vec<&String> = event.as_object().expect("an object").keys().collect();
    assert_eq!(
        keys.len(),
        3,
        "the reference snapshot grew a field: {keys:?}"
    );
}

#[test]
fn the_snapshot_carries_deleted_at_so_a_delete_and_a_restore_differ_by_content() {
    // Not only by their action word: two adjacent snapshots must be diffable, and
    // the whole record of a soft delete is this one field.
    let mut deleted = subset_row();
    deleted.deleted_at = Some(Utc.with_ymd_and_hms(2026, 8, 30, 11, 0, 0).unwrap());

    let live = snapshot_of(&subset_row(), &[]);
    let gone = snapshot_of(&deleted, &[]);
    assert!(live["deleted_at"].is_null());
    assert!(!gone["deleted_at"].is_null());
    assert_ne!(live, gone);
}

#[test]
fn a_subset_with_no_events_snapshots_an_empty_list_not_a_missing_key() {
    // An absent key and an empty array are two different things to a reader of
    // history: "this build did not record the events" versus "there were none".
    let value = snapshot_of(&subset_row(), &[]);
    assert_eq!(value["events"], serde_json::json!([]));
}
