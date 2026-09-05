//! The loose form and the two measures, asserted on the shapes the transcripts
//! actually produce. Every fixture here is a real damage class, not an invented
//! one — a fixture that is not the wire shape proves nothing (the lesson the
//! `evidence_key` arm paid eleven days for).

use super::*;

#[test]
fn the_loose_form_drops_case_whitespace_and_a_trailing_period() {
    assert_eq!(
        loose_normalize("The  Objections\n  seem to have thrusts."),
        "the objections seem to have thrusts"
    );
}

#[test]
fn a_trailing_run_of_punctuation_and_space_all_goes() {
    assert_eq!(loose_normalize("res judicata. ,"), "res judicata");
    assert_eq!(loose_normalize("res judicata.."), "res judicata");
}

#[test]
fn a_hyphen_at_a_line_break_joins_the_word() {
    // The damage class the corrected page text introduced: justified-text
    // hyphenation preserved across the break.
    assert_eq!(
        loose_normalize("the reason-\nableness of the fee"),
        "the reasonableness of the fee"
    );
}

#[test]
fn a_space_before_the_hyphen_still_joins() {
    // The second shape the OCR emits once the gutter numeral is stripped.
    assert_eq!(loose_normalize("prior rul -\nings"), "prior rulings");
}

#[test]
fn indentation_after_the_break_is_swallowed_with_it() {
    assert_eq!(
        loose_normalize("objec-\n       tions were filed"),
        "objections were filed"
    );
}

#[test]
fn a_real_compound_word_is_never_joined() {
    // `two-fold` and `twofold` are different strings in the document, and a
    // matcher that could not tell them apart would match quotes that differ.
    assert_eq!(
        loose_normalize("two-fold thrusts"),
        "two-fold thrusts",
        "a hyphen with no line break after it is a compound, not a split"
    );
}

#[test]
fn a_hyphen_followed_by_a_space_but_no_newline_is_left_alone() {
    // Measured: this is the OLD damage class (`objec- tions`), and the two
    // patterns this module implements both require a line break. Pinned as a
    // TEST rather than left implicit, because it is the difference between
    // tier 2 and tier 3 on the transcripts and it must not change by accident.
    assert_eq!(loose_normalize("objec- tions"), "objec- tions");
}

#[test]
fn a_double_dash_is_not_a_hyphen_break() {
    // Transcripts are full of `that--that`. The look-behind requires a letter
    // immediately before the hyphen, so the second dash of a pair never joins.
    assert_eq!(
        loose_normalize("is that--\nthat the Court"),
        "is that-- that the court",
        "neither dash joins: the first is not followed by whitespace, and the \
         second is not preceded by a letter"
    );
}

#[test]
fn decomposed_and_composed_accents_reach_the_same_loose_form() {
    // NFC first, for the same reason the id arm does it.
    assert_eq!(loose_normalize("Ame\u{301}lie"), loose_normalize("Amélie"));
}

#[test]
fn identical_text_scores_one_and_unrelated_text_scores_low() {
    assert_eq!(similarity("res judicata", "res judicata"), 1.0);
    assert!(
        similarity("res judicata", "the cuff links") < 0.5,
        "two unrelated quotes must not clear any usable threshold"
    );
}

#[test]
fn a_superset_quote_covers_every_old_word_but_scores_below_the_default() {
    // The measured shape of the problem, in one assertion: the new quote holds
    // the whole old statement and adds a leading clause. Coverage sees it;
    // normalized Levenshtein, which divides by the LONGER string, does not.
    let old = "that you may want to take this under advisement";
    let new = "that there is a distinct possibility of a second appeal, that you \
               may want to take this under advisement";

    assert_eq!(word_coverage(old, new), 1.0);
    assert!(
        similarity(old, new) < NearMatchSettings::default().similarity,
        "similarity was {}, which would have made this a tier-3 match",
        similarity(old, new)
    );
}

#[test]
fn coverage_counts_only_the_old_words_that_appear_in_order() {
    assert_eq!(word_coverage("a b c d", "a b c d"), 1.0);
    assert_eq!(word_coverage("a b c d", "a x b x d"), 0.75);
    assert_eq!(
        word_coverage("a b c", "c b a"),
        1.0 / 3.0,
        "reversed words are not the same statement; only one can be in order"
    );
}

#[test]
fn coverage_uses_a_full_lcs_so_a_repeated_word_cannot_be_spent_early() {
    // A greedy forward scan matches the first `the`, then fails on `the court`.
    // LCS does not.
    assert_eq!(
        word_coverage("the fee the court", "the fee and the court"),
        1.0
    );
}

#[test]
fn an_empty_old_quote_covers_nothing_rather_than_everything() {
    assert_eq!(
        word_coverage("", "anything at all"),
        0.0,
        "a node with no words must never match every node on the page"
    );
}

#[test]
fn the_default_thresholds_are_the_documented_pair() {
    let settings = NearMatchSettings::default();
    assert_eq!((settings.similarity, settings.word_coverage), (0.90, 0.80));
}

#[test]
fn an_omitted_override_keeps_the_default_and_a_given_one_wins() {
    let settings = NearMatchSettings::new(Some(0.55), None).expect("in range");
    assert_eq!(settings.similarity, 0.55);
    assert_eq!(settings.word_coverage, 0.80);
}

#[test]
fn a_threshold_outside_zero_to_one_is_refused_by_name() {
    assert_eq!(
        NearMatchSettings::new(Some(5.0), None),
        Err(SettingsError::OutOfRange {
            name: "--near-similarity",
            value: 5.0
        })
    );
    assert_eq!(
        NearMatchSettings::new(None, Some(-1.0)),
        Err(SettingsError::OutOfRange {
            name: "--near-word-coverage",
            value: -1.0
        })
    );
    assert!(
        NearMatchSettings::new(Some(f64::NAN), None).is_err(),
        "NaN is not a ratio and must not pass as one"
    );
}

#[test]
fn the_refusal_names_the_flag_and_the_value_in_its_message() {
    // The message is the whole point of refusing at startup: an operator who
    // typed the wrong flag has to be able to see WHICH one from the log line.
    assert_eq!(
        SettingsError::OutOfRange {
            name: "--near-similarity",
            value: 5.0,
        }
        .to_string(),
        "--near-similarity must be a ratio between 0.0 and 1.0 inclusive, got 5"
    );
}
