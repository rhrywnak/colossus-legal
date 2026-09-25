//! The pinger's two pure helpers: the rules the settings state, and the cap's
//! projection before and after a ping has been measured.

use chrono::NaiveTime;

use super::*;
use crate::services::chat_keepwarm_rates::PRICED_MODEL;

/// The seeded settings with the one priced model (the fixture's model is not).
fn priced_settings() -> Settings {
    let mut s = Settings::for_test();
    s.question_chat.model = PRICED_MODEL.to_string();
    s
}

#[test]
fn the_rules_are_the_settings_read_now() {
    let rules = rules_of(&priced_settings());
    assert!(rules.enabled && rules.priced);
    assert_eq!(
        rules.window_start,
        NaiveTime::from_hms_opt(6, 0, 0).unwrap()
    );
    assert_eq!(rules.window_end, NaiveTime::from_hms_opt(23, 0, 0).unwrap());
    assert_eq!(rules.interval, Duration::minutes(48));
    assert_eq!(rules.lifetime, Duration::minutes(60));
    assert_eq!(rules.daily_cap_dollars, 2.0);
    assert_eq!(rules.timezone, chrono_tz::America::Detroit);
}

#[test]
fn an_edited_setting_is_in_the_next_rules() {
    let mut s = priced_settings();
    s.question_chat.keepwarm.enabled = false;
    s.question_chat.keepwarm.interval_minutes = 5;
    let rules = rules_of(&s);
    assert!(!rules.enabled);
    assert_eq!(rules.interval, Duration::minutes(5));
}

#[test]
fn a_model_without_known_prices_is_not_priced() {
    let mut s = priced_settings();
    s.question_chat.model = "claude-some-other-model".to_string();
    assert!(!rules_of(&s).priced);
}

#[test]
fn the_projection_is_the_last_measured_read_when_there_is_one() {
    let ledger = Ledger {
        last_cost_dollars: Some(0.0738),
        ..Ledger::default()
    };
    assert_eq!(projected_cost(&priced_settings(), &ledger, 1), 0.0738);
}

#[test]
fn before_any_ping_the_projection_is_the_body_at_the_read_price() {
    // 1,106,496 characters at the fixture's 3 per token = 368,832 tokens:
    // Stage P's measured read, $0.0737664 at $0.20 / MTok.
    let p = projected_cost(&priced_settings(), &Ledger::default(), 1_106_496);
    assert!((p - 0.073_766_4).abs() < 1e-9, "{p}");
}

/// The checks before spending, at a measured read of $0.0738.
mod before_spending {
    use super::*;

    fn today() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 9, 25).unwrap()
    }

    fn ledger(spent: f64, reference: Option<&str>) -> Ledger {
        Ledger {
            day: Some(today()),
            spend_dollars: spent,
            prefix_reference: reference.map(str::to_string),
            last_cost_dollars: Some(0.0738),
            ..Ledger::default()
        }
    }

    fn check(l: &Ledger, hash: &str) -> Result<PrefixVerdict, Blocked> {
        let s = priced_settings();
        check_before_spending(&s, &rules_of(&s), l, &Prepared::for_test(hash, 1), today())
    }

    #[test]
    fn the_cap_pauses_before_the_ping_that_would_pass_it() {
        assert_eq!(
            check(&ledger(1.95, Some("h")), "h"),
            Err(Blocked::Pause(Pause::Cap))
        );
        assert_eq!(
            check(&ledger(1.90, Some("h")), "h"),
            Ok(PrefixVerdict::Same)
        );
    }

    #[test]
    fn a_changed_case_file_holds_until_the_next_question() {
        assert_eq!(
            check(&ledger(0.0, Some("old")), "new"),
            Err(Blocked::Hold(Idle::PrefixChanged))
        );
    }

    #[test]
    fn after_a_restart_the_ping_goes_unguarded() {
        assert_eq!(check(&ledger(0.0, None), "h"), Ok(PrefixVerdict::Unknown));
    }

    #[test]
    fn yesterdays_spend_does_not_count_today() {
        let mut l = ledger(1.99, Some("h"));
        l.day = today().pred_opt();
        assert_eq!(check(&l, "h"), Ok(PrefixVerdict::Same));
    }
}
