//! The LLM retry policy: how many automatic retries, and how long to wait.
//!
//! ## Two caps, because there are two kinds of failure (ruled 2026-08-28)
//!
//! The main ruling that day set automatic LLM retries to zero, because twice in
//! one week a deterministic failure had been re-run and paid for twice — a
//! truncation, and a 600s whole-request timeout that killed a healthy
//! 64000-token generation. See [`crate::llm_retry`].
//!
//! The amendment that followed carves out one exemption, and the reason is
//! purely economic:
//!
//! - **HTTP 429 (`rate_limit_error`) and HTTP 529 (`overloaded_error`)** are
//!   rejections. The request is refused at the front door and *no generation
//!   begins*, so nothing is billed and a retry costs nothing but wall-clock.
//!   These get their own bounded budget, [`LlmRetryPolicy::rate_limit_max_retries`].
//! - **Everything else** — including an `overloaded_error` that arrives as an
//!   event *inside* an open stream — may have started generating, and therefore
//!   may already have billed. Those keep the hard zero, and the run stops and
//!   waits for a human.
//!
//! That boundary is the whole design. It is enforced structurally rather than by
//! inspecting error text: a pre-stream rejection is recognised from the HTTP
//! status in [`crate::pipeline::anthropic_transport::classify_status`], before a
//! single body byte is read, while a mid-stream error is produced by the
//! accumulator in [`crate::pipeline::anthropic_stream`] and can never reach the
//! rate-limit branch.
//!
//! ## Rust Learning: why a `Copy` struct instead of two `u32` parameters
//!
//! Both caps are `u32`, and they travel together through five call sites. Two
//! adjacent same-typed positional parameters are a silent footgun — swap them
//! and the compiler is perfectly happy while the policy inverts. A struct makes
//! the swap impossible, costs nothing at runtime (it is two words, `Copy`, and
//! passed by value like a `u64` pair would be), and means the next policy field
//! reaches all five call sites without touching any of their signatures.

use std::time::Duration;

/// Default for `LLM_RETRY_MAX` — automatic retries of a failed LLM call.
///
/// ZERO by ruling (2026-08-28). The safe direction: an operator who has not
/// thought about the key gets no automatic spend, and the failure waits for a
/// human.
///
// CONST: the DEFAULT for an env-var-configured policy, which is exactly what
// Standing Rule 2 asks a default to be — the live value is `LLM_RETRY_MAX`,
// read once at startup. Raising the cap is a config change and a restart, with
// no code change and no rebuild.
pub const DEFAULT_MAX_RETRIES: u32 = 0;

/// Default for `LLM_RATE_LIMIT_RETRY_MAX` — retries of a PRE-GENERATION
/// rejection (HTTP 429 / 529).
///
/// Five, because these cost nothing but time. With the backoff schedule below
/// (1s, 2s, 4s, 8s, 16s) an unlucky burst rides out about half a minute of
/// congestion before the run stops — long enough to absorb the transient
/// crowding that a busy Theme Scan produces against its own concurrency, short
/// enough that a genuine outage is not sat through in silence.
///
// CONST: the DEFAULT for `LLM_RATE_LIMIT_RETRY_MAX`, same reasoning as above.
pub const DEFAULT_RATE_LIMIT_MAX_RETRIES: u32 = 5;

/// First backoff step, in seconds, when the provider gave no `retry-after`.
///
// CONST: the SHAPE of the backoff curve, not a per-deployment value — the knob
// an operator turns is the number of retries, which is the env var above. A
// deployment that needed a different curve would be describing a different
// provider, which is a code change either way.
const BACKOFF_BASE_SECS: u64 = 1;

/// Ceiling on a single backoff step, in seconds.
///
// CONST: the same reasoning as `BACKOFF_BASE_SECS`. 60s matches the longest
// wait `retry-after` realistically asks for, so an un-advised wait never
// exceeds an advised one by an order of magnitude.
const BACKOFF_CAP_SECS: u64 = 60;

/// Largest shift applied when doubling, so the schedule cannot overflow.
///
// CONST: `1u64 << 20` is already ~12 days, far past `BACKOFF_CAP_SECS`; the
// clamp exists so an absurd `LLM_RATE_LIMIT_RETRY_MAX` cannot shift past the
// width of a u64 (which is a panic in debug and a wrong answer in release),
// not because 20 is meaningful.
const MAX_BACKOFF_SHIFT: u32 = 20;

/// The two automatic-retry caps, read once at startup.
///
/// Constructed by [`crate::config::llm_retry_policy_from_env`] — the ONE reader
/// — and carried on both `AppConfig` and `AppContext` so the pipeline steps and
/// the services cannot come to disagree about the policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LlmRetryPolicy {
    /// `LLM_RETRY_MAX`. Governs whether the durable-execution engine may
    /// re-invoke a step whose LLM call failed — see
    /// `crate::pipeline::workflow_steps::llm_extract_classify`. It deliberately
    /// does NOT drive an in-process retry loop: re-running the call here would
    /// hide the spend inside a step that reported one attempt.
    pub max_retries: u32,

    /// `LLM_RATE_LIMIT_RETRY_MAX`. Governs in-process retries of a
    /// pre-generation rejection, which are the only retries this repo takes
    /// without asking a human, because they are the only ones that are free.
    pub rate_limit_max_retries: u32,
}

impl Default for LlmRetryPolicy {
    /// The shipped policy: no automatic retries, five free ones.
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_MAX_RETRIES,
            rate_limit_max_retries: DEFAULT_RATE_LIMIT_MAX_RETRIES,
        }
    }
}

/// The value carried in `PipelineError::RateLimited` when the provider sent no
/// `retry-after` header.
///
/// ## Rust Learning: encoding "absent" in a foreign type that has no room for it
///
/// `colossus_extract::PipelineError::RateLimited` carries a bare
/// `retry_after_secs: u64`. It lives in a git dependency, so this repo cannot
/// widen it to an `Option`, and the distinction is load-bearing: "the provider
/// said 17 seconds" and "the provider said nothing" call for different waits.
/// `Some(0)` cannot stand in for absence either — its own doc says zero means
/// *retry immediately*, which is a third, real state.
///
/// So absence travels as a sentinel, under the same discipline
/// [`crate::pipeline::truncation`] applies to its message signature: ONE writer
/// ([`advice_from_header`], called by the provider bridge), ONE reader
/// ([`advice_to_header`], called by the retry loop), and a round-trip test that
/// fails if the two are ever changed apart. `u64::MAX` is chosen because no
/// duration remotely near it is a real answer — it is roughly 584 billion years
/// — so it cannot collide with a value a provider might actually send.
pub const NO_RETRY_ADVICE: u64 = u64::MAX;

/// Encode a provider's `retry-after` for transit through `PipelineError`.
///
/// The ONE writer of [`NO_RETRY_ADVICE`].
pub fn advice_from_header(retry_after_secs: Option<u64>) -> u64 {
    match retry_after_secs {
        // A provider that genuinely said `u64::MAX` is not a case worth
        // modelling, but mapping it onto the sentinel rather than through it
        // keeps the round trip total: every input has exactly one encoding.
        Some(secs) if secs != NO_RETRY_ADVICE => secs,
        Some(_) | None => NO_RETRY_ADVICE,
    }
}

/// Decode transit back into "did the provider advise a wait?".
///
/// The ONE reader of [`NO_RETRY_ADVICE`]. Everything downstream sees an
/// `Option` again and never compares against the sentinel itself.
pub fn advice_to_header(advice: u64) -> Option<u64> {
    (advice != NO_RETRY_ADVICE).then_some(advice)
}

/// How long to wait before retry number `attempt` (1-based) of a rejected call.
///
/// ## Domain note: the provider's answer beats our guess
///
/// Anthropic's `retry-after` is not a heuristic — it is the API stating when the
/// token bucket will have room for *this* request. Waiting less guarantees
/// another rejection; waiting more wastes time. So when it is present it is
/// honoured exactly, and the backoff schedule is only the fallback for when the
/// provider said nothing (which is the common case for a 529).
pub fn wait_before_retry(advice: u64, attempt: u32) -> Duration {
    match advice_to_header(advice) {
        Some(secs) => Duration::from_secs(secs),
        None => Duration::from_secs(backoff_secs(attempt)),
    }
}

/// The un-advised schedule: 1s, 2s, 4s, 8s, 16s, … capped at
/// [`BACKOFF_CAP_SECS`].
///
/// `attempt` is 1-based; anything below 1 is treated as the first step rather
/// than underflowing.
fn backoff_secs(attempt: u32) -> u64 {
    let step = attempt.saturating_sub(1).min(MAX_BACKOFF_SHIFT);
    BACKOFF_BASE_SECS
        .saturating_mul(1u64 << step)
        .min(BACKOFF_CAP_SECS)
}

#[cfg(test)]
#[path = "llm_retry_policy_tests.rs"]
mod tests;
