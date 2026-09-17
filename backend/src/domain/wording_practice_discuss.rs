// =============================================================================
// backend/src/domain/wording_practice_discuss.rs — the "Discuss with AI" dock
// =============================================================================
//
// CC_TASK_QUESTION_CHAT_v1, ruled mockup QUESTION_CHAT_DOCK_RULED_2026-09-17. The
// words of the dock that opens beside a practice question: its button, title,
// context line, input, footer and failure lines.
//
// ## Why a nested block
//
// Rule 17, and a real seam: these speak about ONE conversation on one question,
// not about the deck or a sitting. Nested inside `PracticeWording` like
// `wording_practice_row`, so they ride the deck payload the question page already
// has — the button renders before the dock ever fetches.
//
// ## Domain note: no model name here, ever
//
// The button reads "Discuss with AI" (ruled amendment 1). Which model answers is
// the `practice_discuss_default_model` settings row and the picker; the chip and
// `{model}` print that model's display name (ruled amendment 2).

/// The words the "Discuss with AI" dock speaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeDiscussWording {
    /// The button that opens the dock. Static words, never a model name (ruled amendment 1).
    pub button_label: String,
    /// The dock title; `{n}` is the question's place in the deck.
    pub title_template: String,
    /// The line under the title; `{code}` is the scenario code.
    pub subtitle_template: String,
    /// The same line while her unsaved draft goes with the message.
    pub subtitle_draft_template: String,
    /// What the model is given with every message.
    pub context_line: String,
    /// The same, while her unsaved draft is what goes.
    pub context_line_draft: String,
    /// The message box's placeholder.
    pub input_placeholder: String,
    /// The send button.
    pub send_label: String,
    /// The ✕ button's accessible name.
    pub close_label: String,
    /// The model picker's accessible name.
    pub model_label: String,
    /// The footer; `{cost}` is the billing-class word, `{model}` the display name.
    pub footer_template: String,
    /// `{cost}` for a billed (API) model.
    pub cost_billed: String,
    /// `{cost}` for a local model.
    pub cost_local: String,
    /// Under a model reply: `{input}` in · `{output}` out · `{seconds}` s — what
    /// that call cost, so spend is observable without SQL.
    pub cost_template: String,
    /// The thread before anyone has written.
    pub empty: String,
    /// While a reply is on its way.
    pub sending_template: String,
    /// When the call fails after the message was stored.
    pub send_failed: String,
    /// When the thread cannot be read.
    pub load_failed: String,
    /// The 409 when the reply cap is reached.
    pub cap_reached_template: String,
    /// Its singular, when the cap is 1.
    pub cap_reached_one: String,
}

// KEYS: the stable identifiers. Renaming one is a migration.
pub(crate) const KEY_BUTTON_LABEL: &str = "practice_discuss_button_label";
pub(crate) const KEY_TITLE_TEMPLATE: &str = "practice_discuss_title_template";
pub(crate) const KEY_SUBTITLE_TEMPLATE: &str = "practice_discuss_subtitle_template";
pub(crate) const KEY_SUBTITLE_DRAFT_TEMPLATE: &str = "practice_discuss_subtitle_draft_template";
pub(crate) const KEY_CONTEXT_LINE: &str = "practice_discuss_context_line";
pub(crate) const KEY_CONTEXT_LINE_DRAFT: &str = "practice_discuss_context_line_draft";
pub(crate) const KEY_INPUT_PLACEHOLDER: &str = "practice_discuss_input_placeholder";
pub(crate) const KEY_SEND_LABEL: &str = "practice_discuss_send_label";
pub(crate) const KEY_CLOSE_LABEL: &str = "practice_discuss_close_label";
pub(crate) const KEY_MODEL_LABEL: &str = "practice_discuss_model_label";
pub(crate) const KEY_FOOTER_TEMPLATE: &str = "practice_discuss_footer_template";
pub(crate) const KEY_COST_BILLED: &str = "practice_discuss_cost_billed";
pub(crate) const KEY_COST_LOCAL: &str = "practice_discuss_cost_local";
pub(crate) const KEY_COST_TEMPLATE: &str = "practice_discuss_cost_template";
pub(crate) const KEY_EMPTY: &str = "practice_discuss_empty";
pub(crate) const KEY_SENDING_TEMPLATE: &str = "practice_discuss_sending_template";
pub(crate) const KEY_SEND_FAILED: &str = "practice_discuss_send_failed";
pub(crate) const KEY_LOAD_FAILED: &str = "practice_discuss_load_failed";
pub(crate) const KEY_CAP_REACHED_TEMPLATE: &str = "practice_discuss_cap_reached_template";
pub(crate) const KEY_CAP_REACHED_ONE: &str = "practice_discuss_cap_reached_one";

/// Every dock key this build reads, so a missing one is caught at boot BY NAME.
pub const PRACTICE_DISCUSS_WORDING_KEYS: &[&str] = &[
    KEY_BUTTON_LABEL,
    KEY_TITLE_TEMPLATE,
    KEY_SUBTITLE_TEMPLATE,
    KEY_SUBTITLE_DRAFT_TEMPLATE,
    KEY_CONTEXT_LINE,
    KEY_CONTEXT_LINE_DRAFT,
    KEY_INPUT_PLACEHOLDER,
    KEY_SEND_LABEL,
    KEY_CLOSE_LABEL,
    KEY_MODEL_LABEL,
    KEY_FOOTER_TEMPLATE,
    KEY_COST_BILLED,
    KEY_COST_LOCAL,
    KEY_COST_TEMPLATE,
    KEY_EMPTY,
    KEY_SENDING_TEMPLATE,
    KEY_SEND_FAILED,
    KEY_LOAD_FAILED,
    KEY_CAP_REACHED_TEMPLATE,
    KEY_CAP_REACHED_ONE,
];

/// Build a [`PracticeDiscussWording`] from the stored rows, or say which key is wrong.
///
/// Called from `wording_practice::build_practice_wording` with the same reader —
/// see [`crate::domain::wording_practice_row::build_practice_row_wording`] for why
/// the closure is taken by reference.
///
/// # Errors
/// Whatever `read` returns for the first key that is missing, mis-kinded or blank.
pub fn build_practice_discuss_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<PracticeDiscussWording, E> {
    Ok(PracticeDiscussWording {
        button_label: read(KEY_BUTTON_LABEL)?,
        title_template: read(KEY_TITLE_TEMPLATE)?,
        subtitle_template: read(KEY_SUBTITLE_TEMPLATE)?,
        subtitle_draft_template: read(KEY_SUBTITLE_DRAFT_TEMPLATE)?,
        context_line: read(KEY_CONTEXT_LINE)?,
        context_line_draft: read(KEY_CONTEXT_LINE_DRAFT)?,
        input_placeholder: read(KEY_INPUT_PLACEHOLDER)?,
        send_label: read(KEY_SEND_LABEL)?,
        close_label: read(KEY_CLOSE_LABEL)?,
        model_label: read(KEY_MODEL_LABEL)?,
        footer_template: read(KEY_FOOTER_TEMPLATE)?,
        cost_billed: read(KEY_COST_BILLED)?,
        cost_local: read(KEY_COST_LOCAL)?,
        cost_template: read(KEY_COST_TEMPLATE)?,
        empty: read(KEY_EMPTY)?,
        sending_template: read(KEY_SENDING_TEMPLATE)?,
        send_failed: read(KEY_SEND_FAILED)?,
        load_failed: read(KEY_LOAD_FAILED)?,
        cap_reached_template: read(KEY_CAP_REACHED_TEMPLATE)?,
        cap_reached_one: read(KEY_CAP_REACHED_ONE)?,
    })
}

#[cfg(test)]
#[path = "wording_practice_discuss_tests.rs"]
pub(crate) mod tests;
