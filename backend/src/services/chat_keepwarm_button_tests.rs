//! The box's arithmetic: loaded-until, read versus wrote, and how times read.

use chrono::TimeZone;

use super::*;

fn at(h: u32, m: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 24, h, m, 0).unwrap()
}

fn usage(read: u64, wrote: u64) -> Usage {
    Usage {
        input_tokens: Some(4),
        output_tokens: Some(0),
        cache_read_input_tokens: Some(read),
        cache_creation_input_tokens: Some(wrote),
    }
}

#[test]
fn loaded_until_counts_from_the_later_of_turn_and_ping() {
    let one_hour = CacheTtl::OneHour;
    assert_eq!(
        loaded_until(Some(at(13, 0)), Some(at(14, 0)), one_hour),
        Some(at(15, 0))
    );
    assert_eq!(
        loaded_until(Some(at(14, 30)), Some(at(14, 0)), one_hour),
        Some(at(15, 30))
    );
    assert_eq!(
        loaded_until(Some(at(13, 0)), None, one_hour),
        Some(at(14, 0))
    );
    assert_eq!(
        loaded_until(None, Some(at(13, 0)), one_hour),
        Some(at(14, 0))
    );
    assert_eq!(loaded_until(None, None, one_hour), None);
}

#[test]
fn loaded_until_follows_the_configured_lifetime() {
    assert_eq!(
        loaded_until(Some(at(13, 0)), None, CacheTtl::FiveMinutes),
        Some(at(13, 5))
    );
}

/// The Stage P receipt reads; a full reload writes; a partial rewrite — system
/// read, documents written — is a reload too.
#[test]
fn a_ping_that_wrote_more_than_it_read_is_a_reload() {
    assert_eq!(outcome(&usage(368_832, 0)), Outcome::Read);
    assert_eq!(outcome(&usage(0, 368_832)), Outcome::Wrote);
    assert_eq!(outcome(&usage(5_000, 363_832)), Outcome::Wrote);
    assert_eq!(outcome(&usage(368_832, 40)), Outcome::Read);
}

/// Today reads as a clock; another day carries its date, so "2:00 pm" is never
/// yesterday's 2:00 pm read as today's. 17:00 UTC is 1:00 pm in Detroit.
#[test]
fn a_time_today_is_a_clock_and_another_day_carries_its_date() {
    let tz = "America/Detroit";
    let now = at(18, 0);
    assert_eq!(when(at(17, 0), now, tz), "1:00 pm");
    let yesterday = Utc.with_ymd_and_hms(2026, 9, 23, 17, 0, 0).unwrap();
    let shown = when(yesterday, now, tz);
    assert!(
        shown.contains("23 Sep") && shown.contains("1:00 pm"),
        "{shown}"
    );
}

#[test]
fn outcomes_serialize_as_the_page_expects() {
    assert_eq!(serde_json::to_value(Outcome::Read).unwrap(), "read");
    assert_eq!(serde_json::to_value(Outcome::Wrote).unwrap(), "wrote");
}

/// The automatic line: the rules decide, the rows speak.
mod automatic_line_tests {
    use super::*;
    use crate::services::chat_keepwarm_rates::PRICED_MODEL;
    use chrono_tz::America::Detroit;
    use colossus_chat::keepwarm::{Ledger, Pause};

    fn settings() -> Settings {
        let mut s = Settings::for_test();
        s.question_chat.model = PRICED_MODEL.to_string();
        s
    }

    /// 10:00 am in Detroit on 2026-09-25, as UTC.
    fn ten_am() -> DateTime<Utc> {
        Detroit
            .with_ymd_and_hms(2026, 9, 25, 10, 0, 0)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn chatted() -> Facts {
        Facts {
            last_turn: Some(ten_am() - Duration::minutes(10)),
            last_ping: None,
        }
    }

    #[test]
    fn on_names_the_window_in_the_clock_format() {
        let line = automatic_line(&settings(), &chatted(), &Ledger::default(), ten_am());
        assert_eq!(line, "Automatic: on, 6:00 am – 11:00 pm");
        // Not armed yet (nobody has chatted) is still "on".
        let idle = automatic_line(&settings(), &Facts::default(), &Ledger::default(), ten_am());
        assert_eq!(idle, line);
    }

    #[test]
    fn off_when_switched_off() {
        let mut s = settings();
        s.question_chat.keepwarm.enabled = false;
        let line = automatic_line(&s, &chatted(), &Ledger::default(), ten_am());
        assert_eq!(line, "Automatic: off");
    }

    #[test]
    fn paused_for_the_cap_an_unpriced_model_and_a_closed_window() {
        let paused = "Automatic: on, paused until tomorrow";
        let capped = Ledger {
            day: Some(ten_am().with_timezone(&Detroit).date_naive()),
            paused: Some(Pause::Cap),
            ..Ledger::default()
        };
        assert_eq!(
            automatic_line(&settings(), &chatted(), &capped, ten_am()),
            paused
        );
        let unpriced = Settings::for_test();
        assert_eq!(
            automatic_line(&unpriced, &chatted(), &Ledger::default(), ten_am()),
            paused
        );
        let late = ten_am() + Duration::hours(13) + Duration::minutes(30);
        assert_eq!(
            automatic_line(&settings(), &chatted(), &Ledger::default(), late),
            paused
        );
    }
}
