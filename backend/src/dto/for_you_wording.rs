//! The wire mirror of the "For you" page's wording block.
//!
//! Same argument as every sibling mirror in this directory: the domain layer
//! does not derive serde, so a change to how a value is STORED cannot silently
//! change the API, and vice versa.
//!
//! ## Why a mirror of its own, and not a field on the practice mirror
//!
//! `practice_wording.rs` is at 299 of 300 lines (Rule 17), and the page is not
//! a practice surface: it is one person's inbox across every deck in the case.
//! Its own mirror also means its own reach test — `for_you_wording_reach_tests`
//! beside this file — rather than widening the practice scanner to a second
//! vocabulary it would then check only loosely.
//!
//! ## The field names are the keys, minus the block's prefix
//!
//! `for_you_group_today` is stored, `group_today` is served, and the browser
//! asks `w("group_today")`. The same convention the practice mirror follows.

use serde::{Deserialize, Serialize};

use crate::domain::wording_for_you::ForYouWording;

/// The page's words, as the browser receives them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForYouWordingDto {
    pub title: String,
    pub subtitle_witness: String,
    pub subtitle_reviewer: String,
    pub not_your_list: String,
    pub tab_unread_template: String,
    pub tab_everything_template: String,
    pub group_today: String,
    pub group_yesterday: String,
    pub group_earlier: String,
    pub deck_line_template: String,
    pub deck_line_no_question_template: String,
    pub body_answer_template: String,
    pub body_change_template: String,
    pub byline_template: String,
    pub byline_note_on_answer_witness: String,
    pub byline_note_on_answer_reviewer: String,
    pub byline_note_on_question: String,
    pub byline_answer: String,
    pub byline_change: String,
    pub byline_read_suffix_template: String,
    pub empty_title: String,
    pub empty_hint_template: String,
    pub empty_last_template: String,
    pub witness_name: String,
    pub unknown_author: String,
}

impl ForYouWordingDto {
    /// Copy the stored block onto the wire.
    ///
    /// ## Rust Learning: `From` would have been the idiom, and is not used here
    ///
    /// A `From<&ForYouWording>` impl reads better at the call site. It is not
    /// used because every sibling mirror in this directory spells the
    /// conversion out as a named constructor, and one file doing it differently
    /// is a question the next reader has to answer before they can trust either
    /// pattern. Consistency wins over idiom inside one directory.
    pub fn from_block(w: &ForYouWording) -> Self {
        Self {
            title: w.title.clone(),
            subtitle_witness: w.subtitle_witness.clone(),
            subtitle_reviewer: w.subtitle_reviewer.clone(),
            not_your_list: w.not_your_list.clone(),
            tab_unread_template: w.tab_unread_template.clone(),
            tab_everything_template: w.tab_everything_template.clone(),
            group_today: w.group_today.clone(),
            group_yesterday: w.group_yesterday.clone(),
            group_earlier: w.group_earlier.clone(),
            deck_line_template: w.deck_line_template.clone(),
            deck_line_no_question_template: w.deck_line_no_question_template.clone(),
            body_answer_template: w.body_answer_template.clone(),
            body_change_template: w.body_change_template.clone(),
            byline_template: w.byline_template.clone(),
            byline_note_on_answer_witness: w.byline_note_on_answer_witness.clone(),
            byline_note_on_answer_reviewer: w.byline_note_on_answer_reviewer.clone(),
            byline_note_on_question: w.byline_note_on_question.clone(),
            byline_answer: w.byline_answer.clone(),
            byline_change: w.byline_change.clone(),
            byline_read_suffix_template: w.byline_read_suffix_template.clone(),
            empty_title: w.empty_title.clone(),
            empty_hint_template: w.empty_hint_template.clone(),
            empty_last_template: w.empty_last_template.clone(),
            witness_name: w.witness_name.clone(),
            unknown_author: w.unknown_author.clone(),
        }
    }
}

#[cfg(test)]
#[path = "for_you_wording_reach_tests.rs"]
mod reach_tests;
