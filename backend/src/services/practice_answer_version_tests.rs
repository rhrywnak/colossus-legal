// Tests for `services::practice_answer_version`.

use super::is_reread;

/// The case the ruling exists for: Stop waiting, then Answer again.
#[test]
fn pressing_answer_twice_on_the_same_words_is_a_re_read() {
    assert!(is_reread(
        Some("nothing stopped them"),
        "nothing stopped them"
    ));
}

/// The case a version is FOR.
#[test]
fn changing_one_character_is_a_new_version() {
    assert!(!is_reread(
        Some("nothing stopped them"),
        "nothing stopped them."
    ));
}

/// The first answer is always a version — there is nothing to re-read.
#[test]
fn the_first_answer_is_a_version() {
    assert!(!is_reread(None, "her first words"));
}

/// Whitespace is a CHANGE, and the doc says why.
///
/// Trimmed-equal would discard an edit whose whole content is whitespace, with
/// the screen reporting success — the worst shape a failure can have here.
#[test]
fn a_whitespace_only_edit_is_a_change_and_not_swallowed() {
    assert!(!is_reread(Some("her words"), "her words "));
    assert!(!is_reread(Some("her words"), " her words"));
    assert!(!is_reread(Some("her words"), "her  words"));
}

/// Case is a change too — she may be fixing a proper noun.
#[test]
fn a_case_change_is_a_change() {
    assert!(!is_reread(
        Some("catholic family service"),
        "Catholic Family Service"
    ));
}

/// An empty box against an empty stored answer is still a re-read.
///
/// The empty answer is a real stored state — `practice_answer_empty_hint`
/// exists — so this arm is reachable, and treating it as a new version would
/// stack identical blanks.
#[test]
fn two_empty_answers_are_the_same_answer() {
    assert!(is_reread(Some(""), ""));
}
