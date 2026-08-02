//! Tests for [`super`] — normalization, the origin map, and `locate`.
//!
//! The normalization behaviour itself is pinned by the suite that has always
//! pinned it, in `api::pipeline::canonical_verifier` (which re-exports this
//! module's `normalize_text`). Those tests are the equivalence proof that moving
//! the implementation here changed nothing. What is tested HERE is what is new:
//! that the map points back at the right bytes, and that `locate` finds a quote
//! whose wording only matches after normalization.

use super::*;

/// The exact tier still works, and reports a span in the original's own bytes.
#[test]
fn an_exact_quote_is_located_where_it_actually_sits() {
    let page = "The witness said the account was closed. Then he left.";
    let span = locate(page, "the account was closed").expect("an exact substring is found");
    assert_eq!(&page[span], "the account was closed");
}

/// THE DEFECT. A quote that differs only in punctuation — the shape the LLM
/// emits versus the shape the page carries — is found, and the span points at
/// the PAGE's characters, smart quotes and all.
#[test]
fn a_smart_quoted_page_is_matched_by_a_straight_quoted_snippet() {
    let page = "She wrote \u{201C}the file was never opened\u{201D} in her notes.";
    let quote = "\"the file was never opened\"";

    let span = locate(page, quote).expect("normalization bridges the quote characters");

    // The span covers the PAGE's text — the smart quotes are still there.
    let matched = &page[span];
    assert!(
        matched.contains('\u{201C}') && matched.contains('\u{201D}'),
        "the located span must be the page's own characters, not normalized \
         prose: {matched:?}"
    );
    assert!(matched.contains("the file was never opened"), "{matched:?}");
}

/// A quote split across a hyphenated line break is found, and the span runs from
/// the first fragment to the last.
#[test]
fn a_hyphenated_line_break_does_not_hide_the_quote() {
    let page = "the defen-\ndant refused to answer";
    let span = locate(page, "the defendant refused").expect("the break is rejoined");
    assert_eq!(&page[span], "the defen-\ndant refused");
}

/// Whitespace differences — the commonest normalized case — are bridged, and the
/// original's own line break survives inside the span.
#[test]
fn collapsed_whitespace_is_bridged_and_the_original_spacing_is_returned() {
    let page = "he said\n   the money   was gone\nand sat down";
    let span = locate(page, "the money was gone").expect("whitespace collapses on both sides");
    assert_eq!(&page[span], "the money   was gone");
}

/// A transcript gutter numeral interleaved into the quote does not hide it.
#[test]
fn a_gutter_line_number_inside_the_quote_does_not_hide_it() {
    let page = "I never saw\n14\nthe document before today";
    let span = locate(page, "I never saw the document before").expect("the numeral is stripped");
    let matched = &page[span];
    assert!(
        matched.contains("14"),
        "the span covers the original numeral"
    );
    assert!(matched.starts_with("I never saw"), "{matched:?}");
}

/// THE HONEST LIMIT. A quote that is genuinely not on this text yields nothing —
/// never a window drawn around the nearest thing, which would read as evidence.
#[test]
fn a_quote_that_is_not_here_is_not_invented() {
    let page = "the witness described the meeting at length";
    assert_eq!(locate(page, "the cheque was cashed on Tuesday"), None);
}

/// An empty needle has no position. Returning `Some(0..0)` would give every
/// quote-less item a spurious context window at the top of its page.
#[test]
fn an_empty_quote_has_no_position() {
    assert_eq!(locate("some page text", ""), None);
    assert_eq!(locate("some page text", "   "), None);
}

/// The map survives multi-byte characters on both sides of the match.
#[test]
fn the_map_holds_across_multibyte_characters() {
    let page = "Ms. Bélanger testified — \u{201C}he never called\u{201D} — and stopped.";
    let span = locate(page, "\"he never called\"").expect("found after normalization");
    assert_eq!(&page[span], "\u{201C}he never called\u{201D}");
}

/// `to_original` refuses an empty or out-of-range span rather than guessing.
#[test]
fn the_map_refuses_a_span_it_cannot_honour() {
    let normalized = normalize_with_map("hello world");
    assert_eq!(
        normalized.to_original(0..0),
        None,
        "an empty span has no origin"
    );
    assert_eq!(
        normalized.to_original(0..999),
        None,
        "past the end is a bug, not a clamp"
    );
    assert_eq!(normalized.to_original(0..5), Some(0..5));
}

/// The mapped normalizer and the plain one cannot disagree — the plain one is
/// defined as the mapped one's text. Asserted over inputs that exercise every
/// stage, so a future edit to one stage cannot quietly split them.
#[test]
fn the_two_entry_points_return_the_same_text() {
    for input in [
        "",
        "   ",
        "Plain text.",
        "smart \u{201C}quotes\u{201D} and \u{2018}singles\u{2019}",
        "em \u{2014} dash and en \u{2013} dash",
        "ellipsis\u{2026}done",
        "liga\u{FB01}ture and \u{FB02}ow",
        "soft\u{00AD}hyphen and zero\u{200B}width",
        "hyphen-\nated break",
        "alpha\n14\nbeta",
        "trailing newline\n",
        "windows\r\nline ending",
        "MIXED Case With   Spaces",
        "para¶graph",
    ] {
        assert_eq!(
            normalize_text(input),
            normalize_with_map(input).text(),
            "the two entry points disagreed on {input:?}"
        );
    }
}

/// Every byte of the normalized text maps inside the original text.
///
/// Anti-vacuity for the map itself: an off-by-one in any stage would put an
/// origin past the end of the source, and slicing on it would panic in
/// production rather than fail here.
#[test]
fn every_mapped_origin_lies_within_the_original() {
    let page = "Ms. Bélanger said \u{201C}the defen-\ndant\u{201D}\n14\n never paid\u{2026}";
    let normalized = normalize_with_map(page);

    for index in 0..normalized.text().len() {
        let span = normalized
            .to_original(index..index + 1)
            .unwrap_or_else(|| panic!("byte {index} has no origin"));
        assert!(
            span.end <= page.len() && span.start <= span.end,
            "byte {index} maps to {span:?}, which is not inside a {}-byte page",
            page.len()
        );
        // The slice must not panic — i.e. the span lands on character
        // boundaries, which is what makes it safe to use for context.
        let _ = &page[span];
    }
}
