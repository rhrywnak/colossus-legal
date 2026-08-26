//! Every string the case-timeline surfaces speak (CASE_CHRONOLOGY_DESIGN_v2, Phase B).
//!
//! A block of its own, for the reason each sibling has one: these are the words
//! ONE surface family speaks — the timeline list, the event page, and the home
//! band — and they move independently of every other screen's language.
//!
//! ## ⚑ THE THREE PARTIES, AND WHY BOTH HALVES ARE HERE
//!
//! A key is only real when all three agree: the DATABASE holds a row, this
//! module DECLARES the key, and the FRONTEND asks for it. Boot refuses if a
//! declared key has no row; `dto::chronology_wording_reach_tests` refuses if the
//! frontend asks for a name no field carries. That third edge is the .407 lesson
//! — seven rows seeded, declared in no block, and a page that rendered blank
//! against a database that was correct the whole time.
//!
//! ## ⚑ TWO STRINGS THIS BLOCK DELIBERATELY DOES NOT CARRY
//!
//! A loading line and a load-failure line. Both were drafted, and both were
//! withdrawn on the same reasoning: the wording store is DELIVERED BY the
//! request whose failure they would describe. A key that can only be read after
//! the read succeeded cannot speak about the read failing, and a key nothing can
//! reach is a row seeded, mirrored and paid for that no screen ever says.
//!
//! The bootstrap text lives in ONE named place in the frontend service instead,
//! marked as the exception it is. See the report's NEEDS A RULING.
//!
//! ## Why the glyphs are IN the stored strings
//!
//! `⚠`, `💬`, `⤢`, `⇲` and `◌` are part of the words, not decoration a component
//! adds. Putting them in the row means Roman can drop one from a label without a
//! rebuild, and it keeps the component free of any user-visible character at all
//! — which is the whole point of the no-wording-in-code law.

/// Every string the chronology surfaces speak.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChronologyWording {
    /// The page's own title.
    pub page_title: String,
    /// The unfiltered subtitle. `{events}` and `{phases}`.
    pub count_template: String,
    /// The FILTERED subtitle. `{phase}`, `{shown}`, `{total}`.
    ///
    /// Design R16: the subtitle is how a filtered state stays visible. A page
    /// that quietly showed six of twenty-two events with nothing saying so is
    /// the failure this template exists to prevent.
    pub filtered_count_template: String,

    /// The search box's placeholder.
    pub search_placeholder: String,
    /// The chip that clears the tag filter.
    pub all_tags_label: String,
    /// The date-range control's label.
    pub dates_label: String,
    /// The earliest-date field's label.
    pub date_from_label: String,
    /// The latest-date field's label.
    pub date_to_label: String,

    /// The always-visible control on every phase header (R16, R17).
    pub expand_label: String,
    /// How a reader leaves an expanded phase.
    pub show_all_phases_label: String,
    /// The line above a scroll window. `{count}`.
    pub scroll_hint_template: String,
    /// The phase header's right-hand meta line. `{range}` and `{count}`.
    pub phase_count_template: String,

    /// The amber mark on an event whose document is not in the system (R12).
    pub no_document_label: String,
    /// A link this build has no resolver for.
    ///
    /// ## Domain note: this is NOT "no document yet"
    ///
    /// `missing` means looked for and not there. `unchecked` means nobody
    /// looked, because this build cannot see that target's store. Rendering the
    /// second as the first would tell a reader a document is absent when the
    /// truth is that nothing checked — which is the claim the three-state
    /// resolution was introduced to stop making.
    pub link_unchecked_label: String,
    /// The note badge, plural. `{count}`.
    pub note_count_template: String,
    /// The note badge when there is exactly one.
    pub note_count_one: String,
    /// Shown beside a link that carries no pinpoint (R9) — the absence is the
    /// to-scan to-do list, so it is marked rather than left blank.
    pub no_pinpoint_label: String,

    /// The case genuinely holds no events.
    pub empty_label: String,
    /// The case holds events but the filters match none of them.
    ///
    /// A DIFFERENT string from `empty_label` on purpose: "there is nothing here"
    /// and "your filters hid everything" send a reader to two different places.
    pub no_matches_label: String,
    /// An event whose phase slug has no phase row. `{id}` and `{phase}`.
    ///
    /// ## Domain note: it renders, loudly, instead of vanishing
    ///
    /// The home band used to count such an event nowhere and show nothing —
    /// silently. An event nobody can see is an event nobody can fix.
    pub unknown_phase_template: String,

    /// The event page's breadcrumb back to the list.
    pub back_label: String,
    /// The event page's documents panel.
    pub documents_heading: String,
    /// The event page's notes panel.
    pub notes_heading: String,
    /// The event page's history panel.
    pub history_heading: String,
    /// Shown in the history panel when there is nothing in it.
    ///
    /// Rendered HONESTLY rather than hiding the panel: history is empty for
    /// every event until Phase C writes the first row, and a missing panel would
    /// read as a feature that does not exist rather than one with nothing in it.
    pub no_history_label: String,
    /// Shown in the notes panel when there is nothing in it.
    pub no_notes_label: String,

    /// The home band's count-mismatch marker. `{shown}` and `{total}`.
    ///
    /// Design B6: the band groups events by phase, so an event whose phase has
    /// no pill was previously counted nowhere and dropped without a word. This
    /// line is what it says instead.
    pub band_mismatch_template: String,

    // ─── Phase C: the words the WRITE controls speak ────────────────────────
    //
    // Every label, placeholder and button the add/edit form, the delete/undo
    // line, the note box and the document picker wear. They arrive in this
    // block rather than one of their own because they are the SAME surface
    // family's words — the list, the event page and the home band — and a
    // second block would mean a second payload field, a second reach scan and
    // a second place for a key to hide.
    /// The control that opens an empty form on the list page.
    pub add_event_label: String,
    /// The always-visible, muted edit control on a card and on the event page (R17).
    pub edit_label: String,
    /// The always-visible, muted delete control (R17). No confirm dialog follows it (R10).
    pub delete_label: String,
    /// The first half of the line that replaces a deleted card in place. The renderer supplies the joining space; the Undo control follows.
    pub deleted_line_label: String,
    /// The control that restores a soft-deleted event. This IS the safety R10 chose instead of a confirm dialog.
    pub undo_label: String,
    /// The form's heading when it is creating.
    pub form_add_title: String,
    /// The form's heading when it is editing. The same form, pre-filled.
    pub form_edit_title: String,
    /// The date field's label. Required by R11.
    pub form_date_label: String,
    /// How much of the date is actually known.
    pub form_precision_label: String,
    /// The `day` precision, as the form offers it.
    pub precision_day_label: String,
    /// The `month` precision, as the form offers it.
    pub precision_month_label: String,
    /// The `year` precision, as the form offers it.
    pub precision_year_label: String,
    /// The approximate flag, which is SEPARATE from precision.
    pub form_approximate_label: String,
    /// The title field's label. Required by R11.
    pub form_title_label: String,
    /// The title field's placeholder, which says what the field is FOR.
    pub form_title_placeholder: String,
    /// The fact field's label. Optional by R11, encouraged by the wording.
    pub form_fact_label: String,
    /// The fact field's placeholder — the standard the sentence is held to.
    pub form_fact_placeholder: String,
    /// The tag multi-select's label. The chips ARE the stored vocabulary.
    pub form_tags_label: String,
    /// The phase select's label. The options are the stored phases.
    pub form_phase_label: String,
    /// The document picker's label on the form.
    pub form_documents_label: String,
    /// The document picker's search box.
    pub document_search_placeholder: String,
    /// Shown when the picker's search matched nothing.
    pub document_search_empty_label: String,
    /// The pinpoint field's placeholder, which states the consequence of leaving it empty.
    pub pinpoint_placeholder: String,
    /// The form's commit control.
    pub save_label: String,
    /// Closes the form without writing.
    pub cancel_label: String,
    /// The label a write control wears while its request is in flight.
    pub saving_label: String,
    /// The note input on the event page (R8).
    pub add_note_placeholder: String,
    /// The note input's commit control.
    pub add_note_button_label: String,
    /// Opens the document picker on the event page.
    pub link_document_label: String,
    /// Removes one link, addressed by its natural key.
    pub remove_link_label: String,
    /// Deletes one note. The author may delete their own.
    pub delete_note_label: String,
    /// One history line. `{when}`, `{who}` and `{what}`.
    pub history_line_template: String,
    /// The display word for the stored `created` action.
    pub history_created_label: String,
    /// The display word for the stored `updated` action — deliberately a different word.
    pub history_updated_label: String,
    /// The display word for the stored `deleted` action.
    pub history_deleted_label: String,
    /// The display word for the stored `restored` action — what Undo lands.
    pub history_restored_label: String,
    /// The fallback for a stored action this build has no word for. `{action}`.
    pub history_unknown_template: String,
    /// Every failed write reaches this rendered sentence. `{reason}`.
    pub write_failed_template: String,
    /// The line under a capped document-picker list. `{shown}` and `{total}`.
    ///
    /// ## Domain note: the cap is never silent
    ///
    /// A short list that looked complete is how somebody links the wrong
    /// document with no idea a better match was cut off. The picker says how
    /// many it is showing of how many matched, so an author who cannot see what
    /// they wanted knows to narrow the search rather than concluding it is not
    /// there.
    pub picker_capped_template: String,
}

pub(crate) const KEY_PAGE_TITLE: &str = "chronology_page_title";
pub(crate) const KEY_COUNT_TEMPLATE: &str = "chronology_count_template";
pub(crate) const KEY_FILTERED_COUNT_TEMPLATE: &str = "chronology_filtered_count_template";
pub(crate) const KEY_SEARCH_PLACEHOLDER: &str = "chronology_search_placeholder";
pub(crate) const KEY_ALL_TAGS_LABEL: &str = "chronology_all_tags_label";
pub(crate) const KEY_DATES_LABEL: &str = "chronology_dates_label";
pub(crate) const KEY_DATE_FROM_LABEL: &str = "chronology_date_from_label";
pub(crate) const KEY_DATE_TO_LABEL: &str = "chronology_date_to_label";
pub(crate) const KEY_EXPAND_LABEL: &str = "chronology_expand_label";
pub(crate) const KEY_SHOW_ALL_PHASES_LABEL: &str = "chronology_show_all_phases_label";
pub(crate) const KEY_SCROLL_HINT_TEMPLATE: &str = "chronology_scroll_hint_template";
pub(crate) const KEY_PHASE_COUNT_TEMPLATE: &str = "chronology_phase_count_template";
pub(crate) const KEY_NO_DOCUMENT_LABEL: &str = "chronology_no_document_label";
pub(crate) const KEY_LINK_UNCHECKED_LABEL: &str = "chronology_link_unchecked_label";
pub(crate) const KEY_NOTE_COUNT_TEMPLATE: &str = "chronology_note_count_template";
pub(crate) const KEY_NOTE_COUNT_ONE: &str = "chronology_note_count_one";
pub(crate) const KEY_NO_PINPOINT_LABEL: &str = "chronology_no_pinpoint_label";
pub(crate) const KEY_EMPTY_LABEL: &str = "chronology_empty_label";
pub(crate) const KEY_NO_MATCHES_LABEL: &str = "chronology_no_matches_label";
pub(crate) const KEY_UNKNOWN_PHASE_TEMPLATE: &str = "chronology_unknown_phase_template";
pub(crate) const KEY_BACK_LABEL: &str = "chronology_back_label";
pub(crate) const KEY_DOCUMENTS_HEADING: &str = "chronology_documents_heading";
pub(crate) const KEY_NOTES_HEADING: &str = "chronology_notes_heading";
pub(crate) const KEY_HISTORY_HEADING: &str = "chronology_history_heading";
pub(crate) const KEY_NO_HISTORY_LABEL: &str = "chronology_no_history_label";
pub(crate) const KEY_NO_NOTES_LABEL: &str = "chronology_no_notes_label";
pub(crate) const KEY_BAND_MISMATCH_TEMPLATE: &str = "chronology_band_mismatch_template";
pub(crate) const KEY_ADD_EVENT_LABEL: &str = "chronology_add_event_label";
pub(crate) const KEY_EDIT_LABEL: &str = "chronology_edit_label";
pub(crate) const KEY_DELETE_LABEL: &str = "chronology_delete_label";
pub(crate) const KEY_DELETED_LINE_LABEL: &str = "chronology_deleted_line_label";
pub(crate) const KEY_UNDO_LABEL: &str = "chronology_undo_label";
pub(crate) const KEY_FORM_ADD_TITLE: &str = "chronology_form_add_title";
pub(crate) const KEY_FORM_EDIT_TITLE: &str = "chronology_form_edit_title";
pub(crate) const KEY_FORM_DATE_LABEL: &str = "chronology_form_date_label";
pub(crate) const KEY_FORM_PRECISION_LABEL: &str = "chronology_form_precision_label";
pub(crate) const KEY_PRECISION_DAY_LABEL: &str = "chronology_precision_day_label";
pub(crate) const KEY_PRECISION_MONTH_LABEL: &str = "chronology_precision_month_label";
pub(crate) const KEY_PRECISION_YEAR_LABEL: &str = "chronology_precision_year_label";
pub(crate) const KEY_FORM_APPROXIMATE_LABEL: &str = "chronology_form_approximate_label";
pub(crate) const KEY_FORM_TITLE_LABEL: &str = "chronology_form_title_label";
pub(crate) const KEY_FORM_TITLE_PLACEHOLDER: &str = "chronology_form_title_placeholder";
pub(crate) const KEY_FORM_FACT_LABEL: &str = "chronology_form_fact_label";
pub(crate) const KEY_FORM_FACT_PLACEHOLDER: &str = "chronology_form_fact_placeholder";
pub(crate) const KEY_FORM_TAGS_LABEL: &str = "chronology_form_tags_label";
pub(crate) const KEY_FORM_PHASE_LABEL: &str = "chronology_form_phase_label";
pub(crate) const KEY_FORM_DOCUMENTS_LABEL: &str = "chronology_form_documents_label";
pub(crate) const KEY_DOCUMENT_SEARCH_PLACEHOLDER: &str = "chronology_document_search_placeholder";
pub(crate) const KEY_DOCUMENT_SEARCH_EMPTY_LABEL: &str = "chronology_document_search_empty_label";
pub(crate) const KEY_PINPOINT_PLACEHOLDER: &str = "chronology_pinpoint_placeholder";
pub(crate) const KEY_SAVE_LABEL: &str = "chronology_save_label";
pub(crate) const KEY_CANCEL_LABEL: &str = "chronology_cancel_label";
pub(crate) const KEY_SAVING_LABEL: &str = "chronology_saving_label";
pub(crate) const KEY_ADD_NOTE_PLACEHOLDER: &str = "chronology_add_note_placeholder";
pub(crate) const KEY_ADD_NOTE_BUTTON_LABEL: &str = "chronology_add_note_button_label";
pub(crate) const KEY_LINK_DOCUMENT_LABEL: &str = "chronology_link_document_label";
pub(crate) const KEY_REMOVE_LINK_LABEL: &str = "chronology_remove_link_label";
pub(crate) const KEY_DELETE_NOTE_LABEL: &str = "chronology_delete_note_label";
pub(crate) const KEY_HISTORY_LINE_TEMPLATE: &str = "chronology_history_line_template";
pub(crate) const KEY_HISTORY_CREATED_LABEL: &str = "chronology_history_created_label";
pub(crate) const KEY_HISTORY_UPDATED_LABEL: &str = "chronology_history_updated_label";
pub(crate) const KEY_HISTORY_DELETED_LABEL: &str = "chronology_history_deleted_label";
pub(crate) const KEY_HISTORY_RESTORED_LABEL: &str = "chronology_history_restored_label";
pub(crate) const KEY_HISTORY_UNKNOWN_TEMPLATE: &str = "chronology_history_unknown_template";
pub(crate) const KEY_WRITE_FAILED_TEMPLATE: &str = "chronology_write_failed_template";
pub(crate) const KEY_PICKER_CAPPED_TEMPLATE: &str = "chronology_picker_capped_template";

/// Declared to the boot loader. A key here with no row in any migration makes
/// the backend REFUSE TO START — which is what the sibling test file exists to
/// catch before a deploy does.
pub const CHRONOLOGY_WORDING_KEYS: &[&str] = &[
    KEY_PAGE_TITLE,
    KEY_COUNT_TEMPLATE,
    KEY_FILTERED_COUNT_TEMPLATE,
    KEY_SEARCH_PLACEHOLDER,
    KEY_ALL_TAGS_LABEL,
    KEY_DATES_LABEL,
    KEY_DATE_FROM_LABEL,
    KEY_DATE_TO_LABEL,
    KEY_EXPAND_LABEL,
    KEY_SHOW_ALL_PHASES_LABEL,
    KEY_SCROLL_HINT_TEMPLATE,
    KEY_PHASE_COUNT_TEMPLATE,
    KEY_NO_DOCUMENT_LABEL,
    KEY_LINK_UNCHECKED_LABEL,
    KEY_NOTE_COUNT_TEMPLATE,
    KEY_NOTE_COUNT_ONE,
    KEY_NO_PINPOINT_LABEL,
    KEY_EMPTY_LABEL,
    KEY_NO_MATCHES_LABEL,
    KEY_UNKNOWN_PHASE_TEMPLATE,
    KEY_BACK_LABEL,
    KEY_DOCUMENTS_HEADING,
    KEY_NOTES_HEADING,
    KEY_HISTORY_HEADING,
    KEY_NO_HISTORY_LABEL,
    KEY_NO_NOTES_LABEL,
    KEY_BAND_MISMATCH_TEMPLATE,
    KEY_ADD_EVENT_LABEL,
    KEY_EDIT_LABEL,
    KEY_DELETE_LABEL,
    KEY_DELETED_LINE_LABEL,
    KEY_UNDO_LABEL,
    KEY_FORM_ADD_TITLE,
    KEY_FORM_EDIT_TITLE,
    KEY_FORM_DATE_LABEL,
    KEY_FORM_PRECISION_LABEL,
    KEY_PRECISION_DAY_LABEL,
    KEY_PRECISION_MONTH_LABEL,
    KEY_PRECISION_YEAR_LABEL,
    KEY_FORM_APPROXIMATE_LABEL,
    KEY_FORM_TITLE_LABEL,
    KEY_FORM_TITLE_PLACEHOLDER,
    KEY_FORM_FACT_LABEL,
    KEY_FORM_FACT_PLACEHOLDER,
    KEY_FORM_TAGS_LABEL,
    KEY_FORM_PHASE_LABEL,
    KEY_FORM_DOCUMENTS_LABEL,
    KEY_DOCUMENT_SEARCH_PLACEHOLDER,
    KEY_DOCUMENT_SEARCH_EMPTY_LABEL,
    KEY_PINPOINT_PLACEHOLDER,
    KEY_SAVE_LABEL,
    KEY_CANCEL_LABEL,
    KEY_SAVING_LABEL,
    KEY_ADD_NOTE_PLACEHOLDER,
    KEY_ADD_NOTE_BUTTON_LABEL,
    KEY_LINK_DOCUMENT_LABEL,
    KEY_REMOVE_LINK_LABEL,
    KEY_DELETE_NOTE_LABEL,
    KEY_HISTORY_LINE_TEMPLATE,
    KEY_HISTORY_CREATED_LABEL,
    KEY_HISTORY_UPDATED_LABEL,
    KEY_HISTORY_DELETED_LABEL,
    KEY_HISTORY_RESTORED_LABEL,
    KEY_HISTORY_UNKNOWN_TEMPLATE,
    KEY_WRITE_FAILED_TEMPLATE,
    KEY_PICKER_CAPPED_TEMPLATE,
];

/// Build a [`ChronologyWording`] from the stored rows, or say which key is wrong.
///
/// ## Rust Learning: one closure, every block, the same rule
///
/// `read` is taken by value as an `impl Fn`, matching the sibling blocks. The
/// caller in `settings_store` owns it; every block is judged by the identical
/// rule, so a key that is missing, blank or of the wrong declared kind fails the
/// same way here as anywhere else.
///
/// # Errors
/// Whatever `read` returns for the first key that is missing, blank, or of the
/// wrong declared kind.
pub fn build_chronology_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<ChronologyWording, E> {
    Ok(ChronologyWording {
        page_title: read(KEY_PAGE_TITLE)?,
        count_template: read(KEY_COUNT_TEMPLATE)?,
        filtered_count_template: read(KEY_FILTERED_COUNT_TEMPLATE)?,
        search_placeholder: read(KEY_SEARCH_PLACEHOLDER)?,
        all_tags_label: read(KEY_ALL_TAGS_LABEL)?,
        dates_label: read(KEY_DATES_LABEL)?,
        date_from_label: read(KEY_DATE_FROM_LABEL)?,
        date_to_label: read(KEY_DATE_TO_LABEL)?,
        expand_label: read(KEY_EXPAND_LABEL)?,
        show_all_phases_label: read(KEY_SHOW_ALL_PHASES_LABEL)?,
        scroll_hint_template: read(KEY_SCROLL_HINT_TEMPLATE)?,
        phase_count_template: read(KEY_PHASE_COUNT_TEMPLATE)?,
        no_document_label: read(KEY_NO_DOCUMENT_LABEL)?,
        link_unchecked_label: read(KEY_LINK_UNCHECKED_LABEL)?,
        note_count_template: read(KEY_NOTE_COUNT_TEMPLATE)?,
        note_count_one: read(KEY_NOTE_COUNT_ONE)?,
        no_pinpoint_label: read(KEY_NO_PINPOINT_LABEL)?,
        empty_label: read(KEY_EMPTY_LABEL)?,
        no_matches_label: read(KEY_NO_MATCHES_LABEL)?,
        unknown_phase_template: read(KEY_UNKNOWN_PHASE_TEMPLATE)?,
        back_label: read(KEY_BACK_LABEL)?,
        documents_heading: read(KEY_DOCUMENTS_HEADING)?,
        notes_heading: read(KEY_NOTES_HEADING)?,
        history_heading: read(KEY_HISTORY_HEADING)?,
        no_history_label: read(KEY_NO_HISTORY_LABEL)?,
        no_notes_label: read(KEY_NO_NOTES_LABEL)?,
        band_mismatch_template: read(KEY_BAND_MISMATCH_TEMPLATE)?,
        add_event_label: read(KEY_ADD_EVENT_LABEL)?,
        edit_label: read(KEY_EDIT_LABEL)?,
        delete_label: read(KEY_DELETE_LABEL)?,
        deleted_line_label: read(KEY_DELETED_LINE_LABEL)?,
        undo_label: read(KEY_UNDO_LABEL)?,
        form_add_title: read(KEY_FORM_ADD_TITLE)?,
        form_edit_title: read(KEY_FORM_EDIT_TITLE)?,
        form_date_label: read(KEY_FORM_DATE_LABEL)?,
        form_precision_label: read(KEY_FORM_PRECISION_LABEL)?,
        precision_day_label: read(KEY_PRECISION_DAY_LABEL)?,
        precision_month_label: read(KEY_PRECISION_MONTH_LABEL)?,
        precision_year_label: read(KEY_PRECISION_YEAR_LABEL)?,
        form_approximate_label: read(KEY_FORM_APPROXIMATE_LABEL)?,
        form_title_label: read(KEY_FORM_TITLE_LABEL)?,
        form_title_placeholder: read(KEY_FORM_TITLE_PLACEHOLDER)?,
        form_fact_label: read(KEY_FORM_FACT_LABEL)?,
        form_fact_placeholder: read(KEY_FORM_FACT_PLACEHOLDER)?,
        form_tags_label: read(KEY_FORM_TAGS_LABEL)?,
        form_phase_label: read(KEY_FORM_PHASE_LABEL)?,
        form_documents_label: read(KEY_FORM_DOCUMENTS_LABEL)?,
        document_search_placeholder: read(KEY_DOCUMENT_SEARCH_PLACEHOLDER)?,
        document_search_empty_label: read(KEY_DOCUMENT_SEARCH_EMPTY_LABEL)?,
        pinpoint_placeholder: read(KEY_PINPOINT_PLACEHOLDER)?,
        save_label: read(KEY_SAVE_LABEL)?,
        cancel_label: read(KEY_CANCEL_LABEL)?,
        saving_label: read(KEY_SAVING_LABEL)?,
        add_note_placeholder: read(KEY_ADD_NOTE_PLACEHOLDER)?,
        add_note_button_label: read(KEY_ADD_NOTE_BUTTON_LABEL)?,
        link_document_label: read(KEY_LINK_DOCUMENT_LABEL)?,
        remove_link_label: read(KEY_REMOVE_LINK_LABEL)?,
        delete_note_label: read(KEY_DELETE_NOTE_LABEL)?,
        history_line_template: read(KEY_HISTORY_LINE_TEMPLATE)?,
        history_created_label: read(KEY_HISTORY_CREATED_LABEL)?,
        history_updated_label: read(KEY_HISTORY_UPDATED_LABEL)?,
        history_deleted_label: read(KEY_HISTORY_DELETED_LABEL)?,
        history_restored_label: read(KEY_HISTORY_RESTORED_LABEL)?,
        history_unknown_template: read(KEY_HISTORY_UNKNOWN_TEMPLATE)?,
        write_failed_template: read(KEY_WRITE_FAILED_TEMPLATE)?,
        picker_capped_template: read(KEY_PICKER_CAPPED_TEMPLATE)?,
    })
}

#[cfg(test)]
#[path = "wording_chronology_tests.rs"]
mod tests;
