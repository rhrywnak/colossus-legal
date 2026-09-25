//! The keep-warm rules against a fake clock: every instant is a literal, so a
//! test says exactly which moment it is about — the two daylight-saving days in
//! Detroit included.

use super::*;
use chrono::TimeZone;

/// The zone every test reads its window in.
const TZ: Tz = chrono_tz::America::Detroit;

fn hm(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).expect("a valid literal time")
}

/// A LOCAL Detroit wall-clock instant, as UTC.
fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
    TZ.with_ymd_and_hms(y, mo, d, h, mi, 0)
        .single()
        .expect("an unambiguous literal local time")
        .with_timezone(&Utc)
}

/// The seeded settings: 6:00 am – 11:00 pm, every 48 minutes, $2.00, 1h lifetime.
fn rules() -> Rules {
    Rules {
        enabled: true,
        window_start: hm(6, 0),
        window_end: hm(23, 0),
        interval: Duration::minutes(48),
        lifetime: Duration::minutes(60),
        daily_cap_dollars: 2.0,
        timezone: TZ,
        priced: true,
    }
}

fn turn(t: DateTime<Utc>) -> Facts {
    Facts {
        last_turn: Some(t),
        last_ping: None,
    }
}

fn decide_at(facts: Facts, now: DateTime<Utc>) -> Decision {
    decide(&rules(), &facts, &Ledger::default(), now)
}

#[test]
fn no_turn_all_day_never_pings() {
    for hour in [6, 9, 12, 15, 18, 22] {
        let d = decide_at(Facts::default(), at(2026, 9, 25, hour, 30));
        assert_eq!(d, Decision::Idle(Idle::NotArmed), "at {hour}:30");
    }
}

#[test]
fn a_turn_before_or_after_the_window_does_not_arm() {
    let early = turn(at(2026, 9, 25, 5, 30));
    assert_eq!(
        decide_at(early, at(2026, 9, 25, 6, 20)),
        Decision::Idle(Idle::NotArmed)
    );
    let late = turn(at(2026, 9, 25, 23, 10));
    assert_eq!(
        decide_at(late, at(2026, 9, 25, 23, 20)),
        Decision::Paused(Pause::WindowClosed)
    );
}

#[test]
fn yesterdays_turn_does_not_arm_today() {
    let t = turn(at(2026, 9, 24, 22, 50));
    assert_eq!(
        decide_at(t, at(2026, 9, 25, 6, 5)),
        Decision::Idle(Idle::NotArmed)
    );
}

#[test]
fn a_turn_inside_the_window_arms_and_is_due_at_touch_plus_interval() {
    let t0 = at(2026, 9, 25, 10, 0);
    assert_eq!(
        decide_at(turn(t0), at(2026, 9, 25, 10, 47)),
        Decision::SleepUntil(at(2026, 9, 25, 10, 48))
    );
    assert_eq!(decide_at(turn(t0), at(2026, 9, 25, 10, 48)), Decision::Due);
}

#[test]
fn a_real_turn_moves_the_deadline() {
    let later = turn(at(2026, 9, 25, 10, 30));
    assert_eq!(
        decide_at(later, at(2026, 9, 25, 10, 48)),
        Decision::SleepUntil(at(2026, 9, 25, 11, 18))
    );
}

#[test]
fn a_ping_by_hand_extends_an_armed_day_but_never_arms_one() {
    let armed = Facts {
        last_turn: Some(at(2026, 9, 25, 10, 0)),
        last_ping: Some(at(2026, 9, 25, 10, 40)),
    };
    assert_eq!(
        decide_at(armed, at(2026, 9, 25, 10, 48)),
        Decision::SleepUntil(at(2026, 9, 25, 11, 28))
    );
    let never_chatted = Facts {
        last_turn: None,
        last_ping: Some(at(2026, 9, 25, 10, 40)),
    };
    assert_eq!(
        decide_at(never_chatted, at(2026, 9, 25, 11, 28)),
        Decision::Idle(Idle::NotArmed)
    );
}

#[test]
fn a_window_end_edited_to_just_ahead_is_honoured_at_the_next_wake() {
    let mut r = rules();
    let t = turn(at(2026, 9, 25, 14, 0));
    r.window_end = hm(14, 45);
    assert_eq!(
        decide(&r, &t, &Ledger::default(), at(2026, 9, 25, 14, 48)),
        Decision::Paused(Pause::WindowClosed)
    );
}

#[test]
fn switched_off_is_idle() {
    let mut r = rules();
    r.enabled = false;
    let t = turn(at(2026, 9, 25, 10, 0));
    assert_eq!(
        decide(&r, &t, &Ledger::default(), at(2026, 9, 25, 10, 48)),
        Decision::Idle(Idle::Disabled)
    );
}

#[test]
fn the_cap_is_checked_before_spending() {
    assert!(!within_cap(1.95, 0.074, 2.0));
    assert!(within_cap(1.90, 0.074, 2.0));
    assert!(within_cap(0.0, 2.0, 2.0), "exactly at the cap is allowed");
    assert!(!within_cap(0.0, 0.074, 0.0), "a zero cap stops everything");
}

#[test]
fn an_automatic_write_pauses_the_day_and_the_next_day_starts_fresh() {
    let today = local_day(at(2026, 9, 25, 10, 48), TZ);
    let mut ledger = Ledger::default();
    ledger.record_ping(today, Some(2.95), true, true);
    let t = turn(at(2026, 9, 25, 11, 0));
    assert_eq!(
        decide(&rules(), &t, &ledger, at(2026, 9, 25, 11, 48)),
        Decision::Paused(Pause::Wrote)
    );
    // The next local day: the pause no longer applies, and the spend is zero.
    let tomorrow = at(2026, 9, 26, 10, 48);
    let next = turn(at(2026, 9, 26, 10, 0));
    assert_eq!(decide(&rules(), &next, &ledger, tomorrow), Decision::Due);
    assert_eq!(ledger.spend_on(local_day(tomorrow, TZ)), 0.0);
    ledger.roll(local_day(tomorrow, TZ));
    assert_eq!((ledger.paused, ledger.spend_dollars), (None, 0.0));
}

#[test]
fn a_ping_by_hand_that_wrote_does_not_pause_but_does_count() {
    let today = local_day(at(2026, 9, 25, 10, 48), TZ);
    let mut ledger = Ledger::default();
    ledger.record_ping(today, Some(2.95), true, false);
    ledger.record_ping(today, Some(0.07), false, true);
    assert_eq!(ledger.paused, None);
    assert!((ledger.spend_on(today) - 3.02).abs() < 1e-9);
    assert_eq!(ledger.last_cost_dollars, Some(0.07));
}

#[test]
fn a_reload_is_not_the_next_pings_projection() {
    let today = local_day(at(2026, 9, 25, 10, 48), TZ);
    let mut ledger = Ledger::default();
    ledger.record_ping(today, Some(2.95), true, false);
    assert_eq!(ledger.last_cost_dollars, None);
}

#[test]
fn a_cap_pause_holds_for_the_day() {
    let now = at(2026, 9, 25, 15, 0);
    let ledger = Ledger {
        day: Some(local_day(now, TZ)),
        paused: Some(Pause::Cap),
        ..Ledger::default()
    };
    let t = turn(at(2026, 9, 25, 14, 30));
    assert_eq!(
        decide(&rules(), &t, &ledger, now),
        Decision::Paused(Pause::Cap)
    );
}

#[test]
fn after_a_restart_a_turn_inside_the_lifetime_arms_and_an_older_one_has_expired() {
    // A restart holds nothing in memory: an empty ledger and no ping.
    let now = at(2026, 9, 25, 12, 0);
    assert_eq!(
        decide_at(turn(at(2026, 9, 25, 11, 30)), now),
        Decision::SleepUntil(at(2026, 9, 25, 12, 18))
    );
    assert_eq!(
        decide_at(turn(at(2026, 9, 25, 10, 50)), now),
        Decision::Idle(Idle::Expired)
    );
}

#[test]
fn a_hold_lasts_until_something_touches_again() {
    let t0 = at(2026, 9, 25, 10, 0);
    let ledger = Ledger {
        held: Some((t0, Idle::PrefixChanged)),
        ..Ledger::default()
    };
    assert_eq!(
        decide(&rules(), &turn(t0), &ledger, at(2026, 9, 25, 10, 48)),
        Decision::Idle(Idle::PrefixChanged)
    );
    let newer = turn(at(2026, 9, 25, 10, 20));
    assert_eq!(
        decide(&rules(), &newer, &ledger, at(2026, 9, 25, 11, 8)),
        Decision::Due
    );
}

#[test]
fn the_prefix_verdicts() {
    assert_eq!(check_prefix(None, "abc"), PrefixVerdict::Unknown);
    assert_eq!(check_prefix(Some("abc"), "abc"), PrefixVerdict::Same);
    assert_eq!(check_prefix(Some("abc"), "abd"), PrefixVerdict::Changed);
}

/// 2026-03-08: Detroit springs forward at 2:00 am (EST → EDT). The window is
/// read on the local clock, so 5:30 am EDT is still before it.
#[test]
fn spring_forward_day_reads_the_window_on_the_local_clock() {
    let early = turn(at(2026, 3, 8, 5, 30));
    assert_eq!(
        decide_at(early, at(2026, 3, 8, 6, 10)),
        Decision::Idle(Idle::NotArmed)
    );
    let t = turn(at(2026, 3, 8, 6, 5));
    assert_eq!(
        decide_at(t, at(2026, 3, 8, 6, 53)),
        Decision::Due,
        "48 real minutes"
    );
    assert_eq!(
        at(2026, 3, 8, 6, 0).to_rfc3339(),
        "2026-03-08T10:00:00+00:00",
        "6:00 am EDT"
    );
}

/// 2026-11-01: Detroit falls back at 2:00 am (EDT → EST). The window closes at
/// 11:00 pm EST, which is 04:00 UTC the next calendar day.
#[test]
fn fall_back_day_closes_the_window_at_local_eleven() {
    let t = turn(at(2026, 11, 1, 22, 30));
    assert_eq!(
        decide_at(t, at(2026, 11, 1, 22, 59)),
        Decision::SleepUntil(at(2026, 11, 1, 23, 18))
    );
    assert_eq!(
        decide_at(t, at(2026, 11, 1, 23, 18)),
        Decision::Paused(Pause::WindowClosed)
    );
    assert_eq!(
        at(2026, 11, 1, 23, 0).to_rfc3339(),
        "2026-11-02T04:00:00+00:00"
    );
    // The day rolls over at LOCAL midnight, not UTC midnight.
    assert_eq!(
        local_day(at(2026, 11, 1, 23, 30), TZ),
        NaiveDate::from_ymd_opt(2026, 11, 1).expect("valid")
    );
}

#[test]
fn an_interval_not_below_the_lifetime_refuses() {
    let mut r = rules();
    r.interval = Duration::minutes(60);
    let t = turn(at(2026, 9, 25, 10, 0));
    assert_eq!(
        decide(&r, &t, &Ledger::default(), at(2026, 9, 25, 10, 30)),
        Decision::Paused(Pause::IntervalNotBelowLifetime)
    );
    r.interval = Duration::minutes(5);
    r.lifetime = Duration::minutes(5);
    assert_eq!(
        decide(&r, &t, &Ledger::default(), at(2026, 9, 25, 10, 30)),
        Decision::Paused(Pause::IntervalNotBelowLifetime)
    );
}

#[test]
fn an_unpriced_model_refuses() {
    let mut r = rules();
    r.priced = false;
    let t = turn(at(2026, 9, 25, 10, 0));
    assert_eq!(
        decide(&r, &t, &Ledger::default(), at(2026, 9, 25, 10, 48)),
        Decision::Paused(Pause::Unpriced)
    );
}

#[test]
fn every_reason_has_its_own_log_name() {
    let idle = [
        Idle::Disabled,
        Idle::NotArmed,
        Idle::Expired,
        Idle::PrefixChanged,
        Idle::PingFailed,
    ];
    let pause = [
        Pause::Unpriced,
        Pause::IntervalNotBelowLifetime,
        Pause::WindowClosed,
        Pause::Cap,
        Pause::Wrote,
    ];
    let mut names: Vec<&str> = idle.iter().map(|i| i.reason()).collect();
    names.extend(pause.iter().map(|p| p.reason()));
    let before = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), before, "two reasons share a log name");
}
