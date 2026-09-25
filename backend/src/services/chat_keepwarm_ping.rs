//! The ONE keep-loaded ping, used by the Admin button and by the automatic
//! pinger alike (CC_TASK_CACHE_KEEPWARM_v1 R3, Law 22).
//!
//! ## Why one function for two callers
//!
//! A ping is only worth its $0.07 if its bytes are exactly the bytes the next
//! question sends, and only honest if its cost is counted where the cap can see
//! it. Two ping paths would be two chances to drift on either. So both callers
//! build with [`prepare`] and send with [`send`], which is the single place that
//! calls the provider, sets the shared `LastPing`, adds to the day's spend and
//! writes the INFO line.
//!
//! The split into two steps exists for the automatic caller: it must compare the
//! prefix fingerprint BEFORE spending anything. The button does not (a person
//! pressing it has decided), and calls [`ping`], which is the two steps in a row.

use chrono::{DateTime, Utc};
use colossus_chat::keepwarm::local_day;
use colossus_chat::{build_prewarm_body, Usage};
use serde_json::Value;

use crate::services::chat_case_prefix::case_file_request;
use crate::services::chat_keepwarm_button::{outcome, ttl_duration, KeepLoadedError, Outcome};
use crate::services::chat_keepwarm_rates::prewarm_cost;
use crate::services::chat_keepwarm_state::prefix_hash;
use crate::services::practice_clock::zone;
use crate::state::AppState;

/// Who asked for the ping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// Someone pressed "Keep loaded for another hour".
    Button,
    /// The automatic pinger.
    Automatic,
}

impl Trigger {
    /// The name the log line carries.
    pub fn name(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Automatic => "automatic",
        }
    }
}

/// A pre-warm body ready to send, with its fingerprint.
#[derive(Debug, Clone)]
pub struct Prepared {
    body: Value,
    /// SHA-256 of the body; compared with what the last real turn cached.
    pub hash: String,
    /// The body's size in characters — the cap's estimate before any ping has
    /// been measured.
    pub chars: usize,
}

#[cfg(test)]
impl Prepared {
    /// A body-less stand-in carrying only what the checks before spending read.
    pub fn for_test(hash: &str, chars: usize) -> Self {
        Self {
            body: Value::Null,
            hash: hash.to_string(),
            chars,
        }
    }
}

/// What one ping did.
#[derive(Debug, Clone, PartialEq)]
pub struct PingReport {
    pub outcome: Outcome,
    /// Measured dollars; `None` for a model whose prices are not known.
    pub cost: Option<f64>,
    /// When the ping landed.
    pub at: DateTime<Utc>,
    /// `at` plus the cache lifetime.
    pub until: DateTime<Utc>,
    /// Today's ping spend after this one, both kinds.
    pub day_spend: f64,
    pub usage: Usage,
}

/// Build the case file's pre-warm body with the chat's current settings.
///
/// # Errors
/// [`KeepLoadedError::CaseFile`] or [`KeepLoadedError::Request`].
pub async fn prepare(state: &AppState) -> Result<Prepared, KeepLoadedError> {
    let request = case_file_request(state).await?;
    let body = build_prewarm_body(&request)?;
    Ok(Prepared {
        hash: prefix_hash(&body),
        chars: body.to_string().len(),
        body,
    })
}

/// Send a prepared ping, record it and log it.
///
/// ## Side effects
/// Sets the shared `LastPing` and adds the measured cost to today's spend. An
/// AUTOMATIC ping that wrote also stops the pinger for the rest of the day.
///
/// # Errors
/// [`KeepLoadedError::EngineOff`] with no API key; [`KeepLoadedError::Provider`]
/// when the provider refuses or fails, logged here at WARN with its own text.
pub async fn send(
    state: &AppState,
    prepared: Prepared,
    trigger: Trigger,
) -> Result<PingReport, KeepLoadedError> {
    let backend = state
        .chat_engine
        .clone()
        .ok_or(KeepLoadedError::EngineOff)?;
    let settings = state.settings.current();
    let chat = &settings.question_chat;
    let usage = match backend.prewarm(&prepared.body).await {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!(trigger = trigger.name(), model = %chat.model, outcome = "error",
                error = %e, "keep loaded: the ping failed");
            return Err(KeepLoadedError::Provider(e));
        }
    };
    let at = Utc::now();
    state.keepwarm_last_ping.set(at);
    let result = outcome(&usage);
    let cost = prewarm_cost(&chat.model, chat.cache_ttl, &usage);
    let until = at + ttl_duration(chat.cache_ttl);
    let today = local_day(at, zone(&settings.practice_read.case_timezone));
    let wrote = result == Outcome::Wrote;
    let automatic = trigger == Trigger::Automatic;
    let day_spend = state.keepwarm.update(|l| {
        l.record_ping(today, cost, wrote, automatic);
        l.spend_dollars
    });
    let report = PingReport {
        outcome: result,
        cost,
        at,
        until,
        day_spend,
        usage,
    };
    log_ping(trigger, &chat.model, &report);
    Ok(report)
}

/// The one INFO line every ping writes — both kinds, the same fields — and the
/// WARN that an automatic ping which had to reload stops the day.
fn log_ping(trigger: Trigger, model: &str, r: &PingReport) {
    tracing::info!(
        trigger = trigger.name(),
        model = %model,
        outcome = ?r.outcome,
        read = ?r.usage.cache_read_input_tokens,
        wrote = ?r.usage.cache_creation_input_tokens,
        input = ?r.usage.input_tokens,
        output = ?r.usage.output_tokens,
        dollars = ?r.cost,
        loaded_until = %r.until,
        day_spend = r.day_spend,
        "keep loaded: ping sent"
    );
    if trigger == Trigger::Automatic && r.outcome == Outcome::Wrote {
        tracing::warn!(dollars = ?r.cost, reason = "ping_wrote",
            "keep loaded: the automatic ping had to reload the case file; stopped until tomorrow");
    }
}

/// Build and send one ping, unguarded — the button's path.
///
/// # Errors
/// See [`prepare`] and [`send`].
pub async fn ping(state: &AppState, trigger: Trigger) -> Result<PingReport, KeepLoadedError> {
    let prepared = prepare(state).await?;
    send(state, prepared, trigger).await
}

#[cfg(test)]
#[path = "chat_keepwarm_ping_tests.rs"]
mod tests;
