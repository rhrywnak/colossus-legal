//! The Admin "Chat case file" box: keep the chat's case file loaded on demand,
//! and say when it was last used and how long it stays loaded.
//!
//! ## Domain note: what the button buys
//!
//! Every chat turn sends the whole case file (about 369k tokens). The provider
//! keeps it in its cache for the configured lifetime (one hour) after the last
//! time anything used it. Inside that hour a turn READS it (about $0.07); after
//! it, the next turn pays to WRITE it again (about $2.95). The button sends a
//! pre-warm, which uses the case file without asking for a reply, so the hour
//! starts again.
//!
//! ## Where "the last ping" lives: in memory, reset on restart
//!
//! [`LastPing`] sits in `AppState` and is never stored. Since
//! CC_TASK_CACHE_KEEPWARM_v1 R3 the automatic pinger sets the SAME value (both
//! go through `chat_keepwarm_ping::send`), so "Loaded until" counts either kind. After a restart the box
//! falls back to the last chat reply alone, so it can say "Not loaded" while the
//! provider in fact still holds the case file. The DEV and PROD servers also
//! share one provider cache, so either one can keep it warm without the other
//! knowing. Both mistakes run the safe way: the page may predict a reload that
//! does not come, never promise a loaded case file that is gone. Accepted in
//! CC_TASK_KEEPWARM_BUTTON_v1; the page does not explain it.

use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};
use colossus_chat::keepwarm::{decide, Decision, Facts, Idle};
use colossus_chat::{CacheTtl, ChatTransportError, RequestError, Usage};
use serde::Serialize;

use crate::domain::settings::Settings;
use crate::domain::wording_templates::render;
use crate::repositories::pipeline_repository::chat_last_turn::last_chat_turn;
use crate::repositories::pipeline_repository::PipelineRepoError;
use crate::services::chat_case_prefix::{case_file_request, CaseFileError};
use crate::services::chat_keepwarm_ping::{ping, Trigger};
use crate::services::chat_keepwarm_rates::reload_cost;
use crate::services::chat_keepwarm_task::rules_of;
use crate::services::chat_prefix_size::{package_size, PackageSize, PrefixParts};
use crate::services::chat_question_gather::display_name;
use crate::services::practice_clock::{local_clock, local_clock_of, local_day, local_stamp};
use crate::state::AppState;

// STRUCTURAL: the provider's `5m` cache lifetime, in minutes — the meaning of
// a wire spelling, not a tunable value (the setting chooses WHICH lifetime).
const FIVE_MINUTES: i64 = 5;
// STRUCTURAL: the provider's `1h` cache lifetime, in minutes, likewise.
const ONE_HOUR_MINUTES: i64 = 60;

/// When the last successful button ping happened, since this process started.
///
/// ## Rust Learning: `Mutex` inside a shared `Arc`
///
/// `AppState` is cloned into every request, so a plain field would be copied and
/// a ping recorded by one request would be invisible to the next. `AppState`
/// holds this behind an `Arc`, so every clone points at the SAME value, and the
/// `Mutex` is what allows a write through that shared reference.
#[derive(Debug, Default)]
pub struct LastPing(Mutex<Option<DateTime<Utc>>>);

impl LastPing {
    /// The last successful ping, if any since boot.
    pub fn get(&self) -> Option<DateTime<Utc>> {
        // best-effort: a poisoned lock means a writer panicked mid-write. Reading
        // it as "no ping" can only make the box say "Not loaded" too early, the
        // safe direction; it is logged, not propagated, because a missing
        // timestamp is not a reason to refuse the Admin page.
        match self.0.lock() {
            Ok(at) => *at,
            Err(_) => {
                tracing::warn!("keep loaded: the last-ping lock was poisoned; showing no ping");
                None
            }
        }
    }

    /// Record a successful ping.
    pub fn set(&self, at: DateTime<Utc>) {
        match self.0.lock() {
            Ok(mut slot) => *slot = Some(at),
            Err(_) => {
                tracing::warn!("keep loaded: the last-ping lock was poisoned; ping not remembered")
            }
        }
    }
}

/// Did the ping find the case file loaded, or have to load it again?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// It read the cached case file: it was still loaded.
    Read,
    /// It wrote the case file: it had run out and was reloaded.
    Wrote,
}

/// Wrote when the ping wrote more than it read. A partial rewrite (the system
/// prompt read, the documents written) is a reload, and counts as one.
pub fn outcome(usage: &Usage) -> Outcome {
    let read = usage.cache_read_input_tokens.unwrap_or(0);
    let wrote = usage.cache_creation_input_tokens.unwrap_or(0);
    if wrote > read {
        Outcome::Wrote
    } else {
        Outcome::Read
    }
}

/// How long a cache entry lives after its last use.
pub fn ttl_duration(ttl: CacheTtl) -> Duration {
    match ttl {
        CacheTtl::FiveMinutes => Duration::minutes(FIVE_MINUTES),
        CacheTtl::OneHour => Duration::minutes(ONE_HOUR_MINUTES),
    }
}

/// The later of the last chat reply and the last button ping, plus the cache
/// lifetime. `None` when neither has happened since boot.
pub fn loaded_until(
    last_turn: Option<DateTime<Utc>>,
    last_ping: Option<DateTime<Utc>>,
    ttl: CacheTtl,
) -> Option<DateTime<Utc>> {
    last_turn.max(last_ping).map(|t| t + ttl_duration(ttl))
}

/// A time as the box shows it: the clock alone today, the date and clock on any
/// other day, so an old time is never read as today's.
pub fn when(at: DateTime<Utc>, now: DateTime<Utc>, timezone: &str) -> String {
    if local_day(at, timezone) == local_day(now, timezone) {
        local_clock(at, timezone)
    } else {
        local_stamp(at, timezone)
    }
}

/// What the box shows about the last activity. Times are already formatted in
/// the case's timezone; the browser composes the sentences around them.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActivityDto {
    /// When the last successful chat reply was stored; `None` if nobody has chatted.
    pub last_question_at: Option<String>,
    /// Whose thread it was, as the practice pages name them.
    pub last_question_by: Option<String>,
    /// Still loaded right now, by this server's knowledge.
    pub loaded: bool,
    /// When it stops being loaded; `None` when nothing is known to have loaded it.
    pub loaded_until: Option<String>,
    /// When not loaded: what the next question's reload will cost. `None` when
    /// loaded, when the model is unpriced, or when the size was not measured.
    pub reload_cost_dollars: Option<f64>,
    /// The automatic ping's line: on with its window, paused for the day, or off.
    pub automatic_line: String,
}

/// The result of one tap.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KeepLoadedDto {
    pub outcome: Outcome,
    /// Now plus the cache lifetime, formatted.
    pub loaded_until: String,
    /// What this ping cost; `None` for a model whose prices are not known.
    pub cost_dollars: Option<f64>,
    /// The box's activity lines, refreshed after the ping.
    pub activity: ActivityDto,
}

/// Everything the box's two routes can fail with.
#[derive(Debug, thiserror::Error)]
pub enum KeepLoadedError {
    #[error("the question chat is not configured on this server: no ANTHROPIC_API_KEY")]
    EngineOff,
    #[error("the case file could not be assembled: {0}")]
    CaseFile(#[from] CaseFileError),
    #[error("the keep-loaded request could not be built: {0}")]
    Request(#[from] RequestError),
    #[error("the model provider did not take the keep-loaded request: {0}")]
    Provider(ChatTransportError),
    #[error("last_chat_turn failed: {0}")]
    Store(#[from] PipelineRepoError),
}

/// The box's activity lines, read now.
///
/// # Errors
/// [`KeepLoadedError::Store`] if the last reply cannot be read; the case-file
/// errors if the reload estimate needs the case file and it cannot be built.
pub async fn activity(state: &AppState) -> Result<ActivityDto, KeepLoadedError> {
    let settings = state.settings.current();
    let timezone = settings.practice_read.case_timezone.as_str();
    let chat = &settings.question_chat;
    let now = Utc::now();
    let last = last_chat_turn(&state.pipeline_pool).await?;
    let until = loaded_until(
        last.as_ref().map(|t| t.at),
        state.keepwarm_last_ping.get(),
        chat.cache_ttl,
    );
    let loaded = until.is_some_and(|u| u > now);
    let reload_cost_dollars = if loaded {
        None
    } else {
        reload_estimate(state).await?
    };
    Ok(ActivityDto {
        last_question_at: last.as_ref().map(|t| when(t.at, now, timezone)),
        last_question_by: last.as_ref().map(|t| display_name(&settings, &t.username)),
        loaded,
        loaded_until: until.filter(|_| loaded).map(|u| when(u, now, timezone)),
        reload_cost_dollars,
        automatic_line: automatic_line(
            &settings,
            &Facts {
                last_turn: last.as_ref().map(|t| t.at),
                last_ping: state.keepwarm_last_ping.get(),
            },
            &state.keepwarm.ledger(),
            now,
        ),
    })
}

/// The box's automatic line, from the SAME rules the pinger runs — so the page
/// can never say "on" about a day the pinger has stopped.
///
/// Off when switched off; paused whenever the rules have stopped for the rest of
/// the day (the cap, a reload, an unpriced model, a closed window — the log says
/// which); otherwise on, with the window's times.
pub fn automatic_line(
    settings: &Settings,
    facts: &Facts,
    ledger: &colossus_chat::keepwarm::Ledger,
    now: DateTime<Utc>,
) -> String {
    let words = &settings.admin_wording.case_file;
    let window = &settings.question_chat.keepwarm;
    match decide(&rules_of(settings), facts, ledger, now) {
        Decision::Idle(Idle::Disabled) => words.automatic_off.clone(),
        Decision::Paused(_) => words.automatic_paused.clone(),
        Decision::Due | Decision::SleepUntil(_) | Decision::Idle(_) => {
            let start = local_clock_of(window.window_start);
            let end = local_clock_of(window.window_end);
            render(
                &words.automatic_on,
                &[("start", start.as_str()), ("end", end.as_str())],
            )
        }
    }
}

/// What reloading the case file would cost, from the provider's own count of it
/// (free, and remembered by [`crate::services::chat_prefix_size`]). An estimate
/// that is only arithmetic is not shown as dollars (Law 10: measured only).
///
/// Shared with the Overview jobs panel (`services::ai_jobs`), whose Discuss chat
/// and case story rows state the same price the box does.
///
/// # Errors
/// [`KeepLoadedError::CaseFile`] when the case file cannot be assembled.
pub(crate) async fn reload_estimate(state: &AppState) -> Result<Option<f64>, KeepLoadedError> {
    let Some(backend) = state.chat_engine.clone() else {
        return Ok(None);
    };
    let settings = state.settings.current();
    let chat = &settings.question_chat;
    let request = case_file_request(state).await?;
    let prefix = PrefixParts {
        system: &request.system,
        documents: request
            .documents
            .iter()
            .map(|d| (d.title.as_str(), d.context.as_deref(), d.text.as_str()))
            .collect(),
    };
    let chars_per_token = usize::try_from(chat.chars_per_token).unwrap_or(1);
    let size = package_size(
        backend.as_ref(),
        &state.chat_prefix_size,
        &prefix,
        "",
        chars_per_token,
        || request.clone(),
    )
    .await;
    Ok(match size {
        PackageSize::Measured(tokens) => reload_cost(&chat.model, chat.cache_ttl, tokens),
        PackageSize::Estimated(_) => None,
    })
}

/// Send one keep-loaded ping now, through the one ping path the automatic
/// pinger also uses (`chat_keepwarm_ping`), and answer with the refreshed lines.
///
/// The button is never refused by the daily cap — a person pressing it has
/// decided — but its cost counts in the day's spend (PLAN R3 §9(1)).
///
/// # Errors
/// See [`KeepLoadedError`]. A provider failure is logged at WARN with the
/// provider's own text before it is returned.
pub async fn keep_loaded(state: &AppState) -> Result<KeepLoadedDto, KeepLoadedError> {
    let report = ping(state, Trigger::Button).await?;
    let settings = state.settings.current();
    let timezone = settings.practice_read.case_timezone.as_str();
    Ok(KeepLoadedDto {
        outcome: report.outcome,
        loaded_until: when(report.until, report.at, timezone),
        cost_dollars: report.cost,
        activity: activity(state).await?,
    })
}

#[cfg(test)]
#[path = "chat_keepwarm_button_tests.rs"]
mod tests;
