//! What the automatic keep-loaded ping remembers between wakes, and how a real
//! chat turn tells it that the case file was just used.
//!
//! ## What lives here, and what deliberately does not
//!
//! - The day's [`Ledger`] (spend, a stop for the day, the prefix fingerprint the
//!   last real turn cached) — in memory. A restart resets the day's spend: the
//!   debt accepted in PLAN R3 §9(3), owed to CC_TASK_COST_PAGE_v1.
//! - A [`Notify`] a real turn fires, so an idle pinger arms at once.
//! - NOT the last real turn: that is read from `discussion_messages` on every
//!   wake, which is what lets the pinger re-arm after a restart (R3 §3).
//! - NOT the last ping: that is the button's own `LastPing`, shared (Law 22).
//!
//! ## Rust Learning: `tokio::sync::Notify`
//!
//! `Notify` is a wake-up signal with no payload. `notify_one()` either wakes a
//! task currently parked in `notified().await`, or — if nobody is waiting —
//! stores ONE permit so the next `notified().await` returns at once. That second
//! half is why a turn that finishes while the pinger is busy pinging is not lost.

use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{DateTime, Utc};
use colossus_chat::keepwarm::Ledger;
use colossus_chat::{build_prewarm_body, ChatRequest, ChatTool, RequestError};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::sync::Notify;

/// The pinger's shared state. One per process, behind an `Arc` in `AppState`.
#[derive(Debug, Default)]
pub struct KeepWarm {
    notify: Notify,
    ledger: Mutex<Ledger>,
}

/// Why a wait ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Woke {
    /// The deadline passed.
    Deadline,
    /// A real turn (or another touch) fired the notify.
    Touched,
}

impl KeepWarm {
    /// The ledger, locked.
    ///
    /// ## Why a poisoned lock is recovered, not refused
    ///
    /// A poisoned `Mutex` means some thread panicked while holding it. The data
    /// inside is still the last value written — a spend total and a reason — and
    /// throwing it away would reset the day's spend, the one direction that can
    /// overspend. `into_inner` keeps it; the warning makes the event visible.
    fn lock(&self) -> MutexGuard<'_, Ledger> {
        self.ledger.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("keep loaded: the ledger lock was poisoned; keeping its last value");
            poisoned.into_inner()
        })
    }

    /// A copy of the ledger as it is now.
    pub fn ledger(&self) -> Ledger {
        self.lock().clone()
    }

    /// Change the ledger under its lock and return what `f` returns.
    pub fn update<R>(&self, f: impl FnOnce(&mut Ledger) -> R) -> R {
        f(&mut self.lock())
    }

    /// A real turn finished: remember what it cached, and wake the pinger.
    pub fn touched(&self, prefix_hash: String) {
        self.update(|l| l.prefix_reference = Some(prefix_hash));
        self.notify.notify_one();
    }

    /// Wait until `until`, or until a real turn fires the notify — whichever
    /// comes first.
    ///
    /// ## Rust Learning: `tokio::select!`
    ///
    /// `select!` polls both futures and runs the arm of whichever finishes first;
    /// the other is dropped (cancelled). Dropping a `sleep` is free, and dropping a
    /// `notified()` leaves any stored permit in place for the next wait.
    pub async fn wait(&self, until: DateTime<Utc>) -> Woke {
        // A deadline already in the past converts to a zero wait, not an error.
        let wait = (until - Utc::now()).to_std().unwrap_or_default();
        tokio::select! {
            _ = self.notify.notified() => Woke::Touched,
            _ = tokio::time::sleep(wait) => Woke::Deadline,
        }
    }
}

/// The fingerprint of a pre-warm body: SHA-256 of its JSON bytes, lowercase hex.
///
/// Two bodies with the same fingerprint are byte-identical, so a ping built
/// from one reads what a turn built from the other cached.
pub fn prefix_hash(body: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}

/// The fingerprint of the case file a real turn sent.
///
/// A turn's `request.tools` is still empty here — `run_turn` fills it from the
/// tool list — so it is filled the same way first, exactly as
/// `chat_case_prefix::case_file_request` does for a ping.
///
/// # Errors
/// Whatever [`build_prewarm_body`] raises (a turn with no documents).
pub fn turn_prefix_hash(
    request: &ChatRequest,
    tools: &[Arc<dyn ChatTool>],
) -> Result<String, RequestError> {
    let mut shaped = request.clone();
    shaped.tools = tools.iter().map(|t| t.spec()).collect();
    Ok(prefix_hash(&build_prewarm_body(&shaped)?))
}

/// Record a finished real turn. Called once, on the stream's success arm.
///
/// A turn whose prefix cannot be fingerprinted still wakes the pinger; the
/// reference is left as it was, so the next ping is guarded by the older one
/// (or unguarded after a restart) — logged, never silent.
pub fn record_turn(keepwarm: &KeepWarm, request: &ChatRequest, tools: &[Arc<dyn ChatTool>]) {
    match turn_prefix_hash(request, tools) {
        Ok(hash) => keepwarm.touched(hash),
        Err(e) => {
            tracing::warn!(error = %e, "keep loaded: the turn's case file could not be fingerprinted");
            keepwarm.notify.notify_one();
        }
    }
}

#[cfg(test)]
#[path = "chat_keepwarm_state_tests.rs"]
mod tests;
