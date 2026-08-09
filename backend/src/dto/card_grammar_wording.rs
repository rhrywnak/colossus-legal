//! The wire mirror of `domain::wording_card_grammar::CardGrammarWording`.
//!
//! ONE_CARD_GRAMMAR, ruling R6. The domain block is the boot loader's shape —
//! read from the store, validated, and held in the settings snapshot. This is the
//! same words in the shape a browser receives them, and it exists for the reason
//! every sibling mirror in this directory exists (`AuthoringWordingDto`,
//! `RehearsalWordingDto`, `ScenarioCreateWordingDto`): the domain layer does not
//! derive serde, so that a change to how a value is STORED cannot silently change
//! the API, and vice versa.
//!
//! ## The one thing a hand-written mirror can get wrong, and the fence for it
//!
//! A field added to the domain block and forgotten here compiles cleanly — the
//! `From` impl names every DTO field, and a DTO missing one is simply a smaller
//! struct. So the word never reaches the browser, the control renders blank, and
//! nothing says why.
//!
//! `the_mirror_carries_every_declared_key` closes that: it serializes the DTO and
//! counts the keys against `CARD_GRAMMAR_WORDING_KEYS`. Both are derived from the
//! thing they describe rather than typed twice, so the assertion cannot pass by
//! being edited into agreement.

use serde::{Deserialize, Serialize};

use crate::domain::wording_card_grammar::CardGrammarWording;

/// Every stored string one evidence card and its queue frame render.
///
/// Field names match the domain block's exactly, so a reader moving between the
/// two files never has to translate. The frontend's `CardGrammarWording` type in
/// `services/evidenceLinks.ts` mirrors this verbatim.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardGrammarWordingDto {
    // The queue frame.
    pub filter_proposed_label: String,
    pub filter_deferred_label: String,
    pub filter_included_label: String,
    pub filter_excluded_label: String,
    pub filter_full_pool_label: String,
    pub full_pool_explainer: String,
    /// Carries `{ruled}`, `{total}` and `{filter}`.
    pub filter_progress_template: String,

    // The card body.
    pub question_expand_label: String,
    pub question_collapse_label: String,
    /// The badge saying who transcribed the QUESTION — never a speaker (R2).
    pub question_machine_authorship_label: String,
    pub speaker_extracted_label: String,
    pub speaker_absent_label: String,
    /// Carries `{count}`.
    pub elements_more_template: String,
    pub elements_fewer_label: String,
    pub context_show_label: String,
    pub context_hide_label: String,
    pub scan_reason_label: String,

    // Linking.
    pub link_typeahead_placeholder: String,
    pub link_typeahead_intro: String,
    pub link_typeahead_no_match: String,
    /// Carries `{code}`.
    pub link_woke_ruling_template: String,

    // The fact wrapper.
    pub weight_picker_label: String,
    /// Carries `{code}` and `{tier}`.
    pub weight_changed_template: String,
    pub weight_undo_label: String,
    pub reset_order_label: String,
    pub reset_order_confirm: String,
    pub reset_order_confirm_yes: String,
    pub reset_order_confirm_cancel: String,
    /// Carries `{count}`.
    pub reset_order_done_template: String,
    /// Carries `{reason}`.
    pub reset_order_failed_template: String,

    // Chips as currency.
    /// Carries `{value}`.
    pub chip_filter_hint_template: String,
    /// Carries `{value}`.
    pub chip_filter_clear_template: String,
}

/// ## Rust Learning: `From<&T>` rather than `From<T>`
///
/// The settings snapshot is behind an `Arc` and is shared by every request in
/// flight, so a caller has a `&Settings` and nothing to give away. Taking a
/// reference and cloning the strings inside is the only shape that works — and it
/// makes the cost visible at the definition instead of hiding it behind a move
/// the caller cannot actually perform.
impl From<&CardGrammarWording> for CardGrammarWordingDto {
    fn from(w: &CardGrammarWording) -> Self {
        Self {
            filter_proposed_label: w.filter_proposed_label.clone(),
            filter_deferred_label: w.filter_deferred_label.clone(),
            filter_included_label: w.filter_included_label.clone(),
            filter_excluded_label: w.filter_excluded_label.clone(),
            filter_full_pool_label: w.filter_full_pool_label.clone(),
            full_pool_explainer: w.full_pool_explainer.clone(),
            filter_progress_template: w.filter_progress_template.clone(),
            question_expand_label: w.question_expand_label.clone(),
            question_collapse_label: w.question_collapse_label.clone(),
            question_machine_authorship_label: w.question_machine_authorship_label.clone(),
            speaker_extracted_label: w.speaker_extracted_label.clone(),
            speaker_absent_label: w.speaker_absent_label.clone(),
            elements_more_template: w.elements_more_template.clone(),
            elements_fewer_label: w.elements_fewer_label.clone(),
            context_show_label: w.context_show_label.clone(),
            context_hide_label: w.context_hide_label.clone(),
            scan_reason_label: w.scan_reason_label.clone(),
            link_typeahead_placeholder: w.link_typeahead_placeholder.clone(),
            link_typeahead_intro: w.link_typeahead_intro.clone(),
            link_typeahead_no_match: w.link_typeahead_no_match.clone(),
            link_woke_ruling_template: w.link_woke_ruling_template.clone(),
            weight_picker_label: w.weight_picker_label.clone(),
            weight_changed_template: w.weight_changed_template.clone(),
            weight_undo_label: w.weight_undo_label.clone(),
            reset_order_label: w.reset_order_label.clone(),
            reset_order_confirm: w.reset_order_confirm.clone(),
            reset_order_confirm_yes: w.reset_order_confirm_yes.clone(),
            reset_order_confirm_cancel: w.reset_order_confirm_cancel.clone(),
            reset_order_done_template: w.reset_order_done_template.clone(),
            reset_order_failed_template: w.reset_order_failed_template.clone(),
            chip_filter_hint_template: w.chip_filter_hint_template.clone(),
            chip_filter_clear_template: w.chip_filter_clear_template.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::wording_card_grammar::CARD_GRAMMAR_WORDING_KEYS;

    /// Every declared key reaches the browser.
    ///
    /// The failure this exists for: a row is added to the domain block and its
    /// migration, the boot loader is happy, and the mirror is forgotten. Nothing
    /// breaks — the control just renders nothing, on a surface whose whole law is
    /// that a missing word must be loud.
    ///
    /// Both sides of the assertion are DERIVED (one from the serialized DTO, one
    /// from the key list), so it cannot be satisfied by editing a number.
    #[test]
    fn the_mirror_carries_every_declared_key() {
        let dto = CardGrammarWordingDto::from(&CardGrammarWording::for_test());
        let value = serde_json::to_value(&dto).expect("the mirror serializes");
        let object = value.as_object().expect("a struct serializes to an object");

        assert_eq!(
            object.len(),
            CARD_GRAMMAR_WORDING_KEYS.len(),
            "the wire mirror has {} fields but {} keys are declared to the boot \
             loader — a stored word that never reaches the browser renders as a \
             blank control with nothing to say why",
            object.len(),
            CARD_GRAMMAR_WORDING_KEYS.len(),
        );
    }

    /// No field crosses the wire empty.
    ///
    /// A `From` impl that assigned one field from the wrong source — or from a
    /// `String::new()` while it was being written — would still serialize 32 keys
    /// and pass the test above. This asserts the values are real.
    #[test]
    fn no_mirrored_word_is_blank() {
        let dto = CardGrammarWordingDto::from(&CardGrammarWording::for_test());
        let value = serde_json::to_value(&dto).expect("the mirror serializes");

        for (key, word) in value.as_object().expect("an object") {
            assert!(
                word.as_str().is_some_and(|s| !s.trim().is_empty()),
                "{key} crosses the wire blank"
            );
        }
    }
}
