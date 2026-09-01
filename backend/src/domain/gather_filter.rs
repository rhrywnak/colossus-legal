//! Which parties a ranked gather is allowed to reach.
//!
//! ## Domain note: this is the setting AT-2 turns on
//!
//! Today a scenario's gather reads every Evidence node filed ABOUT its subject
//! and nothing else. That is [`GatherSubjectFilter::Strict`], and it is why S-9
//! and S-11 — two scenarios about different things that happen to name the same
//! person — receive byte-identical pools.
//!
//! Four of the seven $50,000 admissions S-11 must reach are filed ABOUT Emil
//! Awad ALONE. Under `strict` they are unreachable however good the ranking is,
//! because the read never sees them. [`GatherSubjectFilter::Widened`] is what
//! puts them in the pool, and it is the default.
//!
//! `Off` exists to answer "is this a filter problem or a ranking problem?" when
//! a card is missing — it is a diagnostic, not a mode anyone should run in.

use std::fmt;

/// The stored vocabulary of `gather_subject_filter`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatherSubjectFilter {
    /// The subject only — today's behaviour, kept as the conservation baseline.
    Strict,
    /// The subject plus every party the linked allegations name. The default.
    Widened,
    /// No party filter at all. A diagnostic.
    Off,
}

impl GatherSubjectFilter {
    /// The wire and storage token.
    // STRUCTURAL: these three literals are the stored vocabulary of the
    // `gather_subject_filter` row — the values a human types into the settings
    // page and the migration seeds. They are data contract, not a tunable, and
    // `rename_all = "snake_case"` above produces the same three spellings.
    pub fn as_str(self) -> &'static str {
        match self {
            GatherSubjectFilter::Strict => "strict",
            GatherSubjectFilter::Widened => "widened",
            GatherSubjectFilter::Off => "off",
        }
    }

    /// Every value the row may hold, for the error message and the settings UI.
    pub fn allowed() -> [GatherSubjectFilter; 3] {
        [
            GatherSubjectFilter::Strict,
            GatherSubjectFilter::Widened,
            GatherSubjectFilter::Off,
        ]
    }

    /// Which parties this mode lets the search reach.
    ///
    /// `None` means "no party filter" — distinct from `Some(empty)`, which would
    /// mean "reach nothing" and would silently return an empty pool. The two are
    /// kept apart deliberately: a filter that matches nothing and a filter that
    /// is absent are opposite states, and collapsing them is exactly how a
    /// gather comes back empty with nobody able to say why.
    pub fn parties<'a>(self, subject: &'a str, reachable: &'a [String]) -> Option<Vec<&'a str>> {
        match self {
            GatherSubjectFilter::Strict => Some(vec![subject]),
            GatherSubjectFilter::Widened => Some(reachable.iter().map(String::as_str).collect()),
            GatherSubjectFilter::Off => None,
        }
    }
}

impl fmt::Display for GatherSubjectFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for GatherSubjectFilter {
    type Err = String;

    /// Parse a stored value, naming every accepted spelling when it is not one.
    ///
    /// ## Rust Learning: `FromStr` earns you `"strict".parse()?`
    ///
    /// Implementing this one trait is what lets any caller write
    /// `value.parse::<GatherSubjectFilter>()`, and it is where the settings
    /// boot check gets its refusal from — a row holding `widend` stops the
    /// process with the three legal values printed, rather than falling back to
    /// a default and quietly searching the wrong pool for a year.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::allowed()
            .into_iter()
            .find(|mode| mode.as_str() == value)
            .ok_or_else(|| {
                let legal: Vec<&str> = Self::allowed().iter().map(|m| m.as_str()).collect();
                format!(
                    "'{value}' is not a gather subject filter; expected one of {}",
                    legal.join(", ")
                )
            })
    }
}

#[cfg(test)]
#[path = "gather_filter_tests.rs"]
mod tests;
