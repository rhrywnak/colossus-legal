//! SQL-shape tests for [`super`] — the configuration store's statements.
//!
//! The writes need a live Postgres and are DEV-verified, the house convention.
//! What is pinned here is the SHAPE of the production statements, read from the
//! `const`s themselves rather than from copies — the lesson of task 1.5's
//! self-asserting delete test.

use super::*;

/// A value edit may not widen its own bounds.
///
/// `value_kind`, `min_value`, `max_value`, `default_value` and `meaning` are
/// declarations about what a parameter IS. If the update touched them, a human
/// raising the talking-points cap could also — deliberately or by a bug in the
/// handler — move the ceiling that was supposed to constrain them, and the
/// bounds check would become theatre.
#[test]
fn the_update_changes_the_value_and_nothing_that_constrains_it() {
    for column in [
        "value_kind",
        "min_value",
        "max_value",
        "default_value",
        "meaning",
        "consumed_by",
    ] {
        assert!(
            !UPDATE_SETTING_SQL.contains(column),
            "the settings update touches {column}; an edit must not be able to \
             change a parameter's own declaration: {UPDATE_SETTING_SQL}"
        );
    }
    assert!(
        UPDATE_SETTING_SQL.contains("value = $2"),
        "{UPDATE_SETTING_SQL}"
    );
}

/// The update is fenced to one key.
#[test]
fn the_update_names_its_key() {
    assert!(
        UPDATE_SETTING_SQL.contains("WHERE key = $1"),
        "an unfenced UPDATE would set every parameter to one value: \
         {UPDATE_SETTING_SQL}"
    );
}

/// Every edit records who and when, on the row as well as in the ledger.
#[test]
fn the_update_stamps_the_actor_and_the_time() {
    for column in ["updated_at", "updated_by"] {
        assert!(
            UPDATE_SETTING_SQL.contains(column),
            "the settings update must write {column} so the page can show who \
             last changed a parameter without a join"
        );
    }
}

/// The ledger APPENDS. An upsert would make it a column pair with extra steps.
#[test]
fn the_change_ledger_is_append_only() {
    assert!(
        !INSERT_CHANGE_SQL.contains("ON CONFLICT"),
        "every configuration change is its own row — the history is the point: \
         {INSERT_CHANGE_SQL}"
    );
    assert!(!INSERT_CHANGE_SQL.contains("UPDATE"));
    assert!(!INSERT_CHANGE_SQL.contains("DELETE"));
}

/// A ledger row answers who, what, and both sides of the change.
#[test]
fn the_ledger_records_both_sides_and_the_actor() {
    for column in ["key", "old_value", "new_value", "actor", "at"] {
        assert!(
            INSERT_CHANGE_SQL.contains(column),
            "a change record without {column} cannot answer what changed or who \
             changed it"
        );
    }
    assert!(INSERT_CHANGE_SQL.contains("$5"));
    assert!(!INSERT_CHANGE_SQL.contains("$6"));
}

/// The projection covers exactly the fields `AppSettingRecord` decodes.
///
/// A column added to the struct without being added here fails at runtime as a
/// decode error on a live query — which this target cannot run. Comparing the two
/// lists catches it at `cargo test` instead.
#[test]
fn the_projection_matches_the_record_the_page_needs() {
    for column in [
        "key",
        "value",
        "value_kind",
        "default_value",
        "min_value",
        "max_value",
        "meaning",
        "consumed_by",
        "updated_at",
        "updated_by",
    ] {
        assert!(
            SETTING_COLUMNS.contains(column),
            "{column} is part of AppSettingRecord and must be selected"
        );
    }
}

/// Live parameters sort above dormant ones.
///
/// The grouping is a statement about the system — which knobs do anything today —
/// so it is decided here rather than by whichever client happens to render it.
#[test]
fn the_listing_puts_live_parameters_before_dormant_ones() {
    // `consumed_by IS NOT NULL` sorts false (live) before true (dormant), because
    // Postgres orders false < true.
    let sql = "ORDER BY (consumed_by IS NOT NULL), key";
    assert!(
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/repositories/pipeline_repository/app_settings.rs"),
        )
        .expect("readable")
        .contains(sql),
        "the listing must group live parameters first: {sql}"
    );
}
