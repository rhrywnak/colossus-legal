// =============================================================================
// THE COUPLED GROUPS NAME ROWS THAT EXIST, AND NOTHING ELSE CLAIMS THEM
// =============================================================================
//
// A group is a promise that these rows are edited together and never alone. Two
// ways that promise can quietly break: it names a key no longer declared
// anywhere (the editor then writes a row nothing reads), or a key belongs to two
// groups (the second editor silently overwrites the first). Neither breaks a
// build; both are checked here.

use super::*;

use crate::services::settings_map::locate;

/// Every column names a key some block on the Settings page also claims.
///
/// ## Why this matters more than it looks
///
/// A coupled group whose key no block declares would put the editor in the
/// "Undeclared — read by nothing" area, where the page labels every row dead.
/// The control would work and would sit under a heading saying it does nothing.
#[test]
fn every_coupled_column_names_a_key_a_block_declares() {
    for group in COUPLED_GROUPS {
        for column in group.columns {
            let placed = locate(column.key);
            assert_ne!(
                placed.area_id, "undeclared",
                "{} names {}, which no block declares — its editor would render \
                 in the dead-rows area",
                group.id, column.key
            );
        }
    }
}

/// A group's columns all live in ONE block.
///
/// The editor replaces the rows where they sit on the page. Columns split across
/// two blocks would mean one of them still rendering as a lone row somewhere
/// else — which is the single-row edit this task exists to remove.
#[test]
fn a_groups_columns_all_sit_in_the_same_block() {
    for group in COUPLED_GROUPS {
        let mut blocks = group
            .columns
            .iter()
            .map(|column| locate(column.key).block_id);
        let first = blocks.next().expect("a group has columns");
        for block in blocks {
            assert_eq!(
                block, first,
                "{}'s columns are split across blocks {first} and {block}",
                group.id
            );
        }
    }
}

/// No key is claimed by two groups.
#[test]
fn no_key_belongs_to_two_groups() {
    let mut seen: Vec<&str> = Vec::new();
    for group in COUPLED_GROUPS {
        for column in group.columns {
            assert!(
                !seen.contains(&column.key),
                "{} is claimed by two coupled groups; the second editor would \
                 overwrite the first",
                column.key
            );
            seen.push(column.key);
        }
    }
}

/// A group of one column is not a group.
///
/// Anti-vacuity for the whole mechanism: a "coupled" group with a single column
/// is an ordinary row wearing an editor, and every test above would pass over it
/// happily.
#[test]
fn every_group_couples_at_least_two_rows_and_says_what_it_is() {
    assert!(
        !COUPLED_GROUPS.is_empty(),
        "no coupled groups are declared; every test in this file is vacuous"
    );
    for group in COUPLED_GROUPS {
        assert!(
            group.columns.len() >= 2,
            "{} couples {} row(s) — a group of one is just a row",
            group.id,
            group.columns.len()
        );
        assert!(!group.label.trim().is_empty(), "{} has no label", group.id);
        assert!(!group.note.trim().is_empty(), "{} has no note", group.id);
        assert!(
            !group.entry_noun.trim().is_empty(),
            "{} has no name for one entry — the Add control needs it",
            group.id
        );
        for column in group.columns {
            assert!(
                !column.label.trim().is_empty(),
                "{} has an unlabelled column",
                group.id
            );
        }
    }
}

/// The reviewer bench is declared, and it names both reviewer rows.
///
/// The concrete half: the tests above would all pass against a table declaring
/// some other pair entirely.
#[test]
fn the_reviewer_bench_couples_the_two_rows_the_deadlock_was_about() {
    let bench = group_by_id("reviewer_bench").expect("the reviewer bench is declared");
    let keys: Vec<&str> = bench.columns.iter().map(|column| column.key).collect();

    assert_eq!(
        keys,
        vec![
            "practice_reviewer_usernames",
            "practice_reviewer_display_names"
        ],
        "the logins column comes first — the editor's column order is this order"
    );
    assert!(is_coupled("practice_reviewer_usernames"));
    assert!(is_coupled("practice_reviewer_display_names"));
}

/// An ordinary row is not coupled, and an unknown group is `None`.
#[test]
fn an_uncoupled_row_and_an_unknown_group_both_answer_plainly() {
    assert!(!is_coupled("talking_points_cap"));
    assert!(group_of("talking_points_cap").is_none());
    assert!(group_by_id("no_such_group").is_none());
}

// ── `entries_of` — the transpose, and the store it is meant to repair ────────
//
// Raised by the test-auditor gate: this function was reached only through the
// API assembler and a static contract fixture, so the padding branch — the one
// written for a store nobody should ever have — had no coverage at all.

use crate::repositories::pipeline_repository::AppSettingRecord;
use chrono::{TimeZone, Utc};
use std::collections::HashMap;

fn stored(key: &str, value: &str) -> AppSettingRecord {
    AppSettingRecord {
        key: key.to_string(),
        value: value.to_string(),
        value_kind: "text".to_string(),
        default_value: value.to_string(),
        min_value: None,
        max_value: None,
        meaning: "A stored list.".to_string(),
        consumed_by: None,
        updated_at: Utc
            .with_ymd_and_hms(2026, 9, 20, 12, 0, 0)
            .single()
            .expect("a real timestamp"),
        updated_by: "migration".to_string(),
    }
}

fn store(logins: &str, names: &str) -> HashMap<String, AppSettingRecord> {
    let mut rows = HashMap::new();
    rows.insert(
        "practice_reviewer_usernames".to_string(),
        stored("practice_reviewer_usernames", logins),
    );
    rows.insert(
        "practice_reviewer_display_names".to_string(),
        stored("practice_reviewer_display_names", names),
    );
    rows
}

fn bench() -> &'static CoupledGroup {
    group_by_id("reviewer_bench").expect("declared")
}

/// Two lists become rows of cells — the transpose the editor draws.
#[test]
fn aligned_rows_become_one_entry_per_reviewer() {
    let entries = entries_of(bench(), &store("cpenzien,roman", "Chuck,Roman"));

    assert_eq!(
        entries,
        vec![
            vec!["cpenzien".to_string(), "Chuck".to_string()],
            vec!["roman".to_string(), "Roman".to_string()],
        ]
    );
}

/// The shipped single reviewer transposes to one entry.
#[test]
fn one_reviewer_is_one_entry() {
    assert_eq!(
        entries_of(bench(), &store("cpenzien", "Chuck")),
        vec![vec!["cpenzien".to_string(), "Chuck".to_string()]]
    );
}

/// ⚑ A MISALIGNED store is padded so the editor can DRAW the gap. (M)
///
/// This state cannot normally exist — the boot check refuses it, so the process
/// would not be running. If one ever does (a hand-edited database, a restore),
/// this control is the tool that repairs it, and it can only repair what it can
/// show. Dropping the short column's missing cell would hide the defect behind
/// a table that looked complete.
#[test]
fn a_misaligned_store_is_padded_rather_than_truncated() {
    let entries = entries_of(bench(), &store("cpenzien,roman,docmarie", "Chuck"));

    assert_eq!(entries.len(), 3, "the LONGEST column decides the row count");
    assert_eq!(
        entries[0],
        vec!["cpenzien".to_string(), "Chuck".to_string()]
    );
    // The two reviewers with no name arrive with an empty cell the operator can
    // see and fill — and `encode_entries` refuses to save it blank.
    assert_eq!(entries[1], vec!["roman".to_string(), String::new()]);
    assert_eq!(entries[2], vec!["docmarie".to_string(), String::new()]);
}

/// A store missing the rows entirely yields no entries rather than panicking.
#[test]
fn an_empty_store_yields_no_entries() {
    assert!(entries_of(bench(), &HashMap::new()).is_empty());
}

/// A row the reader cannot parse shows as empty rather than vanishing.
///
/// `none` is the store's word for a deliberately empty list, so a bench set to
/// it reads back as zero entries — which the editor then shows as an empty
/// table with an Add button, not as a missing control.
#[test]
fn the_none_token_reads_back_as_an_empty_table() {
    assert!(entries_of(bench(), &store("none", "none")).is_empty());
}
