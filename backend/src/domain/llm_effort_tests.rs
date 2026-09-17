//! Unit tests for the effort vocabulary and the two-family policy.
//!
//! Pure parsing and mapping — no API call, no clock, no environment.

use super::*;

#[test]
fn every_documented_level_parses_to_its_own_wire_string() {
    // The five values `output_config.effort` accepts. A sixth would be rejected
    // by the API as an HTTP 400 mid-run, which is the whole reason this is a
    // closed enum parsed at startup rather than a string passed through.
    for level in Effort::ALL {
        let round_tripped: Effort = level
            .as_wire()
            .parse()
            .expect("every level must parse from its own wire string");
        assert_eq!(round_tripped, level);
    }
    assert_eq!(Effort::XHigh.as_wire(), "xhigh", "one word on the wire");
}

#[test]
fn a_typo_names_the_value_and_lists_what_was_expected() {
    // The operator is reading this out of a failed boot. It has to say what they
    // typed and what would have worked, or the next step is a grep through the
    // source for the accepted set.
    let err = "hihg".parse::<Effort>().expect_err("a typo must not parse");
    assert!(
        err.contains("hihg"),
        "the message must quote the input: {err}"
    );
    for level in Effort::ALL {
        assert!(
            err.contains(level.as_wire()),
            "the message must list {level}: {err}"
        );
    }
}

#[test]
fn casing_and_surrounding_whitespace_are_forgiven() {
    // `LLM_EXTRACTION_EFFORT=LOW ` in a .env file is not an operator error.
    assert_eq!("LOW".parse::<Effort>(), Ok(Effort::Low));
    assert_eq!("  XHigh  ".parse::<Effort>(), Ok(Effort::XHigh));
}

#[test]
fn an_empty_value_is_an_error_and_not_a_silent_default() {
    // `LLM_EXTRACTION_EFFORT=` is a half-finished edit. It must not look
    // identical to never having written the line (Standing Rule 1).
    assert!("".parse::<Effort>().is_err());
    assert!("   ".parse::<Effort>().is_err());
}

#[test]
fn the_shipped_policy_turns_extraction_down_and_leaves_scans_alone() {
    // The whole ruling in one assertion. Extraction is transcription-shaped and
    // its budget belongs to the answer; a scan is a judgement and may want the
    // thinking, so it keeps whatever the provider defaults to rather than being
    // pinned to a value nobody chose.
    let policy = LlmEffortPolicy::default();
    assert_eq!(policy.extraction, Some(Effort::Low));
    assert_eq!(
        policy.scan, None,
        "no effort field is sent for scans unless LLM_SCAN_EFFORT is set"
    );
}

// ─── The stored vocabulary (CC_TASK_READ_V4_BUDGET_FIX_v1) ───────────────────

/// A settings row carries the five levels, case-insensitively.
#[test]
fn a_stored_level_parses_like_an_env_var() {
    assert_eq!(parse_stored_effort("low"), Ok(Some(Effort::Low)));
    assert_eq!(parse_stored_effort("  MAX "), Ok(Some(Effort::Max)));
}

/// `absent` is the row's way of saying what an unset env var says: send no key.
///
/// Without this word an operator could not put the read back to how it ran before
/// 2026-09-17 without a redeploy — and "no effort key" is a real state, not a
/// synonym for `high`.
#[test]
fn absent_means_send_no_effort_key() {
    assert_eq!(parse_stored_effort(ABSENT), Ok(None));
    assert_eq!(parse_stored_effort("Absent"), Ok(None));
}

/// Anything else is refused BY NAME, and the message offers both vocabularies.
#[test]
fn an_unknown_word_is_refused_naming_absent_too() {
    let error = parse_stored_effort("thorough").expect_err("not a level");
    assert!(error.contains("thorough"));
    assert!(error.contains("low, medium, high, xhigh, max"));
    assert!(error.contains("absent"));
}
