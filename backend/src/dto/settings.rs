//! Wire DTOs for the admin Settings page (task 1.6, v2 §2b).
//!
//! §2b names what the v1 surface must show: "every parameter with its current
//! value, its default, and a one-line plain-language meaning". These shapes are
//! that list, composed server-side.
//!
//! ## Everything user-visible arrives composed
//!
//! The meaning, the input hint ("a ratio written n/m"), the bounds sentence
//! ("between 0 and 1") and the dormancy label ("used by readiness verdicts —
//! Phase 2") are all built on the backend. The browser renders them. A page that
//! assembled its own explanation of what a parameter does would be a second
//! source of truth about the configuration law, in the one place least able to
//! check itself.

use serde::{Deserialize, Serialize};

/// One parameter, ready to render.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingDto {
    /// The stable key, shown small — it is what a log line or a psql query names.
    pub key: String,
    /// The current value, as text, exactly as stored and as it should be edited.
    pub value: String,
    /// What it shipped as, so a human can see they have moved it.
    pub default_value: String,
    /// One line of plain language with its § citation.
    pub meaning: String,
    /// What to type — "a whole number, e.g. 240".
    pub input_hint: String,
    /// "Between 0 and 1" / "At least 1" / `None` when unbounded both ways.
    ///
    /// Composed rather than shipped as two numbers: the browser would have to
    /// decide how to phrase one-sided bounds, and that phrasing is part of what
    /// the parameter MEANS.
    pub bounds_label: Option<String>,
    /// Present when nothing reads this parameter yet — "Used by readiness
    /// verdicts — Phase 2 (task 2.4). Changing this has no effect today."
    ///
    /// A settings page that silently lists inert knobs is a page that lies about
    /// its own reach, so the honesty is a field rather than a convention.
    pub dormant_note: Option<String>,
    /// "Last changed by Roman" — or the seed, for a parameter nobody has touched.
    pub last_changed: String,
}

/// The whole page in one read.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsPageDto {
    /// Live parameters first, then dormant ones — ordered by the backend, because
    /// which knobs currently do anything is a fact about the system.
    pub settings: Vec<SettingDto>,
}

/// Request body for changing one parameter.
///
/// ## Why the value is a STRING on the wire
///
/// The store holds every parameter as text with a declared kind, and the page
/// edits it as text. Typing the field as a number here would force the browser to
/// decide whether `9/10` is a number — and would make an unparseable entry a JSON
/// rejection naming nothing useful, instead of the backend's sentence naming the
/// parameter, the bound and what to type instead.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetSettingRequest {
    pub value: String,
}

/// What a change reports back.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingChangedDto {
    pub key: String,
    pub value: String,
    /// The plain confirmation, composed server-side.
    pub message: String,
}
