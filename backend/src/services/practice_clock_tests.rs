//! One test per surface, plus the two ways a zone can be wrong.
//!
//! The instants below are chosen deliberately: 21:36 UTC is 5:36 pm EDT (summer,
//! UTC-4) and 4:36 pm EST (winter, UTC-5). A formatter that hard-coded an offset
//! would pass one of these tests and fail the other, which is the point of
//! having both — an offset is right for half the year, and nobody notices in
//! November.

use chrono::{TimeZone, Utc};

use super::{local_clock, local_date, local_stamp};

const CASE_TZ: &str = "America/Detroit";

/// 2026-08-19 21:36 UTC — a Wednesday evening in Michigan, on daylight time.
fn summer_evening() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 19, 21, 36, 0)
        .single()
        .expect("a real instant")
}

/// 2026-11-19 21:36 UTC — the same clock time in UTC, on STANDARD time locally.
fn winter_evening() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 19, 21, 36, 0)
        .single()
        .expect("a real instant")
}

/// The clock reads as Marie's watch reads: 12-hour, lower case, no leading zero.
#[test]
fn the_clock_is_local_and_twelve_hour() {
    assert_eq!(local_clock(summer_evening(), CASE_TZ), "5:36 pm");
}

/// Daylight saving is the IANA database's job, not ours.
///
/// The same UTC instant is 5:36 pm in August and 4:36 pm in November. A
/// formatter carrying a fixed `-4` would print 5:36 pm all year and be an hour
/// wrong every winter — visible to nobody until a sitting after the clocks go
/// back.
#[test]
fn the_same_utc_instant_reads_an_hour_earlier_in_winter() {
    assert_eq!(local_clock(summer_evening(), CASE_TZ), "5:36 pm");
    assert_eq!(local_clock(winter_evening(), CASE_TZ), "4:36 pm");
}

/// Morning has no leading zero, and says `am`.
#[test]
fn a_morning_time_has_no_leading_zero() {
    let at = Utc
        .with_ymd_and_hms(2026, 8, 19, 13, 5, 0)
        .single()
        .expect("a real instant");
    assert_eq!(local_clock(at, CASE_TZ), "9:05 am");
}

/// The date is the day IN THE CASE'S ZONE, which can differ from the UTC day.
///
/// 01:30 UTC on the 20th is 9:30 pm on the 19th in Michigan. A date formatted
/// off the UTC value would put that evening's sitting on the wrong DAY — the
/// same class of defect as the clock, and harder to spot because a date looks
/// plausible either way.
#[test]
fn a_late_evening_sitting_keeps_its_own_day() {
    let at = Utc
        .with_ymd_and_hms(2026, 8, 20, 1, 30, 0)
        .single()
        .expect("a real instant");
    assert_eq!(local_date(at, CASE_TZ), "Wed 19 Aug");
    assert_eq!(local_clock(at, CASE_TZ), "9:30 pm");
}

/// The full stamp, as the unfinished-session line and the review page use it.
#[test]
fn the_stamp_is_the_day_and_the_clock() {
    assert_eq!(
        local_stamp(summer_evening(), CASE_TZ),
        "Wed 19 Aug · 5:36 pm"
    );
}

/// An unrecognised zone renders in UTC rather than refusing.
///
/// A misspelt settings row is a cosmetic problem; a practice page that would not
/// load because of one is not. It warns (see `zone`) and keeps going.
#[test]
fn an_unknown_zone_degrades_to_utc_rather_than_failing() {
    assert_eq!(
        local_clock(summer_evening(), "Mars/Olympus_Mons"),
        "9:36 pm"
    );
    assert_eq!(local_clock(summer_evening(), ""), "9:36 pm");
}

/// A DIFFERENT real zone is honoured — the zone is read, not assumed.
///
/// Without this, a formatter that ignored its argument entirely and always used
/// Detroit would pass every test above.
#[test]
fn the_zone_argument_is_actually_read() {
    assert_eq!(
        local_clock(summer_evening(), "America/Los_Angeles"),
        "2:36 pm"
    );
    assert_eq!(local_clock(summer_evening(), "UTC"), "9:36 pm");
}
