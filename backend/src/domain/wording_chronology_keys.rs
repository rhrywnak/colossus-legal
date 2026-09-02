//! Every stored KEY the chronology block reads, and the list the boot loader
//! is handed.
//!
//! ## Why the keys live beside the block and not inside it
//!
//! `wording_chronology.rs` holds a struct of 82 fields and a builder that fills
//! all 82 — 166 code lines that cannot be split, because a struct literal has to
//! be written in one place. The 82 `KEY_*` constants and the list that names
//! them are another 166, and together they were over Rule 17's 300-line limit
//! the moment the subsets block was declared (T1.2).
//!
//! So the file split along the only seam it has. This half is a lookup table:
//! the join keys between Rust field names and `app_settings` rows. The other
//! half is the shape those rows are read into. Neither can drift from the other
//! without failing `wording_chronology_tests`, which reads the migration off
//! disk and pins every key to the value it seeds.
//!
//! ## ⚑ THE NAMING RULE, ENFORCED BY A TEST
//!
//! Every stored key is `chronology_` + the wire field name, with no exceptions —
//! `dto::chronology_wording`'s `every_wire_key_is_the_stored_key_without_its_prefix`
//! fails otherwise. That is why the three SCENARIO-surface keys added by T1.2
//! are `chronology_scenario_…` and not `scenario_…`: they are spoken on the
//! scenario pages, but they are this block's words and they ride this block's
//! payload.

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

// ─── Timeline subsets (T1.2) ─────────────────────────────────────────────────
pub(crate) const KEY_SUBSETS_SECTION_TITLE: &str = "chronology_subsets_section_title";
pub(crate) const KEY_SUBSETS_SECTION_SUBTITLE: &str = "chronology_subsets_section_subtitle";
pub(crate) const KEY_SUBSETS_ADD_BUTTON: &str = "chronology_subsets_add_button";
pub(crate) const KEY_SUBSETS_CARRIED_BY_PREFIX: &str = "chronology_subsets_carried_by_prefix";
pub(crate) const KEY_SUBSETS_GAP_COUNT_TEMPLATE: &str = "chronology_subsets_gap_count_template";
pub(crate) const KEY_SUBSETS_REMOVED_EVENT_LINE: &str = "chronology_subsets_removed_event_line";
pub(crate) const KEY_SUBSETS_SIZE_LINE_TEMPLATE: &str = "chronology_subsets_size_line_template";
pub(crate) const KEY_SUBSETS_PICKER_HINT: &str = "chronology_subsets_picker_hint";
pub(crate) const KEY_SUBSETS_PICKER_GAP_HINT: &str = "chronology_subsets_picker_gap_hint";
pub(crate) const KEY_SCENARIO_VIEW_TIMELINE_BUTTON: &str =
    "chronology_scenario_view_timeline_button";
pub(crate) const KEY_SUBSETS_WINDOW_OPEN_TIMELINE: &str = "chronology_subsets_window_open_timeline";
pub(crate) const KEY_SUBSETS_WINDOW_EDIT: &str = "chronology_subsets_window_edit";
pub(crate) const KEY_SUBSETS_WINDOW_FOOTER_EVENTS_TEMPLATE: &str =
    "chronology_subsets_window_footer_events_template";
pub(crate) const KEY_SUBSETS_EMPTY_STATE: &str = "chronology_subsets_empty_state";

// Timeline subsets, task 2 (2026-08-30): the seven Screens 2 and 3 needed and
// T1.2 did not seed. See the migration header for why a declared-ahead block
// came up short.
pub(crate) const KEY_SUBSETS_EVENT_COUNT_TEMPLATE: &str = "chronology_subsets_event_count_template";
pub(crate) const KEY_SUBSETS_FORM_ADD_TITLE: &str = "chronology_subsets_form_add_title";
pub(crate) const KEY_SUBSETS_PICKED_COUNT_TEMPLATE: &str =
    "chronology_subsets_picked_count_template";
pub(crate) const KEY_SUBSETS_PILL_GAPS_TEMPLATE: &str = "chronology_subsets_pill_gaps_template";
pub(crate) const KEY_SUBSETS_FORM_NAME_LABEL: &str = "chronology_subsets_form_name_label";
pub(crate) const KEY_SUBSETS_FORM_DESCRIPTION_LABEL: &str =
    "chronology_subsets_form_description_label";
pub(crate) const KEY_SUBSETS_NOTE_PLACEHOLDER: &str = "chronology_subsets_note_placeholder";

// Timeline subsets, task 3 (2026-08-30): the two aria rows task 2 left as a
// recorded gap, and the four the floating window speaks.
pub(crate) const KEY_SUBSETS_MOVE_EARLIER_LABEL: &str = "chronology_subsets_move_earlier_label";
pub(crate) const KEY_SUBSETS_MOVE_LATER_LABEL: &str = "chronology_subsets_move_later_label";
pub(crate) const KEY_SUBSETS_WINDOW_MINIMIZE_LABEL: &str =
    "chronology_subsets_window_minimize_label";
pub(crate) const KEY_SUBSETS_WINDOW_CLOSE_LABEL: &str = "chronology_subsets_window_close_label";
pub(crate) const KEY_SUBSETS_WINDOW_EVENTS_COUNT_TEMPLATE: &str =
    "chronology_subsets_window_events_count_template";
pub(crate) const KEY_SUBSETS_GAP_BADGE_LABEL: &str = "chronology_subsets_gap_badge_label";
pub(crate) const KEY_SUBSETS_WINDOW_LOADING_LABEL: &str = "chronology_subsets_window_loading_label";

// Timeline subsets, task 4 (2026-08-31): the redrawn row and Pop out. Two aria
// rows for glyphs that say nothing on their own, the two precision captions
// under an approximate date, and the cross-phase divider.
//
// ⚑ KEY_SUBSETS_DATE_TO_CONFIRM_BADGE was here — "the badge on an unsettled
// date" — and is RETIRED (Roman's ruling, 2026-08-31, reversing his own T4
// call). It could only read `approximate`, so it claimed four of the case's
// thirty-one events needed a date confirmed. The two precision captions below
// SURVIVE it: they say the source stated a month or a year, which is true.
pub(crate) const KEY_SUBSETS_WINDOW_POPOUT_LABEL: &str = "chronology_subsets_window_popout_label";
pub(crate) const KEY_SUBSETS_WINDOW_POPIN_LABEL: &str = "chronology_subsets_window_popin_label";
pub(crate) const KEY_SUBSETS_PRECISION_MONTH_LABEL: &str =
    "chronology_subsets_precision_month_label";
pub(crate) const KEY_SUBSETS_PRECISION_YEAR_LABEL: &str = "chronology_subsets_precision_year_label";
pub(crate) const KEY_SUBSETS_YEAR_PHASE_DIVIDER_TEMPLATE: &str =
    "chronology_subsets_year_phase_divider_template";

// Timeline subsets, task 6 (2026-08-31): the Edit modal's honest banner and its
// drag handle. The two banner halves exist because the old one said "That change
// was not saved" when half of it had been.
pub(crate) const KEY_SUBSETS_SAVED_NAME_ONLY_BANNER: &str =
    "chronology_subsets_saved_name_only_banner";
pub(crate) const KEY_SUBSETS_EVENTS_NOT_SAVED_BANNER_TEMPLATE: &str =
    "chronology_subsets_events_not_saved_banner_template";
pub(crate) const KEY_SUBSETS_MODAL_DRAG_LABEL: &str = "chronology_subsets_modal_drag_label";

// Timeline subsets, polish (2026-09-02): the third state of the window's body.
// Loading and failed already had words; an EMPTY story rendered a blank band,
// which a reader cannot tell from a window that did not load.
pub(crate) const KEY_SUBSETS_WINDOW_NO_EVENTS: &str = "chronology_subsets_window_no_events";
// Timeline subsets, compact view (2026-09-02): the footer toggle that strips
// every row to its date and title, and the same button's other half.
//
// ⚑ A PAIR. The control is a toggle, so each label names what pressing it DOES
// rather than the state it is in — the same rule the ⧉ / ⇲ pair follows. Edit
// the two together, or the button changes vocabulary halfway through.
pub(crate) const KEY_SUBSETS_WINDOW_DATES_ONLY: &str = "chronology_subsets_window_dates_only";
pub(crate) const KEY_SUBSETS_WINDOW_SHOW_DETAILS: &str = "chronology_subsets_window_show_details";

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
    // Timeline subsets (T1.2).
    KEY_SUBSETS_SECTION_TITLE,
    KEY_SUBSETS_SECTION_SUBTITLE,
    KEY_SUBSETS_ADD_BUTTON,
    KEY_SUBSETS_CARRIED_BY_PREFIX,
    KEY_SUBSETS_GAP_COUNT_TEMPLATE,
    KEY_SUBSETS_REMOVED_EVENT_LINE,
    KEY_SUBSETS_SIZE_LINE_TEMPLATE,
    KEY_SUBSETS_PICKER_HINT,
    KEY_SUBSETS_PICKER_GAP_HINT,
    KEY_SCENARIO_VIEW_TIMELINE_BUTTON,
    KEY_SUBSETS_WINDOW_OPEN_TIMELINE,
    KEY_SUBSETS_WINDOW_EDIT,
    KEY_SUBSETS_WINDOW_FOOTER_EVENTS_TEMPLATE,
    KEY_SUBSETS_EMPTY_STATE,
    // Timeline subsets, task 2.
    KEY_SUBSETS_EVENT_COUNT_TEMPLATE,
    KEY_SUBSETS_FORM_ADD_TITLE,
    KEY_SUBSETS_PICKED_COUNT_TEMPLATE,
    KEY_SUBSETS_PILL_GAPS_TEMPLATE,
    KEY_SUBSETS_FORM_NAME_LABEL,
    KEY_SUBSETS_FORM_DESCRIPTION_LABEL,
    KEY_SUBSETS_NOTE_PLACEHOLDER,
    // Timeline subsets, task 3.
    KEY_SUBSETS_MOVE_EARLIER_LABEL,
    KEY_SUBSETS_MOVE_LATER_LABEL,
    KEY_SUBSETS_WINDOW_MINIMIZE_LABEL,
    KEY_SUBSETS_WINDOW_CLOSE_LABEL,
    KEY_SUBSETS_WINDOW_EVENTS_COUNT_TEMPLATE,
    KEY_SUBSETS_GAP_BADGE_LABEL,
    KEY_SUBSETS_WINDOW_LOADING_LABEL,
    // Timeline subsets, task 4.
    KEY_SUBSETS_WINDOW_POPOUT_LABEL,
    KEY_SUBSETS_WINDOW_POPIN_LABEL,
    KEY_SUBSETS_PRECISION_MONTH_LABEL,
    KEY_SUBSETS_PRECISION_YEAR_LABEL,
    KEY_SUBSETS_YEAR_PHASE_DIVIDER_TEMPLATE,
    // Timeline subsets, task 6.
    KEY_SUBSETS_SAVED_NAME_ONLY_BANNER,
    KEY_SUBSETS_EVENTS_NOT_SAVED_BANNER_TEMPLATE,
    KEY_SUBSETS_MODAL_DRAG_LABEL,
    // Timeline subsets, polish.
    KEY_SUBSETS_WINDOW_NO_EVENTS,
    // Timeline subsets, compact view.
    KEY_SUBSETS_WINDOW_DATES_ONLY,
    KEY_SUBSETS_WINDOW_SHOW_DETAILS,
];
