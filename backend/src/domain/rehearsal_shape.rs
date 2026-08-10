//! Whether a rehearsal section arrives open — the page's SHAPE, not its words.
//!
//! Split out of `domain::wording_rehearsal` when that module crossed the 300-line
//! limit under task R3's sixteen new rows, and the seam is one the two halves
//! already had. `wording_rehearsal` holds STRINGS a human reads; this decides
//! STRUCTURE from a stored value. They also change for different reasons: Roman
//! rewords a heading because the legal framing shifted, and changes a section's
//! default state because he is tired of scrolling past it.

use crate::domain::settings::SettingError;

/// Whether a section starts open.
///
/// ## Rust Learning: a two-variant enum instead of `value == "open"`
///
/// The store has no boolean kind (ruled 2026-08-05), so the tempting decode is a
/// string comparison — which silently treats every typo ("Open", "opne", "true")
/// as the other variant. On this surface that means a section a witness needs
/// quietly folding shut with nothing in the log. Parsing into a closed enum makes
/// an unrecognised token a named failure at boot, the same discipline
/// `BackgroundDefaultState` uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionState {
    Open,
    Collapsed,
}

impl SectionState {
    /// Whether a section holding this state renders open.
    pub fn is_open(self) -> bool {
        matches!(self, SectionState::Open)
    }

    /// Read one stored state token, naming the key that carried it.
    ///
    /// # Errors
    /// Returns [`SettingError::Unreadable`] naming the key and what was expected.
    pub fn parse(key: &str, token: &str) -> Result<Self, SettingError> {
        match token.trim() {
            "open" => Ok(SectionState::Open),
            "collapsed" => Ok(SectionState::Collapsed),
            other => Err(SettingError::Unreadable {
                key: key.to_string(),
                value: other.to_string(),
                expected: "either 'open' or 'collapsed'",
            }),
        }
    }
}
