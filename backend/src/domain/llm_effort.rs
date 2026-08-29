//! The Messages API's `effort` dial, and which call families turn it down.
//!
//! ## The incident this exists for (2026-08-28)
//!
//! A post-appeal transcript, pass 1, Opus 5, `max_tokens = 64000`: the model
//! generated for 727 seconds and returned ONLY reasoning blocks. Zero text. The
//! call did not fail, time out, or truncate in the sense the census-R-3 gate
//! catches — it produced a complete, well-formed message with nothing in it.
//!
//! Anthropic's thinking-troubleshooting guidance names the mechanism exactly:
//! adaptive thinking is ON BY DEFAULT on Opus 5 (a change from Opus 4.8/4.7,
//! where omitting `thinking` meant no thinking at all), thinking tokens count
//! against `max_tokens`, and a long thinking pass can therefore consume the
//! entire budget before a single text block is emitted.
//!
//! The documented remedy is this dial. It is explicitly NOT "disable thinking":
//! on Opus 5 that is discouraged, and it has its own failure modes — the model
//! occasionally writes a tool call into visible text instead of a `tool_use`
//! block, and `<thinking>` tags can leak into the response. Turning thinking
//! down is the supported move; turning it off is not.
//!
//! ## Two families, two defaults, and why they differ
//!
//! - **Extraction** (pass 1 and pass 2) defaults to [`Effort::Low`]. Extraction
//!   is a transcription-shaped job: the template says exactly what to find and
//!   the answer is a large JSON document. Deep deliberation buys little and
//!   costs the token budget the ANSWER needs.
//! - **Theme Scan and the practice reader** send NO effort field unless
//!   `LLM_SCAN_EFFORT` is set. Those calls are judgements, not transcription,
//!   and they may genuinely benefit from thinking. Defaulting them down would
//!   be a quality change nobody asked for, made silently, on the way past.
//!
//! Absent is a real state and not a synonym for `high`: omitting the key leaves
//! the provider's own default in force, which is what those calls have always
//! run under. Sending `"high"` explicitly would look identical today and would
//! quietly pin them if Anthropic ever moved the default.
//!
//! ## Rust Learning: a closed vocabulary for a wire enum
//!
//! `Effort` is an enum rather than a `String` because the API rejects anything
//! outside these five values, and a rejection arrives as an HTTP 400 in the
//! middle of a paid run. Parsing at STARTUP instead means a typo in
//! `LLM_EXTRACTION_EFFORT` refuses to boot, which is the cheapest possible place
//! to find out.

use std::fmt;
use std::str::FromStr;

/// How much thinking and overall token spend the model should put into a call.
///
/// The five documented values of `output_config.effort` (GA — no beta header).
/// The API's own default is `high`, which is what omitting the field means.
///
// STRUCTURAL: the Messages API's wire vocabulary, not a setting. A deployment
// cannot invent a sixth level; if Anthropic adds one, that is a code change
// following an API change, which is exactly what a compiled enum should track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effort {
    /// Least thinking. The extraction default — see the module doc.
    Low,
    Medium,
    High,
    /// Added on Opus 4.7, between `high` and `max`.
    XHigh,
    Max,
}

impl Effort {
    /// The exact string the API expects.
    ///
    /// `xhigh` is one word on the wire — the Rust name is `XHigh` only because
    /// identifiers cannot start with a digit-adjacent lowercase run and stay
    /// idiomatic. The two must not be assumed to match; this is the mapping.
    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::XHigh => "xhigh",
            Self::Max => "max",
        }
    }

    /// Every documented value, for error messages and tests.
    pub const ALL: [Effort; 5] = [
        Effort::Low,
        Effort::Medium,
        Effort::High,
        Effort::XHigh,
        Effort::Max,
    ];
}

impl fmt::Display for Effort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_wire())
    }
}

/// ## Rust Learning: `FromStr` is what makes `parse_env_or` work here
///
/// `crate::config::parse_env_or` is generic over `T: FromStr` with a printable
/// error. Implementing this trait — rather than writing a bespoke reader — is
/// what lets the effort keys go through the SAME startup path as every other
/// tunable, so a malformed value fails the boot with the key named, exactly as a
/// malformed `VERIFY_MAX_GAP_CHARS` does.
impl FromStr for Effort {
    type Err = String;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        // Case-insensitive, and whitespace-trimmed by the caller's `parse_or`.
        // An operator who typed `LOW` in a .env file meant `low`; refusing that
        // would be pedantry, not a safety property.
        match raw.trim().to_ascii_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "xhigh" => Ok(Self::XHigh),
            "max" => Ok(Self::Max),
            other => Err(format!(
                "'{other}' is not a documented effort level — expected one of {}",
                Self::ALL
                    .iter()
                    .map(|e| e.as_wire())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }
}

/// Which effort each call family sends, read once at startup.
///
/// Built by [`crate::config::llm_effort_policy_from_env`] — the ONE reader — and
/// carried on both `AppConfig` and `AppContext`, so the pipeline steps and the
/// services cannot come to disagree about the policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LlmEffortPolicy {
    /// `LLM_EXTRACTION_EFFORT`. Pass 1 and pass 2. Defaults to [`Effort::Low`].
    ///
    /// An `Option` even though it is always `Some` today: the field's job is to
    /// say whether a key is sent, and an extraction deployment that ever needs
    /// the provider default back should be able to express that without this
    /// type changing shape.
    pub extraction: Option<Effort>,

    /// `LLM_SCAN_EFFORT`. Theme Scan and the practice reader. `None` — no field
    /// on the wire — unless the operator sets it.
    pub scan: Option<Effort>,
}

/// The default for `LLM_EXTRACTION_EFFORT`.
///
// CONST: the DEFAULT for an env-var-configured policy, which is what Standing
// Rule 2 asks a default to be. Raising it is a config change and a restart, with
// no code change and no rebuild.
pub const DEFAULT_EXTRACTION_EFFORT: Effort = Effort::Low;

impl Default for LlmEffortPolicy {
    /// Extraction turned down, everything else left as the provider ships it.
    fn default() -> Self {
        Self {
            extraction: Some(DEFAULT_EXTRACTION_EFFORT),
            scan: None,
        }
    }
}

#[cfg(test)]
#[path = "llm_effort_tests.rs"]
mod tests;
