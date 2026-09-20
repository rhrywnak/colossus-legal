//! The deck's REVIEW BAR — its count, its button, and the question it now asks.
//!
//! Eight fields, flattened into [`super::practice_wording::PracticeWordingDto`]
//! so the browser still receives ONE flat object and never has to know which
//! backend block a sentence lives in — the contract that module's header sets,
//! and the whole reason the mirror exists.
//!
//! ## Why a fourth file, and why NOW
//!
//! `practice_wording_review` was split off for this exact arithmetic a task ago,
//! and its header records the numbers: one field per stored key, and a mirror
//! pair that had four lines of headroom between them. CC_TASK_REVIEW_COUNTS_
//! HONEST_v1 adds FOUR — the confirmation's two sentences and its two button
//! labels — which is more headroom than either file had. Declared inline they
//! would have carried `practice_wording` to exactly Rule 17's 300 and left the
//! next task no room at all.
//!
//! So the bar's own strings move here as a group, and the four new ones join
//! them. The wire shape is byte-for-byte what it would have been.
//!
//! ## Why the bar, and not "the next four fields"
//!
//! A split made purely by line count puts a seam through the middle of a screen,
//! and the next reader has to hold two files to answer one question. These eight
//! are ONE control: the sentence that says how much waits, the button that
//! clears it, the question that button now asks first, and the line shown when
//! the write fails. `PracticeReviewBar.tsx` reads all eight and nothing else
//! reads any of them.
//!
//! ## Why `deck_review_oldest_template` is NOT here
//!
//! It is the ninth string that bar speaks, and it stays in
//! `practice_wording_review` where the previous split put it. Moving it would
//! buy one line and invalidate a page of accurate documentation about why it
//! sits there — and the mirror's tests already hold the only line that matters:
//! every wire name is checked against the union of the declared key lists, so
//! WHICH sibling declares a field is not a fact any test or any caller depends
//! on. The seam is arithmetic, and arithmetic does not need tidying twice.

use serde::{Deserialize, Serialize};

use crate::domain::wording_practice::PracticeWording;

/// The review bar's words, as the browser receives them.
// serde: allows unknown fields because this struct is `#[serde(flatten)]`'d into
// `PracticeWordingDto`, and serde documents `deny_unknown_fields` as
// incompatible with flatten on either side of the relationship. The same
// carve-out, for the same reason, that `practice_wording_review` records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PracticeDeckReviewWordingDto {
    pub deck_review_awaiting_template: String,
    pub deck_review_awaiting_one: String,
    pub deck_review_done_label: String,
    /// The confirmation's question — plural. See the domain field's note for why
    /// the press is worth asking about at all.
    pub deck_review_confirm_template: String,
    /// Its singular, read at exactly 1 by `pickByCount`.
    pub deck_review_confirm_one: String,
    /// The affirmative, and the only control that moves the mark.
    pub deck_review_confirm_yes_label: String,
    /// The retreat. Escape does the same.
    pub deck_review_confirm_cancel_label: String,
    pub deck_review_failed: String,
}

impl PracticeDeckReviewWordingDto {
    /// Flatten the bar's eight strings into the shape the page reads.
    ///
    /// Takes the whole [`PracticeWording`] rather than its `row` field alone,
    /// for the reason `PracticeReviewWordingDto::from_blocks` gives: a signature
    /// naming one block is a signature that grows loose `&str` parameters the
    /// day a ninth string is declared somewhere else, and a loose `&str` beside
    /// a struct is the shape a caller eventually passes the wrong string to.
    pub fn from_blocks(drill: &PracticeWording) -> Self {
        Self {
            deck_review_awaiting_template: drill.row.deck_review_awaiting_template.clone(),
            deck_review_awaiting_one: drill.row.deck_review_awaiting_one.clone(),
            deck_review_done_label: drill.row.deck_review_done_label.clone(),
            deck_review_confirm_template: drill.row.deck_review_confirm_template.clone(),
            deck_review_confirm_one: drill.row.deck_review_confirm_one.clone(),
            deck_review_confirm_yes_label: drill.row.deck_review_confirm_yes_label.clone(),
            deck_review_confirm_cancel_label: drill.row.deck_review_confirm_cancel_label.clone(),
            deck_review_failed: drill.row.deck_review_failed.clone(),
        }
    }
}
