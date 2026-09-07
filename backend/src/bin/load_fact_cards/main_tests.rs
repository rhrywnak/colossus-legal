// Tests for the loader's own helpers.
//
// The count guard and the two printers. Small, and worth pinning: the count guard
// is the only thing standing between a regenerated file and a half-loaded deck,
// and the printers are what an operator reads a dry run through.

use super::*;

/// The expected count passes; any other number stops the load and names both.
#[test]
fn the_count_guard_names_both_numbers() {
    assert!(expect(10, 10, "cards").is_ok());
    let err = expect(9, 10, "cards").expect_err("nine is not ten");
    let message = format!("{err}");
    assert!(message.contains("10"), "{message}");
    assert!(message.contains('9'), "{message}");
    assert!(message.contains("cards"), "names the unit: {message}");
}

/// Zero records against an expectation of zero is fine — S-1's Job B file is
/// genuinely empty, and the candidates file is what carries its ten.
#[test]
fn an_empty_file_matching_an_expectation_of_zero_passes() {
    assert!(expect(0, 0, "cards").is_ok());
}

/// A graph id prints as its hash suffix, which is what a human recognises.
#[test]
fn a_graph_id_prints_as_its_suffix() {
    assert_eq!(
        short("doc-judge-tighe-opinion:evidence:8a240a75"),
        "8a240a75"
    );
    // An id with no colon prints whole rather than empty.
    assert_eq!(short("plain"), "plain");
}

/// A value prints on one line, cut so a 59-card plan stays readable.
#[test]
fn a_long_value_is_cut_to_one_line() {
    let long = "a ".repeat(80);
    let printed = preview(Some(&long));
    assert!(printed.chars().count() <= 72, "{}", printed.chars().count());
    assert!(printed.ends_with("..."));
}

/// Newlines are flattened — a drafted answer with a line break must not break
/// the plan's columns.
#[test]
fn newlines_are_flattened() {
    assert_eq!(preview(Some("first\n\nsecond")), "first second");
}

/// A short value prints whole, with no ellipsis.
#[test]
fn a_short_value_prints_whole() {
    assert_eq!(
        preview(Some("The money was Dad's.")),
        "The money was Dad's."
    );
}

/// A cleared field says so rather than printing an empty column.
#[test]
fn a_cleared_field_says_so() {
    assert_eq!(preview(None), "(cleared)");
}
