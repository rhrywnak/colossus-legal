// Tests for `services::for_you` — whose list, and what the page says.
//
// The two decisions this module owns, held down without a database. The side
// rule is the one CC_TASK_REVIEW_PERMISSION_v1 exists to protect: permission is
// "listed OR administrator", and a display list must never be able to take the
// reviewers' page away from somebody who may review.

use super::*;
use crate::domain::wording_for_you::ForYouWording;
use crate::dto::for_you::ForYouDay;
use crate::services::for_you_rows::RowVoice;
use chrono::{NaiveDate, TimeZone, Utc};
use uuid::Uuid;

const TZ: &str = "America/Detroit";

fn bench() -> Vec<String> {
    vec!["cpenzien".to_string()]
}

/// (M) An UNLISTED administrator is served the REVIEWERS' list.
///
/// Journey (c) of the plan, and the whole point of the permission task: Roman
/// took himself off the war room's display list so it would stop naming him. If
/// this page read the list instead of asking `may_review`, that edit would have
/// silently taken his inbox away too.
#[test]
fn an_unlisted_admin_is_served_the_reviewers_list() {
    assert_eq!(
        side_for("roman", true, &bench(), "docmarie"),
        ForYouSide::Reviewers
    );
    // And a listed reviewer who is NOT an administrator still is.
    assert_eq!(
        side_for("cpenzien", false, &bench(), "docmarie"),
        ForYouSide::Reviewers
    );
}

/// The witness is served her own list.
#[test]
fn the_witness_is_served_her_own_list() {
    assert_eq!(
        side_for("docmarie", false, &bench(), "docmarie"),
        ForYouSide::Witness
    );
}

/// (M) A person who is NEITHER gets an empty list, not an error.
///
/// Standing Rule 1 read the other way round: nothing failed, so nothing may be
/// reported as a failure. A 403 would say something was refused; an empty list
/// with no sentence would say "you are up to date", which is a different fact.
#[test]
fn a_person_who_is_neither_gets_an_empty_list_not_an_error() {
    assert_eq!(
        side_for("astranger", false, &bench(), "docmarie"),
        ForYouSide::None
    );
    assert_eq!(query_side(ForYouSide::None), None, "and no query is run");
}

/// A BLANK witness row matches nobody — including a caller whose login the auth
/// layer left empty.
///
/// The boot check refuses a blank row, so this cannot reach a running server.
/// The guard is here anyway for the reason `can_mark_reviewed`'s is: a blank
/// that matched would hand the witness's inbox to an unauthenticated caller,
/// which is the worst possible reading of a typo.
#[test]
fn a_blank_witness_row_matches_nobody() {
    assert_eq!(side_for("", false, &[], ""), ForYouSide::None);
    assert_eq!(side_for("   ", false, &[], "   "), ForYouSide::None);
}

/// The two page sides map to the two query sides, and nothing else does.
#[test]
fn each_page_side_asks_the_query_for_its_own_half() {
    assert_eq!(
        query_side(ForYouSide::Reviewers),
        Some(WaitingSide::Reviewers)
    );
    assert_eq!(query_side(ForYouSide::Witness), Some(WaitingSide::Witness));
}

fn item(when_day: u32) -> WaitingItemRow {
    WaitingItemRow {
        kind: "note".to_string(),
        item_id: Uuid::from_u128(1),
        scenario_id: Uuid::from_u128(2),
        code_ordinal: Some(11),
        scenario_name: "The $50,000".to_string(),
        question_id: Some(Uuid::from_u128(3)),
        question_text: Some("Whose money?".to_string()),
        at: Utc
            .with_ymd_and_hms(2026, 9, when_day, 15, 30, 0)
            .single()
            .expect("a real moment"),
        author: Some("cpenzien".to_string()),
        seen_at: None,
        body: Some("look again".to_string()),
        subject_at: None,
    }
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
        today: NaiveDate::from_ymd_opt(2026, 9, 22).expect("a real day"),
    }
}

/// The page names the OTHER side, in both directions.
///
/// The witness is told whose notes she is waiting on; a reviewer is told whose
/// answers he is. One template each, filled from the settings rows — so a case
/// with different people needs no new binary.
#[test]
fn the_subtitle_names_whoever_the_reader_is_waiting_on() {
    let w = ForYouWording::for_test();
    let (l, n) = (bench(), vec!["Chuck".to_string()]);
    let rows = [item(22)];

    let hers = assemble_page(
        &voice(&w, &l, &n, ForYouSide::Witness),
        "Chuck",
        &rows,
        &rows,
    );
    assert_eq!(
        hers.subtitle,
        "Chuck's notes on your answers. Newest first. A row clears when you open it."
    );
    assert_eq!(
        hers.empty_hint,
        "Chuck's notes and answers appear here as they arrive."
    );

    let his = assemble_page(
        &voice(&w, &l, &n, ForYouSide::Reviewers),
        "Marie",
        &rows,
        &rows,
    );
    assert_eq!(
        his.subtitle,
        "Marie's new answers and her notes. Newest first. A row clears when you open it."
    );
}

/// Nobody's list says so in the subtitle rather than leaving it blank.
#[test]
fn a_page_for_neither_side_carries_the_sentence_that_explains_it() {
    let w = ForYouWording::for_test();
    let (l, n) = (bench(), vec!["Chuck".to_string()]);
    let page = assemble_page(&voice(&w, &l, &n, ForYouSide::None), "Marie", &[], &[]);
    assert_eq!(page.subtitle, w.not_your_list);
    assert_eq!(page.unread_count, 0);
    assert!(page.unread.is_empty() && page.everything.is_empty());
}

/// (M) The tab labels carry the counts, composed here so the browser fills no
/// template — and the two tabs count two different things.
#[test]
fn the_tab_labels_carry_their_own_counts() {
    let w = ForYouWording::for_test();
    let (l, n) = (bench(), vec!["Chuck".to_string()]);
    let unread = [item(22)];
    let everything = [item(22), item(21), item(20)];
    let page = assemble_page(
        &voice(&w, &l, &n, ForYouSide::Witness),
        "Chuck",
        &unread,
        &everything,
    );
    assert_eq!(page.tab_unread_label, "Unread · 1");
    assert_eq!(page.tab_everything_label, "Everything · 3");
    assert_eq!(page.unread_count, 1, "the badge's number is the unread one");
    assert_eq!(page.everything.len(), 3);
    assert_eq!(page.unread[0].day, ForYouDay::Today);
    assert_eq!(page.everything[2].day, ForYouDay::Earlier);
}

/// The empty state names the day of the last item — and says nothing when there
/// has never been one.
///
/// A line reading "Last one: ." is worse than no line: it tells the reader the
/// page half-failed. So it is `None`, and the browser renders nothing.
#[test]
fn the_empty_state_names_the_last_item_only_when_there_is_one() {
    let w = ForYouWording::for_test();
    let (l, n) = (bench(), vec!["Chuck".to_string()]);
    let v = voice(&w, &l, &n, ForYouSide::Witness);

    let never = assemble_page(&v, "Chuck", &[], &[]);
    assert_eq!(never.empty_last, None);

    let had = assemble_page(&v, "Chuck", &[], &[item(20)]);
    assert_eq!(
        had.empty_last.as_deref(),
        Some("Last one: Sun 20 Sep · 11:30 am.")
    );
}
