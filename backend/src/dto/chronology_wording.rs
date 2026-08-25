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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::wording_chronology::CHRONOLOGY_WORDING_KEYS;

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
