//! Keep-warm: WHEN to re-send a cached prefix so the provider keeps holding it.
//!
//! ## Domain note: what keep-warm buys
//!
//! A long prefix (a whole document package) costs about ten times more to WRITE
//! into the provider's cache than to READ from it, and the cache forgets an entry
//! a fixed lifetime after it was last used. A cheap pre-warm request inside that
//! lifetime counts as a use, so the entry lives on and the next real turn reads
//! instead of writes. These rules decide when such a ping is worth sending.
//!
//! ## Why a pure function and not a task
//!
//! Every rule here is about TIME — a window, a local day, an interval, daylight
//! saving — and time is what makes background code hard to test. So nothing in
//! this module reads a clock, a database or a network: [`decide`] takes `now`
//! as an argument, and the caller (a task in the consuming application) does
//! the waiting, the reading and the sending. A test is then a list of instants
//! and the decisions they must produce, the two daylight-saving days included.
//!
//! ## Reusability (Standing Rule 11)
//!
//! Nothing here names a case, a person or a model. Every value — the window,
//! the interval, the cap, the zone, whether the model has known prices —
//! arrives in [`Rules`] from the caller's configuration.

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use chrono_tz::Tz;

/// The configuration the rules read, re-read by the caller on every wake.
#[derive(Debug, Clone, PartialEq)]
pub struct Rules {
    /// Master switch.
    pub enabled: bool,
    /// Local time the window opens (inclusive).
    pub window_start: NaiveTime,
    /// Local time the window closes (exclusive).
    pub window_end: NaiveTime,
    /// How long after the last use a ping is sent.
    pub interval: Duration,
    /// How long the provider keeps an entry after its last use.
    pub lifetime: Duration,
    /// The most automatic pings may cost in one local day, in dollars.
    pub daily_cap_dollars: f64,
    /// The zone the window and the day are read in.
    pub timezone: Tz,
    /// Whether the model's prices are known. An unpriced model cannot be held
    /// to a dollar cap, so it never arms.
    pub priced: bool,
}

/// What has happened, as the caller read it this wake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Facts {
    /// The last successful real turn, from the caller's store.
    pub last_turn: Option<DateTime<Utc>>,
    /// The last successful ping of any kind (automatic or by hand).
    pub last_ping: Option<DateTime<Utc>>,
}

/// Why keep-warm is waiting for a real turn (it is not armed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Idle {
    /// Switched off.
    Disabled,
    /// No real turn today inside the window yet.
    NotArmed,
    /// The entry already expired: a ping now would pay a full write.
    Expired,
    /// The prefix no longer matches what the last real turn cached.
    PrefixChanged,
    /// The last automatic ping failed; no retry until something touches again.
    PingFailed,
}

/// Why keep-warm has stopped for the rest of the local day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pause {
    /// The model has no known prices.
    Unpriced,
    /// The interval is not shorter than the cache lifetime, so a ping would
    /// always arrive after the entry has gone.
    IntervalNotBelowLifetime,
    /// Past the window's end.
    WindowClosed,
    /// The next ping would take the day past its cap.
    Cap,
    /// An automatic ping wrote the prefix instead of reading it.
    Wrote,
}

impl Idle {
    /// The name the logs carry.
    pub fn reason(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::NotArmed => "not_armed",
            Self::Expired => "expired",
            Self::PrefixChanged => "prefix_changed",
            Self::PingFailed => "ping_failed",
        }
    }
}

impl Pause {
    /// The name the logs carry.
    pub fn reason(self) -> &'static str {
        match self {
            Self::Unpriced => "unpriced_model",
            Self::IntervalNotBelowLifetime => "interval_not_below_lifetime",
            Self::WindowClosed => "window_closed",
            Self::Cap => "daily_cap",
            Self::Wrote => "ping_wrote",
        }
    }
}

/// What to do at this wake.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Decision {
    /// A ping is due now (subject to the caller's cap and prefix checks).
    Due,
    /// Nothing to do before this instant.
    SleepUntil(DateTime<Utc>),
    /// Not armed; wait for a real turn.
    Idle(Idle),
    /// Stopped for the rest of the day.
    Paused(Pause),
}

/// The day's running state, kept by the caller between wakes.
///
/// ## Rust Learning: `Option<(A, B)>` as "a fact with its reason"
///
/// `held` is either nothing, or a PAIR: the touch it was set against and why.
/// One `Option` of a tuple rather than two `Option`s means the two can never
/// disagree — there is no state with a reason but no time.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ledger {
    /// The local day the spend and the pause belong to.
    pub day: Option<NaiveDate>,
    /// Dollars spent on pings today, both kinds.
    pub spend_dollars: f64,
    /// A stop that holds for the rest of `day` (a cap or a write).
    pub paused: Option<Pause>,
    /// Idle until something touches after this instant.
    pub held: Option<(DateTime<Utc>, Idle)>,
    /// The prefix fingerprint the last real turn cached. `None` after a restart.
    /// Deliberately NOT reset at midnight: a prefix does not change with the date.
    pub prefix_reference: Option<String>,
    /// What the last ping that READ measurably cost — the cap's projection. A
    /// ping that wrote is left out: a deliberate reload by hand must not make
    /// every later read look like it will cost a reload.
    pub last_cost_dollars: Option<f64>,
}

impl Ledger {
    /// Start a new day's spend and pause when the local date has moved on.
    pub fn roll(&mut self, today: NaiveDate) {
        if self.day != Some(today) {
            self.day = Some(today);
            self.spend_dollars = 0.0;
            self.paused = None;
            self.held = None;
        }
    }

    /// Today's spend; zero when the ledger still holds another day.
    pub fn spend_on(&self, today: NaiveDate) -> f64 {
        if self.day == Some(today) {
            self.spend_dollars
        } else {
            0.0
        }
    }

    /// Record one ping: add its cost, and pause for the day when an AUTOMATIC
    /// one wrote. A ping by hand that wrote is a person reloading on purpose.
    pub fn record_ping(
        &mut self,
        today: NaiveDate,
        cost: Option<f64>,
        wrote: bool,
        automatic: bool,
    ) {
        self.roll(today);
        if let Some(dollars) = cost {
            self.spend_dollars += dollars;
            if !wrote {
                self.last_cost_dollars = Some(dollars);
            }
        }
        if wrote && automatic {
            self.paused = Some(Pause::Wrote);
        }
    }
}

/// The local calendar day `at` falls on in `tz`.
pub fn local_day(at: DateTime<Utc>, tz: Tz) -> NaiveDate {
    at.with_timezone(&tz).date_naive()
}

/// Is `at` today (in `tz`) and inside the window?
fn inside_window_today(rules: &Rules, at: DateTime<Utc>, today: NaiveDate) -> bool {
    let local = at.with_timezone(&rules.timezone);
    let time = local.time();
    local.date_naive() == today && time >= rules.window_start && time < rules.window_end
}

/// Decide what this wake should do. Pure: every input is an argument.
///
/// The checks run in the order a person would ask them: is it on, can it be
/// priced, can it ever work, has today already stopped, is the window open,
/// has anyone chatted, is the entry still alive, is a ping due yet.
pub fn decide(rules: &Rules, facts: &Facts, ledger: &Ledger, now: DateTime<Utc>) -> Decision {
    if !rules.enabled {
        return Decision::Idle(Idle::Disabled);
    }
    if !rules.priced {
        return Decision::Paused(Pause::Unpriced);
    }
    if rules.interval >= rules.lifetime {
        return Decision::Paused(Pause::IntervalNotBelowLifetime);
    }
    let today = local_day(now, rules.timezone);
    if let Some(pause) = ledger.paused.filter(|_| ledger.day == Some(today)) {
        return Decision::Paused(pause);
    }
    let local_now = now.with_timezone(&rules.timezone).time();
    if local_now >= rules.window_end {
        return Decision::Paused(Pause::WindowClosed);
    }
    // Arming needs a REAL turn today inside the window: a ping by hand extends
    // an armed day (it is part of the touch below) but never arms one.
    let armed = facts
        .last_turn
        .is_some_and(|t| inside_window_today(rules, t, today));
    if local_now < rules.window_start || !armed {
        return Decision::Idle(Idle::NotArmed);
    }
    // `max` on two Options: `None` sorts below any `Some`, so this is the later
    // of the two that exist. `armed` guarantees `last_turn` is `Some`.
    let Some(touch) = facts.last_turn.max(facts.last_ping) else {
        return Decision::Idle(Idle::NotArmed);
    };
    if let Some((at, why)) = ledger.held {
        if touch <= at {
            return Decision::Idle(why);
        }
    }
    if now >= touch + rules.lifetime {
        return Decision::Idle(Idle::Expired);
    }
    let due = touch + rules.interval;
    if now < due {
        return Decision::SleepUntil(due);
    }
    Decision::Due
}

/// Would a ping costing `projected` keep today's spend within the cap?
/// Checked BEFORE spending, so the cap is never overshot by a ping.
pub fn within_cap(spent: f64, projected: f64, cap: f64) -> bool {
    spent + projected <= cap
}

/// How the prefix about to be sent compares with the one the last turn cached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixVerdict {
    /// Byte-identical: the ping will read.
    Same,
    /// Different: the ping would write a prefix no turn has used.
    Changed,
    /// No reference (a restart): the ping runs unguarded.
    Unknown,
}

/// Compare two prefix fingerprints.
pub fn check_prefix(reference: Option<&str>, current: &str) -> PrefixVerdict {
    match reference {
        None => PrefixVerdict::Unknown,
        Some(r) if r == current => PrefixVerdict::Same,
        Some(_) => PrefixVerdict::Changed,
    }
}

#[cfg(test)]
#[path = "keepwarm_tests.rs"]
mod tests;
