//! The wire mirror of `domain::wording_fact_card::FactCardWording`.
//!
//! FACT_CARD_v2. The domain block is the boot loader's shape — read from the
//! store, validated, and held in the settings snapshot. This is the same words in
//! the shape a browser receives them, and it exists for the reason every sibling
//! mirror in this directory exists (`CardGrammarWordingDto`, `MatrixWordingDto`,
//! `AuthoringWordingDto`): the domain layer does not derive serde, so a change to
//! how a value is STORED cannot silently change the API, and vice versa.
//!
//! ## Why these words ride the allegation-options payload
//!
//! The scenario page already fetches that payload once for `card_grammar` and
//! hands it to both the queue and the facts section — precisely so two surfaces
//! cannot hold two copies of one catalogue and disagree mid-session. A second
//! endpoint for a witness's twenty-three words would reintroduce that for no gain.

use serde::{Deserialize, Serialize};

use crate::domain::wording_fact_card::FactCardWording;

/// The fact card's words, as the browser receives them.
///
/// Field names match the domain block's exactly, so a reader moving between the
/// two files never has to translate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactCardWordingDto {
    pub proof_label: String,
    pub backs_label: String,
    pub supports_label: String,
    pub watch_out_label: String,
    pub answer_label: String,
    pub empty_value: String,
    pub draft_mark: String,
    pub context_label: String,
    pub context_hide_label: String,
    pub backs_template: String,
    pub supports_template: String,
    pub stance_supports_verb: String,
    pub stance_rebuts_verb: String,
    pub rfa_template: String,
    pub rfa_unnumbered_template: String,
    pub deck_template: String,
    pub edit_label: String,
    pub save_label: String,
    pub cancel_label: String,
    pub save_failed_template: String,
    pub no_answer_notice: String,
    pub accusation_heading: String,
    pub no_cards_notice: String,
}

/// ## Rust Learning: `From<&T>` rather than `From<T>`
///
/// The settings snapshot is shared behind an `Arc` and must outlive this
/// conversion — taking it by value would mean cloning the whole `Settings` to
/// build one response. Borrowing and cloning the twenty-three `String`s is the
/// smaller copy, and it is the shape every sibling mirror uses.
impl From<&FactCardWording> for FactCardWordingDto {
    fn from(w: &FactCardWording) -> Self {
        Self {
            proof_label: w.proof_label.clone(),
            backs_label: w.backs_label.clone(),
            supports_label: w.supports_label.clone(),
            watch_out_label: w.watch_out_label.clone(),
            answer_label: w.answer_label.clone(),
            empty_value: w.empty_value.clone(),
            draft_mark: w.draft_mark.clone(),
            context_label: w.context_label.clone(),
            context_hide_label: w.context_hide_label.clone(),
            backs_template: w.backs_template.clone(),
            supports_template: w.supports_template.clone(),
            stance_supports_verb: w.stance_supports_verb.clone(),
            stance_rebuts_verb: w.stance_rebuts_verb.clone(),
            rfa_template: w.rfa_template.clone(),
            rfa_unnumbered_template: w.rfa_unnumbered_template.clone(),
            deck_template: w.deck_template.clone(),
            edit_label: w.edit_label.clone(),
            save_label: w.save_label.clone(),
            cancel_label: w.cancel_label.clone(),
            save_failed_template: w.save_failed_template.clone(),
            no_answer_notice: w.no_answer_notice.clone(),
            accusation_heading: w.accusation_heading.clone(),
            no_cards_notice: w.no_cards_notice.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::wording_fact_card::FACT_CARD_WORDING_KEYS;

    /// The mirror carries every declared key.
    ///
    /// Both sides are DERIVED — the serialized key set from the struct, the
    /// expected count from the boot loader's list — so a field added to the domain
    /// block and forgotten here fails at `cargo test` rather than as an
    /// `undefined` in a React component.
    #[test]
    fn the_mirror_carries_every_declared_key() {
        let dto = FactCardWordingDto::from(&FactCardWording::for_test());
        let value = serde_json::to_value(&dto).expect("the mirror serializes");
        let keys = value.as_object().expect("an object body");
        assert_eq!(
            keys.len(),
            FACT_CARD_WORDING_KEYS.len(),
            "the wire mirror and the boot loader disagree about how many words \
             this surface speaks",
        );
    }

    /// The wire names are the domain names, minus the block prefix the stored key
    /// carries. A silent rename here would leave the frontend reading `undefined`
    /// on a row label a witness reads.
    #[test]
    fn every_wire_key_is_the_stored_key_without_its_prefix() {
        let dto = FactCardWordingDto::from(&FactCardWording::for_test());
        let value = serde_json::to_value(&dto).expect("the mirror serializes");
        for key in value.as_object().expect("an object body").keys() {
            let stored = format!("fact_card_{key}");
            assert!(
                FACT_CARD_WORDING_KEYS.contains(&stored.as_str()),
                "wire field '{key}' implies stored key '{stored}', which is not declared",
            );
        }
    }
}
