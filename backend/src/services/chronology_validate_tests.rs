//! Tests for `services::chronology_validate`.
//!
//! Every refusal has a test, because a validation rule with no test is a rule
//! that will be relaxed by accident (Rule 6, and §11's test-auditor). The happy
//! path has several, because the DEFAULTS are where a write silently stores
//! something nobody asked for.

use super::*;

/// The four phases the case actually has, as `chronology_phases` seeds them.
fn phases() -> Vec<String> {
    ["estate", "probate", "appeals", "civil_lawsuit"]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

/// The five tags `chronology_tags` seeds.
fn tags() -> Vec<String> {
    [
        "financial",
        "court_action",
        "filing",
        "discovery",
        "personal",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect()
}

/// A submitted event that is valid, so each test can spoil exactly one thing.
fn ok_event<'a>(phase: &'a str, title: &'a str, date: &'a str) -> SubmittedEvent<'a> {
    SubmittedEvent {
        event_date: date,
        title,
        phase,
        fact: None,
        date_precision: None,
        approximate: None,
        tags: None,
    }
}

fn vocab<'a>(phases: &'a [String], tags: &'a [String]) -> Vocabularies<'a> {
    Vocabularies { phases, tags }
}

// ── The happy path, and what it defaults to ─────────────────────────────────

#[test]
fn a_date_and_a_title_are_enough() {
    // R11, exactly: "date and title required, one-sentence fact encouraged but
    // optional". An event with nothing else must be writable.
    let p = phases();
    let t = tags();
    let valid = validate_event(
        ok_event("probate", "CFS appointed PR", "2009-11-03"),
        vocab(&p, &t),
    )
    .expect("date + title + phase is a writable event");

    assert_eq!(valid.title, "CFS appointed PR");
    assert_eq!(
        valid.event_date,
        NaiveDate::from_ymd_opt(2009, 11, 3).expect("a real date")
    );
    assert_eq!(valid.fact, None);
    assert!(valid.tags.is_empty());
}

#[test]
fn an_unspecified_precision_is_day_and_an_unspecified_approximate_is_false() {
    // These two defaults are the column's defaults. If this test and the
    // migration ever disagree, an event written through the API and one written
    // by the seed would mean different things by the same absent field.
    let p = phases();
    let t = tags();
    let valid =
        validate_event(ok_event("estate", "x", "2008-06-01"), vocab(&p, &t)).expect("valid");
    assert_eq!(valid.date_precision, DatePrecision::Day);
    assert!(!valid.approximate);
}

#[test]
fn the_title_and_the_fact_are_trimmed_and_an_empty_fact_becomes_none() {
    let p = phases();
    let t = tags();
    let mut submitted = ok_event("appeals", "  Tighe order  ", "2012-04-12");
    submitted.fact = Some("   ");
    let valid = validate_event(submitted, vocab(&p, &t)).expect("valid");
    assert_eq!(valid.title, "Tighe order");
    // NULL, not "". A stored empty string is a value somebody wrote; NULL is the
    // absence of one, and the history snapshot must be able to tell them apart.
    assert_eq!(valid.fact, None);
}

#[test]
fn tags_are_trimmed_deduplicated_and_kept_in_the_authors_order() {
    let p = phases();
    let t = tags();
    let submitted_tags = vec![
        " filing ".to_string(),
        "financial".to_string(),
        "filing".to_string(),
        "  ".to_string(),
    ];
    let mut submitted = ok_event("probate", "x", "2010-01-01");
    submitted.tags = Some(&submitted_tags);
    let valid = validate_event(submitted, vocab(&p, &t)).expect("valid");
    // The ORDER is the author's: the first tag decides the card's dot colour,
    // so sorting here would silently recolour the event.
    assert_eq!(
        valid.tags,
        vec!["filing".to_string(), "financial".to_string()]
    );
}

// ── The refusals, one test each ─────────────────────────────────────────────

#[test]
fn a_blank_title_is_refused_as_a_shape_problem() {
    let p = phases();
    let t = tags();
    let refusal = validate_event(ok_event("probate", "   ", "2010-01-01"), vocab(&p, &t))
        .expect_err("a card with no title is a card nobody can pick out of a list");
    assert_eq!(refusal, ChronologyWriteRefusal::BlankTitle);
    assert!(
        !refusal.is_unprocessable(),
        "a blank box is a 400, not a 422"
    );
    assert_eq!(refusal.field(), Some("title"));
}

#[test]
fn an_unreadable_date_names_what_was_sent_and_the_shape_expected() {
    let p = phases();
    let t = tags();
    let refusal = validate_event(ok_event("probate", "x", "11/03/2009"), vocab(&p, &t))
        .expect_err("a US-format date is not the wire format");
    assert_eq!(refusal.value(), Some("11/03/2009"));
    // The message must carry BOTH: what was sent, and what was wanted. One
    // without the other sends the author back to guess.
    let message = refusal.to_string();
    assert!(message.contains("11/03/2009"), "got: {message}");
    assert!(message.contains("YYYY-MM-DD"), "got: {message}");
    assert!(!refusal.is_unprocessable());
}

#[test]
fn a_date_that_does_not_exist_is_refused() {
    // The 31st of February parses as a SHAPE and is not a day. chrono refuses
    // it; this pins that the refusal reaches the caller rather than rolling
    // over into the 2nd of March.
    let p = phases();
    let t = tags();
    let refusal = validate_event(ok_event("probate", "x", "2010-02-31"), vocab(&p, &t))
        .expect_err("there is no 31st of February");
    assert_eq!(refusal.value(), Some("2010-02-31"));
}

#[test]
fn an_unknown_precision_is_refused_and_names_the_valid_ones() {
    let p = phases();
    let t = tags();
    let mut submitted = ok_event("probate", "x", "2010-01-01");
    submitted.date_precision = Some("decade");
    let refusal = validate_event(submitted, vocab(&p, &t)).expect_err("no such precision");
    let message = refusal.to_string();
    assert!(message.contains("decade"), "got: {message}");
    assert!(
        message.contains("day"),
        "the refusal must list what IS valid: {message}"
    );
    assert!(!refusal.is_unprocessable());
}

#[test]
fn the_unknown_precision_is_refused_because_an_event_always_has_a_date() {
    // `unknown` is a real DatePrecision — a DOCUMENT may carry no usable date at
    // all — and it is refused HERE for a different reason from a typo. The two
    // messages must differ, or an operator is told to check the spelling of a
    // word they spelled correctly.
    let p = phases();
    let t = tags();
    let mut typo = ok_event("probate", "x", "2010-01-01");
    typo.date_precision = Some("dya");
    let mut deliberate = ok_event("probate", "x", "2010-01-01");
    deliberate.date_precision = Some("unknown");

    let typo_message = validate_event(typo, vocab(&p, &t))
        .expect_err("a typo is refused")
        .to_string();
    let deliberate_message = validate_event(deliberate, vocab(&p, &t))
        .expect_err("unknown is refused on an event")
        .to_string();
    assert_ne!(typo_message, deliberate_message);
    assert!(
        deliberate_message.contains("always has a date"),
        "got: {deliberate_message}"
    );
}

#[test]
fn an_unknown_phase_is_a_422_that_names_the_value_and_the_real_phases() {
    // ⚑ The instruction's own words: "an unknown phase is a 422 naming the
    // value, never a 500". Without this rule the slug reaches Postgres, the
    // foreign key refuses it, and the caller is handed a 500 over a typo.
    let p = phases();
    let t = tags();
    let refusal = validate_event(ok_event("apeals", "x", "2012-04-12"), vocab(&p, &t))
        .expect_err("no phase named apeals");
    assert!(
        refusal.is_unprocessable(),
        "an unknown value is a 422, not a 400"
    );
    assert_eq!(refusal.value(), Some("apeals"));
    assert_eq!(refusal.field(), Some("phase"));
    let message = refusal.to_string();
    assert!(message.contains("apeals"), "got: {message}");
    assert!(
        message.contains("appeals"),
        "the real phases must be listed: {message}"
    );
}

#[test]
fn a_phase_is_matched_exactly_and_not_by_prefix_or_case() {
    let p = phases();
    let t = tags();
    for wrong in ["Probate", "prob", "probates", " "] {
        assert!(
            validate_event(ok_event(wrong, "x", "2010-01-01"), vocab(&p, &t)).is_err(),
            "'{wrong}' must not pass as a phase"
        );
    }
}

#[test]
fn an_unknown_tag_is_a_422_that_says_a_new_tag_is_a_row() {
    let p = phases();
    let t = tags();
    let submitted_tags = vec!["sanctions".to_string()];
    let mut submitted = ok_event("probate", "x", "2010-01-01");
    submitted.tags = Some(&submitted_tags);
    let refusal = validate_event(submitted, vocab(&p, &t)).expect_err("no such tag");
    assert!(refusal.is_unprocessable());
    assert_eq!(refusal.value(), Some("sanctions"));
    // The message tells the reader what to DO about it. "A new tag is a row"
    // is the design's own answer (R7) and the fix takes no code.
    assert!(refusal.to_string().contains("row"), "got: {refusal}");
}

#[test]
fn an_empty_note_is_refused() {
    let refusal = validate_note("  \n ").expect_err("an empty note says nothing");
    assert_eq!(refusal, ChronologyWriteRefusal::BlankNote);
    assert!(!refusal.is_unprocessable());
}

#[test]
fn a_note_keeps_its_words_and_loses_its_padding() {
    assert_eq!(
        validate_note("  Need the certified copy.  ").expect("a real note"),
        "Need the certified copy."
    );
}

#[test]
fn a_link_needs_both_halves_of_its_identity() {
    let refusal = validate_link("", "doc-x", None, None).expect_err("no target type");
    assert_eq!(refusal.field(), Some("target_type"));
    let refusal = validate_link("document", "   ", None, None).expect_err("no target id");
    assert_eq!(refusal.field(), Some("target_id"));
}

#[test]
fn an_absent_pinpoint_stays_absent_and_a_blank_one_becomes_absent() {
    // ⚑ Design R9: the ABSENCE is what marks the link, so it must survive as an
    // absence. Normalising `None` to `Some("")` would store a pinpoint of
    // nothing and the "no pinpoint" mark would stop appearing — which is how
    // the to-scan to-do list would quietly empty itself.
    let absent = validate_link("document", "doc-x", None, None).expect("valid");
    assert_eq!(absent.pinpoint, None);
    let blank = validate_link("document", "doc-x", Some("Morris"), Some("   ")).expect("valid");
    assert_eq!(blank.pinpoint, None);
    assert_eq!(blank.label, Some("Morris".to_string()));

    let real = validate_link("document", "doc-x", None, Some(" p. 2 ¶ 4 ")).expect("valid");
    assert_eq!(real.pinpoint, Some("p. 2 ¶ 4".to_string()));
    // No label supplied stays None — the surface falls back to the target id at
    // render time rather than this layer inventing a name for the document.
    assert_eq!(real.label, None);
}

// ── The bag ─────────────────────────────────────────────────────────────────

#[test]
fn editing_tags_keeps_every_other_attribute() {
    // ⚑ THE CHANGE RULE AT THE MOMENT IT MATTERS. Every seeded event carries
    // `source: legacy_json`, and a future task may promote `people` or `spine`.
    // Rebuilding the bag from the request would delete all of them on the first
    // edit of every one of the 22.
    let stored = serde_json::json!({
        "tags": ["filing"],
        "source": "legacy_json",
        "people": ["Judge Tighe"],
        "spine": true,
    });
    let merged = merged_attributes(&stored, Some(&["court_action".to_string()]));
    assert_eq!(merged["tags"], serde_json::json!(["court_action"]));
    assert_eq!(merged["source"], serde_json::json!("legacy_json"));
    assert_eq!(merged["people"], serde_json::json!(["Judge Tighe"]));
    assert_eq!(merged["spine"], serde_json::json!(true));
}

#[test]
fn an_absent_tag_list_leaves_the_stored_tags_alone() {
    let stored = serde_json::json!({ "tags": ["filing"], "source": "legacy_json" });
    let merged = merged_attributes(&stored, None);
    assert_eq!(merged["tags"], serde_json::json!(["filing"]));
}

#[test]
fn an_empty_tag_list_clears_the_tags_rather_than_being_ignored() {
    // Absent and empty are DIFFERENT instructions: "leave them" and "remove
    // them". Collapsing the two would make un-tagging an event impossible.
    let stored = serde_json::json!({ "tags": ["filing"] });
    let merged = merged_attributes(&stored, Some(&[]));
    assert_eq!(merged["tags"], serde_json::json!([]));
}

#[test]
fn a_bag_that_is_not_an_object_becomes_one() {
    // The column is documented as an object and defaults to `{}`. A row that
    // somehow holds an array has no keys worth keeping, and returning it
    // unchanged would write a value the reader cannot use.
    let merged = merged_attributes(
        &serde_json::json!(["filing"]),
        Some(&["filing".to_string()]),
    );
    assert!(merged.is_object(), "got: {merged}");
    assert_eq!(merged["tags"], serde_json::json!(["filing"]));

    let from_null = merged_attributes(&serde_json::Value::Null, None);
    assert_eq!(from_null, serde_json::json!({}));
}
