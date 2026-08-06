// =============================================================================
// backend/src/domain/wording_authoring.rs — the two authoring sections' words
// =============================================================================
//
// Task 2.11 C, Phase B, ruling C4b. The twenty-three strings the talking-points
// and watch-list sections speak ON THE SCENARIO WORKING PAGE.
//
// ## Why these moved, and why now
//
// `TalkingPointsSection.tsx` carried ten user-facing literals and
// `WatchListBlock.tsx` carried nine more. That was tolerable while both were
// scenario-page components. Task 2.11 C makes them SHARED with the rehearsal
// page, whose standing law is that every visible word is a stored row — and a
// component holding a literal cannot be reused on a surface that forbids one.
// So the literals became rows rather than the law becoming a suggestion.
//
// ## Why the rehearsal page does NOT read these rows
//
// It has its own (`rehearsal_add_point_label`, `rehearsal_add_watch_label`,
// `rehearsal_points_authoring_note`, `rehearsal_point_no_exhibit_notice`),
// approved 2026-08-06. Two rows saying "+ Add talking point" looks like
// duplication until you read the pair either side of it: this page says "+ Add
// watch-list note" where the witness surface says "+ Add watch item", because
// "list" is curation vocabulary §10 keeps off a rehearsal page. The surfaces
// genuinely speak differently, and one row serving both would force one voice on
// two audiences. Same reasoning `wording_accusation` and `wording_rehearsal`
// already stand on.
//
// The COMPONENTS take their words as a prop and hold none. That is what makes
// one component serve two vocabularies without forking.

/// Every stored string the two shared authoring sections render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoringWording {
    // ── Marie's talking points ──────────────────────────────────────────────
    pub points_section_heading: String,
    /// Carries `{cap}` — the stored `talking_points_cap`.
    pub points_section_meta_template: String,
    /// Shown when nobody has written one. Names the absence AND what the block
    /// is for: the person reading it is the person who fills it.
    pub points_empty_notice: String,
    pub points_no_exhibit_notice: String,
    pub points_add_label: String,
    pub points_edit_label: String,
    pub points_save_label: String,
    pub points_saving_label: String,
    pub points_cancel_label: String,
    /// Carries `{cap}`. Explains the DISABLED add control — a control that
    /// refuses without saying why reads as a broken one.
    pub points_cap_reached_notice: String,
    /// Carries `{n}`. Names each editing box for a screen reader.
    pub points_field_label_template: String,
    pub points_authoring_note: String,
    pub points_save_failed_notice: String,

    // ── The watch-list ──────────────────────────────────────────────────────
    pub watch_section_heading: String,
    pub watch_section_meta: String,
    pub watch_field_label: String,
    pub watch_add_label: String,
    pub watch_save_label: String,
    pub watch_edit_label: String,
    pub watch_cancel_label: String,
    pub watch_remove_label: String,
    /// Follows the authorship tag on an item whose text has changed since it was
    /// written. The provenance stays honest through an edit.
    pub watch_edited_suffix: String,
    pub watch_save_failed_notice: String,
}

// KEYS: the stable identifiers of the twenty-three stored strings. Renaming one
// is a migration, and until it runs the boot loader refuses to start.
pub(crate) const KEY_POINTS_SECTION_HEADING: &str = "talking_points_section_heading";
pub const KEY_POINTS_SECTION_META: &str = "talking_points_section_meta_template";
pub(crate) const KEY_POINTS_EMPTY: &str = "talking_points_empty_notice";
pub(crate) const KEY_POINTS_NO_EXHIBIT: &str = "talking_points_no_exhibit_notice";
pub(crate) const KEY_POINTS_ADD: &str = "talking_points_add_label";
pub(crate) const KEY_POINTS_EDIT: &str = "talking_points_edit_label";
pub(crate) const KEY_POINTS_SAVE: &str = "talking_points_save_label";
pub(crate) const KEY_POINTS_SAVING: &str = "talking_points_saving_label";
pub(crate) const KEY_POINTS_CANCEL: &str = "talking_points_cancel_label";
pub const KEY_POINTS_CAP_REACHED: &str = "talking_points_cap_reached_notice";
pub const KEY_POINTS_FIELD_LABEL: &str = "talking_points_field_label_template";
pub(crate) const KEY_POINTS_AUTHORING_NOTE: &str = "talking_points_authoring_note";
pub(crate) const KEY_POINTS_SAVE_FAILED: &str = "talking_points_save_failed_notice";
pub(crate) const KEY_WATCH_SECTION_HEADING: &str = "watch_list_section_heading";
pub(crate) const KEY_WATCH_SECTION_META: &str = "watch_list_section_meta";
pub(crate) const KEY_WATCH_FIELD_LABEL: &str = "watch_list_field_label";
pub(crate) const KEY_WATCH_ADD: &str = "watch_list_add_label";
pub(crate) const KEY_WATCH_SAVE: &str = "watch_list_save_label";
pub(crate) const KEY_WATCH_EDIT: &str = "watch_list_edit_label";
pub(crate) const KEY_WATCH_CANCEL: &str = "watch_list_cancel_label";
pub(crate) const KEY_WATCH_REMOVE: &str = "watch_list_remove_label";
pub(crate) const KEY_WATCH_EDITED_SUFFIX: &str = "watch_list_edited_suffix";
pub(crate) const KEY_WATCH_SAVE_FAILED: &str = "watch_list_save_failed_notice";

/// Every authoring key this build reads, so a missing one is caught at boot BY
/// NAME rather than as a blank button in front of a human mid-sentence.
pub const AUTHORING_WORDING_KEYS: &[&str] = &[
    KEY_POINTS_SECTION_HEADING,
    KEY_POINTS_SECTION_META,
    KEY_POINTS_EMPTY,
    KEY_POINTS_NO_EXHIBIT,
    KEY_POINTS_ADD,
    KEY_POINTS_EDIT,
    KEY_POINTS_SAVE,
    KEY_POINTS_SAVING,
    KEY_POINTS_CANCEL,
    KEY_POINTS_CAP_REACHED,
    KEY_POINTS_FIELD_LABEL,
    KEY_POINTS_AUTHORING_NOTE,
    KEY_POINTS_SAVE_FAILED,
    KEY_WATCH_SECTION_HEADING,
    KEY_WATCH_SECTION_META,
    KEY_WATCH_FIELD_LABEL,
    KEY_WATCH_ADD,
    KEY_WATCH_SAVE,
    KEY_WATCH_EDIT,
    KEY_WATCH_CANCEL,
    KEY_WATCH_REMOVE,
    KEY_WATCH_EDITED_SUFFIX,
    KEY_WATCH_SAVE_FAILED,
];

/// Build an [`AuthoringWording`] from the stored rows, or say which key is wrong.
///
/// Same shape and same reasoning as its four siblings — see
/// `wording_rehearsal_chrome::build_rehearsal_chrome_wording` for the note on
/// taking a closure that can fail.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_authoring_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<AuthoringWording, E> {
    Ok(AuthoringWording {
        points_section_heading: read(KEY_POINTS_SECTION_HEADING)?,
        points_section_meta_template: read(KEY_POINTS_SECTION_META)?,
        points_empty_notice: read(KEY_POINTS_EMPTY)?,
        points_no_exhibit_notice: read(KEY_POINTS_NO_EXHIBIT)?,
        points_add_label: read(KEY_POINTS_ADD)?,
        points_edit_label: read(KEY_POINTS_EDIT)?,
        points_save_label: read(KEY_POINTS_SAVE)?,
        points_saving_label: read(KEY_POINTS_SAVING)?,
        points_cancel_label: read(KEY_POINTS_CANCEL)?,
        points_cap_reached_notice: read(KEY_POINTS_CAP_REACHED)?,
        points_field_label_template: read(KEY_POINTS_FIELD_LABEL)?,
        points_authoring_note: read(KEY_POINTS_AUTHORING_NOTE)?,
        points_save_failed_notice: read(KEY_POINTS_SAVE_FAILED)?,
        watch_section_heading: read(KEY_WATCH_SECTION_HEADING)?,
        watch_section_meta: read(KEY_WATCH_SECTION_META)?,
        watch_field_label: read(KEY_WATCH_FIELD_LABEL)?,
        watch_add_label: read(KEY_WATCH_ADD)?,
        watch_save_label: read(KEY_WATCH_SAVE)?,
        watch_edit_label: read(KEY_WATCH_EDIT)?,
        watch_cancel_label: read(KEY_WATCH_CANCEL)?,
        watch_remove_label: read(KEY_WATCH_REMOVE)?,
        watch_edited_suffix: read(KEY_WATCH_EDITED_SUFFIX)?,
        watch_save_failed_notice: read(KEY_WATCH_SAVE_FAILED)?,
    })
}

#[cfg(test)]
#[path = "wording_authoring_tests.rs"]
pub(crate) mod tests;
