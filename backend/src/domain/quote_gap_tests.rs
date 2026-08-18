//! Tests for the second-chance matcher.
//!
//! A sibling file, mirroring the layout `quote_match` already uses: the bodies
//! live next to the module they exercise without inflating it past the 300-line
//! limit (CLAUDE.md §17).

use std::collections::BTreeSet;

use crate::domain::quote_gap::{
    bare_numerals, is_bare_numeral, locate_with_gap, normalize_without_foreign_numerals, GapPolicy,
};

/// The policy the build ships with, so the tests measure what DEV runs.
fn shipped() -> GapPolicy {
    GapPolicy {
        max_gap_chars: 240,
        min_half_fraction: 0.05,
        min_half_words: 3,
    }
}

fn keep_of(quote: &str) -> BTreeSet<String> {
    bare_numerals(quote)
}

#[test]
fn bare_numeral_recognises_markers_and_refuses_prose() {
    assert!(is_bare_numeral("24"));
    assert!(is_bare_numeral("24."));
    assert!(is_bare_numeral("7"));
    // Money and dates are prose, not markers.
    assert!(!is_bare_numeral("$50,000.00"));
    assert!(!is_bare_numeral("2009,"));
    assert!(!is_bare_numeral("12-15-2009"));
    assert!(!is_bare_numeral("p.12"));
    assert!(!is_bare_numeral(""));
    assert!(!is_bare_numeral("."));
}

#[test]
fn footnote_marker_mid_sentence_is_removed() {
    let quote = "the record seems to indicate";
    let page = "the record seems to indicate 24 . Second, the fee";
    let keep = keep_of(quote);
    let hay = normalize_without_foreign_numerals(page, &keep);
    let needle = normalize_without_foreign_numerals(quote, &keep);
    assert!(
        hay.text().contains(needle.text()),
        "stripped page was: {:?}",
        hay.text()
    );
}

#[test]
fn gutter_numeral_at_line_start_is_removed() {
    let quote = "he is not entitled to that fee";
    let page = "12 he is not entitled 13 to that fee";
    let keep = keep_of(quote);
    let hay = normalize_without_foreign_numerals(page, &keep);
    let needle = normalize_without_foreign_numerals(quote, &keep);
    assert!(
        hay.text().contains(needle.text()),
        "stripped page was: {:?}",
        hay.text()
    );
}

#[test]
fn numerals_inside_the_quote_are_never_stripped() {
    // The quote carries its own year; the page carries it plus a footnote
    // marker. The year must survive on BOTH sides, so a wrong year cannot match.
    let quote = "paid the estate on 2009 in the amount agreed";
    let page = "paid the estate on 2009 7 in the amount agreed";
    let keep = keep_of(quote);
    assert!(keep.contains("2009"), "keep set was {keep:?}");

    let hay = normalize_without_foreign_numerals(page, &keep);
    let needle = normalize_without_foreign_numerals(quote, &keep);
    assert!(
        hay.text().contains("2009"),
        "year was stripped: {:?}",
        hay.text()
    );
    assert!(
        hay.text().contains(needle.text()),
        "page {:?} did not contain quote {:?}",
        hay.text(),
        needle.text()
    );

    // The same quote against a DIFFERENT year must not match.
    let wrong_year = "paid the estate on 2011 7 in the amount agreed";
    let wrong = normalize_without_foreign_numerals(wrong_year, &keep);
    assert!(!wrong.text().contains(needle.text()));
}

#[test]
fn a_punctuated_numeral_is_not_in_the_keep_set_and_needs_no_protection() {
    // `23,` and `$50,000.00` are not bare tokens, so nothing ever strips them —
    // widening the keep set to include them would protect nothing while making
    // the page retain footnote markers that merely share their digits.
    let keep = keep_of("paid $50,000.00 on July 23, 2009 to the estate");
    assert!(keep.contains("2009"), "keep set was {keep:?}");
    assert!(!keep.contains("23"), "keep set was {keep:?}");
    assert!(!keep.contains("50"), "keep set was {keep:?}");
}

#[test]
fn gap_match_finds_both_halves_in_order() {
    let quote = "the guardian ad litem billed for the title search he is not entitled";
    let page = "the guardian ad litem billed for the title \
                Exhibit 11 Transcript of the hearing held that day \
                search he is not entitled";
    let m = locate_with_gap(page, quote, shipped()).expect("should match around one gap");
    assert_eq!(
        m.head_words + m.tail_words,
        quote.split_whitespace().count()
    );
    assert!(
        m.gap_chars > 0 && m.gap_chars <= 240,
        "gap was {}",
        m.gap_chars
    );
}

#[test]
fn gap_match_rejects_when_one_half_is_below_the_word_floor() {
    // "For" alone is the head; the rest follows the gap. Under the shipped floor
    // of 3 words this must be refused however small the gap is — a single common
    // word is a coincidence, not a match. This is the shape of item 9402.
    let quote = "For the amount claimed here";
    let page = "For some entirely unrelated sentence the amount claimed here";
    assert_eq!(locate_with_gap(page, quote, shipped()), None);
}

#[test]
fn gap_match_rejects_when_the_gap_is_too_wide() {
    let quote = "the guardian ad litem billed for the title search";
    let filler = "x".repeat(400);
    let page = format!("the guardian ad litem {filler} billed for the title search");
    assert_eq!(locate_with_gap(&page, quote, shipped()), None);
}

#[test]
fn gap_match_refuses_a_quote_that_is_simply_absent() {
    let quote = "a sentence that appears nowhere in this document at all";
    let page = "the guardian ad litem billed for the title search";
    assert_eq!(locate_with_gap(page, quote, shipped()), None);
}

#[test]
fn gap_match_prefers_the_tightest_gap() {
    // The head occurs twice: once far from the tail, once close to it. The
    // matcher must report the closer pair, because the tighter gap is the one
    // more likely to be the real sentence.
    let quote = "the fee the estate paid to counsel in full";
    let page = "the fee the estate AAAAAAAAAAAAAAAAAAAAAAAA \
                repeated: the fee the estate BBB paid to counsel in full";
    let m = locate_with_gap(page, quote, shipped()).expect("should match");
    // " BBB " — five characters between the halves, against the twenty-four of
    // the earlier occurrence.
    assert_eq!(
        m.gap_chars, 5,
        "expected the tighter of the two occurrences"
    );
}

#[test]
fn a_one_word_quote_can_never_gap_match() {
    // There is no split of a single word into two halves, and a "match" of one
    // word against a whole page would be noise, not grounding.
    assert_eq!(
        locate_with_gap("anything at all here", "anything", shipped()),
        None
    );
}

#[test]
fn the_fraction_floor_refuses_a_split_the_word_floor_would_allow() {
    // The word floor and the fraction floor are independent guards, and this is
    // the case where only the FRACTION decides: a 66-word quote split 63/3. The
    // short half is 3 words, so `min_half_words: 3` is satisfied exactly — and
    // 3/66 = 4.5%, under the shipped 5%.
    //
    // This is the shape of item 9441 on the Phillips default motion (66 of 69
    // words on page 23, the last 3 on page 24), which is why the threshold is
    // worth a test of its own rather than being covered incidentally.
    let words: Vec<String> = (1..=66).map(|i| format!("w{i}")).collect();
    let quote = words.join(" ");
    let page = format!(
        "{} INTERRUPTION {}",
        words[..63].join(" "),
        words[63..].join(" ")
    );

    let shipped = GapPolicy {
        max_gap_chars: 240,
        min_half_fraction: 0.05,
        min_half_words: 3,
    };
    assert_eq!(
        locate_with_gap(&page, &quote, shipped),
        None,
        "3/66 = 4.5% is under the 5% floor and must be refused"
    );

    // Proof that the FRACTION is what refused it and not the word floor or the
    // gap width: lower only the fraction and the same split is accepted.
    let looser = GapPolicy {
        min_half_fraction: 0.04,
        ..shipped
    };
    let m = locate_with_gap(&page, &quote, looser).expect("4.5% clears a 4% floor");
    assert_eq!((m.head_words, m.tail_words), (63, 3));
}
