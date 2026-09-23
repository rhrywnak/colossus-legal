// Tests for `services::for_you_rows` — the three sentences one row speaks.
//
// Pure: no pool, no clock of its own beyond the fixtures' fixed moments. What
// they hold down is the part of this page a live test cannot see — that the
// same event reads differently to the two sides, that a login never reaches the
// screen, and that each row says what KIND of thing it is (ruling Q1).

use super::*;
use chrono::TimeZone;
use uuid::Uuid;

/// The case's timezone in every fixture below — the DEV/PROD value.
const TZ: &str = "America/Detroit";

fn wording() -> ForYouWording {
    // The block's own fixture, built through the PRODUCTION builder — so these
    // tests read the values the migration seeds, not a second copy of them.
    ForYouWording::for_test()
}

fn logins() -> Vec<String> {
    vec!["cpenzien".to_string(), "roman".to_string()]
}

fn names() -> Vec<String> {
    vec!["Chuck".to_string(), "Roman".to_string()]
}

/// A moment in the case's timezone, as UTC.
fn at(year: i32, month: u32, day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, hour, 30, 0)
        .single()
        .expect("a real moment")
}

fn voice<'a>(
    w: &'a ForYouWording,
    logins: &'a [String],
    names: &'a [String],
    side: ForYouSide,
) -> RowVoice<'a> {
    RowVoice {
        wording: w,
        timezone: TZ,
        side,
        reviewer_logins: logins,
        reviewer_names: names,
        witness_login: "docmarie",
        // 22 Sep 2026, the day the fixtures below are written against.
        today: NaiveDate::from_ymd_opt(2026, 9, 22).expect("a real day"),
    }
}

/// One waiting item, with everything the composer reads.
fn row(kind: &str, author: Option<&str>, when: DateTime<Utc>) -> WaitingItemRow {
    WaitingItemRow {
        kind: kind.to_string(),
        item_id: Uuid::from_u128(1),
        scenario_id: Uuid::from_u128(2),
        code_ordinal: Some(11),
        scenario_name: "The $50,000".to_string(),
        question_id: Some(Uuid::from_u128(3)),
        question_text: Some("Whose money was the check?".to_string()),
        at: when,
        author: author.map(|a| a.to_string()),
        seen_at: None,
        body: Some("her words".to_string()),
        subject_at: None,
        reply_to: None,
    }
}

/// (M) A login NEVER reaches the screen: every party has a stored name.
///
/// `cpenzien left a note` is a database identifier shown to a lawyer. The
/// reviewers' names are an index-aligned settings row, the witness's and the
/// unattributed case are stored strings, and only a person in none of those
/// lists falls back to their login — which still tells the reader who to ask.
#[test]
fn every_party_is_named_by_a_stored_string_not_a_login() {
    let w = wording();
    let (l, n) = (logins(), names());
    let v = voice(&w, &l, &n, ForYouSide::Witness);
    assert_eq!(
        v.display_name(Some("cpenzien")),
        "Chuck",
        "the first reviewer"
    );
    assert_eq!(v.display_name(Some("roman")), "Roman", "the second");
    assert_eq!(v.display_name(Some("docmarie")), "Marie", "the witness");
    assert_eq!(
        v.display_name(None),
        "Someone",
        "an item from before attribution existed"
    );
    assert_eq!(
        v.display_name(Some("astranger")),
        "astranger",
        "somebody with no stored name at all — the login is the last resort"
    );
}

/// A names list SHORTER than the logins list never panics.
///
/// The boot check refuses two lists of different lengths, so this state cannot
/// reach a running server — but `get` rather than `[at]` is what makes that
/// refusal a safety net instead of the only thing between a Settings edit and a
/// panic on a page.
#[test]
fn a_short_display_name_list_falls_back_instead_of_panicking() {
    let w = wording();
    let (l, n) = (logins(), vec!["Chuck".to_string()]);
    let v = voice(&w, &l, &n, ForYouSide::Witness);
    assert_eq!(v.display_name(Some("cpenzien")), "Chuck");
    assert_eq!(v.display_name(Some("roman")), "roman", "no name stored");
}

/// (M) Today, yesterday and everything older, in the CASE's timezone.
#[test]
fn rows_group_into_today_yesterday_and_earlier() {
    let w = wording();
    let (l, n) = (logins(), names());
    let v = voice(&w, &l, &n, ForYouSide::Witness);
    // 12:30 UTC on 22 Sep is 08:30 in Detroit — the same calendar day.
    assert_eq!(v.day_of(at(2026, 9, 22, 12)), ForYouDay::Today);
    assert_eq!(v.day_of(at(2026, 9, 21, 12)), ForYouDay::Yesterday);
    assert_eq!(v.day_of(at(2026, 9, 20, 12)), ForYouDay::Earlier);
    // A stamp from the FUTURE — two machines' clocks disagreeing — is listed
    // rather than dropped, and "earlier" is the honest heading for it.
    assert_eq!(v.day_of(at(2026, 9, 23, 12)), ForYouDay::Earlier);
}

/// (M) THE MIDNIGHT CASE: 02:00 UTC is still YESTERDAY in Detroit.
///
/// This is the whole reason the day is decided on the server. A browser reading
/// the same instant in UTC would file this row under today and the deck it came
/// from would call it yesterday.
#[test]
fn the_day_is_the_cases_day_and_not_utcs() {
    let w = wording();
    let (l, n) = (logins(), names());
    let v = voice(&w, &l, &n, ForYouSide::Witness);
    // 2026-09-22 02:00 UTC = 2026-09-21 22:00 in Detroit.
    let late = Utc
        .with_ymd_and_hms(2026, 9, 22, 2, 0, 0)
        .single()
        .expect("a real moment");
    assert_eq!(v.day_of(late), ForYouDay::Yesterday);
}

/// (M) The SAME note reads differently to the two sides (ruling Q1).
#[test]
fn a_note_on_an_answer_speaks_to_whoever_is_reading_it() {
    let w = wording();
    let (l, n) = (logins(), names());
    let mut item = row("note", Some("cpenzien"), at(2026, 9, 22, 15));
    item.subject_at = Some(at(2026, 9, 21, 15));
    item.body = Some("Don't say \"my dad's money\".".to_string());

    let hers = voice(&w, &l, &n, ForYouSide::Witness).compose(&item);
    assert_eq!(hers.byline, "Chuck · on your answer of Mon 21 Sep");
    assert_eq!(
        hers.body, "Don't say \"my dad's money\".",
        "a note is shown as written — it IS the sentence somebody wrote"
    );

    let his = voice(&w, &l, &n, ForYouSide::Reviewers).compose(&item);
    assert_eq!(his.byline, "Chuck · note on her answer");
}

/// Each kind names itself, and a note on the QUESTION is not a note on an answer.
#[test]
fn every_kind_says_what_it_is() {
    let w = wording();
    let (l, n) = (logins(), names());
    let v = voice(&w, &l, &n, ForYouSide::Reviewers);

    let answer = v.compose(&row("answer", Some("docmarie"), at(2026, 9, 22, 15)));
    assert_eq!(answer.byline, "Marie · new answer");
    assert_eq!(answer.body, "Answered: “her words”");

    let change = v.compose(&row("change", Some("cpenzien"), at(2026, 9, 22, 15)));
    assert_eq!(change.byline, "Chuck · changed the question");
    assert_eq!(change.body, "Now reads: “her words”");

    // No `subject_at`: the note stands on the question, not on one attempt.
    let note = v.compose(&row("note", Some("docmarie"), at(2026, 9, 22, 15)));
    assert_eq!(note.byline, "Marie · on the question");
}

/// A read row says WHEN it was read — and only on the tab that shows read rows.
#[test]
fn a_read_row_carries_the_day_it_was_read() {
    let w = wording();
    let (l, n) = (logins(), names());
    let v = voice(&w, &l, &n, ForYouSide::Witness);
    let mut item = row("note", Some("cpenzien"), at(2026, 9, 20, 15));
    item.seen_at = Some(at(2026, 9, 21, 15));
    let composed = v.compose(&item);
    assert!(composed.read);
    assert_eq!(
        composed.byline,
        "Chuck · on the question · read by you Mon 21 Sep"
    );

    item.seen_at = None;
    let unread = v.compose(&item);
    assert!(!unread.read);
    assert_eq!(unread.byline, "Chuck · on the question");
}

/// The deck line names the deck, and quotes the question when there is one.
#[test]
fn the_deck_line_quotes_the_question_or_names_the_deck_alone() {
    let w = wording();
    let (l, n) = (logins(), names());
    let v = voice(&w, &l, &n, ForYouSide::Witness);
    let item = row("note", Some("cpenzien"), at(2026, 9, 22, 15));
    assert_eq!(
        v.compose(&item).deck_line,
        "S-11 · The $50,000 — “Whose money was the check?”"
    );

    // A note about a whole scenario has no question to quote.
    let mut scenario_note = item.clone();
    scenario_note.question_id = None;
    scenario_note.question_text = None;
    assert_eq!(v.compose(&scenario_note).deck_line, "S-11 · The $50,000");

    // A scenario minted before codes existed renders its name, not "S-".
    let mut uncoded = item;
    uncoded.code_ordinal = None;
    assert!(
        v.compose(&uncoded).deck_line.starts_with(" · The $50,000"),
        "no code renders as nothing"
    );
}

/// Today shows the clock alone; anything older carries its day.
///
/// "4:18 pm" under the EARLIER heading is a time with no anchor — the reader
/// cannot tell whether it was three days ago or three weeks.
#[test]
fn todays_rows_show_the_clock_and_older_rows_show_the_day() {
    let w = wording();
    let (l, n) = (logins(), names());
    let v = voice(&w, &l, &n, ForYouSide::Witness);
    let today = v.compose(&row("note", Some("cpenzien"), at(2026, 9, 22, 15)));
    assert_eq!(today.when, "11:30 am", "the clock alone");
    let older = v.compose(&row("note", Some("cpenzien"), at(2026, 9, 19, 15)));
    assert_eq!(older.when, "Sat 19 Sep · 11:30 am");
}

/// (M) A REPLY carries what it answers — both halves, in one line (L3).
///
/// "Yes, that's right." on its own is a row that tells the reader nothing and
/// costs them a click to understand. The question page draws the exchange in
/// two lines because it has the room; a list row has one, so it quotes the pair.
#[test]
fn a_reply_row_quotes_the_note_it_answers() {
    let w = ForYouWording::for_test();
    let (l, n) = (logins(), names());
    let v = voice(&w, &l, &n, ForYouSide::Reviewers);

    let mut item = row("note", Some("docmarie"), at(2026, 9, 22, 9));
    item.body = Some("Yes, that is right.".to_string());
    item.reply_to = Some("Whose account was it in?".to_string());

    let composed = v.compose(&item);
    assert_eq!(
        composed.body,
        "Reply to “Whose account was it in?”: “Yes, that is right.”"
    );
}

/// A plain note is NOT wrapped in the reply sentence.
///
/// The discriminator is the presence of a parent, not the kind — every reply is
/// a note, so a `kind` test would wrap every note ever written.
#[test]
fn a_note_that_answers_nothing_reads_as_itself() {
    let w = ForYouWording::for_test();
    let (l, n) = (logins(), names());
    let v = voice(&w, &l, &n, ForYouSide::Reviewers);

    let mut item = row("note", Some("docmarie"), at(2026, 9, 22, 9));
    item.body = Some("Yes, that is right.".to_string());

    assert_eq!(v.compose(&item).body, "Yes, that is right.");
}
