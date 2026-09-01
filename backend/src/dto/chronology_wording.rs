//! The wire mirror of `domain::wording_chronology::ChronologyWording`.
//!
//! Chronology Phase B. The domain block is the boot loader's shape — read from
//! the store, validated, held in the settings snapshot. This is the same words
//! in the shape a browser receives them, and it exists for the reason every
//! sibling mirror in this directory does: the domain layer does not derive
//! serde, so a change to how a value is STORED cannot silently change the API,
//! and vice versa.
//!
//! ## Why these words ride the timeline payload
//!
//! The page cannot render a single row without that read, so a second request
//! for twenty-nine strings fired at the same instant would buy nothing. The same
//! argument `MatrixWordingDto` makes for riding the causes-of-action payload.

use serde::{Deserialize, Serialize};

use crate::domain::wording_chronology::ChronologyWording;

/// The case timeline's words, as the browser receives them.
///
/// Field names match the domain block's exactly, so a reader moving between the
/// two files never has to translate.
// serde: deny_unknown_fields is CORRECT here and not a change-rule violation —
// this is a closed mirror of a declared key list, not an additive payload. A
// field the sender knows and this build does not is precisely the drift the
// mirror test exists to catch, and failing loudly beats ignoring it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChronologyWordingDto {
    pub page_title: String,
    pub count_template: String,
    pub filtered_count_template: String,
    pub search_placeholder: String,
    pub all_tags_label: String,
    pub dates_label: String,
    pub date_from_label: String,
    pub date_to_label: String,
    pub expand_label: String,
    pub show_all_phases_label: String,
    pub scroll_hint_template: String,
    pub phase_count_template: String,
    pub no_document_label: String,
    pub link_unchecked_label: String,
    pub note_count_template: String,
    pub note_count_one: String,
    pub no_pinpoint_label: String,
    pub empty_label: String,
    pub no_matches_label: String,
    pub unknown_phase_template: String,
    pub back_label: String,
    pub documents_heading: String,
    pub notes_heading: String,
    pub history_heading: String,
    pub no_history_label: String,
    pub no_notes_label: String,
    pub band_mismatch_template: String,
    pub add_event_label: String,
    pub edit_label: String,
    pub delete_label: String,
    pub deleted_line_label: String,
    pub undo_label: String,
    pub form_add_title: String,
    pub form_edit_title: String,
    pub form_date_label: String,
    pub form_precision_label: String,
    pub precision_day_label: String,
    pub precision_month_label: String,
    pub precision_year_label: String,
    pub form_approximate_label: String,
    pub form_title_label: String,
    pub form_title_placeholder: String,
    pub form_fact_label: String,
    pub form_fact_placeholder: String,
    pub form_tags_label: String,
    pub form_phase_label: String,
    pub form_documents_label: String,
    pub document_search_placeholder: String,
    pub document_search_empty_label: String,
    pub pinpoint_placeholder: String,
    pub save_label: String,
    pub cancel_label: String,
    pub saving_label: String,
    pub add_note_placeholder: String,
    pub add_note_button_label: String,
    pub link_document_label: String,
    pub remove_link_label: String,
    pub delete_note_label: String,
    pub history_line_template: String,
    pub history_created_label: String,
    pub history_updated_label: String,
    pub history_deleted_label: String,
    pub history_restored_label: String,
    pub history_unknown_template: String,
    pub write_failed_template: String,
    pub picker_capped_template: String,
    // Timeline subsets (T1.2). Declared ahead of tasks 2 and 3, which render
    // them; each is named in the reach test's DECLARED_AHEAD_OF_THEIR_SCREEN
    // list until its screen lands.
    pub subsets_section_title: String,
    pub subsets_section_subtitle: String,
    pub subsets_add_button: String,
    pub subsets_carried_by_prefix: String,
    pub subsets_gap_count_template: String,
    pub subsets_removed_event_line: String,
    pub subsets_size_line_template: String,
    pub subsets_picker_hint: String,
    pub subsets_picker_gap_hint: String,
    pub scenario_view_timeline_button: String,
    pub subsets_window_open_timeline: String,
    pub subsets_window_edit: String,
    pub subsets_window_footer_events_template: String,
    pub subsets_empty_state: String,

    // Timeline subsets, task 2: the seven Screens 2 and 3 needed.
    pub subsets_event_count_template: String,
    pub subsets_form_add_title: String,
    pub subsets_picked_count_template: String,
    pub subsets_pill_gaps_template: String,
    pub subsets_form_name_label: String,
    pub subsets_form_description_label: String,
    pub subsets_note_placeholder: String,

    // Timeline subsets, task 3: the two aria rows and the window's four.
    pub subsets_move_earlier_label: String,
    pub subsets_move_later_label: String,
    pub subsets_window_minimize_label: String,
    pub subsets_window_close_label: String,
    pub subsets_window_events_count_template: String,
    pub subsets_gap_badge_label: String,
    pub subsets_window_loading_label: String,

    // Timeline subsets, task 4: the redrawn row and Pop out.
    pub subsets_window_popout_label: String,
    pub subsets_window_popin_label: String,
    pub subsets_precision_month_label: String,
    pub subsets_precision_year_label: String,
    pub subsets_year_phase_divider_template: String,

    // Timeline subsets, task 6: the Edit modal.
    pub subsets_saved_name_only_banner: String,
    pub subsets_events_not_saved_banner_template: String,
    pub subsets_modal_drag_label: String,
}

/// ## Rust Learning: `From<&T>` rather than `From<T>`
///
/// The settings snapshot is shared behind an `Arc` and must outlive this
/// conversion — taking it by value would mean cloning the whole `Settings` to
/// build one response. Borrowing and cloning the strings is the smaller copy,
/// and it is the shape every sibling mirror uses.
impl From<&ChronologyWording> for ChronologyWordingDto {
    fn from(w: &ChronologyWording) -> Self {
        Self {
            page_title: w.page_title.clone(),
            count_template: w.count_template.clone(),
            filtered_count_template: w.filtered_count_template.clone(),
            search_placeholder: w.search_placeholder.clone(),
            all_tags_label: w.all_tags_label.clone(),
            dates_label: w.dates_label.clone(),
            date_from_label: w.date_from_label.clone(),
            date_to_label: w.date_to_label.clone(),
            expand_label: w.expand_label.clone(),
            show_all_phases_label: w.show_all_phases_label.clone(),
            scroll_hint_template: w.scroll_hint_template.clone(),
            phase_count_template: w.phase_count_template.clone(),
            no_document_label: w.no_document_label.clone(),
            link_unchecked_label: w.link_unchecked_label.clone(),
            note_count_template: w.note_count_template.clone(),
            note_count_one: w.note_count_one.clone(),
            no_pinpoint_label: w.no_pinpoint_label.clone(),
            empty_label: w.empty_label.clone(),
            no_matches_label: w.no_matches_label.clone(),
            unknown_phase_template: w.unknown_phase_template.clone(),
            back_label: w.back_label.clone(),
            documents_heading: w.documents_heading.clone(),
            notes_heading: w.notes_heading.clone(),
            history_heading: w.history_heading.clone(),
            no_history_label: w.no_history_label.clone(),
            no_notes_label: w.no_notes_label.clone(),
            band_mismatch_template: w.band_mismatch_template.clone(),
            add_event_label: w.add_event_label.clone(),
            edit_label: w.edit_label.clone(),
            delete_label: w.delete_label.clone(),
            deleted_line_label: w.deleted_line_label.clone(),
            undo_label: w.undo_label.clone(),
            form_add_title: w.form_add_title.clone(),
            form_edit_title: w.form_edit_title.clone(),
            form_date_label: w.form_date_label.clone(),
            form_precision_label: w.form_precision_label.clone(),
            precision_day_label: w.precision_day_label.clone(),
            precision_month_label: w.precision_month_label.clone(),
            precision_year_label: w.precision_year_label.clone(),
            form_approximate_label: w.form_approximate_label.clone(),
            form_title_label: w.form_title_label.clone(),
            form_title_placeholder: w.form_title_placeholder.clone(),
            form_fact_label: w.form_fact_label.clone(),
            form_fact_placeholder: w.form_fact_placeholder.clone(),
            form_tags_label: w.form_tags_label.clone(),
            form_phase_label: w.form_phase_label.clone(),
            form_documents_label: w.form_documents_label.clone(),
            document_search_placeholder: w.document_search_placeholder.clone(),
            document_search_empty_label: w.document_search_empty_label.clone(),
            pinpoint_placeholder: w.pinpoint_placeholder.clone(),
            save_label: w.save_label.clone(),
            cancel_label: w.cancel_label.clone(),
            saving_label: w.saving_label.clone(),
            add_note_placeholder: w.add_note_placeholder.clone(),
            add_note_button_label: w.add_note_button_label.clone(),
            link_document_label: w.link_document_label.clone(),
            remove_link_label: w.remove_link_label.clone(),
            delete_note_label: w.delete_note_label.clone(),
            history_line_template: w.history_line_template.clone(),
            history_created_label: w.history_created_label.clone(),
            history_updated_label: w.history_updated_label.clone(),
            history_deleted_label: w.history_deleted_label.clone(),
            history_restored_label: w.history_restored_label.clone(),
            history_unknown_template: w.history_unknown_template.clone(),
            write_failed_template: w.write_failed_template.clone(),
            picker_capped_template: w.picker_capped_template.clone(),
            subsets_section_title: w.subsets_section_title.clone(),
            subsets_section_subtitle: w.subsets_section_subtitle.clone(),
            subsets_add_button: w.subsets_add_button.clone(),
            subsets_carried_by_prefix: w.subsets_carried_by_prefix.clone(),
            subsets_gap_count_template: w.subsets_gap_count_template.clone(),
            subsets_removed_event_line: w.subsets_removed_event_line.clone(),
            subsets_size_line_template: w.subsets_size_line_template.clone(),
            subsets_picker_hint: w.subsets_picker_hint.clone(),
            subsets_picker_gap_hint: w.subsets_picker_gap_hint.clone(),
            scenario_view_timeline_button: w.scenario_view_timeline_button.clone(),
            subsets_window_open_timeline: w.subsets_window_open_timeline.clone(),
            subsets_window_edit: w.subsets_window_edit.clone(),
            subsets_window_footer_events_template: w.subsets_window_footer_events_template.clone(),
            subsets_empty_state: w.subsets_empty_state.clone(),
            subsets_event_count_template: w.subsets_event_count_template.clone(),
            subsets_form_add_title: w.subsets_form_add_title.clone(),
            subsets_picked_count_template: w.subsets_picked_count_template.clone(),
            subsets_pill_gaps_template: w.subsets_pill_gaps_template.clone(),
            subsets_form_name_label: w.subsets_form_name_label.clone(),
            subsets_form_description_label: w.subsets_form_description_label.clone(),
            subsets_note_placeholder: w.subsets_note_placeholder.clone(),
            subsets_move_earlier_label: w.subsets_move_earlier_label.clone(),
            subsets_move_later_label: w.subsets_move_later_label.clone(),
            subsets_window_minimize_label: w.subsets_window_minimize_label.clone(),
            subsets_window_close_label: w.subsets_window_close_label.clone(),
            subsets_window_events_count_template: w.subsets_window_events_count_template.clone(),
            subsets_gap_badge_label: w.subsets_gap_badge_label.clone(),
            subsets_window_loading_label: w.subsets_window_loading_label.clone(),
            subsets_window_popout_label: w.subsets_window_popout_label.clone(),
            subsets_window_popin_label: w.subsets_window_popin_label.clone(),
            subsets_precision_month_label: w.subsets_precision_month_label.clone(),
            subsets_precision_year_label: w.subsets_precision_year_label.clone(),
            subsets_year_phase_divider_template: w.subsets_year_phase_divider_template.clone(),
            subsets_saved_name_only_banner: w.subsets_saved_name_only_banner.clone(),
            subsets_events_not_saved_banner_template: w
                .subsets_events_not_saved_banner_template
                .clone(),
            subsets_modal_drag_label: w.subsets_modal_drag_label.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::wording_chronology::CHRONOLOGY_WORDING_KEYS;

    /// ⚑ The DOCK is served the SAME block, not a second shape.
    ///
    /// The scenario dock's read carries the wording so five surfaces that share
    /// no header and no read can still speak. The promise made in
    /// `ScenarioSubsetsDto`'s header is that it is the same block — so a row
    /// edited once is edited for both surfaces. This is that promise, checked:
    /// the two payloads' wording objects must have identical key sets, not
    /// merely compatible ones.
    ///
    /// A subset of the block would pass a "does it compile" reading and fail a
    /// reader on the scenario page, whose control would render blank for a key
    /// the timeline had and the dock did not.
    #[test]
    fn the_dock_is_served_the_same_wording_block_as_the_timeline() {
        use crate::dto::chronology_subset::ScenarioSubsetsDto;

        let wording = ChronologyWordingDto::from(&ChronologyWording::for_test());
        let dock = ScenarioSubsetsDto {
            subsets: vec![],
            wording: wording.clone(),
        };

        let from_dock = serde_json::to_value(&dock).expect("the dock payload serializes");
        let dock_keys: Vec<&String> = from_dock
            .get("wording")
            .and_then(|w| w.as_object())
            .expect("the dock carries a wording object")
            .keys()
            .collect();
        let direct = serde_json::to_value(&wording).expect("the mirror serializes");
        let timeline_keys: Vec<&String> =
            direct.as_object().expect("an object body").keys().collect();

        assert_eq!(
            dock_keys, timeline_keys,
            "the dock and the timeline are serving different wording shapes; a \
             control on a scenario page would render blank for any key only one \
             of them carries",
        );
        // Anti-vacuity: two empty key sets would compare equal and prove nothing.
        assert_eq!(dock_keys.len(), CHRONOLOGY_WORDING_KEYS.len());
    }

    /// The mirror carries every declared key.
    ///
    /// Both sides are DERIVED — the serialized key set from the struct, the
    /// expected count from the boot loader's list — so a field added to the
    /// domain block and forgotten here fails at `cargo test` rather than as an
    /// `undefined` in a React component. This is the .408 mirror-field lesson.
    #[test]
    fn the_mirror_carries_every_declared_key() {
        let dto = ChronologyWordingDto::from(&ChronologyWording::for_test());
        let value = serde_json::to_value(&dto).expect("the mirror serializes");
        let keys = value.as_object().expect("an object body");
        assert_eq!(
            keys.len(),
            CHRONOLOGY_WORDING_KEYS.len(),
            "the wire mirror and the boot loader disagree about how many words \
             this surface speaks",
        );
    }

    /// The wire names are the stored names minus the block prefix.
    #[test]
    fn every_wire_key_is_the_stored_key_without_its_prefix() {
        let dto = ChronologyWordingDto::from(&ChronologyWording::for_test());
        let value = serde_json::to_value(&dto).expect("the mirror serializes");
        for key in value.as_object().expect("an object body").keys() {
            let stored = format!("chronology_{key}");
            assert!(
                CHRONOLOGY_WORDING_KEYS.contains(&stored.as_str()),
                "wire field '{key}' implies stored key '{stored}', which is not declared",
            );
        }
    }

    /// No word arrives blank.
    ///
    /// A blank string is the one value that passes every shape check and renders
    /// as nothing at all — a control with no label, which the standing rule of
    /// 2026-08-19 forbids on any page a witness reads.
    #[test]
    fn no_word_reaches_the_browser_blank() {
        let dto = ChronologyWordingDto::from(&ChronologyWording::for_test());
        let value = serde_json::to_value(&dto).expect("the mirror serializes");
        for (key, v) in value.as_object().expect("an object body") {
            assert!(
                v.as_str().is_some_and(|s| !s.trim().is_empty()),
                "{key} would reach the browser blank"
            );
        }
    }
}

#[cfg(test)]
#[path = "chronology_wording_reach_tests.rs"]
mod reach_tests;
