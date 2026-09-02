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

    // ─── Timeline subsets (T1.2): the words the Subsets surfaces will speak ──
    //
    // Seeded and declared one commit ahead of the screens that render them
    // (tasks 2 and 3), so each is named in the reach test's
    // `DECLARED_AHEAD_OF_THEIR_SCREEN` list until its screen lands. That list is
    // the promise with a name on it; an undeclared row would be the silence.
    //
    // The three `scenario_*` fields are spoken on the scenario pages rather than
    // the timeline, and they are still this block's words: what they say is
    // "there is a timeline behind this button".
    /// The heading over the Subsets section on the timeline home page (design §5A).
    pub subsets_section_title: String,
    /// The muted line under that heading. It states the design's first ruling on the one screen where a reader could get it wrong.
    pub subsets_section_subtitle: String,
    /// The control that opens the new-subset form. The `+` is in the stored string, like `add_event_label`'s.
    pub subsets_add_button: String,
    /// Introduces the scenario codes carrying a subset — "Carried by S-11, S-12". Stored WITHOUT a trailing space: the store trims, so the renderer supplies the joining one.
    pub subsets_carried_by_prefix: String,
    /// How many of a subset's events have been removed from the chronology. `{count}`.
    pub subsets_gap_count_template: String,
    /// What a subset shows in place of a soft-deleted event (design R1). The row is MARKED, never dropped, and the line says where the Undo is — because it is not here.
    pub subsets_removed_event_line: String,
    /// The size line on the picker and on an over-long subset. `{count}`. Design §5D: the page SAYS the limit rather than enforcing it.
    pub subsets_size_line_template: String,
    /// The instruction line at the top of the event picker — ruling 2026-08-30 (1), stated at the moment the author is deciding.
    pub subsets_picker_hint: String,
    /// Shown when the author looks for an event that is not on the chronology. R1 from the other side: the picker cannot offer what does not exist.
    pub subsets_picker_gap_hint: String,
    /// Opens a scenario's attached subset in the floating window. R2 as made explicit 2026-08-30: EVERY scenario view carries it when a subset is attached.
    pub scenario_view_timeline_button: String,
    /// The floating window's footer link to the full page filtered to this subset. It opens the page; it never navigates the page under the window.
    pub subsets_window_open_timeline: String,
    /// Opens the subset's name, description and picker from inside the floating window.
    pub subsets_window_edit: String,
    /// The window's footer count — "15 events". `{count}` is every reference the subset holds, gaps included: the SAME number the title bar shows, so one window cannot report two counts of one story. The footer may carry " · {n} ⚑" after this, composed in code and dropped when n is zero. REPLACED `subsets_window_footer_template`, whose two numbers answered a question nobody was asking.
    pub subsets_window_footer_events_template: String,
    /// Shown in the Subsets section when the case holds none. It teaches rather than reporting: this is where a reader meets the idea.
    pub subsets_empty_state: String,
    /// How many events one subset holds, on its row in the Subsets section. Counts every REFERENCE, gaps included — the amber gap line below it says how many of those are gaps.
    pub subsets_event_count_template: String,
    /// The subset modal's title when it is creating one. The EDIT variant reuses `subsets_window_edit`; this exists because `subsets_add_button` carries a glyph and a heading is not a button.
    pub subsets_form_add_title: String,
    /// How many events are ticked. ONE row for TWO places — the modal's pill and each phase header's suffix — because the mockup spells them identically.
    pub subsets_picked_count_template: String,
    /// The pill's second half. Its own row because it is OMITTED at zero: "15 picked · 0 are gaps" reports an absence as if it were news.
    pub subsets_pill_gaps_template: String,
    /// Labels the subset's name field. NOT `form_title_label` ("Title"), which labels an EVENT's title on a form one click away.
    pub subsets_form_name_label: String,
    /// Labels the subset's description field, and instructs while it labels. The instruction is the point: "Description" alone gets a restatement of the name.
    pub subsets_form_description_label: String,
    /// The placeholder in each picked row's one-line note field. Lowercase and bare: it sits in a dense list where a sentence would shout.
    pub subsets_note_placeholder: String,
    /// The accessible name of the picker's ▲ control. It exists because an aria-label is a user-visible string and the rule admits no exception for the ones only a screen reader speaks.
    pub subsets_move_earlier_label: String,
    /// The mirror of [`Self::subsets_move_earlier_label`]. Edit the two together.
    pub subsets_move_later_label: String,
    /// The floating window's – control. The glyph says nothing to a screen reader, which is why this row exists.
    pub subsets_window_minimize_label: String,
    /// The floating window's × control. Close HIDES; the View Timeline button reopens. Never reads like a delete.
    pub subsets_window_close_label: String,
    /// The count in the window's title bar. Same VALUE as `subsets_event_count_template`, a different surface — see the migration's meaning column.
    pub subsets_window_events_count_template: String,
    /// The amber badge on a window row whose event is gone. The SHORT form; `subsets_removed_event_line` is the sentence.
    pub subsets_gap_badge_label: String,
    /// What the floating window says while the subset's events are being read. Says "the story" because that is what the window is for. NOT `saving_label`, which is a WRITE.
    pub subsets_window_loading_label: String,
    /// The floating window's ⧉ control, which reopens the story as its own desktop window. A glyph with no accessible name of its own.
    pub subsets_window_popout_label: String,
    /// The popped-out window's ⇲ control. The mirror of [`Self::subsets_window_popout_label`]; ⇲ returns the story to the page, × puts it away.
    pub subsets_window_popin_label: String,
    /// The caption under a month-precision date: the source stated a month, so a day would be fabricated.
    pub subsets_precision_month_label: String,
    /// The mirror of [`Self::subsets_precision_month_label`], for year precision. Edit the two together.
    pub subsets_precision_year_label: String,
    /// The window's divider where a story crosses a phase boundary — "2009 · probate". A bare year when the phase does not change.
    pub subsets_year_phase_divider_template: String,

    // ── The Edit-subset modal (T6) ─────────────────────────────────────────
    /// The GREEN half of the split banner: what saved when the events did not.
    pub subsets_saved_name_only_banner: String,
    /// The RED half. `{status}` and `{reason}` — the server's own sentence.
    pub subsets_events_not_saved_banner_template: String,
    /// The accessible name of the ⠿ grip. The glyph says nothing aloud.
    pub subsets_modal_drag_label: String,

    /// What the window's body says when the story carries no events. The third state of that slot — loading and failed already had words, and an empty story rendered a blank band.
    pub subsets_window_no_events: String,
}

/// Every stored key, and the list the boot loader is handed.
///
/// ## Rust Learning: a glob re-export that keeps one public path
///
/// The constants moved to a sibling module for Rule 17 (see that file's header),
/// and this line means nothing else had to move with them: `settings_boot`,
/// `settings_store_tests` and this module's own tests all still say
/// `domain::wording_chronology::CHRONOLOGY_WORDING_KEYS`. A `use` declaration
/// marked `pub(crate)` re-exports what it imports, so the split is invisible
/// to every caller — which is the point of splitting for a line limit rather
/// than for a boundary that means something.
pub(crate) use super::wording_chronology_keys::*;

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
        subsets_section_title: read(KEY_SUBSETS_SECTION_TITLE)?,
        subsets_section_subtitle: read(KEY_SUBSETS_SECTION_SUBTITLE)?,
        subsets_add_button: read(KEY_SUBSETS_ADD_BUTTON)?,
        subsets_carried_by_prefix: read(KEY_SUBSETS_CARRIED_BY_PREFIX)?,
        subsets_gap_count_template: read(KEY_SUBSETS_GAP_COUNT_TEMPLATE)?,
        subsets_removed_event_line: read(KEY_SUBSETS_REMOVED_EVENT_LINE)?,
        subsets_size_line_template: read(KEY_SUBSETS_SIZE_LINE_TEMPLATE)?,
        subsets_picker_hint: read(KEY_SUBSETS_PICKER_HINT)?,
        subsets_picker_gap_hint: read(KEY_SUBSETS_PICKER_GAP_HINT)?,
        scenario_view_timeline_button: read(KEY_SCENARIO_VIEW_TIMELINE_BUTTON)?,
        subsets_window_open_timeline: read(KEY_SUBSETS_WINDOW_OPEN_TIMELINE)?,
        subsets_window_edit: read(KEY_SUBSETS_WINDOW_EDIT)?,
        subsets_window_footer_events_template: read(KEY_SUBSETS_WINDOW_FOOTER_EVENTS_TEMPLATE)?,
        subsets_empty_state: read(KEY_SUBSETS_EMPTY_STATE)?,
        subsets_event_count_template: read(KEY_SUBSETS_EVENT_COUNT_TEMPLATE)?,
        subsets_form_add_title: read(KEY_SUBSETS_FORM_ADD_TITLE)?,
        subsets_picked_count_template: read(KEY_SUBSETS_PICKED_COUNT_TEMPLATE)?,
        subsets_pill_gaps_template: read(KEY_SUBSETS_PILL_GAPS_TEMPLATE)?,
        subsets_form_name_label: read(KEY_SUBSETS_FORM_NAME_LABEL)?,
        subsets_form_description_label: read(KEY_SUBSETS_FORM_DESCRIPTION_LABEL)?,
        subsets_note_placeholder: read(KEY_SUBSETS_NOTE_PLACEHOLDER)?,
        subsets_move_earlier_label: read(KEY_SUBSETS_MOVE_EARLIER_LABEL)?,
        subsets_move_later_label: read(KEY_SUBSETS_MOVE_LATER_LABEL)?,
        subsets_window_minimize_label: read(KEY_SUBSETS_WINDOW_MINIMIZE_LABEL)?,
        subsets_window_close_label: read(KEY_SUBSETS_WINDOW_CLOSE_LABEL)?,
        subsets_window_events_count_template: read(KEY_SUBSETS_WINDOW_EVENTS_COUNT_TEMPLATE)?,
        subsets_gap_badge_label: read(KEY_SUBSETS_GAP_BADGE_LABEL)?,
        subsets_window_loading_label: read(KEY_SUBSETS_WINDOW_LOADING_LABEL)?,
        subsets_window_popout_label: read(KEY_SUBSETS_WINDOW_POPOUT_LABEL)?,
        subsets_window_popin_label: read(KEY_SUBSETS_WINDOW_POPIN_LABEL)?,
        subsets_precision_month_label: read(KEY_SUBSETS_PRECISION_MONTH_LABEL)?,
        subsets_precision_year_label: read(KEY_SUBSETS_PRECISION_YEAR_LABEL)?,
        subsets_year_phase_divider_template: read(KEY_SUBSETS_YEAR_PHASE_DIVIDER_TEMPLATE)?,
        subsets_saved_name_only_banner: read(KEY_SUBSETS_SAVED_NAME_ONLY_BANNER)?,
        subsets_events_not_saved_banner_template: read(
            KEY_SUBSETS_EVENTS_NOT_SAVED_BANNER_TEMPLATE,
        )?,
        subsets_modal_drag_label: read(KEY_SUBSETS_MODAL_DRAG_LABEL)?,
        subsets_window_no_events: read(KEY_SUBSETS_WINDOW_NO_EVENTS)?,
    })
}

#[cfg(test)]
#[path = "wording_chronology_tests.rs"]
mod tests;
