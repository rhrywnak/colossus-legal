//! The answer box's saved-state words, and the read's one failure line
//! (CC_TASK_PRACTICE_FIXES_v2.2.1).
//!
//! Flattened into [`super::practice_wording::PracticeWordingDto`] like
//! `practice_wording_deck_review` and `practice_wording_chat`: the browser
//! receives ONE flat object, and the wire name of each string is its stored key
//! minus the `practice_` prefix. Its own file because `practice_wording` sits at
//! 297 of Rule 17's 300 lines, and three more fields declared inline would take
//! it over.
//!
//! ## Why these three ride the wire at all
//!
//! None of them is read by a `w("…")` in the browser: the saved label and the
//! failure line are composed server-side, and the analysis-off line arrives on
//! the answer response. They are here for the contract the mirror's test
//! enforces — every declared practice key is on the wire — which is the same
//! reason `row_answered_on_template`, another server-composed template, is.

use serde::{Deserialize, Serialize};

use crate::domain::wording_practice::PracticeWording;
use crate::domain::wording_practice_report::PracticeReportWording;

/// The answer box's words on the wire.
// serde: allows unknown fields because this struct is `#[serde(flatten)]`'d into
// `PracticeWordingDto`, and serde documents `deny_unknown_fields` as incompatible
// with flatten — the carve-out `practice_wording_deck_review` records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PracticeAnswerBoxWordingDto {
    /// `Your answer — saved {when}`, filled server-side.
    pub row_answer_saved_template: String,
    /// The one line under the buttons after an analysis-off press.
    pub row_answer_saved_off_line: String,
    /// The ONE line a system-failed read shows, composed into `read_text`.
    pub read_failed_line: String,
}

impl PracticeAnswerBoxWordingDto {
    /// Flatten the three strings into the shape the page reads.
    ///
    /// Takes both blocks for the reason `PracticeDeckReviewWordingDto::from_blocks`
    /// gives: a signature naming whole blocks never grows loose `&str` parameters.
    pub fn from_blocks(drill: &PracticeWording, report: &PracticeReportWording) -> Self {
        Self {
            row_answer_saved_template: drill.row.answer_saved_template.clone(),
            row_answer_saved_off_line: drill.row.answer_saved_off_line.clone(),
            read_failed_line: report.read_failed_line.clone(),
        }
    }
}
