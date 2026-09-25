//! The automatic keep-loaded pinger: a background task that re-sends the chat's
//! case file shortly before the provider would forget it, while people are
//! chatting (CC_TASK_CACHE_KEEPWARM_v1 R3).
//!
//! ## One wake, start to finish
//!
//! 1. Read the settings NOW (`SettingsHandle::current()`), so an edit on the
//!    Settings page takes effect at this wake with no restart.
//! 2. Read the last successful chat reply from the store and the last ping from
//!    the shared `LastPing`.
//! 3. Ask the pure rules (`colossus_chat::keepwarm::decide`) what to do.
//! 4. Ping, or wait: until the next ping is due, or — when idle or paused — for
//!    one interval, so a switch turned back on is seen without waiting for a
//!    question. A finished real turn wakes the wait early.
//!
//! ## Why the last turn comes from the store
//!
//! Because a restart forgets everything in memory. Reading the store on every
//! wake means a restart mid-conversation re-arms from Marie's last reply, if it
//! is still inside the window and the hour (R3 §3) — the same code path, no
//! special case.
//!
//! ## No silent retries
//!
//! A ping that fails is logged at WARN and not retried: keep-warm holds until
//! something touches the case file again (a real turn, or the button). With a
//! 48-minute interval and a one-hour lifetime there is no room for a second try
//! that would still land in time, and a quiet retry loop is what Rule 14 forbids.

use chrono::{DateTime, Duration, Utc};
use colossus_chat::keepwarm::{
    check_prefix, decide, local_day, within_cap, Decision, Facts, Idle, Ledger, Pause,
    PrefixVerdict, Rules,
};

use crate::domain::settings::Settings;
use crate::repositories::pipeline_repository::chat_last_turn::last_chat_turn;
use crate::services::chat_keepwarm_button::ttl_duration;
use crate::services::chat_keepwarm_ping::{prepare, send, Prepared, Trigger};
use crate::services::chat_keepwarm_rates::{is_priced, read_cost};
use crate::services::practice_clock::zone;
use crate::state::AppState;

/// The rules as the current settings state them.
pub fn rules_of(settings: &Settings) -> Rules {
    let chat = &settings.question_chat;
    let k = &chat.keepwarm;
    Rules {
        enabled: k.enabled,
        window_start: k.window_start,
        window_end: k.window_end,
        interval: Duration::minutes(i64::from(k.interval_minutes)),
        lifetime: ttl_duration(chat.cache_ttl),
        daily_cap_dollars: k.daily_cap_dollars,
        timezone: zone(&settings.practice_read.case_timezone),
        priced: is_priced(&chat.model),
    }
}

/// Start the pinger, unless this server has no chat engine to ping with.
///
/// ## Rust Learning: `tokio::spawn` and a task that never ends
///
/// `spawn` runs the future on the runtime's worker threads, independent of the
/// caller; it needs `'static + Send`, which is why it takes `AppState` BY VALUE
/// (a cheap clone of `Arc`s). The loop has no exit: it lives as long as the
/// process, and the runtime drops it at shutdown.
pub fn start(state: AppState) {
    if state.chat_engine.is_none() {
        tracing::info!(
            "keep loaded: the automatic pinger is not started — no chat engine (no API key)"
        );
        return;
    }
    tracing::info!(
        "keep loaded: the automatic pinger started; it arms from the last chat reply in the store"
    );
    tokio::spawn(async move {
        let mut status = Status::default();
        loop {
            let next = wake(&state, &mut status).await;
            state.keepwarm.wait(next).await;
        }
    });
}

/// What the previous wake decided, so each change is logged ONCE.
#[derive(Debug, Default)]
struct Status {
    last: Option<&'static str>,
}

impl Status {
    /// Log `now_is` at INFO if it differs from the previous wake's status.
    fn note(&mut self, now_is: &'static str, next_ping: Option<DateTime<Utc>>) {
        if self.last == Some(now_is) {
            return;
        }
        let rearmed = self.last.is_none() && now_is == "armed";
        tracing::info!(status = now_is, next_ping = ?next_ping, rearmed_after_restart = rearmed,
            "keep loaded: the automatic pinger's status changed");
        self.last = Some(now_is);
    }
}

/// One wake. Returns when the next one should be.
async fn wake(state: &AppState, status: &mut Status) -> DateTime<Utc> {
    let now = Utc::now();
    let settings = state.settings.current();
    let rules = rules_of(&settings);
    let recheck = now + rules.interval;
    let last_turn = match last_chat_turn(&state.pipeline_pool).await {
        Ok(t) => t.map(|t| t.at),
        Err(e) => {
            tracing::error!(error = %e, "keep loaded: the last chat reply could not be read; trying again in one interval");
            return recheck;
        }
    };
    let facts = Facts {
        last_turn,
        last_ping: state.keepwarm_last_ping.get(),
    };
    state
        .keepwarm
        .update(|l| l.roll(local_day(now, rules.timezone)));
    let ledger = state.keepwarm.ledger();
    match decide(&rules, &facts, &ledger, now) {
        Decision::SleepUntil(due) => {
            status.note("armed", Some(due));
            due
        }
        Decision::Idle(why) => {
            status.note(why.reason(), None);
            recheck
        }
        Decision::Paused(why) => {
            status.note(why.reason(), None);
            recheck
        }
        Decision::Due => {
            attempt(state, &settings, &rules, &facts, &ledger, status).await;
            // Decide again at once: the ping moved the touch (or set a hold or a
            // pause), so the next decision is a sleep, never another ping.
            Utc::now()
        }
    }
}

/// A ping is due: check the cap and the prefix, then send.
async fn attempt(
    state: &AppState,
    settings: &Settings,
    rules: &Rules,
    facts: &Facts,
    ledger: &Ledger,
    status: &mut Status,
) {
    // `decide` returned Due only because a touch exists; the fallback is never used.
    let touch = facts
        .last_turn
        .max(facts.last_ping)
        .unwrap_or_else(Utc::now);
    let hold = |why: Idle| state.keepwarm.update(|l| l.held = Some((touch, why)));
    let prepared = match prepare(state).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "keep loaded: the automatic ping could not be built; holding until the next question");
            hold(Idle::PingFailed);
            return status.note(Idle::PingFailed.reason(), None);
        }
    };
    let today = local_day(Utc::now(), rules.timezone);
    let verdict = match check_before_spending(settings, rules, ledger, &prepared, today) {
        Ok(v) => v,
        Err(Blocked::Pause(why)) => {
            state.keepwarm.update(|l| l.paused = Some(why));
            return status.note(why.reason(), None);
        }
        Err(Blocked::Hold(why)) => {
            hold(why);
            return status.note(why.reason(), None);
        }
    };
    let hash = prepared.hash.clone();
    match send(state, prepared, Trigger::Automatic).await {
        Ok(_) if verdict == PrefixVerdict::Unknown => state.keepwarm.update(|l| {
            l.prefix_reference.get_or_insert(hash);
        }),
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error = %e, reason = Idle::PingFailed.reason(),
                "keep loaded: the automatic ping failed; holding until the next question (no retry)");
            hold(Idle::PingFailed);
            status.note(Idle::PingFailed.reason(), None);
        }
    }
}

/// Why a due ping was not sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Blocked {
    /// Stopped for the rest of the day.
    Pause(Pause),
    /// Held until the next question.
    Hold(Idle),
}

/// The two checks made BEFORE spending, each logged with its reason: would this
/// ping pass the daily cap, and is the case file still the one the last question
/// cached? Pure over what the wake has read, so both are unit-tested.
///
/// # Errors
/// [`Blocked::Pause`] for the cap; [`Blocked::Hold`] for a changed case file.
fn check_before_spending(
    settings: &Settings,
    rules: &Rules,
    ledger: &Ledger,
    prepared: &Prepared,
    today: chrono::NaiveDate,
) -> Result<PrefixVerdict, Blocked> {
    let spent = ledger.spend_on(today);
    let projected = projected_cost(settings, ledger, prepared.chars);
    if !within_cap(spent, projected, rules.daily_cap_dollars) {
        tracing::info!(
            day_spend = spent,
            projected,
            cap = rules.daily_cap_dollars,
            reason = Pause::Cap.reason(),
            "keep loaded: the daily cap is reached; stopped until tomorrow"
        );
        return Err(Blocked::Pause(Pause::Cap));
    }
    let verdict = check_prefix(ledger.prefix_reference.as_deref(), &prepared.hash);
    match verdict {
        PrefixVerdict::Changed => {
            tracing::info!(reason = Idle::PrefixChanged.reason(),
                "keep loaded: the case file changed since the last question; holding until the next one");
            return Err(Blocked::Hold(Idle::PrefixChanged));
        }
        PrefixVerdict::Unknown => tracing::info!(
            "keep loaded: no question since this server started, so this ping cannot check the case file is unchanged; sending it unguarded"
        ),
        PrefixVerdict::Same => {}
    }
    Ok(verdict)
}

/// What the next ping is expected to cost: the last one's measured cost, or —
/// before any has been measured — the body's size at the read price, counted at
/// the chat's own characters-per-token ratio. `0.0` for an unpriced model, which
/// `decide` has already refused.
fn projected_cost(settings: &Settings, ledger: &Ledger, chars: usize) -> f64 {
    if let Some(last) = ledger.last_cost_dollars {
        return last;
    }
    let chat = &settings.question_chat;
    let per_token = u64::from(chat.chars_per_token.max(1));
    read_cost(&chat.model, chars as u64 / per_token).unwrap_or(0.0)
}

#[cfg(test)]
#[path = "chat_keepwarm_task_tests.rs"]
mod tests;
