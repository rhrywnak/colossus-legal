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
        question: None,
        quote_first_line: "…the parties did not cooperate.".to_string(),
        answer: answered.then(|| RehearsalAnswer {
            who: "Marie Awad".to_string(),
            when: None,
            when_gap: None,
            code: Some("C-15".to_string()),
            source: source(),
            quote: "I wrote to him twice.".to_string(),
            question: None,
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

// ── The span clause against a MIXED list (task 394, P4b) ─────────────────────
//
// The three tests above cover all-dated, all-one-date and none-dated. The list
// this page actually renders is none of those: measured on DEV, S-5 has three
// dated instances and two undated, and S-6 has one dated of four. Both are
// exercised here, because the endpoints are read off a list whose TAIL is
// undated and an off-by-one at that boundary would name an empty date.

/// Undated instances at the tail do not become the range's far endpoint.
///
/// `walk_instances` sorts undated LAST, so the final element of the list is
/// routinely `None`. A span that read the list's last ELEMENT rather than its
/// last DATED one would compose "through " with nothing after it — the empty
/// endpoint the clause is omitted entirely to avoid.
#[test]
fn the_span_ignores_the_undated_tail_and_ends_at_the_last_dated_item() {
    let instances = [
        instance(1, Some("December 2009"), true),
        instance(2, Some("January 2012"), true),
        instance(3, None, false),
        instance(4, None, false),
    ];
    let line = plain_count_line(&instances, 3, &settings()).expect("a line");

    assert!(line.contains("from December 2009"), "{line}");
    assert!(line.contains("through January 2012"), "{line}");
    assert!(
        line.contains("4 times"),
        "the count is of ALL of them: {line}"
    );
    assert!(
        !line.contains("through  ") && !line.trim_end().ends_with("through"),
        "an undated tail must never become an endpoint: {line}"
    );
}

/// One dated instance among undated ones reads as the one date it has.
///
/// ## What this test PINS, and the honesty question it does not settle
///
/// This is S-6 exactly: four placed statements, one of which carries a date. The
/// clause says "on <that date>" because the dated items — all one of them —
/// genuinely share a day, which is the rule as ruled.
///
/// What it cannot say is that the OTHER three happened then, and the sentence
/// does not distinguish: "They said it 4 times, in 3 documents, on 4 Oct 2010"
/// reads as a claim about all four. The clause describes the dated SUBSET while
/// the sentence around it describes the whole set. Naming the subset would need
/// a wording row of its own and is filed rather than smuggled in here; the
/// behaviour is pinned so the day that row lands, this test is what has to
/// change deliberately.
#[test]
fn one_dated_instance_among_undated_ones_names_that_date_alone() {
    let instances = [
        instance(1, Some("October 2010"), true),
        instance(2, None, true),
        instance(3, None, false),
        instance(4, None, false),
    ];
    let line = plain_count_line(&instances, 3, &settings()).expect("a line");

    assert!(line.contains("on October 2010"), "{line}");
    assert!(!line.contains("through"), "one date is not a range: {line}");
}

/// A blank date string is an absence, not an endpoint.
///
/// `when` is composed by `when_of`, which never returns a blank — but the filter
/// that guarantees it lives in `plain_count_line`, and a future caller building
/// this list another way would otherwise open the sentence with "from  through".
#[test]
fn a_blank_date_string_is_not_an_endpoint() {
    let instances = [
        instance(1, Some("   "), true),
        instance(2, Some("March 2011"), true),
    ];
    let line = plain_count_line(&instances, 2, &settings()).expect("a line");

    assert!(line.contains("on March 2011"), "{line}");
    assert!(!line.contains("from  "), "{line}");
}
