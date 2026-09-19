//! What the Settings PAGE is told about grouping (CC_TASK_ADMIN_SETTINGS_PAGE_v1).
//!
//! `settings_tests` pins what one parameter says about ITSELF — its bounds
//! sentence, its dormancy note, its hint. This module pins what the page is told
//! about the SHAPE of the store: which rows have moved off their defaults, and
//! how many rows each area and block actually holds.
//!
//! Split from its sibling rather than appended to it because that file reached
//! Rule 17's limit, and because these are a different subject: not "what is this
//! parameter", but "where does it go and how many are there".

use super::*;

use chrono::{TimeZone, Utc};

/// A record with the key, value and default the caller cares about.
///
/// Deliberately self-contained rather than built on `settings_tests`' own
/// fixture: two test modules sharing one fixture is how a change made for one
/// of them quietly rewrites the other's inputs.
fn keyed(key: &str, value: &str, default_value: &str) -> AppSettingRecord {
    AppSettingRecord {
        key: key.to_string(),
        value: value.to_string(),
        value_kind: "text".to_string(),
        default_value: default_value.to_string(),
        min_value: None,
        max_value: None,
        meaning: "What this parameter does.".to_string(),
        consumed_by: None,
        updated_at: Utc
            .with_ymd_and_hms(2026, 8, 1, 12, 0, 0)
            .single()
            .expect("a real timestamp"),
        updated_by: "roman".to_string(),
    }
}

#[test]
fn a_row_that_has_moved_off_its_default_says_so_and_names_it() {
    let dto = to_dto(&keyed("practice_read_max_words", "40", "25"));
    assert_eq!(
        dto.changed_from_default.as_deref(),
        Some("Changed — default: 25"),
        "the phrase must carry the old value: 'changed' without saying changed \
         FROM WHAT leaves a human no way to put it back"
    );
}

#[test]
fn a_row_sitting_on_its_default_carries_no_changed_phrase() {
    let dto = to_dto(&keyed("talking_points_cap", "3", "3"));
    assert!(
        dto.changed_from_default.is_none(),
        "a row on its default is not in the landing list"
    );
}

#[test]
fn every_row_carries_the_area_and_block_that_claim_it() {
    let dto = to_dto(&keyed("talking_points_cap", "3", "3"));
    assert_eq!(dto.area_id, "core");
    assert_eq!(dto.block_id, "core");

    let stray = to_dto(&keyed("practice_notes_save_label", "Save", "Save"));
    assert_eq!(stray.area_id, UNDECLARED_AREA_ID);
    assert_eq!(stray.block_id, UNDECLARED_BLOCK_ID);
}

#[test]
fn the_rail_counts_the_rows_that_arrived_not_the_keys_declared() {
    // Two Core rows arrive. `REQUIRED_KEYS` declares thirty-one.
    let areas = summarise(&[
        keyed("talking_points_cap", "3", "3"),
        keyed("chat_default_model", "claude-opus-5", "claude-opus-5"),
    ]);
    let core = areas
        .iter()
        .find(|area| area.id == "core")
        .expect("Core is always in the rail");
    assert_eq!(
        core.count, 2,
        "the rail must count the STORED rows; counting the declared keys would \
         promise rows the page cannot show"
    );
    assert_eq!(core.blocks.len(), 1);
    assert_eq!(core.blocks[0].count, 2);
}

#[test]
fn an_area_with_no_stored_rows_is_shown_at_zero_rather_than_hidden() {
    let areas = summarise(&[keyed("talking_points_cap", "3", "3")]);
    let matrix = areas.iter().find(|area| area.id == "matrix").expect(
        "a declared area stays in the rail at zero — a block the store \
                 has not been seeded with is exactly what someone needs to see",
    );
    assert_eq!(matrix.count, 0);
}

#[test]
fn rows_no_block_declares_get_their_own_area_last_carrying_its_note() {
    let areas = summarise(&[
        keyed("talking_points_cap", "3", "3"),
        keyed("practice_notes_save_label", "Save", "Save"),
        keyed("practice_row_review_link", "Review", "Review"),
    ]);
    let last = areas.last().expect("the rail is never empty");
    assert_eq!(last.id, UNDECLARED_AREA_ID);
    assert_eq!(last.count, 2);
    assert_eq!(last.label, UNDECLARED_AREA_LABEL);
    assert!(
        last.note
            .as_deref()
            .is_some_and(|note| note.contains("dead")),
        "the group says what it is: the ruling of 2026-09-19 is that these rows \
         are SHOWN and labelled dead, not hidden"
    );
}

#[test]
fn the_undeclared_area_is_absent_when_every_row_is_declared() {
    let areas = summarise(&[keyed("talking_points_cap", "3", "3")]);
    assert!(
        !areas.iter().any(|area| area.id == UNDECLARED_AREA_ID),
        "an empty 'read by nothing' heading accuses a store with nothing wrong \
         with it"
    );
}

#[test]
fn the_rail_is_every_declared_area_in_the_order_the_map_declares_them() {
    let areas = summarise(&[]);
    let ids: Vec<&str> = areas.iter().map(|area| area.id.as_str()).collect();
    let expected: Vec<&str> = AREAS.iter().map(|area| area.id).collect();
    assert_eq!(
        ids, expected,
        "the order is the shape of the product and is decided in settings_map"
    );
}

/// The WARN names the rows, not just how many there are. (M)
///
/// Ruled by the observability gate on 2026-09-19: "13 undeclared" tells an
/// operator that something is wrong and then sends them to the admin UI to find
/// out what, which makes the log entry a prompt to investigate rather than
/// something a retirement task can be written from.
#[test]
fn the_undeclared_warning_names_every_row_it_counts() {
    let records = [
        keyed("talking_points_cap", "3", "3"),
        keyed("practice_notes_save_label", "Save", "Save"),
        keyed("practice_row_review_link", "Review", "Review"),
    ];

    assert_eq!(
        undeclared_keys(&records),
        vec!["practice_notes_save_label", "practice_row_review_link"],
        "the log line must carry the keys, in the order the store served them"
    );
}

/// A store where every row is declared logs nothing. (M)
///
/// The other half: a filter that named every row would make the WARN fire on
/// every page load of a perfectly healthy store, which is how a warning stops
/// being read.
#[test]
fn a_store_whose_rows_are_all_declared_has_nothing_to_warn_about() {
    assert!(undeclared_keys(&[keyed("talking_points_cap", "3", "3")]).is_empty());
}
