//! Tests for [`super`] — one row's sentences.
//!
//! Small, pure functions, and every one of them decides something a witness reads
//! under oath: which words are shown, whether a date is claimed, whether a source
//! can be produced. The interesting cases are all absences.

use super::*;
use crate::domain::wording_rehearsal::RehearsalWording;

fn wording() -> RehearsalWording {
    RehearsalWording::for_test()
}

fn fact() -> RehearsalFactRow {
    RehearsalFactRow {
        graph_node_id: "ev-a".to_string(),
        quote: Some("They refused.".to_string()),
        speaker: Some("George Phillips".to_string()),
        statement_type: Some("attorney_argument".to_string()),
        occurred_on: Some("2009-12-15".to_string()),
        document_id: Some("doc-hearing".to_string()),
        document_title: Some("Hearing to approve plan".to_string()),
        page: Some(24),
    }
}

// ── The first line ───────────────────────────────────────────────────────────

#[test]
fn a_short_quote_is_returned_whole_with_no_ellipsis() {
    // An ellipsis on a complete sentence claims there is more. On a page whose
    // whole promise is that its quotes are verbatim, that is a lie in miniature.
    assert_eq!(first_line("They refused."), "They refused.");
    assert!(!first_line("They refused.").ends_with('…'));
}

#[test]
fn a_long_quote_is_cut_at_a_word_boundary() {
    // "…refused to divide the prop" reads as a transcription error.
    let long = "They refused every reasonable attempt to divide the property amicably, and \
                continued to refuse for eleven months afterwards.";
    let cut = first_line(long);

    assert!(cut.ends_with('…'));
    let body = cut.trim_end_matches('…');
    assert!(
        long.starts_with(body),
        "the cut must be a prefix of the quote: {body:?}"
    );
    assert!(
        !body.ends_with(' '),
        "no trailing space before the ellipsis: {cut:?}"
    );
    // The last word survives whole — the cut backed up to a boundary.
    let last = body.split_whitespace().last().expect("a word");
    assert!(long.contains(&format!("{last} ")) || long.ends_with(last));
}

#[test]
fn a_single_enormous_token_is_cut_rather_than_dropped() {
    // No word boundary to back up to. Cutting mid-token is then the only option,
    // and it beats returning nothing on a row that has to identify a statement.
    let token = "a".repeat(200);
    let cut = first_line(&token);
    assert!(cut.ends_with('…'));
    assert!(cut.len() > 1);
}

#[test]
fn a_multi_byte_quote_does_not_panic_at_the_cut() {
    // Quotes are arbitrary human text. `&s[..n]` on a byte offset inside a
    // character panics, which on this path would take the whole page down.
    let quote = "«".repeat(200);
    let cut = first_line(&quote);
    assert!(cut.ends_with('…'));
}

#[test]
fn surrounding_whitespace_never_reaches_the_row() {
    assert_eq!(first_line("  They refused.  "), "They refused.");
}

// ── The source ───────────────────────────────────────────────────────────────

#[test]
fn the_source_names_the_document_and_the_page_and_opens_at_it() {
    let source = source_of(&fact(), &wording());
    assert_eq!(source.label, "Hearing to approve plan, p. 24");
    assert_eq!(source.href, "/documents/doc-hearing?page=24&tab=document");
}

#[test]
fn a_statement_with_no_page_still_opens_its_document() {
    let mut f = fact();
    f.page = None;
    let source = source_of(&f, &wording());
    assert_eq!(source.href, "/documents/doc-hearing?tab=document");
}

#[test]
fn a_statement_whose_document_is_unknown_gets_no_link_rather_than_a_dead_one() {
    // The same rule the working view's pinpoints follow: a link to nowhere is
    // worse than no link, because a reader clicks it in front of opposing counsel.
    let mut f = fact();
    f.document_id = None;
    assert!(source_of(&f, &wording()).href.is_empty());
}

// ── Who, and when ────────────────────────────────────────────────────────────

#[test]
fn an_unrecorded_speaker_is_named_as_unrecorded() {
    // A blank reads as a rendering fault; this says the record is silent, which is
    // a different and checkable thing. Measured on S-2: one statement of 46.
    let mut f = fact();
    f.speaker = None;
    assert_eq!(who_of(&f, &wording()), "Speaker not recorded");

    f.speaker = Some("   ".to_string());
    assert_eq!(who_of(&f, &wording()), "Speaker not recorded");
}

#[test]
fn a_recorded_speaker_is_carried_verbatim() {
    assert_eq!(who_of(&fact(), &wording()), "George Phillips");
}

#[test]
fn exactly_one_of_the_date_and_its_gap_is_ever_present() {
    let (when, gap) = when_of(&fact(), &wording());
    assert_eq!(when.as_deref(), Some("15 Dec 2009"));
    assert_eq!(gap, None);

    let mut undated = fact();
    undated.occurred_on = None;
    let (when, gap) = when_of(&undated, &wording());
    assert_eq!(when, None);
    assert_eq!(gap.as_deref(), Some("No date on this statement"));
}

#[test]
fn a_stored_date_reads_the_way_a_person_says_it() {
    // Ruled 2026-08-06. A month NAME cannot be misread day-month by one reader and
    // month-day by another, which is the ambiguity that matters here — and Marie
    // reads this under stress, where "2009-12-15" is engineer-speak.
    let (when, _) = when_of(&fact(), &wording());
    assert_eq!(when.as_deref(), Some("15 Dec 2009"));
}

#[test]
fn a_single_digit_day_is_not_zero_padded() {
    // "5 Dec 2009", not "05 Dec 2009" — how a person says it out loud, which is
    // what the `-` in `%-d` is for.
    assert_eq!(display_date("2009-12-05"), "5 Dec 2009");
    assert_eq!(display_date("2009-01-01"), "1 Jan 2009");
}

#[test]
fn a_date_that_does_not_parse_is_rendered_exactly_as_stored() {
    // THE POINT of the function. A partial or malformed value is a data-integrity
    // question for task 2.5; inventing a day for it would put a date in a witness's
    // mouth, and blanking it would hide a date the record does have. Showing it raw
    // tells the human precisely what is stored.
    for stored in ["2009", "2009-12", "circa 2009", "15/12/2009", "not a date"] {
        assert_eq!(
            display_date(stored),
            stored,
            "an unparseable value must survive verbatim"
        );
    }
}

#[test]
fn an_impossible_date_is_not_rolled_forward_into_a_real_one() {
    // The failure a lenient parser would produce: 2009-02-30 silently becoming
    // "2 Mar 2009", a date the record does not contain, on a page read under oath.
    // chrono refuses it, so it falls to the verbatim branch.
    assert_eq!(display_date("2009-02-30"), "2009-02-30");
    assert_eq!(display_date("2009-13-01"), "2009-13-01");
}

#[test]
fn surrounding_whitespace_never_reaches_a_rendered_date() {
    assert_eq!(display_date("  2009-12-15  "), "15 Dec 2009");
    assert_eq!(display_date("  circa 2009  "), "circa 2009");
}

#[test]
fn every_month_renders_its_three_letter_english_name() {
    // A shared case file read by three people: the month must not depend on the
    // machine's locale, and a wrong month is a wrong answer under oath.
    let months = [
        ("2009-01-15", "Jan"),
        ("2009-02-15", "Feb"),
        ("2009-03-15", "Mar"),
        ("2009-04-15", "Apr"),
        ("2009-05-15", "May"),
        ("2009-06-15", "Jun"),
        ("2009-07-15", "Jul"),
        ("2009-08-15", "Aug"),
        ("2009-09-15", "Sep"),
        ("2009-10-15", "Oct"),
        ("2009-11-15", "Nov"),
        ("2009-12-15", "Dec"),
    ];
    for (stored, month) in months {
        assert_eq!(display_date(stored), format!("15 {month} 2009"));
    }
}

#[test]
fn a_blank_date_string_is_an_absence_not_a_date() {
    let mut f = fact();
    f.occurred_on = Some("  ".to_string());
    let (when, gap) = when_of(&f, &wording());
    assert_eq!(when, None);
    assert!(gap.is_some());
}

// ── The kind ─────────────────────────────────────────────────────────────────

#[test]
fn the_kind_is_humanized_and_an_unknown_one_is_not_invented() {
    assert_eq!(kind_of(&fact()), "attorney argument");

    let mut f = fact();
    f.statement_type = Some("some_new_kind".to_string());
    assert_eq!(kind_of(&f), "some new kind");

    f.statement_type = None;
    assert_eq!(kind_of(&f), "");
}

// ── The answer ───────────────────────────────────────────────────────────────

#[test]
fn an_answer_the_record_no_longer_holds_is_none_not_a_hollow_row() {
    // The caller then renders the Remove law's named gap. A half-built answer with
    // an empty quote would put empty quotation marks under "Our answer", which
    // reads as us having said nothing.
    let facts: HashMap<String, RehearsalFactRow> = HashMap::new();
    assert!(answer_of("ev-answer", &facts, &wording()).is_none());
}

#[test]
fn an_answer_with_no_words_is_also_none() {
    let mut hollow = fact();
    hollow.graph_node_id = "ev-answer".to_string();
    hollow.quote = Some("   ".to_string());
    let facts: HashMap<String, RehearsalFactRow> =
        [("ev-answer".to_string(), hollow)].into_iter().collect();

    assert!(answer_of("ev-answer", &facts, &wording()).is_none());
}

#[test]
fn a_real_answer_carries_its_words_its_speaker_and_its_source() {
    let mut answer = fact();
    answer.graph_node_id = "ev-answer".to_string();
    answer.quote = Some("I am open to dividing the property.".to_string());
    answer.speaker = Some("Marie Awad".to_string());
    let facts: HashMap<String, RehearsalFactRow> =
        [("ev-answer".to_string(), answer)].into_iter().collect();

    let built = answer_of("ev-answer", &facts, &wording()).expect("an answer");
    assert_eq!(built.quote, "I am open to dividing the property.");
    assert_eq!(built.who, "Marie Awad");
    assert_eq!(built.source.label, "Hearing to approve plan, p. 24");
}
