//! The one-page question list — the practice bar, and the footnote under it.
//!
//! The seventh practice wording block, nested under
//! [`super::wording_practice::PracticeWording`] beside `flow`, `row`, `editor`
//! and `print`. Its own file for the reason each of those has one: a block
//! belongs beside the surface that reads it.
//!
//! ## Why a NEW block rather than four more keys in `flow`
//!
//! `wording_practice_flow` is the SITTING's block — Start, Resume, Skip today,
//! the end-of-sitting sheet — and CC_TASK_PRACTICE_ONE_PAGE retires the sitting
//! from the interface. Filing the words of the surface that REPLACES it inside
//! the block it replaces would leave the next reader unable to tell which of the
//! two a given key belongs to, at exactly the moment that distinction decides
//! whether a key is live or dead.
//!
//! ## ⚑ The dropdown's two options are NOT here
//!
//! "The defense asks" and "Chuck asks" are `practice_who_george_title` and
//! `practice_who_chuck_title`, which already hold those exact words for the
//! side cards. The cards go; the strings stay where they are. A second pair
//! would be two places to edit and one of them eventually not edited — and the
//! two would disagree on the one screen that shows the choice.
//!
//! ## Domain note: practice mode writes NOTHING
//!
//! Two of these four strings describe a walk that makes no model call and no
//! database write of any kind. It offers only questions Marie has already
//! answered, because practising an answer she has not written is nothing to
//! practise. If a future change makes that surface write, these words stop being
//! true and are the first thing to fix.

/// Every string the one-page list speaks that no older block already holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeListWording {
    /// The label at the left of the practice bar.
    pub practice_mode_label: String,
    /// The button that begins a practice walk.
    pub start_practising_label: String,
    /// The one line of hint beside it.
    ///
    /// Standing rule of 2026-08-19: no control on a practice page is dim and
    /// silent. A person must be able to tell what a control does before pressing
    /// it — and "Start practising" beside a Start button that used to open a
    /// sitting is exactly the case where a reader needs telling.
    pub practice_hint: String,
    /// The small line under the question list.
    ///
    /// It exists because the row's marks were REMOVED. A reader who remembers
    /// `answered today · repeat · attempt 2` needs to be told once that their
    /// absence is not a fault in the page.
    pub status_footnote: String,

    /// Shown under a critique that is ONE SENTENCE rather than three parts.
    ///
    /// Measured 2026-08-23: 12 of 14 stored answers carry only a composed
    /// sentence, so this is the common rendering and not a rare one. Without
    /// the line, one answer showing three parts and the next showing one
    /// sentence reads as breakage. It points at the fix — pressing Answer on
    /// unchanged text re-runs the read and attaches the parts to the same row.
    pub read_plain_hint: String,
}

pub(crate) const KEY_PRACTICE_MODE_LABEL: &str = "practice_practice_mode_label";
pub(crate) const KEY_START_PRACTISING_LABEL: &str = "practice_start_practising_label";
pub(crate) const KEY_PRACTICE_HINT: &str = "practice_practice_hint";
pub(crate) const KEY_STATUS_FOOTNOTE: &str = "practice_deck_status_footnote";
pub(crate) const KEY_READ_PLAIN_HINT: &str = "practice_read_plain_hint";

/// Declared to the boot loader. A key here with no row in any migration makes
/// the backend REFUSE TO START — which is what the sibling test file exists to
/// catch before a deploy does.
pub(crate) const PRACTICE_LIST_WORDING_KEYS: &[&str] = &[
    KEY_PRACTICE_MODE_LABEL,
    KEY_START_PRACTISING_LABEL,
    KEY_PRACTICE_HINT,
    KEY_STATUS_FOOTNOTE,
    KEY_READ_PLAIN_HINT,
];

/// Build a [`PracticeListWording`] from the stored rows, or say which key is
/// wrong.
///
/// ## Rust Learning: taking the reader by reference
///
/// The caller is [`super::wording_practice::build_practice_wording`], which owns
/// its own `read` closure and still needs it afterwards. `&F` implements `Fn`
/// whenever `F` does, so one closure serves every nested block without being
/// cloned — and every block is judged by exactly the same rule.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_practice_list_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<PracticeListWording, E> {
    Ok(PracticeListWording {
        practice_mode_label: read(KEY_PRACTICE_MODE_LABEL)?,
        start_practising_label: read(KEY_START_PRACTISING_LABEL)?,
        practice_hint: read(KEY_PRACTICE_HINT)?,
        status_footnote: read(KEY_STATUS_FOOTNOTE)?,
        read_plain_hint: read(KEY_READ_PLAIN_HINT)?,
    })
}

#[cfg(test)]
#[path = "wording_practice_list_tests.rs"]
mod tests;
