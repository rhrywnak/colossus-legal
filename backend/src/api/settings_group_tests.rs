//! What the coupled-group route SAYS — the three refusals and the confirmation.
//!
//! The happy path needs a live pipeline pool and is proved end to end by the
//! journey tests (`settings_journey_tests`). What is pinned here is every
//! sentence this route can produce without one, because each exists to replace a
//! bare 400 that told an operator nothing.

use super::*;

use crate::services::settings_pair::PairError;
use crate::services::settings_store::SettingsError;

/// A coupled row saved on its own is a 400 that names the editor. (M)
///
/// This is the refusal the whole task turns on. Before it, saving
/// `practice_reviewer_usernames` alone produced the boot check's own sentence —
/// "the same number of entries as practice_reviewer_display_names" — which is
/// true, unhelpful, and says nothing about the control on the same page that
/// would have worked.
#[test]
fn a_coupled_row_saved_alone_is_a_400_that_points_at_the_editor() {
    let refused = settings_error_to_app_error(SettingsError::Coupled {
        key: "practice_reviewer_usernames".to_string(),
        group: "Who may press “Done reviewing”".to_string(),
    });

    match refused {
        AppError::BadRequest { message, details } => {
            assert!(message.contains("practice_reviewer_usernames"), "{message}");
            assert!(
                message.contains("Done reviewing"),
                "names the editor: {message}"
            );
            assert!(message.contains("edited together"), "{message}");
            assert!(
                message.contains("saves them in one go"),
                "and says what to do instead: {message}"
            );
            // Group labels carry their own quotes; wrapping them nested a pair.
            assert!(!message.contains("““"), "no nested quotes: {message}");
            // The page keys on this to OPEN the editor rather than only print
            // the sentence, so it is asserted rather than left to chance.
            assert_eq!(details["reason"], "coupled_row");
        }
        other => panic!("expected a 400, got {other:?}"),
    }
}

/// A refused pair is a 400 carrying the column and the row. (M)
#[test]
fn a_refused_pair_is_a_400_naming_the_column_and_the_entry() {
    let refused = settings_error_to_app_error(SettingsError::Pair {
        source: PairError::BlankCell {
            group: "Who may press “Done reviewing”",
            column: "Name shown on screen",
            noun: "reviewer",
            position: 2,
        },
    });

    match refused {
        AppError::BadRequest { message, details } => {
            assert!(message.contains("Name shown on screen"), "{message}");
            assert!(message.contains("reviewer 2"), "names the row: {message}");
            assert_eq!(details["reason"], "invalid_pair");
        }
        other => panic!("expected a 400, got {other:?}"),
    }
}

/// An unknown group is a 404, not a 400.
///
/// Same reasoning as `UnknownKey`: the path names a resource this build does not
/// have, which is a different thing from a bad value.
#[test]
fn an_unknown_group_is_a_404_naming_it() {
    let refused = settings_error_to_app_error(SettingsError::UnknownGroup {
        id: "no_such_group".to_string(),
    });

    match refused {
        AppError::NotFound { message } => assert!(message.contains("no_such_group"), "{message}"),
        other => panic!("expected a 404, got {other:?}"),
    }
}

/// The confirmation says what it lists now, and that nothing was rebuilt.
#[test]
fn the_confirmation_counts_the_entries_and_promises_no_rebuild() {
    let bench = group_by_id("reviewer_bench").expect("declared");

    let two = confirmation(bench, 2);
    assert!(two.contains("2 reviewers"), "{two}");
    // The label carries its own quotes; wrapping it added a second pair.
    assert!(!two.contains("““"), "no nested quotes: {two}");
    // The label as the group declares it. It stopped saying "who may press" in
    // CC_TASK_REVIEW_PERMISSION_v1: the list is who the war room NAMES, and an
    // administrator may review whether or not it names them.
    assert!(two.starts_with("Reviewers shown on the war room"), "{two}");
    assert!(two.contains("no rebuild, no redeploy"), "{two}");

    // Singular is not "1 reviewers". The bench ships with one entry, so this is
    // the sentence every deployment sees first.
    let one = confirmation(bench, 1);
    assert!(one.contains("1 reviewer."), "{one}");
    assert!(!one.contains("1 reviewers"), "{one}");
}
