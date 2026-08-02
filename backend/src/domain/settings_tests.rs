//! Unit tests for [`super`] — parsing and bounds for stored parameters.
//!
//! Every branch is reachable without a database, which is the point of keeping
//! this module pure: the configuration law's teeth are the REFUSALS, and a
//! refusal nobody has exercised is a refusal nobody can trust.

use super::*;

// ── The kind vocabulary ──────────────────────────────────────────────────────

#[test]
fn every_known_kind_round_trips() {
    for &kind in ValueKind::ALL {
        assert_eq!(
            ValueKind::try_from(kind.code()).expect("its own token"),
            kind
        );
    }
}

#[test]
fn an_unknown_kind_is_refused_and_the_message_lists_the_vocabulary() {
    let Err(error) = ValueKind::try_from("percentage") else {
        panic!("an unknown kind must be refused");
    };
    let message = error.to_string();
    assert!(message.contains("percentage"), "the bad token: {message}");
    for known in ValueKind::ALL {
        assert!(
            message.contains(known.code()),
            "{} must be offered: {message}",
            known.code()
        );
    }
}

#[test]
fn every_kind_offers_a_hint_showing_the_shape() {
    // The hint sits beside the edit box. An empty one would leave a human
    // guessing whether "9 of 10" or "0.9" is wanted.
    for &kind in ValueKind::ALL {
        assert!(!kind.hint().is_empty(), "{} needs a hint", kind.code());
    }
}

// ── Floats ───────────────────────────────────────────────────────────────────

fn unit_interval() -> Bounds {
    Bounds {
        min: Some(0.0),
        max: Some(1.0),
    }
}

#[test]
fn a_float_reads_and_keeps_its_value() {
    let parsed = parse_float("confidence_band_high", "0.80", unit_interval()).expect("valid");
    assert!((parsed - 0.80).abs() < f32::EPSILON);
}

#[test]
fn surrounding_whitespace_does_not_make_a_value_unreadable() {
    // A human editing a text box leaves a trailing space. Refusing that would be
    // pedantry dressed as rigour.
    assert!(parse_float("k", "  0.5  ", unit_interval()).is_ok());
    assert!(parse_count("k", " 240 ", Bounds::default()).is_ok());
    assert!(parse_ratio("k", " 9 / 10 ").is_ok());
}

#[test]
fn a_float_that_is_not_a_number_is_refused_by_name() {
    let Err(error) = parse_float("confidence_band_high", "high", unit_interval()) else {
        panic!("refused");
    };
    let message = error.to_string();
    assert!(message.contains("confidence_band_high"), "{message}");
    assert!(message.contains("high"), "the offending value: {message}");
    assert!(message.contains("a number"), "{message}");
}

/// NaN is refused explicitly rather than left to the bounds check.
///
/// `f32::NAN` fails EVERY comparison, so `< min` and `> max` are both false and
/// it would pass a bounds check that looks watertight. Downstream it would then
/// make `band_for_score` return `Low` for every score in the product, silently.
#[test]
fn nan_is_refused_rather_than_slipping_through_the_bounds() {
    assert!(parse_float("k", "NaN", unit_interval()).is_err());
    assert!(parse_float("k", "nan", unit_interval()).is_err());
}

#[test]
fn infinity_is_refused_by_the_upper_bound() {
    // Unlike NaN, infinity DOES compare, so the bounds catch it — but only when
    // a maximum is declared. This pins that the unit-interval parameters have one.
    assert!(parse_float("k", "inf", unit_interval()).is_err());
}

#[test]
fn a_float_below_the_minimum_is_refused_and_the_message_names_the_bound() {
    let Err(error) = parse_float("confidence_band_high", "-0.2", unit_interval()) else {
        panic!("refused");
    };
    let message = error.to_string();
    assert!(message.contains("at least 0"), "{message}");
    assert!(message.contains("confidence_band_high"), "{message}");
}

#[test]
fn a_float_above_the_maximum_is_refused_and_the_message_names_the_bound() {
    let Err(error) = parse_float("confidence_band_high", "1.5", unit_interval()) else {
        panic!("refused");
    };
    assert!(error.to_string().contains("at most 1"), "{error}");
}

#[test]
fn the_bounds_are_inclusive_at_both_ends() {
    // A cutoff of exactly 1.0 means "nothing is ever High"; a window of exactly 0
    // means "show no context". Both are strange and neither is an error —
    // refusing them would be this module deciding policy.
    assert!(parse_float("k", "0", unit_interval()).is_ok());
    assert!(parse_float("k", "1", unit_interval()).is_ok());
}

// ── Counts ───────────────────────────────────────────────────────────────────

#[test]
fn a_count_reads_as_a_whole_number() {
    assert_eq!(
        parse_count("quote_context_window_chars", "240", Bounds::default()).expect("valid"),
        240
    );
}

#[test]
fn a_fractional_count_is_refused_rather_than_truncated() {
    // Silently truncating 2.7 to 2 would give a human a cap they did not set and
    // no indication anything happened.
    assert!(parse_count("talking_points_cap", "2.7", Bounds::default()).is_err());
}

#[test]
fn a_negative_count_is_refused_with_a_message_about_whole_numbers() {
    let Err(error) = parse_count("talking_points_cap", "-1", Bounds::default()) else {
        panic!("refused");
    };
    // `usize` cannot hold it, so this fails at the PARSE — the message must still
    // say what was wanted rather than leaking an integer-overflow surprise.
    assert!(error.to_string().contains("whole number"), "{error}");
}

#[test]
fn a_count_below_its_minimum_is_refused() {
    // The talking-points cap declares min 1: a cap of zero would forbid C5
    // entirely, which is a broken product rather than a configuration.
    let bounds = Bounds {
        min: Some(1.0),
        max: None,
    };
    assert!(parse_count("talking_points_cap", "0", bounds).is_err());
    assert!(parse_count("talking_points_cap", "1", bounds).is_ok());
}

#[test]
fn a_count_with_no_upper_bound_accepts_a_large_value() {
    // The context window declares no maximum. A silly-but-legal 100000 is the
    // human's call, and the page shows them the default they moved away from.
    assert!(parse_count(
        "quote_context_window_chars",
        "100000",
        Bounds {
            min: Some(0.0),
            max: None
        }
    )
    .is_ok());
}

// ── Ratios ───────────────────────────────────────────────────────────────────

#[test]
fn a_ratio_reads_both_halves() {
    let ratio = parse_ratio("card_test_ratio", "9/10").expect("valid");
    assert_eq!(ratio.numerator, 9);
    assert_eq!(ratio.denominator, 10);
}

#[test]
fn a_ratio_renders_back_the_way_it_was_written() {
    // The page round-trips this text. A ratio that displayed as 0.9 after being
    // typed as 9/10 would look like the edit had been rewritten.
    assert_eq!(parse_ratio("k", "9/10").expect("valid").to_string(), "9/10");
}

#[test]
fn a_ratio_converts_to_a_fraction_for_comparison() {
    let ratio = parse_ratio("k", "9/10").expect("valid");
    assert!((ratio.as_fraction() - 0.9).abs() < f64::EPSILON);
}

#[test]
fn a_zero_denominator_is_refused_before_anything_can_divide_by_it() {
    let Err(error) = parse_ratio("card_test_ratio", "9/0") else {
        panic!("refused");
    };
    assert!(error.to_string().contains("divide by nothing"), "{error}");
}

#[test]
fn a_ratio_without_a_slash_is_refused_with_an_example() {
    let Err(error) = parse_ratio("card_test_ratio", "0.9") else {
        panic!("refused");
    };
    let message = error.to_string();
    // A human who typed a decimal needs to be shown the shape, not scolded.
    assert!(message.contains("9/10"), "{message}");
}

#[test]
fn a_ratio_with_unreadable_halves_is_refused() {
    assert!(parse_ratio("k", "nine/ten").is_err());
    assert!(parse_ratio("k", "9/").is_err());
    assert!(parse_ratio("k", "/10").is_err());
}

// ── The refusal messages ─────────────────────────────────────────────────────
//
// Each reaches the Settings page verbatim. A `matches!` on the variant proves the
// right branch was taken; it does not prove the sentence still says anything.

#[test]
fn the_missing_parameter_refusal_explains_why_there_is_no_fallback() {
    let error = SettingError::Missing {
        key: "talking_points_cap".to_string(),
    };
    let message = error.to_string();
    assert!(message.contains("talking_points_cap"), "{message}");
    // The reason matters: a reader's first instinct is "why not just default it?"
    assert!(message.contains("no compiled-in defaults"), "{message}");
}

#[test]
fn the_crossed_bands_refusal_explains_the_consequence() {
    let error = SettingError::BandsCrossed {
        high: 0.4,
        medium: 0.6,
    };
    let message = error.to_string();
    assert!(
        message.contains("0.4") && message.contains("0.6"),
        "{message}"
    );
    // Not just "invalid" — what would actually go wrong.
    assert!(message.contains("medium band"), "{message}");
}
