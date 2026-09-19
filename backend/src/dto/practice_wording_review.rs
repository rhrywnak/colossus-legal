//! The Review answers page's half of the practice wording mirror.
//!
//! Ten fields, flattened into [`super::practice_wording::PracticeWordingDto`] so
//! the browser still receives ONE flat object and never has to know which
//! backend block a sentence lives in — the contract that module's header sets,
//! and the whole reason the mirror exists.
//!
//! ## Why a third file, when the mirror is deliberately ONE struct
//!
//! `practice_wording.rs` sat at 294 of Rule 17's 300 lines before this task and
//! `practice_wording_map.rs` at 292 — both one surface away from the limit, and
//! both by design one line per stored key. Ten more fields declared inline would
//! have carried the pair over. Declared here and flattened, they cost the
//! sibling three lines and the map two, and the wire shape is byte-for-byte what
//! it would have been.
//!
//! This is the same split `practice_wording_map` itself is — SHAPE from MAPPING
//! there, one page's words from the rest here — and it is not a general escape
//! valve: it works only because `#[serde(flatten)]` makes two structs serialize
//! as one object, which is true of nothing else in this directory.
//!
//! ## Why `deck_review_oldest_template` is here and not beside its four siblings
//!
//! Its stored key belongs to the ROW block, beside the other four strings the
//! review bar speaks, and that is where it is declared and boot-checked. Only
//! its WIRE field lives here, because this file's boundary is arithmetic (which
//! fields fit in which struct) and not a statement about which block owns what.
//! The mirror's own tests hold that line: every wire name is checked against the
//! union of the declared key lists, so a field here whose key is declared in the
//! row block passes, and a field whose key is declared nowhere fails.

use serde::{Deserialize, Serialize};

use crate::domain::wording_practice::PracticeWording;

/// The Review answers page's words, as the browser receives them.
// serde: allows unknown fields because this struct is `#[serde(flatten)]`'d into
// `PracticeWordingDto`, and serde documents `deny_unknown_fields` as
// incompatible with flatten on either side of the relationship. The same
// carve-out, for the same reason, that `dto::chronology` records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PracticeReviewWordingDto {
    pub review_title: String,
    pub review_unanswered: String,
    pub review_note_placeholder_answer: String,
    pub review_note_placeholder_question: String,
    pub review_answered_template: String,
    pub review_author_unknown: String,
    pub review_load_failed: String,
    pub review_empty_deck: String,
    pub review_name_joiner: String,
    /// The review bar's oldest-waiting clause. Stored key
    /// `practice_deck_review_oldest_template`, declared in the ROW block — see
    /// this module's header for why its wire field sits here.
    pub deck_review_oldest_template: String,
}

impl PracticeReviewWordingDto {
    /// Flatten the review block — and the row block's one bar clause — into the
    /// shape the page reads.
    ///
    /// Takes the whole [`PracticeWording`] rather than its `review` field alone
    /// precisely because of that one clause: a signature naming only the review
    /// block would have to take the tenth string as a loose `&str` parameter,
    /// and a loose `&str` beside a struct is the shape a caller eventually
    /// passes the wrong string to.
    pub fn from_blocks(drill: &PracticeWording) -> Self {
        Self {
            review_title: drill.review.title.clone(),
            review_unanswered: drill.review.unanswered.clone(),
            review_note_placeholder_answer: drill.review.note_placeholder_answer.clone(),
            review_note_placeholder_question: drill.review.note_placeholder_question.clone(),
            review_answered_template: drill.review.answered_template.clone(),
            review_author_unknown: drill.review.author_unknown.clone(),
            review_load_failed: drill.review.load_failed.clone(),
            review_empty_deck: drill.review.empty_deck.clone(),
            review_name_joiner: drill.review.name_joiner.clone(),
            deck_review_oldest_template: drill.row.deck_review_oldest_template.clone(),
        }
    }
}
