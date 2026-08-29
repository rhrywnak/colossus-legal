//! Unit tests for the one-line response description.
//!
//! Pure string building — no stream, no socket, no API call.

use super::*;

/// The counts the 2026-08-28 incident actually produced.
fn incident_counts() -> BlockCounts {
    let mut counts = BlockCounts::default();
    for _ in 0..14 {
        counts.record("thinking");
    }
    counts
}

#[test]
fn the_incident_line_says_what_arrived_instead_of_what_did_not() {
    // The whole point. "No text content" was true and useless; this says the
    // model spent the budget on fourteen reasoning blocks and stopped happily.
    let line = anatomy_line(&incident_counts(), Some(63_997), Some("end_turn"));
    assert!(line.contains("14 content blocks"), "{line}");
    assert!(line.contains("thinking ×14"), "{line}");
    assert!(line.contains("output_tokens=63997"), "{line}");
    assert!(line.contains("stop_reason=end_turn"), "{line}");
    assert_eq!(line.lines().count(), 1, "it has to be ONE line: {line}");
}

#[test]
fn a_healthy_extraction_reads_as_the_baseline_it_is() {
    // The reason counts are collected on every call and not only failing ones:
    // `thinking ×14, text ×0` is only legible as a departure if someone knows
    // what a normal response looks like.
    let mut counts = BlockCounts::default();
    counts.record("text");
    let line = anatomy_line(&counts, Some(8_120), Some("end_turn"));
    assert!(line.contains("1 content blocks"), "{line}");
    assert!(line.contains("text ×1"), "{line}");
}

#[test]
fn several_types_are_listed_in_a_stable_order() {
    // Rendered from a BTreeMap on purpose: with a HashMap the same failure would
    // print its types in a different order on every run, and two error messages
    // describing one incident would not compare equal by eye.
    let mut counts = BlockCounts::default();
    counts.record("text");
    counts.record("thinking");
    counts.record("thinking");
    counts.record("tool_use");
    let first = anatomy_line(&counts, Some(1), Some("tool_use"));
    let second = anatomy_line(&counts, Some(1), Some("tool_use"));
    assert_eq!(first, second);
    assert!(
        first.contains("text ×1, thinking ×2, tool_use ×1"),
        "alphabetical and stable: {first}"
    );
}

#[test]
fn a_response_with_no_blocks_at_all_says_so() {
    // Distinct from "blocks arrived but none were text". An empty content array
    // is its own state and the line must not render as a bare trailing paren.
    let line = anatomy_line(&BlockCounts::default(), Some(0), Some("end_turn"));
    assert!(line.contains("0 content blocks (none)"), "{line}");
}

#[test]
fn a_provider_that_reported_nothing_is_not_reported_as_zero() {
    // Standing Rule 1 at the level of one sentence: "the provider did not say"
    // and "the provider said none" are different facts, and this line exists to
    // be trusted about exactly that kind of distinction.
    let line = anatomy_line(&incident_counts(), None, None);
    assert!(line.contains("output_tokens=not reported"), "{line}");
    assert!(line.contains("stop_reason=not reported"), "{line}");
    assert!(
        !line.contains("=0"),
        "a missing count must not render as 0: {line}"
    );
}

#[test]
fn counts_are_queryable_as_well_as_printable() {
    // `get` is what lets a caller ask the question the incident raised — "were
    // there any text blocks at all?" — without parsing the rendered line back.
    let counts = incident_counts();
    assert_eq!(counts.get("thinking"), 14);
    assert_eq!(
        counts.get("text"),
        0,
        "absent reads as zero, not as a panic"
    );
    assert_eq!(counts.total(), 14);
}
