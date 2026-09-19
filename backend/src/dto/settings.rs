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
    /// The area of the page this row appears in — `services::settings_map`.
    pub area_id: String,
    /// The block inside that area. Together with `area_id` this is the whole of
    /// what the browser knows about the grouping; it computes none of it.
    pub block_id: String,
    /// `Some("Changed — default: 2048")` when the stored value has moved off its
    /// default; `None` when it has not.
    ///
    /// ## Why a phrase and not a `bool`
    ///
    /// Eight of the store's 863 rows differ from their default, and the page's
    /// landing state is exactly that list. The browser needs both the FACT (is
    /// this row in the list?) and the SENTENCE that goes under it, and shipping
    /// the fact alone would leave the page to compose "Changed — default: …"
    /// itself — the one thing this module's header says the browser never does.
    /// `Option` carries both: `is_some()` is the fact, the string is the words.
    pub changed_from_default: Option<String>,
}

/// One openable group in the rail, with how many rows it actually holds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockDto {
    pub id: String,
    pub label: String,
    /// STORED rows that landed here — not the length of the declared key list.
    ///
    /// A key a block declares but the store has never been seeded with cannot be
    /// edited, so counting it would promise a row the page cannot show.
    pub count: usize,
}

/// One entry in the page's left-hand rail.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AreaDto {
    pub id: String,
    pub label: String,
    /// The sum of its blocks' counts, computed on this side of the wire.
    ///
    /// ## Domain note: why the count is never typed into the page
    ///
    /// The mockup this page was built from carried ten counts summing to 863.
    /// Four branches merged between the mockup and the build, and three of those
    /// counts had moved — Practice 303 → 313, Core 29 → 31, Other 85 → 76. Every
    /// one of them would have been a number on screen that was wrong, and wrong
    /// in the direction of looking plausible. They are counted from the rows.
    pub count: usize,
    /// Said under the heading when the area needs explaining. `None` for the
    /// ordinary areas, which explain themselves.
    pub note: Option<String>,
    pub blocks: Vec<BlockDto>,
}

/// The whole page in one read.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsPageDto {
    /// Live parameters first, then dormant ones — ordered by the backend, because
    /// which knobs currently do anything is a fact about the system.
    pub settings: Vec<SettingDto>,
    /// The rail: every area, in the order the page shows them, each with its
    /// blocks and their counts. The browser renders this list; it holds no copy
    /// of the grouping and cannot derive one.
    pub areas: Vec<AreaDto>,
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
