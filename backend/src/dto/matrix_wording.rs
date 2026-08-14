//! The wire mirror of `domain::wording_matrix::MatrixWording`.
//!
//! Task 396 P1. The domain block is the boot loader's shape — read from the
//! store, validated, and held in the settings snapshot. This is the same words in
//! the shape a browser receives them, and it exists for the reason every sibling
//! mirror in this directory exists (`CardGrammarWordingDto`,
//! `ModelParamsWordingDto`, `AuthoringWordingDto`): the domain layer does not
//! derive serde, so a change to how a value is STORED cannot silently change the
//! API, and vice versa.
//!
//! ## Why these words ride the causes-of-action payload
//!
//! The Proof Matrix page already GATES on that read — it cannot render a row
//! without it — and both surfaces that speak these words (the row's headline and
//! its drill-down) are on that page. A second request for eight strings, fired at
//! the same instant, would buy nothing; the same argument
//! `ScenarioIdentityWording` makes for riding the augmentation payload.

use serde::{Deserialize, Serialize};

use crate::domain::wording_matrix::MatrixWording;

/// The Proof Matrix's words, as the browser receives them.
///
/// Field names match the domain block's exactly, so a reader moving between the
/// two files never has to translate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixWordingDto {
    pub strong_column_label: String,
    pub raw_approved_template: String,
    pub strong_hint: String,
    pub tier_strong_chip: String,
    pub tier_hedged_chip: String,
    pub tier_other_chip: String,
    pub duplicate_template: String,
    pub ranked_list_note: String,
}

/// ## Rust Learning: `From<&T>` rather than `From<T>`
///
/// The settings snapshot is shared behind an `Arc` and must outlive this
/// conversion — taking it by value would mean cloning the whole `Settings` to
/// build one response. Borrowing and cloning the eight `String`s is the smaller
/// copy, and it is the shape every sibling mirror uses.
impl From<&MatrixWording> for MatrixWordingDto {
    fn from(w: &MatrixWording) -> Self {
        Self {
            strong_column_label: w.strong_column_label.clone(),
            raw_approved_template: w.raw_approved_template.clone(),
            strong_hint: w.strong_hint.clone(),
            tier_strong_chip: w.tier_strong_chip.clone(),
            tier_hedged_chip: w.tier_hedged_chip.clone(),
            tier_other_chip: w.tier_other_chip.clone(),
            duplicate_template: w.duplicate_template.clone(),
            ranked_list_note: w.ranked_list_note.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::wording_matrix::MATRIX_WORDING_KEYS;

    /// The mirror carries every declared key.
    ///
    /// Both sides are DERIVED — the serialized key set from the struct, the
    /// expected count from the boot loader's list — so a field added to the domain
    /// block and forgotten here fails at `cargo test` rather than as an
    /// `undefined` in a React component.
    #[test]
    fn the_mirror_carries_every_declared_key() {
        let dto = MatrixWordingDto::from(&MatrixWording::for_test());
        let value = serde_json::to_value(&dto).expect("the mirror serializes");
        let keys = value.as_object().expect("an object body");
        assert_eq!(
            keys.len(),
            MATRIX_WORDING_KEYS.len(),
            "the wire mirror and the boot loader disagree about how many words \
             this surface speaks",
        );
    }

    /// The wire names are the domain names, minus the block prefix the stored key
    /// carries. A silent rename here would leave the frontend reading `undefined`
    /// on a column header.
    #[test]
    fn every_wire_key_is_the_stored_key_without_its_prefix() {
        let dto = MatrixWordingDto::from(&MatrixWording::for_test());
        let value = serde_json::to_value(&dto).expect("the mirror serializes");
        for key in value.as_object().expect("an object body").keys() {
            let stored = format!("matrix_{key}");
            assert!(
                MATRIX_WORDING_KEYS.contains(&stored.as_str()),
                "wire field '{key}' implies stored key '{stored}', which is not declared",
            );
        }
    }
}
