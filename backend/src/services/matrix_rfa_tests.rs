// Tests for `services::matrix_rfa`.
//
// The line this builds goes into a Word document a lawyer takes to a hearing, so
// the tests are about what must NEVER be printed as much as what must.

use super::*;

fn words() -> MatrixWording {
    MatrixWording::for_test()
}

/// A request for admission renders with its number, its request and its answer.
#[test]
fn an_rfa_renders_number_request_and_answer() {
    let line = rfa_line(
        Some("Admit that you were engaged by Catholic Family Service."),
        Some("Admitted"),
        Some("RFA 8"),
        &words(),
    )
    .expect("a card with a question renders");

    assert!(line.contains("RFA 8"), "{line}");
    assert!(line.contains("Admit that you were engaged"), "{line}");
    assert!(line.contains("Admitted"), "{line}");
    assert!(
        !line.contains("{number}"),
        "the template was filled: {line}"
    );
    assert!(
        !line.contains("{request}"),
        "the template was filled: {line}"
    );
    assert!(
        !line.contains("{answer}"),
        "the template was filled: {line}"
    );
}

/// AN INTERROGATORY IS NOT LABELLED AS AN RFA.
///
/// The sharpest test in this file. `Q19` carries the number 19, and the obvious
/// implementation — take the trailing digits — would print "RFA 19" over an
/// interrogatory. That is a citation to a request for admission that says
/// something else, in a document going to a hearing.
#[test]
fn an_interrogatory_renders_without_an_rfa_number() {
    let line = rfa_line(
        Some("Were sanctions ever sought against Camille Hanley?"),
        Some("No."),
        Some("Q13"),
        &words(),
    )
    .expect("an interrogatory is still a Q&A card");

    assert!(
        !line.contains("RFA"),
        "an interrogatory must not be labelled RFA: {line}"
    );
    assert!(line.contains("Were sanctions ever sought"), "{line}");
    assert!(line.contains("No."), "{line}");
}

/// The prefix is matched case-insensitively, with or without a space.
#[test]
fn the_rfa_prefix_is_matched_loosely_but_the_number_is_not_guessed() {
    for paragraph in ["RFA 17", "RFA17", "rfa 17", "Rfa17"] {
        let line = rfa_line(
            Some("Admit it."),
            Some("Admitted."),
            Some(paragraph),
            &words(),
        )
        .expect("renders");
        assert!(
            line.contains("RFA 17"),
            "{paragraph} should yield number 17: {line}"
        );
    }
}

/// A paragraph naming an RFA with no digits renders WITHOUT a number rather than
/// with an empty one — "RFA  — Admit that…" reads as a citation and points at
/// nothing.
#[test]
fn an_rfa_with_no_digits_renders_unnumbered() {
    let line =
        rfa_line(Some("Admit it."), Some("Admitted."), Some("RFA"), &words()).expect("renders");
    assert!(!line.contains("RFA"), "{line}");
}

/// A paragraph carrying trailing text after the number renders unnumbered.
///
/// "RFA 17(a)" and "RFA 17 amended" are values this function does not fully
/// understand. Half-reading one into a citation is the failure mode; declining to
/// cite is not.
#[test]
fn a_paragraph_this_build_does_not_fully_understand_renders_unnumbered() {
    for paragraph in ["RFA 17(a)", "RFA 17 amended", "RFA 1-3"] {
        let line = rfa_line(
            Some("Admit it."),
            Some("Admitted."),
            Some(paragraph),
            &words(),
        )
        .expect("renders");
        assert!(
            !line.contains("RFA"),
            "{paragraph} must not be cited as a bare number: {line}"
        );
    }
}

/// A card with NO question is not a Q&A card, and renders as its plain quote.
#[test]
fn a_card_with_no_question_is_not_an_rfa_card() {
    assert_eq!(
        rfa_line(None, Some("Admitted."), Some("RFA 8"), &words()),
        None
    );
}

/// A card with a question and no answer renders as its plain quote.
///
/// An unanswered request is a real state. "RFA 8 — Admit that … — " would print a
/// template with its point removed, and would look like a finished sentence.
#[test]
fn a_question_with_no_answer_falls_back_to_the_plain_quote() {
    assert_eq!(
        rfa_line(Some("Admit it."), None, Some("RFA 8"), &words()),
        None
    );
    assert_eq!(
        rfa_line(Some("Admit it."), Some("   "), Some("RFA 8"), &words()),
        None
    );
}

/// A blank question is the same as none.
#[test]
fn a_blank_question_is_the_same_as_none() {
    assert_eq!(
        rfa_line(Some("  "), Some("Admitted."), Some("RFA 8"), &words()),
        None
    );
}

/// A card with no paragraph at all still renders — unnumbered.
#[test]
fn a_card_with_no_paragraph_renders_unnumbered() {
    let line = rfa_line(Some("Admit it."), Some("Admitted."), None, &words()).expect("renders");
    assert!(!line.contains("RFA"), "{line}");
    assert!(line.contains("Admit it."), "{line}");
}

/// A paragraph shorter than the prefix does not panic.
///
/// `text.get(..3)` on `"Q"` returns `None` rather than slicing out of bounds —
/// asserted because the obvious `&text[..3]` would panic here, and 29 DEV cards
/// carry a paragraph starting with `A`.
#[test]
fn a_paragraph_shorter_than_the_prefix_is_handled() {
    let line =
        rfa_line(Some("Admit it."), Some("Admitted."), Some("Q"), &words()).expect("renders");
    assert!(!line.contains("RFA"), "{line}");
}

/// A multi-byte paragraph does not panic on the prefix slice.
///
/// `str::get` returns `None` on a non-boundary index rather than panicking, which
/// is the whole reason it is used instead of indexing.
#[test]
fn a_multibyte_paragraph_does_not_panic() {
    let line =
        rfa_line(Some("Admit it."), Some("Admitted."), Some("¶17"), &words()).expect("renders");
    assert!(!line.contains("RFA"), "{line}");
}

/// The request and answer are surrounded by the STORED separators, so re-wording
/// the line on the Settings page changes both surfaces at once.
#[test]
fn the_line_is_built_from_the_stored_template() {
    let mut custom = words();
    custom.rfa_template = "[{number}] {request} >> {answer}".to_string();
    let line =
        rfa_line(Some("Admit it."), Some("Admitted."), Some("RFA 8"), &custom).expect("renders");
    assert_eq!(line, "[8] Admit it. >> Admitted.");
}
