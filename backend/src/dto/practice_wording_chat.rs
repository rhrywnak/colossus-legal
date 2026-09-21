//! The discussion panel's words, as the browser receives them.
//!
//! Flattened into [`super::practice_wording::PracticeWordingDto`] like
//! `practice_wording_deck_review`: the browser receives ONE flat object, and the
//! wire name of each string is its stored key minus the `practice_` prefix. Its
//! own file so `practice_wording` stays under Rule 17's 300 lines.
//! Generated from the migration, like the domain block it mirrors.

use serde::{Deserialize, Serialize};

use crate::domain::wording_practice::PracticeWording;

/// The panel's words on the wire.
// serde: allows unknown fields because this struct is `#[serde(flatten)]`'d into
// `PracticeWordingDto`, and serde documents `deny_unknown_fields` as incompatible
// with flatten — the carve-out `practice_wording_deck_review` records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PracticeChatWordingDto {
    pub chat_open_label: String,
    pub chat_open_hint: String,
    pub chat_your_thread: String,
    pub chat_thread_of_template: String,
    pub chat_switch_label: String,
    pub chat_visibility_template: String,
    pub chat_resumed_template: String,
    pub chat_readonly_line: String,
    pub chat_earlier_readonly_line: String,
    pub chat_and: String,
    pub chat_grounded_chip_template: String,
    pub chat_expand_label: String,
    pub chat_collapse_label: String,
    pub chat_back_label: String,
    pub chat_back_aria: String,
    pub chat_resize_label: String,
    pub chat_strip_answer_template: String,
    pub chat_input_placeholder: String,
    pub chat_message_label: String,
    pub chat_send_label: String,
    pub chat_switcher_own_template: String,
    pub chat_switcher_own_one: String,
    pub chat_switcher_own_empty: String,
    pub chat_switcher_other_template: String,
    pub chat_switcher_other_one: String,
    pub chat_switcher_other_empty: String,
    pub chat_unread_template: String,
    pub chat_readonly_mark: String,
    pub chat_switcher_footer: String,
    pub chat_earlier_label: String,
    pub chat_earlier_meta_template: String,
    pub chat_earlier_meta_one: String,
    pub chat_empty: String,
    pub chat_waiting: String,
    pub chat_tool_line: String,
    pub chat_send_failed: String,
    pub chat_stalled: String,
    pub chat_refused: String,
    pub chat_truncated: String,
    pub chat_load_failed: String,
    pub chat_cap_reached_template: String,
}

impl PracticeChatWordingDto {
    /// Flatten the panel's strings into the shape the page reads.
    pub fn from_blocks(drill: &PracticeWording) -> Self {
        let c = &drill.chat;
        Self {
            chat_open_label: c.open_label.clone(),
            chat_open_hint: c.open_hint.clone(),
            chat_your_thread: c.your_thread.clone(),
            chat_thread_of_template: c.thread_of_template.clone(),
            chat_switch_label: c.switch_label.clone(),
            chat_visibility_template: c.visibility_template.clone(),
            chat_resumed_template: c.resumed_template.clone(),
            chat_readonly_line: c.readonly_line.clone(),
            chat_earlier_readonly_line: c.earlier_readonly_line.clone(),
            chat_and: c.and.clone(),
            chat_grounded_chip_template: c.grounded_chip_template.clone(),
            chat_expand_label: c.expand_label.clone(),
            chat_collapse_label: c.collapse_label.clone(),
            chat_back_label: c.back_label.clone(),
            chat_back_aria: c.back_aria.clone(),
            chat_resize_label: c.resize_label.clone(),
            chat_strip_answer_template: c.strip_answer_template.clone(),
            chat_input_placeholder: c.input_placeholder.clone(),
            chat_message_label: c.message_label.clone(),
            chat_send_label: c.send_label.clone(),
            chat_switcher_own_template: c.switcher_own_template.clone(),
            chat_switcher_own_one: c.switcher_own_one.clone(),
            chat_switcher_own_empty: c.switcher_own_empty.clone(),
            chat_switcher_other_template: c.switcher_other_template.clone(),
            chat_switcher_other_one: c.switcher_other_one.clone(),
            chat_switcher_other_empty: c.switcher_other_empty.clone(),
            chat_unread_template: c.unread_template.clone(),
            chat_readonly_mark: c.readonly_mark.clone(),
            chat_switcher_footer: c.switcher_footer.clone(),
            chat_earlier_label: c.earlier_label.clone(),
            chat_earlier_meta_template: c.earlier_meta_template.clone(),
            chat_earlier_meta_one: c.earlier_meta_one.clone(),
            chat_empty: c.empty.clone(),
            chat_waiting: c.waiting.clone(),
            chat_tool_line: c.tool_line.clone(),
            chat_send_failed: c.send_failed.clone(),
            chat_stalled: c.stalled.clone(),
            chat_refused: c.refused.clone(),
            chat_truncated: c.truncated.clone(),
            chat_load_failed: c.load_failed.clone(),
            chat_cap_reached_template: c.cap_reached_template.clone(),
        }
    }
}
