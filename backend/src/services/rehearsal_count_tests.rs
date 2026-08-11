// Tests for `services::rehearsal_count`.
//
// These pin the two sentences the prep page opens and heads its chronology with.
// Both are composed rather than templated because they have to be grammatical at
// ONE, and "They said it 1 times" is the kind of slip that makes a reader stop
// trusting a page they are about to be cross-examined against.

use super::*;
use crate::domain::settings::Settings;
use crate::dto::rehearsal::{RehearsalAnswer, RehearsalInstance, RehearsalSource};

fn source() -> RehearsalSource {
    RehearsalSource {
        label: String::new(),
        href: String::new(),
        open_label: String::new(),
    }
}

fn settings() -> Settings {
    Settings::for_test()
}

/// One instance, with or without a date and with or without an answer.
fn instance(position: usize, when: Option<&str>, answered: bool) -> RehearsalInstance {
    RehearsalInstance {
        position,
        // The count line does not read the code; a fixture value keeps the shape
        // honest without pretending this test is about it.
        code: Some("C-14".to_string()),
        phase: "Probate".to_string(),
        who: "George Phillips".to_string(),
        when: when.map(str::to_string),
        when_gap: when.is_none().then(|| "No date yet".to_string()),
        source: source(),
        kind_label: "Statement".to_string(),
        quote: "…the parties did not cooperate.".to_string(),
        quote_first_line: "…the parties did not cooperate.".to_string(),
        answer: answered.then(|| RehearsalAnswer {
            who: "Marie Awad".to_string(),
            when: None,
            when_gap: None,
            code: Some("C-15".to_string()),
            source: source(),
            quote: "I wrote to him twice.".to_string(),
        }),
        answer_tag: "ANSWERED".to_string(),
        answer_banner: None,
    }
}

// ── The opening count line ──────────────────────────────────────────────────

/// The plural forms are CHOSEN, not concatenated.
///
/// The whole reason this function exists rather than one template: at one
/// instance in one document the sentence has to read "1 time, in 1 document".
#[test]
fn one_instance_in_one_document_reads_singular() {
    let line = plain_count_line(&[instance(1, Some("2009-12-15"), true)], 1, &settings())
        .expect("a marked instance produces a line");
    assert!(line.contains("1 time,"), "{line}");
    assert!(line.contains("1 document"), "{line}");
    assert!(
        !line.contains("1 times"),
        "the singular must not be the plural: {line}"
    );
}

#[test]
fn several_instances_read_plural() {
    let instances = [
        instance(1, Some("2009-12-15"), true),
        instance(2, Some("2011-03-01"), true),
        instance(3, Some("2012-01-12"), false),
    ];
    let line = plain_count_line(&instances, 2, &settings()).expect("a line");
    assert!(line.contains("3 times"), "{line}");
    assert!(line.contains("2 documents"), "{line}");
}

/// The span is a RANGE when the dated instances differ.
///
/// The endpoints are read off the list in order, which is why the caller has to
/// have sorted it — a second ordering here could disagree with the cards.
#[test]
fn the_span_names_the_first_and_last_dates() {
    let instances = [
        instance(1, Some("December 2009"), true),
        instance(2, Some("March 2011"), true),
        instance(3, Some("October 2015"), true),
    ];
    let line = plain_count_line(&instances, 3, &settings()).expect("a line");
    assert!(line.contains("from December 2009"), "{line}");
    assert!(line.contains("through October 2015"), "{line}");
}

/// One shared date gets its own clause, not a range with both ends the same.
#[test]
fn a_single_shared_date_reads_on_that_date() {
    let instances = [
        instance(1, Some("December 2009"), true),
        instance(2, Some("December 2009"), true),
    ];
    let line = plain_count_line(&instances, 1, &settings()).expect("a line");
    assert!(line.contains("on December 2009"), "{line}");
    assert!(
        !line.contains("through"),
        "\"from December 2009 through December 2009\" is a sentence nobody would \
         write: {line}"
    );
}

/// No dates at all — the clause is OMITTED, not invented.
///
/// 57% of this case's evidence carries no date, so this is the common path and
/// not an edge. The sentence has to close up cleanly around the missing clause.
#[test]
fn nothing_dated_drops_the_span_clause_entirely() {
    let instances = [instance(1, None, false), instance(2, None, false)];
    let line = plain_count_line(&instances, 1, &settings()).expect("a line");
    assert!(line.contains("2 times"), "{line}");
    assert!(!line.contains("from"), "{line}");
    assert!(!line.contains(" on "), "{line}");
    assert!(
        !line.contains(",,"),
        "the missing clause must not strand a comma: {line}"
    );
}

#[test]
fn nothing_marked_produces_no_line_at_all() {
    assert_eq!(plain_count_line(&[], 0, &settings()), None);
}

// ── The chronology section's count ──────────────────────────────────────────

/// Everything answered: no "to prepare" clause.
#[test]
fn a_finished_scenario_does_not_offer_an_empty_to_do() {
    let instances = [
        instance(1, Some("2009-12"), true),
        instance(2, Some("2011-03"), true),
    ];
    let line = answered_line(&instances, &settings()).expect("a line");
    assert!(line.contains("2 of 2 answered"), "{line}");
    assert!(
        !line.contains("to prepare"),
        "\"2 of 2 answered — 0 to prepare\" is a to-do list with an empty item: {line}"
    );
}

/// Work remaining: the second clause says how much, so nobody has to subtract.
#[test]
fn an_unfinished_scenario_says_how_much_is_left() {
    let instances = [
        instance(1, Some("2009-12"), true),
        instance(2, Some("2011-03"), false),
        instance(3, Some("2012-01"), false),
    ];
    let line = answered_line(&instances, &settings()).expect("a line");
    assert!(line.contains("1 of 3 answered"), "{line}");
    assert!(line.contains("2 to prepare"), "{line}");
}

#[test]
fn nothing_marked_heads_the_section_with_nothing() {
    assert_eq!(answered_line(&[], &settings()), None);
}
