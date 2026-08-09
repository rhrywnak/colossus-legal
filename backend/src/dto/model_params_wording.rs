//! The wire mirror of `domain::wording_model_params::ModelParamsWording`.
//!
//! Ruling R5 (2026-08-09). The domain block is the boot loader's shape — read
//! from the store, validated, and held in the settings snapshot. This is the same
//! words in the shape a browser receives them, and it exists for the reason every
//! sibling mirror in this directory exists (`CardGrammarWordingDto`,
//! `AuthoringWordingDto`, `RehearsalWordingDto`): the domain layer does not derive
//! serde, so that a change to how a value is STORED cannot silently change the
//! API, and vice versa.
//!
//! The fence against a forgotten field is the same one the siblings carry —
//! `the_mirror_carries_every_declared_key` counts the serialized keys against
//! `MODEL_PARAMS_WORDING_KEYS`, both sides derived rather than typed twice.

use serde::{Deserialize, Serialize};

use crate::domain::wording_model_params::ModelParamsWording;

/// The temperature control's words, as the models admin receives them.
///
/// Field names match the domain block's exactly, so a reader moving between the
/// two files never has to translate. The frontend's `ModelParamsWording` type in
/// `services/configApi.ts` mirrors this verbatim.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelParamsWordingDto {
    pub temperature_mode_label: String,
    pub temperature_mode_help: String,
    pub temperature_mode_omit_label: String,
    pub temperature_mode_zero_ok_label: String,
    pub temperature_mode_unset_label: String,
    pub temperature_value_label: String,
    pub temperature_value_disabled_help: String,
}

/// ## Rust Learning: `From<&T>` rather than `From<T>`
///
/// The settings snapshot is shared behind an `Arc` and must outlive this
/// conversion — taking it by value would mean cloning the whole `Settings` to
/// build one response. Borrowing and cloning the seven `String`s is the smaller
/// copy, and it is the shape every sibling mirror uses.
impl From<&ModelParamsWording> for ModelParamsWordingDto {
    fn from(w: &ModelParamsWording) -> Self {
        Self {
            temperature_mode_label: w.temperature_mode_label.clone(),
            temperature_mode_help: w.temperature_mode_help.clone(),
            temperature_mode_omit_label: w.temperature_mode_omit_label.clone(),
            temperature_mode_zero_ok_label: w.temperature_mode_zero_ok_label.clone(),
            temperature_mode_unset_label: w.temperature_mode_unset_label.clone(),
            temperature_value_label: w.temperature_value_label.clone(),
            temperature_value_disabled_help: w.temperature_value_disabled_help.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::wording_model_params::MODEL_PARAMS_WORDING_KEYS;

    /// Every declared key reaches the browser.
    ///
    /// The failure this exists for: a row is added to the domain block and its
    /// migration, the boot loader is happy, and the mirror is forgotten. Nothing
    /// breaks — the dropdown just renders an unlabelled option, on the one screen
    /// whose whole purpose is to stop a model being configured by guesswork.
    ///
    /// Both sides of the assertion are DERIVED (one from the serialized DTO, one
    /// from the key list), so it cannot be satisfied by editing a number.
    #[test]
    fn the_mirror_carries_every_declared_key() {
        let dto = ModelParamsWordingDto::from(&ModelParamsWording::for_test());
        let value = serde_json::to_value(&dto).expect("the mirror serializes");
        let object = value.as_object().expect("a struct serializes to an object");

        assert_eq!(
            object.len(),
            MODEL_PARAMS_WORDING_KEYS.len(),
            "the wire mirror has {} fields but {} keys are declared to the boot \
             loader — a stored word that never reaches the browser renders as a \
             blank control with nothing to say why",
            object.len(),
            MODEL_PARAMS_WORDING_KEYS.len(),
        );
    }

    /// No field crosses the wire empty.
    ///
    /// A `From` impl that assigned one field from the wrong source — or from a
    /// `String::new()` while it was being written — would still serialize seven
    /// keys and pass the test above. This asserts the values are real.
    #[test]
    fn no_mirrored_word_is_blank() {
        let dto = ModelParamsWordingDto::from(&ModelParamsWording::for_test());
        let value = serde_json::to_value(&dto).expect("the mirror serializes");

        for (key, word) in value.as_object().expect("an object") {
            assert!(
                word.as_str().is_some_and(|s| !s.trim().is_empty()),
                "{key} crosses the wire empty"
            );
        }
    }
}
