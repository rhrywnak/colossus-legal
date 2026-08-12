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
        question: None,
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
    assert!(answer_of("ev-answer", &facts, &wording(), &HashMap::new()).is_none());
}

#[test]
fn an_answer_with_no_words_is_also_none() {
    let mut hollow = fact();
    hollow.graph_node_id = "ev-answer".to_string();
    hollow.quote = Some("   ".to_string());
    let facts: HashMap<String, RehearsalFactRow> =
        [("ev-answer".to_string(), hollow)].into_iter().collect();

    assert!(answer_of("ev-answer", &facts, &wording(), &HashMap::new()).is_none());
}

#[test]
fn a_real_answer_carries_its_words_its_speaker_and_its_source() {
    let mut answer = fact();
    answer.graph_node_id = "ev-answer".to_string();
    answer.quote = Some("I am open to dividing the property.".to_string());
    answer.speaker = Some("Marie Awad".to_string());
    let facts: HashMap<String, RehearsalFactRow> =
        [("ev-answer".to_string(), answer)].into_iter().collect();

    // An ordinal for this node, so the handle it carries is asserted rather
    // than assumed absent (task R4, P3).
    let ordinals: HashMap<String, i32> = [("ev-answer".to_string(), 14)].into_iter().collect();

    let built = answer_of("ev-answer", &facts, &wording(), &ordinals).expect("an answer");
    assert_eq!(built.quote, "I am open to dividing the property.");
    assert_eq!(built.who, "Marie Awad");
    assert_eq!(built.source.label, "Hearing to approve plan, p. 24");
    assert_eq!(built.code.as_deref(), Some("C-14"));
}

// ── The authorship line (task 2.11 C, ruling C2) ─────────────────────────────

/// A fixed instant, so the assertion below is about the FORMAT and not the day
/// the suite happens to run.
fn at(year: i32, month: u32, day: u32) -> chrono::DateTime<chrono::Utc> {
    chrono::NaiveDate::from_ymd_opt(year, month, day)
        .expect("a real date")
        .and_hms_opt(14, 30, 0)
        .expect("a real time")
        .and_utc()
}

#[test]
fn a_recorded_author_and_date_compose_the_stored_sentence() {
    let line = attribution_line(
        "Written in plain words by {who} · {when}",
        "Author not recorded — written before authorship was kept.",
        Some("Roman"),
        Some(at(2026, 8, 6)),
    );
    assert_eq!(line, "Written in plain words by Roman · 6 Aug 2026");
}

#[test]
fn the_date_reads_the_way_a_person_says_it() {
    // The same rule the instance rows follow: a month NAME cannot be misread
    // day-month by one reader and month-day by another, and no leading zero —
    // "6 Aug 2026", not "06 Aug 2026".
    let line = attribution_line(
        "{who} · {when}",
        "unknown",
        Some("Roman"),
        Some(at(2026, 8, 6)),
    );
    assert_eq!(line, "Roman · 6 Aug 2026");
}

#[test]
fn a_sentence_written_before_authorship_was_kept_says_so() {
    // The honest-gap law, applied to provenance. Never an invented name, never a
    // blank — a blank reads as a rendering fault.
    let line = attribution_line(
        "Written by {who} · {when}",
        "Author not recorded — written before authorship was kept.",
        None,
        None,
    );
    assert_eq!(
        line,
        "Author not recorded — written before authorship was kept."
    );
}

#[test]
fn a_half_recorded_authorship_is_treated_as_none() {
    // The two columns are written by ONE statement, so an author without a date
    // means something wrote around that route. Composing "Written by Roman · "
    // would render an attribution with the day silently missing, claiming more
    // than the record holds.
    let unknown = "Author not recorded — written before authorship was kept.";

    assert_eq!(
        attribution_line("{who} · {when}", unknown, Some("Roman"), None),
        unknown,
        "an author with no date is not an attribution"
    );
    assert_eq!(
        attribution_line("{who} · {when}", unknown, None, Some(at(2026, 8, 6))),
        unknown,
        "a date with no author is not an attribution"
    );
}

#[test]
fn a_blank_author_column_is_an_absence_and_not_a_name() {
    // Whitespace is invisible in psql. An author of "   " would otherwise
    // compose "Written by  · 6 Aug 2026" — an attribution attributing nobody.
    let unknown = "not recorded";
    assert_eq!(
        attribution_line("{who} · {when}", unknown, Some("   "), Some(at(2026, 8, 6))),
        unknown
    );
}

/// A candidate nothing has numbered carries NO code — never its node id.
///
/// The auditor's note on task R4: the `None` arm was reachable only through
/// `answer_of` returning early, so it was never exercised on its own. It matters
/// on its own, because the alternative a future edit would reach for is
/// `unwrap_or(graph_node_id)` — and an id in a slot labelled "code" reads as a
/// handle and gets quoted as one out loud.
#[test]
fn an_unnumbered_candidate_has_no_code_rather_than_an_id() {
    assert_eq!(code_of("ev-unnumbered", &HashMap::new()), None);

    let ordinals: HashMap<String, i32> = [("ev-other".to_string(), 14)].into_iter().collect();
    assert_eq!(code_of("ev-unnumbered", &ordinals), None);
    assert_eq!(code_of("ev-other", &ordinals).as_deref(), Some("C-14"));
}

// ── The question a statement answers (task 394, P2) ──────────────────────────

/// The question travels verbatim when the record holds one.
///
/// Measured on DEV, this is S-6's own case: `…response-to-discovery:evidence:
/// 0fd1a748` quotes the single word "Yes." and carries the interrogatory that
/// makes it an admission. The card is unreadable without the pair.
#[test]
fn a_discovery_answer_carries_the_question_it_answers() {
    let mut fact = fact();
    fact.quote = Some("Yes.".to_string());
    fact.question = Some(
        "Did George Phillips on behalf of Catholic Family Services make the argument?".to_string(),
    );

    assert_eq!(
        question_of(&fact).as_deref(),
        Some("Did George Phillips on behalf of Catholic Family Services make the argument?")
    );
}

/// Documentary evidence answers nobody, and says so by carrying nothing.
///
/// A court finding has no question. An empty `Q:` line above one would assert a
/// question exists and was lost — on the page read in front of opposing counsel.
#[test]
fn documentary_evidence_carries_no_question() {
    assert_eq!(question_of(&fact()), None);
}

/// A blank question is the same absence as a missing one.
///
/// The extraction writes the property on every discovery item it reads, and an
/// item whose question it could not read gets `""`. There is no different act a
/// human could take for the two states — see [`super::question_of`].
#[test]
fn a_blank_question_is_an_absence_and_not_an_empty_line() {
    let mut fact = fact();

    fact.question = Some(String::new());
    assert_eq!(question_of(&fact), None);

    fact.question = Some("   \n ".to_string());
    assert_eq!(question_of(&fact), None, "whitespace is invisible in psql");
}

/// A question with surrounding whitespace is TRIMMED, never rendered padded.
#[test]
fn a_padded_question_is_trimmed_to_its_words() {
    let mut fact = fact();
    fact.question = Some("  Identify the time period.  ".to_string());

    assert_eq!(
        question_of(&fact).as_deref(),
        Some("Identify the time period.")
    );
}

/// The question reaches OUR ANSWER too, not only the accusation it answers.
///
/// Five of the nine pairings on DEV point at a discovery response and four of
/// those quote a bare affirmation, so the answer half is where the bare-syllable
/// defect actually bites.
#[test]
fn our_answer_carries_its_own_question() {
    let mut fact = fact();
    fact.quote = Some("Yes".to_string());
    fact.question = Some("Did you receive the certified letter?".to_string());

    let facts: HashMap<String, RehearsalFactRow> =
        [("ev-answer".to_string(), fact)].into_iter().collect();

    let answer = answer_of("ev-answer", &facts, &wording(), &HashMap::new())
        .expect("a quoted answer composes");

    assert_eq!(
        answer.question.as_deref(),
        Some("Did you receive the certified letter?")
    );
}
