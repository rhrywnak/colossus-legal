// =============================================================================
// backend/src/domain/wording_question_chat.rs — the question-anchor discussion panel
// =============================================================================
//
// CC_TASK_CHAT_ENGINE_v1, mockup of record DISCUSS_CHAT_MOCKUP_v2_2026-09-21 (+
// ADDENDUM_1's "Earlier team discussion"). The words of the side panel that opens
// beside a practice question: its switcher, header, chip, composer and failure lines.
//
// ## Why a nested block inside `PracticeWording`
//
// For the reason `wording_practice_discuss` gives: the panel is part of the
// question page and must render its button before any chat fetch, so its words
// ride the deck payload the page already has. And the frontend wording-reach test
// (`dto::practice_wording_reach_tests`) then covers every key the panel asks for.
//
// ## Generated, not hand-typed
//
// The KEY list, struct, builder and the test fixture were emitted from the
// migration's INSERT (`20260921140958_chat_discussions_and_grounded_flag.sql`),
// so the four copies of 42 strings cannot drift by a typo.

/// The words the discussion panel speaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionChatWording {
    /// The button beside Try again that opens the discussion panel.
    pub open_label: String,
    /// The line under the Discuss button.
    pub open_hint: String,
    /// The switcher button while your own thread is open.
    pub your_thread: String,
    /// The switcher button while someone else's thread is open. {name} is their name.
    pub thread_of_template: String,
    /// The switcher button's accessible name.
    pub switch_label: String,
    /// The header line on your own thread. {others} is everyone else on the case, joined in words.
    pub visibility_template: String,
    /// Appended to the header line when the thread began on an earlier day. {date} is that day.
    pub resumed_template: String,
    /// The header line and the composer's replacement on someone else's thread.
    pub readonly_line: String,
    /// The header line and the composer's replacement on the Earlier team discussion.
    pub earlier_readonly_line: String,
    /// The word that joins the last two names in {others}.
    pub and: String,
    /// The model chip. {model} is the model's short name. Grounded: every quoted passage is checked against the stored document.
    pub grounded_chip_template: String,
    /// The expand button's accessible name.
    pub expand_label: String,
    /// The collapse button's accessible name.
    pub collapse_label: String,
    /// The side panel's close (×) button: its accessible name and tooltip.
    ///
    /// Domain note: before v2.2.1 the side panel offered only Expand, and the
    /// only way to shut it was to expand it first and press Back — a panel you
    /// can open in one click and close in two reads as a panel you cannot close.
    pub close_label: String,
    /// The Back control on the full-screen strip.
    pub back_label: String,
    /// The Back control's accessible name.
    pub back_aria: String,
    /// The divider's tooltip.
    pub resize_label: String,
    /// On the full-screen strip, after the question. {answer} is her saved answer.
    pub strip_answer_template: String,
    /// The message box's placeholder.
    pub input_placeholder: String,
    /// The message box's accessible name.
    pub message_label: String,
    /// The send button.
    pub send_label: String,
    /// Your row in the switcher. {count} is its number of messages.
    pub switcher_own_template: String,
    /// Your row in the switcher at exactly one message.
    pub switcher_own_one: String,
    /// Your row in the switcher before you have written.
    pub switcher_own_empty: String,
    /// Someone else's row in the switcher.
    pub switcher_other_template: String,
    /// Someone else's row at exactly one message.
    pub switcher_other_one: String,
    /// Someone else's row before they have written.
    pub switcher_other_empty: String,
    /// The unread badge on a switcher row. {count} is how many messages you have not seen.
    pub unread_template: String,
    /// The small mark under a switcher row you cannot write in.
    pub readonly_mark: String,
    /// The switcher's footer — the visibility rule, said plainly.
    pub switcher_footer: String,
    /// The switcher row for the old shared thread (ADDENDUM_1).
    pub earlier_label: String,
    /// Under the Earlier team discussion row. {date} is its last message's day.
    pub earlier_meta_template: String,
    /// The same at exactly one message.
    pub earlier_meta_one: String,
    /// Shown in a thread before anyone has written.
    pub empty: String,
    /// Shown while a reply is on its way and nothing has arrived yet.
    pub waiting: String,
    /// Shown while the model is reading documents or answer history mid-reply.
    pub tool_line: String,
    /// Shown when the reply fails after your message was stored.
    pub send_failed: String,
    /// Shown when the browser hears nothing from the server for too long.
    pub stalled: String,
    /// Shown when the model refuses a message.
    pub refused: String,
    /// Shown when a reply is cut off at question_chat_max_tokens.
    pub truncated: String,
    /// Shown when the threads cannot be read.
    pub load_failed: String,
    /// Shown when opening a thread could not move your read mark (the unread badge would stay lit).
    pub read_mark_failed: String,
    /// Shown when question_chat_max_turns is reached; the message is not sent.
    pub cap_reached_template: String,
}

// KEYS: the stable identifiers. Renaming one is a migration.
pub(crate) const KEY_OPEN_LABEL: &str = "practice_chat_open_label";
pub(crate) const KEY_OPEN_HINT: &str = "practice_chat_open_hint";
pub(crate) const KEY_YOUR_THREAD: &str = "practice_chat_your_thread";
pub(crate) const KEY_THREAD_OF_TEMPLATE: &str = "practice_chat_thread_of_template";
pub(crate) const KEY_SWITCH_LABEL: &str = "practice_chat_switch_label";
pub(crate) const KEY_VISIBILITY_TEMPLATE: &str = "practice_chat_visibility_template";
pub(crate) const KEY_RESUMED_TEMPLATE: &str = "practice_chat_resumed_template";
pub(crate) const KEY_READONLY_LINE: &str = "practice_chat_readonly_line";
pub(crate) const KEY_EARLIER_READONLY_LINE: &str = "practice_chat_earlier_readonly_line";
pub(crate) const KEY_AND: &str = "practice_chat_and";
pub(crate) const KEY_GROUNDED_CHIP_TEMPLATE: &str = "practice_chat_grounded_chip_template";
pub(crate) const KEY_EXPAND_LABEL: &str = "practice_chat_expand_label";
pub(crate) const KEY_COLLAPSE_LABEL: &str = "practice_chat_collapse_label";
/// Added by CC_TASK_PRACTICE_FIXES_v2.2.1 — seeded by its own migration, not
/// the chat engine's.
pub(crate) const KEY_CLOSE_LABEL: &str = "practice_chat_close_label";
pub(crate) const KEY_BACK_LABEL: &str = "practice_chat_back_label";
pub(crate) const KEY_BACK_ARIA: &str = "practice_chat_back_aria";
pub(crate) const KEY_RESIZE_LABEL: &str = "practice_chat_resize_label";
pub(crate) const KEY_STRIP_ANSWER_TEMPLATE: &str = "practice_chat_strip_answer_template";
pub(crate) const KEY_INPUT_PLACEHOLDER: &str = "practice_chat_input_placeholder";
pub(crate) const KEY_MESSAGE_LABEL: &str = "practice_chat_message_label";
pub(crate) const KEY_SEND_LABEL: &str = "practice_chat_send_label";
pub(crate) const KEY_SWITCHER_OWN_TEMPLATE: &str = "practice_chat_switcher_own_template";
pub(crate) const KEY_SWITCHER_OWN_ONE: &str = "practice_chat_switcher_own_one";
pub(crate) const KEY_SWITCHER_OWN_EMPTY: &str = "practice_chat_switcher_own_empty";
pub(crate) const KEY_SWITCHER_OTHER_TEMPLATE: &str = "practice_chat_switcher_other_template";
pub(crate) const KEY_SWITCHER_OTHER_ONE: &str = "practice_chat_switcher_other_one";
pub(crate) const KEY_SWITCHER_OTHER_EMPTY: &str = "practice_chat_switcher_other_empty";
pub(crate) const KEY_UNREAD_TEMPLATE: &str = "practice_chat_unread_template";
pub(crate) const KEY_READONLY_MARK: &str = "practice_chat_readonly_mark";
pub(crate) const KEY_SWITCHER_FOOTER: &str = "practice_chat_switcher_footer";
pub(crate) const KEY_EARLIER_LABEL: &str = "practice_chat_earlier_label";
pub(crate) const KEY_EARLIER_META_TEMPLATE: &str = "practice_chat_earlier_meta_template";
pub(crate) const KEY_EARLIER_META_ONE: &str = "practice_chat_earlier_meta_one";
pub(crate) const KEY_EMPTY: &str = "practice_chat_empty";
pub(crate) const KEY_WAITING: &str = "practice_chat_waiting";
pub(crate) const KEY_TOOL_LINE: &str = "practice_chat_tool_line";
pub(crate) const KEY_SEND_FAILED: &str = "practice_chat_send_failed";
pub(crate) const KEY_STALLED: &str = "practice_chat_stalled";
pub(crate) const KEY_REFUSED: &str = "practice_chat_refused";
pub(crate) const KEY_TRUNCATED: &str = "practice_chat_truncated";
pub(crate) const KEY_LOAD_FAILED: &str = "practice_chat_load_failed";
pub(crate) const KEY_READ_MARK_FAILED: &str = "practice_chat_read_mark_failed";
pub(crate) const KEY_CAP_REACHED_TEMPLATE: &str = "practice_chat_cap_reached_template";

/// Every panel key this build reads, so a missing one is caught at boot BY NAME.
pub const QUESTION_CHAT_WORDING_KEYS: &[&str] = &[
    KEY_OPEN_LABEL,
    KEY_OPEN_HINT,
    KEY_YOUR_THREAD,
    KEY_THREAD_OF_TEMPLATE,
    KEY_SWITCH_LABEL,
    KEY_VISIBILITY_TEMPLATE,
    KEY_RESUMED_TEMPLATE,
    KEY_READONLY_LINE,
    KEY_EARLIER_READONLY_LINE,
    KEY_AND,
    KEY_GROUNDED_CHIP_TEMPLATE,
    KEY_EXPAND_LABEL,
    KEY_COLLAPSE_LABEL,
    KEY_CLOSE_LABEL,
    KEY_BACK_LABEL,
    KEY_BACK_ARIA,
    KEY_RESIZE_LABEL,
    KEY_STRIP_ANSWER_TEMPLATE,
    KEY_INPUT_PLACEHOLDER,
    KEY_MESSAGE_LABEL,
    KEY_SEND_LABEL,
    KEY_SWITCHER_OWN_TEMPLATE,
    KEY_SWITCHER_OWN_ONE,
    KEY_SWITCHER_OWN_EMPTY,
    KEY_SWITCHER_OTHER_TEMPLATE,
    KEY_SWITCHER_OTHER_ONE,
    KEY_SWITCHER_OTHER_EMPTY,
    KEY_UNREAD_TEMPLATE,
    KEY_READONLY_MARK,
    KEY_SWITCHER_FOOTER,
    KEY_EARLIER_LABEL,
    KEY_EARLIER_META_TEMPLATE,
    KEY_EARLIER_META_ONE,
    KEY_EMPTY,
    KEY_WAITING,
    KEY_TOOL_LINE,
    KEY_SEND_FAILED,
    KEY_STALLED,
    KEY_REFUSED,
    KEY_TRUNCATED,
    KEY_LOAD_FAILED,
    KEY_READ_MARK_FAILED,
    KEY_CAP_REACHED_TEMPLATE,
];

/// Build a [`QuestionChatWording`] from the stored rows, or say which key is wrong.
///
/// Called from `wording_practice::build_practice_wording` with the same reader.
///
/// # Errors
/// Whatever `read` returns for the first key that is missing, mis-kinded or blank.
pub fn build_question_chat_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<QuestionChatWording, E> {
    Ok(QuestionChatWording {
        open_label: read(KEY_OPEN_LABEL)?,
        open_hint: read(KEY_OPEN_HINT)?,
        your_thread: read(KEY_YOUR_THREAD)?,
        thread_of_template: read(KEY_THREAD_OF_TEMPLATE)?,
        switch_label: read(KEY_SWITCH_LABEL)?,
        visibility_template: read(KEY_VISIBILITY_TEMPLATE)?,
        resumed_template: read(KEY_RESUMED_TEMPLATE)?,
        readonly_line: read(KEY_READONLY_LINE)?,
        earlier_readonly_line: read(KEY_EARLIER_READONLY_LINE)?,
        and: read(KEY_AND)?,
        grounded_chip_template: read(KEY_GROUNDED_CHIP_TEMPLATE)?,
        expand_label: read(KEY_EXPAND_LABEL)?,
        collapse_label: read(KEY_COLLAPSE_LABEL)?,
        close_label: read(KEY_CLOSE_LABEL)?,
        back_label: read(KEY_BACK_LABEL)?,
        back_aria: read(KEY_BACK_ARIA)?,
        resize_label: read(KEY_RESIZE_LABEL)?,
        strip_answer_template: read(KEY_STRIP_ANSWER_TEMPLATE)?,
        input_placeholder: read(KEY_INPUT_PLACEHOLDER)?,
        message_label: read(KEY_MESSAGE_LABEL)?,
        send_label: read(KEY_SEND_LABEL)?,
        switcher_own_template: read(KEY_SWITCHER_OWN_TEMPLATE)?,
        switcher_own_one: read(KEY_SWITCHER_OWN_ONE)?,
        switcher_own_empty: read(KEY_SWITCHER_OWN_EMPTY)?,
        switcher_other_template: read(KEY_SWITCHER_OTHER_TEMPLATE)?,
        switcher_other_one: read(KEY_SWITCHER_OTHER_ONE)?,
        switcher_other_empty: read(KEY_SWITCHER_OTHER_EMPTY)?,
        unread_template: read(KEY_UNREAD_TEMPLATE)?,
        readonly_mark: read(KEY_READONLY_MARK)?,
        switcher_footer: read(KEY_SWITCHER_FOOTER)?,
        earlier_label: read(KEY_EARLIER_LABEL)?,
        earlier_meta_template: read(KEY_EARLIER_META_TEMPLATE)?,
        earlier_meta_one: read(KEY_EARLIER_META_ONE)?,
        empty: read(KEY_EMPTY)?,
        waiting: read(KEY_WAITING)?,
        tool_line: read(KEY_TOOL_LINE)?,
        send_failed: read(KEY_SEND_FAILED)?,
        stalled: read(KEY_STALLED)?,
        refused: read(KEY_REFUSED)?,
        truncated: read(KEY_TRUNCATED)?,
        load_failed: read(KEY_LOAD_FAILED)?,
        read_mark_failed: read(KEY_READ_MARK_FAILED)?,
        cap_reached_template: read(KEY_CAP_REACHED_TEMPLATE)?,
    })
}

#[cfg(test)]
#[path = "wording_question_chat_tests.rs"]
pub(crate) mod tests;
