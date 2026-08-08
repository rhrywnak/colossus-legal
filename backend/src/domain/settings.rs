// =============================================================================
// backend/src/domain/settings.rs — parsing and bounds for stored parameters
// =============================================================================
//
// Task 1.6, v2 §2b — the configuration law. Every tunable is stored as TEXT with
// a declared KIND, and this module is the only place that turns those two into a
// number. Pure: no database, no state, every branch unit-testable.
//
// ## Why the parsing is fallible everywhere, with no defaults
//
// Before this task, a bad value could fall back to a compiled-in constant. After
// it there are no compiled-in constants — that is the whole point of §2b — so a
// value this module cannot read has no safe interpretation. Every function here
// returns a `Result` naming the key, the value, and what was wrong with it, and
// the callers turn that into either a boot refusal or a 400.
//
// ## Rust Learning: parse, don't validate
//
// The pattern throughout: a `&str` goes in and a TYPE comes out — `f32`, `usize`,
// `Ratio`. Once you hold a `Ratio` you know its denominator is non-zero, because
// the only way to build one checked that. Contrast with "validate": a function
// that returns `bool` leaves you holding the same unchecked string afterwards,
// and nothing stops the next caller forgetting to ask.

use std::fmt;

use crate::domain::wording::Wording;
use crate::domain::wording_accusation::AccusationWording;
use crate::domain::wording_authoring::AuthoringWording;
use crate::domain::wording_rehearsal::RehearsalWording;
use crate::domain::wording_rehearsal_chrome::RehearsalChromeWording;
use crate::domain::wording_scan::ScanWording;
use crate::domain::wording_scenario_authoring::ScenarioAuthoringWording;

/// The parsed parameters, as every consumer sees them.
///
/// ## Rust Learning: this struct WAS `Copy`, and task 2.10 took the derive off
///
/// Until 2026-08-04 every field was a plain number, so the whole struct was a
/// handful of bytes and `Copy` let callers pass it by value without ceremony.
/// Roman's ruling extending the configuration law from numbers to TEXT ended
/// that: `wording` holds `String`s, a `String` owns heap memory, and a type
/// containing one cannot be `Copy` — the compiler will not allow a bitwise
/// duplicate of an owning pointer.
///
/// The blast radius was one line, because the snapshot was already threaded by
/// reference everywhere: every consumer takes `&Settings`, and `SettingsHandle`
/// hands out an `Arc`. Nothing was passing it by value except the handle's own
/// constructor. The consistency property `Copy` was there to protect — one
/// request, one snapshot, no stale reference to a swapped-out set of values — is
/// provided by that `Arc`, not by the derive.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// Confidence at or above this reads as the High band (§7).
    pub confidence_band_high: f32,
    /// Confidence at or above this — but below high — reads as Medium (§7).
    pub confidence_band_medium: f32,
    /// Characters of page text shown each side of a quote on a card (§7.1).
    pub quote_context_window_chars: usize,
    /// The most talking points one scenario may carry (§10).
    pub talking_points_cap: usize,
    /// Included citable items needed before ARMED (§9). No consumer until 2.4.
    pub readiness_item_threshold_n: usize,
    /// The share of candidates that must be rulable from the card alone (§7).
    /// No consumer until 2.4.
    pub card_test_ratio: Ratio,
    /// How close a re-found quote must be to count as the same quote (§12.1).
    /// Provisional; no consumer until 2.5.
    pub reanchor_close_match_tolerance: f32,
    /// How many accusations the link panel's short list offers before "Show all"
    /// (task 2.10). The full complaint is always one click behind it.
    pub link_short_list_max: usize,
    /// Every user-facing string task 2.10 introduces, read from the same store.
    ///
    /// Nested rather than flattened into twenty more fields here for one reason:
    /// this struct is the parameters that decide how the system JUDGES, and those
    /// are the words it SPEAKS. A reader looking for a cutoff should not have to
    /// scroll past twenty sentences to find it.
    pub wording: Wording,
    /// The twenty-five strings task 2.11's accusation section speaks.
    ///
    /// A second nested block rather than more fields on `wording` for the reason
    /// that module's own header gives, and for Rule 17: `domain::wording` has no
    /// room left. Both are read from the same `app_settings` table by the same
    /// rules; what separates them is which surface speaks them.
    pub accusation_wording: AccusationWording,
    /// The thirty-nine strings task 2.11 B2's rehearsal page speaks.
    ///
    /// A third nested block, for the reason the second one has: these are the
    /// words ONE surface speaks. The rehearsal page and the working view describe
    /// the same judgments to two different readers — Marie preparing to testify,
    /// and Roman curating — and their language moves independently.
    pub rehearsal_wording: RehearsalWording,
    /// How many DISTINCT dates the PLACED items must carry before the rehearsal
    /// timeline is drawn at all (task 2.11 B2, ruled 2026-08-06).
    ///
    /// ## Domain note: measured over what is placed, never over the pool
    ///
    /// The timeline interleaves marked instances and their paired answers. A
    /// threshold read against the scenario's whole included pool would let the
    /// block promise a timeline the placed set has no rows for — measured on S-2,
    /// four distinct dates in the pool and zero placed items. The honest-gap law
    /// is what decides this, not a display preference.
    pub rehearsal_timeline_min_distinct_dates: usize,
    /// The eighteen strings on the rehearsal page's CONTROLS and markers (task
    /// 2.11 C). Its prose lives in [`Self::rehearsal_wording`]; see
    /// `domain::wording_rehearsal_chrome` for the seam.
    pub rehearsal_chrome_wording: RehearsalChromeWording,
    /// The twenty-three strings the two shared authoring sections speak on the
    /// scenario working page (task 2.11 C, ruling C4b).
    pub authoring_wording: AuthoringWording,
    /// The twelve strings the surfaces that DEFINE a scenario speak — the create
    /// form, the identity modal, and the notice a target-less scenario shows in
    /// place of a candidate queue (2026-08-07 fix).
    ///
    /// A sixth nested block for the reason the others are separate: these words
    /// belong to the moment a scenario is being defined, which is a different
    /// surface and a different reader from curating one that already exists.
    pub scenario_authoring_wording: ScenarioAuthoringWording,
    /// Which judging prompt a Theme Scan reads at START (task 2.15 Tier 2, item
    /// 1d — Roman's amendment of 2026-08-08).
    ///
    /// ## Why this stopped being an env var
    ///
    /// It was `THEME_SCAN_PROMPT_FILE`, with a compiled-in default. Both are gone.
    /// An env var is a deploy to change and invisible once set — measured on DEV,
    /// it was never set at all, so the compiled default silently decided which
    /// prompt judged every scan. As a row it is visible on the Settings page,
    /// editable without a rebuild, asserted at boot, and recorded per run in
    /// `scan_runs.resolved_params`.
    ///
    /// A filename only; it resolves against the registry's template directory.
    pub theme_scan_prompt_file: String,
    /// Shortest quote (with NO paired question) that still reaches the judge.
    /// `0` disables the rule.
    ///
    /// Domain note: the no-question clause is what makes this safe. A four-word
    /// answer is decisive when the interrogatory that prompted it is in evidence,
    /// and the judge is shown both — so only an unanchored fragment is set aside.
    pub theme_scan_prefilter_min_chars: usize,
    /// Statement kinds that never reach the judge, lower-cased and de-duplicated
    /// at parse time. Empty (the stored token `none`) disables the rule.
    ///
    /// Domain note: `referral` — "See the responses to the previous
    /// interrogatories." — is a cross-reference with no assertion in it. The
    /// vocabulary belongs to the extractor, not to this build, which is why the
    /// list is a stored row and not a `match` arm (Standing Rule 2).
    pub theme_scan_prefilter_statement_types: Vec<String>,
    /// The three strings the scan surface speaks (task 2.15 Tier 2).
    pub scan_wording: ScanWording,
    /// How many instances a scenario may carry before the rehearsal page's rows
    /// arrive COMPACT rather than expanded (task 2.11 C).
    ///
    /// ## Domain note: a display default, never a limit on what is shown
    ///
    /// Every instance is always rendered and always reachable — the list is not
    /// paginated at any size, because a page boundary in the middle of "he said
    /// it five times" breaks the one thing the block exists to show. This number
    /// decides only whether a row ARRIVES open or one line tall.
    pub rehearsal_instance_rows_expand_max: usize,
}

/// A snapshot for TESTS ONLY.
///
/// ## Why this is `#[cfg(test)]` and not a `Default` impl
///
/// A `Default for Settings` would be a compiled-in set of parameters — the exact
/// defect v2 §2b bans — and worse, it would be reachable from production code by
/// accident (`..Default::default()`, `unwrap_or_default()`). Gating it on
/// `cfg(test)` means it cannot exist in a release binary at all: production has
/// one way to obtain a `Settings`, and that is to read the store.
///
/// The values match the migration's seed so a test reads the way the product
/// behaves. `settings_store_tests` separately asserts the seed still produces
/// exactly these numbers, so the two cannot drift apart silently.
#[cfg(test)]
impl Settings {
    pub fn for_test() -> Self {
        Settings {
            confidence_band_high: 0.80,
            confidence_band_medium: 0.50,
            quote_context_window_chars: 240,
            talking_points_cap: 3,
            readiness_item_threshold_n: 5,
            card_test_ratio: Ratio {
                numerator: 9,
                denominator: 10,
            },
            reanchor_close_match_tolerance: 0.85,
            link_short_list_max: 8,
            wording: Wording::for_test(),
            accusation_wording: AccusationWording::for_test(),
            rehearsal_wording: RehearsalWording::for_test(),
            rehearsal_timeline_min_distinct_dates: 2,
            rehearsal_chrome_wording: RehearsalChromeWording::for_test(),
            authoring_wording: AuthoringWording::for_test(),
            scenario_authoring_wording: ScenarioAuthoringWording::for_test(),
            theme_scan_prompt_file: "theme_scan_prompt_v3.md".to_string(),
            theme_scan_prefilter_min_chars: 60,
            theme_scan_prefilter_statement_types: vec!["referral".to_string()],
            scan_wording: ScanWording::for_test(),
            rehearsal_instance_rows_expand_max: 3,
        }
    }
}

/// How a stored value should be read.
///
/// Extensible by design: adding a kind is a code change plus a seed row, never a
/// schema migration. That is why the column is free TEXT rather than a CHECK —
/// see the migration header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    /// A real number, e.g. a band cutoff or a similarity tolerance.
    Float,
    /// A non-negative whole number, e.g. a cap or a character window.
    Count,
    /// `n/m` — a share written the way a human says it ("9 of 10").
    Ratio,
    /// Words a human reads on screen — a heading, a button, a refusal, a
    /// sentence template (task 2.10, the configuration law extended to text).
    ///
    /// ## Why text needed a KIND at all, rather than "anything not a number"
    ///
    /// Two reasons, both about being loud. A row whose declared kind says `text`
    /// while this build reads it as a float is a store that has drifted from the
    /// code, and `expect_kind` reports it as exactly that instead of as a parse
    /// failure that sends a human hunting through the value. And the Settings
    /// page's input hint is composed from the kind, so a text row that had no
    /// kind would advertise a number.
    Text,
}

impl ValueKind {
    /// The full vocabulary, so a refusal can list what IS accepted.
    pub const ALL: &'static [ValueKind] = &[
        ValueKind::Float,
        ValueKind::Count,
        ValueKind::Ratio,
        ValueKind::Text,
    ];

    /// The stable token stored in `app_settings.value_kind`.
    pub fn code(self) -> &'static str {
        match self {
            ValueKind::Float => "float",
            ValueKind::Count => "count",
            ValueKind::Ratio => "ratio",
            ValueKind::Text => "text",
        }
    }

    /// What a human should type, shown beside the edit box.
    ///
    /// Composed here rather than in the browser: it is a statement about how this
    /// build parses the field, and the browser does not know that.
    ///
    /// ## Why the example is the parameter's OWN default (task 1.7A, D4)
    ///
    /// The example used to be a fixed literal per kind, so every whole-number
    /// field advertised "e.g. 240" — including `talking_points_cap`, which
    /// defaults to 3 and has a minimum of 1. A worked example that would be
    /// refused by the very field it sits under teaches the wrong number and
    /// undermines the hint's only job. The row's own default is the one example
    /// guaranteed to be both well-formed and in bounds.
    pub fn hint(self, default: &str) -> String {
        match self {
            ValueKind::Float => format!("a number, e.g. {default}"),
            ValueKind::Count => format!("a whole number, e.g. {default}"),
            ValueKind::Ratio => format!("n/m, e.g. {default}"),
            // The default is not offered as an example here: it IS the sentence
            // in the box beside the hint, so "e.g. Save and next" under a field
            // already reading "Save and next" is noise. What a human needs to
            // know about a text field is the one rule it has.
            ValueKind::Text => "words, as they should read on screen".to_string(),
        }
    }
}

impl TryFrom<&str> for ValueKind {
    type Error = SettingError;

    fn try_from(token: &str) -> Result<Self, Self::Error> {
        match token {
            "float" => Ok(ValueKind::Float),
            "count" => Ok(ValueKind::Count),
            "ratio" => Ok(ValueKind::Ratio),
            "text" => Ok(ValueKind::Text),
            other => Err(SettingError::UnknownKind {
                kind: other.to_string(),
            }),
        }
    }
}

/// A share, as `n` of `m`.
///
/// ## Domain note: why a ratio and not a decimal
///
/// The card test is "rulable from the card alone for at least 9 of 10
/// candidates" (§7). Storing that as `0.9` would lose the denominator, and the
/// denominator is part of what the rule MEANS — 9 of 10 and 90 of 100 are
/// different claims about how much evidence the test needs before it says
/// anything. A human also reads and edits it the way they say it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ratio {
    pub numerator: u32,
    /// Guaranteed non-zero: the only constructor rejects zero.
    pub denominator: u32,
}

impl fmt::Display for Ratio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

impl Ratio {
    /// The ratio as a fraction, for comparing against a measured share.
    ///
    /// ## Rust Learning: why this cannot divide by zero
    ///
    /// `denominator` is private to construct — `parse_ratio` is the only way to
    /// build a `Ratio`, and it refuses zero. The invariant is held by the TYPE,
    /// so this function needs no guard and no `Result`. That is the payoff of
    /// parse-don't-validate: the check happens once, at the boundary, instead of
    /// at every use.
    pub fn as_fraction(self) -> f64 {
        f64::from(self.numerator) / f64::from(self.denominator)
    }
}

/// Why a stored or submitted parameter could not be used.
///
/// Every message is written for the HUMAN editing the Settings page — they are
/// the only one who can fix any of these — and names the key, so the same text
/// is equally useful in a boot-refusal log line.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SettingError {
    #[error("'{kind}' is not a value kind this build understands (float, count, ratio, text)")]
    UnknownKind { kind: String },

    /// A stored or submitted string that is empty, or only whitespace.
    ///
    /// Its own variant rather than an `Unreadable`: "this is not a number" and
    /// "you left this blank" are different mistakes with different remedies, and a
    /// blank LABEL is the specific failure worth naming — it does not produce an
    /// error anywhere downstream, it produces an invisible button.
    #[error(
        "{key} cannot be blank — it is words a human reads on screen, and an \
         empty one leaves a control with no label at all"
    )]
    Blank { key: String },

    /// A template edited into one that no longer carries its own facts.
    ///
    /// The refusal NAMES the missing placeholders, because the alternative — a
    /// sentence like "You linked this to  · they'll use it against us." — is
    /// grammatical, renders perfectly, and is missing the fact it exists to state.
    #[error(
        "{key} must still contain {missing} — without it the sentence renders \
         with the fact removed, and nothing downstream can tell"
    )]
    MissingPlaceholders { key: String, missing: String },

    #[error("{key} needs {expected}, but the value is '{value}'")]
    Unreadable {
        key: String,
        value: String,
        expected: &'static str,
    },

    #[error("{key} must be at least {min}, but the value is {value}")]
    BelowMinimum {
        key: String,
        value: String,
        min: f64,
    },

    #[error("{key} must be at most {max}, but the value is {value}")]
    AboveMaximum {
        key: String,
        value: String,
        max: f64,
    },

    #[error("a ratio needs a denominator above zero — '{value}' would divide by nothing")]
    ZeroDenominator { value: String },

    #[error(
        "the high confidence cutoff ({high}) must be above the medium cutoff \
         ({medium}) — otherwise no score could ever land in the medium band"
    )]
    BandsCrossed { high: f32, medium: f32 },

    #[error(
        "no parameter named '{key}' is stored. Every parameter this build reads \
         must exist in app_settings — there are no compiled-in defaults to fall \
         back to (v2 §2b)"
    )]
    Missing { key: String },
}

/// Bounds as declared on the row. `None` on a side means unbounded there.
#[derive(Debug, Clone, Copy, Default)]
pub struct Bounds {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

impl Bounds {
    /// Check one number against the declared bounds.
    ///
    /// Bounds are INCLUSIVE. A cutoff of exactly 1.0 is a legitimate "nothing is
    /// ever High"; a window of exactly 0 is a legitimate "show no context". Both
    /// are strange settings and neither is an error — refusing them would be this
    /// module deciding policy, which is the human's job.
    ///
    /// # Errors
    /// Returns [`SettingError`] naming the key and the bound it crossed.
    pub fn check(self, key: &str, value: f64) -> Result<(), SettingError> {
        if let Some(min) = self.min {
            if value < min {
                return Err(SettingError::BelowMinimum {
                    key: key.to_string(),
                    value: value.to_string(),
                    min,
                });
            }
        }
        if let Some(max) = self.max {
            if value > max {
                return Err(SettingError::AboveMaximum {
                    key: key.to_string(),
                    value: value.to_string(),
                    max,
                });
            }
        }
        Ok(())
    }
}

/// Read a `float` parameter and check its bounds.
///
/// # Errors
/// Returns [`SettingError`] if the text is not a number or falls outside bounds.
pub fn parse_float(key: &str, value: &str, bounds: Bounds) -> Result<f32, SettingError> {
    let parsed: f32 = value.trim().parse().map_err(|_| SettingError::Unreadable {
        key: key.to_string(),
        value: value.to_string(),
        expected: "a number",
    })?;

    // NaN passes no comparison, so it would slip through both bound checks and
    // then poison every band decision downstream (see `band_for_score`'s note on
    // f32 ordering). Refused explicitly rather than left to the bounds.
    if parsed.is_nan() {
        return Err(SettingError::Unreadable {
            key: key.to_string(),
            value: value.to_string(),
            expected: "a number",
        });
    }

    bounds.check(key, f64::from(parsed))?;
    Ok(parsed)
}

/// Read a `count` parameter and check its bounds.
///
/// # Errors
/// Returns [`SettingError`] if the text is not a whole number or is out of bounds.
pub fn parse_count(key: &str, value: &str, bounds: Bounds) -> Result<usize, SettingError> {
    let parsed: usize = value.trim().parse().map_err(|_| SettingError::Unreadable {
        key: key.to_string(),
        value: value.to_string(),
        expected: "a whole number (0 or more)",
    })?;

    // `usize` cannot be negative, so a "-1" fails the parse above with the
    // message naming what was expected rather than an integer-overflow surprise.
    bounds.check(key, parsed as f64)?;
    Ok(parsed)
}

/// Read a `ratio` parameter written `n/m`.
///
/// # Errors
/// Returns [`SettingError`] if the text is not `n/m`, or the denominator is zero.
pub fn parse_ratio(key: &str, value: &str) -> Result<Ratio, SettingError> {
    let unreadable = || SettingError::Unreadable {
        key: key.to_string(),
        value: value.to_string(),
        expected: "a ratio written n/m, such as 9/10",
    };

    let (left, right) = value.trim().split_once('/').ok_or_else(unreadable)?;
    let numerator: u32 = left.trim().parse().map_err(|_| unreadable())?;
    let denominator: u32 = right.trim().parse().map_err(|_| unreadable())?;

    if denominator == 0 {
        return Err(SettingError::ZeroDenominator {
            value: value.to_string(),
        });
    }
    Ok(Ratio {
        numerator,
        denominator,
    })
}

/// Read a `text` parameter: the words themselves, trimmed, never blank.
///
/// ## Why this is fallible when "it's just a string" looks like it cannot fail
///
/// It can fail in the one way that matters. A blank value parses fine as text and
/// then renders as a button with no words on it — a control the human cannot see,
/// produced with nothing in the log to say why. That is precisely the class of
/// silent failure Standing Rule 1 exists for, so the empty case is a REFUSAL at
/// the boundary rather than a surprise on screen.
///
/// Trimmed for the same reason the summary-override write path trims: leading
/// whitespace in a stored label is invisible in psql and visible on screen.
/// Bounds do not apply — `min_value` / `max_value` are numeric comparisons and a
/// text row leaves them NULL.
///
/// # Errors
/// Returns [`SettingError::Blank`] naming the key when the value has no
/// non-whitespace characters.
pub fn parse_text(key: &str, value: &str) -> Result<String, SettingError> {
    let text = value.trim();
    if text.is_empty() {
        return Err(SettingError::Blank {
            key: key.to_string(),
        });
    }
    Ok(text.to_string())
}

/// The stored token that means "this list is deliberately empty".
///
/// A `text` row may not be blank (see [`parse_text`] — a blank label is an
/// invisible control), so "no statement types are filtered" needs a word. It is
/// spelled out in the row's own `meaning` on the Settings page, and the resolved
/// list is logged at every scan start, so a human can always see which of the two
/// states they are in.
pub const LIST_NONE_TOKEN: &str = "none";

/// Read a `text` parameter holding a comma-separated LIST of tokens.
///
/// Lower-cased and de-duplicated, empty entries dropped, order preserved. The
/// literal `none` (alone, in any case) yields an empty list.
///
/// ## Why this is a parse and not a `split(',')` at the call site
///
/// Two call sites splitting the same row would eventually disagree about
/// whitespace or case — and a mismatch there is silent: the setting simply stops
/// matching anything, with no error and no log line. Parsing once, here, means
/// every consumer holds an already-normalised `Vec<String>` and the normalisation
/// rules are testable on their own.
///
/// # Errors
/// Returns [`SettingError::Blank`] when the value has no non-whitespace
/// characters — the same rule every text row obeys. Use `none` for an empty list.
pub fn parse_token_list(key: &str, value: &str) -> Result<Vec<String>, SettingError> {
    let text = parse_text(key, value)?;
    if text.eq_ignore_ascii_case(LIST_NONE_TOKEN) {
        return Ok(Vec::new());
    }

    let mut tokens: Vec<String> = Vec::new();
    for raw in text.split(',') {
        let token = raw.trim().to_lowercase();
        // A trailing comma or a double comma is a typo, not an instruction to
        // filter the empty string — which would match every unclassified node.
        if token.is_empty() || tokens.contains(&token) {
            continue;
        }
        tokens.push(token);
    }
    Ok(tokens)
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
