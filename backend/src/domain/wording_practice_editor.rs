// =============================================================================
// backend/src/domain/wording_practice_editor.rs — the words CHUCK reads
// =============================================================================
//
// Part B of CC_TASK_PRACTICE_V1_CHUCK_REVIEW_v1 (mockup v4, §B1–B2): the deck
// editor, the record it writes, and the box that tells Marie what changed since
// she was last here.
//
// ## Why a fifth practice block
//
// Rule 17 first — the four existing blocks are all near the 300-line limit. But
// the seam is the sharpest one in this surface: every other practice string is
// addressed to MARIE, alone, the night before she testifies. These are addressed
// to CHUCK and to Roman, editing a deck she will be asked from. Two audiences,
// two registers, and Chuck's Thursday critique will move these and nothing else.
//
// The two exceptions prove it rather than breaking it: the `changed_*` and
// `badge_*` rows ARE Marie's, and they are here because what they say is
// entirely determined by what the editor did. Splitting them off would put one
// half of a cause-and-effect pair in another file.
//
// ## Two of these are VOCABULARIES, not sentences
//
// `note_authors` and `editor_authors` are comma-separated lists of real people's
// names — case-specific data, which is exactly what Rule 2 keeps out of code.
// They are carried as the stored string rather than parsed into a `Vec` here so
// the wire mirror stays "one field per stored key, and every value a string",
// which two tests in `dto::practice_wording` pin. The readers split them.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeEditorWording {
    pub editor_switch_label: String,
    pub editor_done_label: String,
    pub editor_edit_label: String,
    pub editor_hide_label: String,
    /// The drag grip's hint, in edit mode. Both `title` and `aria-label`.
    pub editor_drag_hint: String,
    pub editor_unhide_label: String,
    pub editor_hidden_badge: String,
    pub editor_up_label: String,
    pub editor_down_label: String,
    pub editor_save_label: String,
    pub editor_cancel_label: String,
    pub editor_saved_hint_template: String,
    pub editor_field_question: String,
    pub editor_field_tactic: String,
    pub editor_field_follows: String,
    pub editor_field_watch_for: String,
    pub editor_field_stronger: String,
    pub editor_field_side: String,
    pub editor_field_attach: String,
    pub editor_side_cross: String,
    pub editor_side_direct: String,
    pub editor_side_redirect: String,
    pub editor_attach_none: String,
    pub editor_attach_instance_template: String,
    pub editor_attach_point_template: String,
    pub editor_add_label: String,
    pub editor_add_heading: String,
    pub editor_add_button: String,
    pub editor_add_hint: String,
    pub editor_question_placeholder: String,
    pub editor_failed: String,
    pub changed_heading_template: String,
    pub changed_notes_template: String,
    pub changed_summary: String,
    pub change_added_template: String,
    pub change_reworded_template: String,
    pub change_edited_template: String,
    pub change_moved_template: String,
    pub change_hidden_template: String,
    pub change_unhidden_template: String,
    pub badge_changed: String,
    pub badge_draft: String,
    pub sheet_changes_heading: String,
    pub sheet_change_item_template: String,
    /// The hint on every control the editor disables. Hard rule from this
    /// task: no control on a practice page may silently do nothing.
    pub editor_busy_hint: String,
    /// Asked when the editor is left with an inline edit still open. Saved
    /// changes are already written and are not at risk.
    pub editor_discard_confirm_template: String,
}

pub(crate) const KEY_EDITOR_SWITCH_LABEL: &str = "practice_editor_switch_label";
pub(crate) const KEY_EDITOR_DONE_LABEL: &str = "practice_editor_done_label";
pub(crate) const KEY_EDITOR_EDIT_LABEL: &str = "practice_editor_edit_label";
pub(crate) const KEY_EDITOR_HIDE_LABEL: &str = "practice_editor_hide_label";
pub(crate) const KEY_EDITOR_DRAG_HINT: &str = "practice_editor_drag_hint";
pub(crate) const KEY_EDITOR_UNHIDE_LABEL: &str = "practice_editor_unhide_label";
pub(crate) const KEY_EDITOR_HIDDEN_BADGE: &str = "practice_editor_hidden_badge";
pub(crate) const KEY_EDITOR_UP_LABEL: &str = "practice_editor_up_label";
pub(crate) const KEY_EDITOR_DOWN_LABEL: &str = "practice_editor_down_label";
pub(crate) const KEY_EDITOR_SAVE_LABEL: &str = "practice_editor_save_label";
pub(crate) const KEY_EDITOR_CANCEL_LABEL: &str = "practice_editor_cancel_label";
pub(crate) const KEY_EDITOR_SAVED_HINT_TEMPLATE: &str = "practice_editor_saved_hint_template";
pub(crate) const KEY_EDITOR_FIELD_QUESTION: &str = "practice_editor_field_question";
pub(crate) const KEY_EDITOR_FIELD_TACTIC: &str = "practice_editor_field_tactic";
pub(crate) const KEY_EDITOR_FIELD_FOLLOWS: &str = "practice_editor_field_follows";
pub(crate) const KEY_EDITOR_FIELD_WATCH_FOR: &str = "practice_editor_field_watch_for";
pub(crate) const KEY_EDITOR_FIELD_STRONGER: &str = "practice_editor_field_stronger";
pub(crate) const KEY_EDITOR_FIELD_SIDE: &str = "practice_editor_field_side";
pub(crate) const KEY_EDITOR_FIELD_ATTACH: &str = "practice_editor_field_attach";
pub(crate) const KEY_EDITOR_SIDE_CROSS: &str = "practice_editor_side_cross";
pub(crate) const KEY_EDITOR_SIDE_DIRECT: &str = "practice_editor_side_direct";
pub(crate) const KEY_EDITOR_SIDE_REDIRECT: &str = "practice_editor_side_redirect";
pub(crate) const KEY_EDITOR_ATTACH_NONE: &str = "practice_editor_attach_none";
pub(crate) const KEY_EDITOR_ATTACH_INSTANCE_TEMPLATE: &str =
    "practice_editor_attach_instance_template";
pub(crate) const KEY_EDITOR_ATTACH_POINT_TEMPLATE: &str = "practice_editor_attach_point_template";
pub(crate) const KEY_EDITOR_ADD_LABEL: &str = "practice_editor_add_label";
pub(crate) const KEY_EDITOR_ADD_HEADING: &str = "practice_editor_add_heading";
pub(crate) const KEY_EDITOR_ADD_BUTTON: &str = "practice_editor_add_button";
pub(crate) const KEY_EDITOR_ADD_HINT: &str = "practice_editor_add_hint";
pub(crate) const KEY_EDITOR_QUESTION_PLACEHOLDER: &str = "practice_editor_question_placeholder";
pub(crate) const KEY_EDITOR_FAILED: &str = "practice_editor_failed";
pub(crate) const KEY_CHANGED_HEADING_TEMPLATE: &str = "practice_changed_heading_template";
pub(crate) const KEY_CHANGED_NOTES_TEMPLATE: &str = "practice_changed_notes_template";
pub(crate) const KEY_CHANGED_SUMMARY: &str = "practice_changed_summary";
pub(crate) const KEY_CHANGE_ADDED_TEMPLATE: &str = "practice_change_added_template";
pub(crate) const KEY_CHANGE_REWORDED_TEMPLATE: &str = "practice_change_reworded_template";
pub(crate) const KEY_CHANGE_EDITED_TEMPLATE: &str = "practice_change_edited_template";
pub(crate) const KEY_CHANGE_MOVED_TEMPLATE: &str = "practice_change_moved_template";
pub(crate) const KEY_CHANGE_HIDDEN_TEMPLATE: &str = "practice_change_hidden_template";
pub(crate) const KEY_CHANGE_UNHIDDEN_TEMPLATE: &str = "practice_change_unhidden_template";
pub(crate) const KEY_BADGE_CHANGED: &str = "practice_badge_changed";
pub(crate) const KEY_BADGE_DRAFT: &str = "practice_badge_draft";
pub(crate) const KEY_SHEET_CHANGES_HEADING: &str = "practice_sheet_changes_heading";
pub(crate) const KEY_SHEET_CHANGE_ITEM_TEMPLATE: &str = "practice_sheet_change_item_template";

/// Every key in this block, so a missing one is caught at boot BY NAME rather
/// than as a blank control in front of the person editing.
pub(crate) const KEY_EDITOR_BUSY_HINT: &str = "practice_editor_busy_hint";

pub(crate) const KEY_EDITOR_DISCARD_CONFIRM_TEMPLATE: &str =
    "practice_editor_discard_confirm_template";

pub const PRACTICE_EDITOR_WORDING_KEYS: &[&str] = &[
    KEY_EDITOR_DISCARD_CONFIRM_TEMPLATE,
    KEY_EDITOR_BUSY_HINT,
    KEY_EDITOR_SWITCH_LABEL,
    KEY_EDITOR_DONE_LABEL,
    KEY_EDITOR_EDIT_LABEL,
    KEY_EDITOR_HIDE_LABEL,
    KEY_EDITOR_DRAG_HINT,
    KEY_EDITOR_UNHIDE_LABEL,
    KEY_EDITOR_HIDDEN_BADGE,
    KEY_EDITOR_UP_LABEL,
    KEY_EDITOR_DOWN_LABEL,
    KEY_EDITOR_SAVE_LABEL,
    KEY_EDITOR_CANCEL_LABEL,
    KEY_EDITOR_SAVED_HINT_TEMPLATE,
    KEY_EDITOR_FIELD_QUESTION,
    KEY_EDITOR_FIELD_TACTIC,
    KEY_EDITOR_FIELD_FOLLOWS,
    KEY_EDITOR_FIELD_WATCH_FOR,
    KEY_EDITOR_FIELD_STRONGER,
    KEY_EDITOR_FIELD_SIDE,
    KEY_EDITOR_FIELD_ATTACH,
    KEY_EDITOR_SIDE_CROSS,
    KEY_EDITOR_SIDE_DIRECT,
    KEY_EDITOR_SIDE_REDIRECT,
    KEY_EDITOR_ATTACH_NONE,
    KEY_EDITOR_ATTACH_INSTANCE_TEMPLATE,
    KEY_EDITOR_ATTACH_POINT_TEMPLATE,
    KEY_EDITOR_ADD_LABEL,
    KEY_EDITOR_ADD_HEADING,
    KEY_EDITOR_ADD_BUTTON,
    KEY_EDITOR_ADD_HINT,
    KEY_EDITOR_QUESTION_PLACEHOLDER,
    KEY_EDITOR_FAILED,
    KEY_CHANGED_HEADING_TEMPLATE,
    KEY_CHANGED_NOTES_TEMPLATE,
    KEY_CHANGED_SUMMARY,
    KEY_CHANGE_ADDED_TEMPLATE,
    KEY_CHANGE_REWORDED_TEMPLATE,
    KEY_CHANGE_EDITED_TEMPLATE,
    KEY_CHANGE_MOVED_TEMPLATE,
    KEY_CHANGE_HIDDEN_TEMPLATE,
    KEY_CHANGE_UNHIDDEN_TEMPLATE,
    KEY_BADGE_CHANGED,
    KEY_BADGE_DRAFT,
    KEY_SHEET_CHANGES_HEADING,
    KEY_SHEET_CHANGE_ITEM_TEMPLATE,
];

/// Build a [`PracticeEditorWording`] from the stored rows, or say which key is wrong.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_practice_editor_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<PracticeEditorWording, E> {
    Ok(PracticeEditorWording {
        editor_discard_confirm_template: read(KEY_EDITOR_DISCARD_CONFIRM_TEMPLATE)?,
        editor_busy_hint: read(KEY_EDITOR_BUSY_HINT)?,
        editor_switch_label: read(KEY_EDITOR_SWITCH_LABEL)?,
        editor_done_label: read(KEY_EDITOR_DONE_LABEL)?,
        editor_edit_label: read(KEY_EDITOR_EDIT_LABEL)?,
        editor_hide_label: read(KEY_EDITOR_HIDE_LABEL)?,
        editor_drag_hint: read(KEY_EDITOR_DRAG_HINT)?,
        editor_unhide_label: read(KEY_EDITOR_UNHIDE_LABEL)?,
        editor_hidden_badge: read(KEY_EDITOR_HIDDEN_BADGE)?,
        editor_up_label: read(KEY_EDITOR_UP_LABEL)?,
        editor_down_label: read(KEY_EDITOR_DOWN_LABEL)?,
        editor_save_label: read(KEY_EDITOR_SAVE_LABEL)?,
        editor_cancel_label: read(KEY_EDITOR_CANCEL_LABEL)?,
        editor_saved_hint_template: read(KEY_EDITOR_SAVED_HINT_TEMPLATE)?,
        editor_field_question: read(KEY_EDITOR_FIELD_QUESTION)?,
        editor_field_tactic: read(KEY_EDITOR_FIELD_TACTIC)?,
        editor_field_follows: read(KEY_EDITOR_FIELD_FOLLOWS)?,
        editor_field_watch_for: read(KEY_EDITOR_FIELD_WATCH_FOR)?,
        editor_field_stronger: read(KEY_EDITOR_FIELD_STRONGER)?,
        editor_field_side: read(KEY_EDITOR_FIELD_SIDE)?,
        editor_field_attach: read(KEY_EDITOR_FIELD_ATTACH)?,
        editor_side_cross: read(KEY_EDITOR_SIDE_CROSS)?,
        editor_side_direct: read(KEY_EDITOR_SIDE_DIRECT)?,
        editor_side_redirect: read(KEY_EDITOR_SIDE_REDIRECT)?,
        editor_attach_none: read(KEY_EDITOR_ATTACH_NONE)?,
        editor_attach_instance_template: read(KEY_EDITOR_ATTACH_INSTANCE_TEMPLATE)?,
        editor_attach_point_template: read(KEY_EDITOR_ATTACH_POINT_TEMPLATE)?,
        editor_add_label: read(KEY_EDITOR_ADD_LABEL)?,
        editor_add_heading: read(KEY_EDITOR_ADD_HEADING)?,
        editor_add_button: read(KEY_EDITOR_ADD_BUTTON)?,
        editor_add_hint: read(KEY_EDITOR_ADD_HINT)?,
        editor_question_placeholder: read(KEY_EDITOR_QUESTION_PLACEHOLDER)?,
        editor_failed: read(KEY_EDITOR_FAILED)?,
        changed_heading_template: read(KEY_CHANGED_HEADING_TEMPLATE)?,
        changed_notes_template: read(KEY_CHANGED_NOTES_TEMPLATE)?,
        changed_summary: read(KEY_CHANGED_SUMMARY)?,
        change_added_template: read(KEY_CHANGE_ADDED_TEMPLATE)?,
        change_reworded_template: read(KEY_CHANGE_REWORDED_TEMPLATE)?,
        change_edited_template: read(KEY_CHANGE_EDITED_TEMPLATE)?,
        change_moved_template: read(KEY_CHANGE_MOVED_TEMPLATE)?,
        change_hidden_template: read(KEY_CHANGE_HIDDEN_TEMPLATE)?,
        change_unhidden_template: read(KEY_CHANGE_UNHIDDEN_TEMPLATE)?,
        badge_changed: read(KEY_BADGE_CHANGED)?,
        badge_draft: read(KEY_BADGE_DRAFT)?,
        sheet_changes_heading: read(KEY_SHEET_CHANGES_HEADING)?,
        sheet_change_item_template: read(KEY_SHEET_CHANGE_ITEM_TEMPLATE)?,
    })
}

#[cfg(test)]
#[path = "wording_practice_editor_tests.rs"]
pub(crate) mod tests;
