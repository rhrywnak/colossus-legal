// =============================================================================
// backend/src/domain/wording_fact_card.rs — the words a WITNESS reads
// =============================================================================
//
// FACT_CARD_v2 §2 and §3. The twenty-three strings the scenario fact card puts
// on screen, on the working page and on the rehearsal page.
//
// ## Why a new block rather than more keys on `card_grammar`
//
// The house test: which SURFACE speaks these, and does its vocabulary move
// independently? `card_grammar` speaks to a CURATOR triaging a queue — filter
// chips, fold controls, provenance badges. These speak to MARIE, preparing to
// answer a question under oath, and to Chuck writing what she will say. They will
// move when she finds a word confusing, which has nothing to do with what a
// filter chip says.
//
// ## Domain note: nothing here is a placeholder for a missing field
//
// [`FactCardWording::empty_value`] is printed IN a row nobody has filled in — the
// row itself is never hidden. §2 is explicit: nothing is hidden for lacking a
// field, because a card with no Answer is work somebody still owes, and hiding it
// hides the work rather than the gap. (The mockup of record says otherwise; the
// instruction governs, and this is the difference.)

/// The strings the fact card renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactCardWording {
    /// The label on the row carrying the statement's own words.
    pub proof_label: String,
    /// The label on the row naming which talking point this card backs.
    pub backs_label: String,
    /// The label on the row naming the accusation this card goes to.
    pub supports_label: String,
    /// The label on the row saying how the other side will use it.
    pub watch_out_label: String,
    /// The label on the row carrying her reply.
    pub answer_label: String,
    /// Printed in a row nobody has filled in.
    ///
    /// Domain note: the row is NEVER hidden. A card missing its Answer is a card
    /// somebody still has to write, and hiding it would hide the work rather than
    /// the gap — which is why §2 says nothing is hidden for lacking a field.
    pub empty_value: String,
    /// The small grey mark on a field the machine wrote.
    ///
    /// Domain note: this is what keeps a drafted deck from reading as a prepared
    /// one. Job B wrote 59 of these cards; every field it touched carries this
    /// until a human edits it.
    pub draft_mark: String,
    /// The control that opens the source text around a quote.
    pub context_label: String,
    /// The same control once the surrounding text is showing.
    pub context_hide_label: String,
    /// How the Backs row reads. Carries `{position}` and `{text}`.
    pub backs_template: String,
    /// How the Supports row reads. Carries `{verb}`, `{code}` and `{text}`.
    pub supports_template: String,
    /// The verb for a card that helps the accusation stand.
    pub stance_supports_verb: String,
    /// The verb for a card whose stance is `rebuts`.
    ///
    /// Domain note: the WORD and the TOKEN differ on purpose. The graph's own
    /// `r.stance` and Job B's files both say `rebuts`; a witness reads
    /// "Disputes". Never a negation of the verb above — a reader skimming five
    /// cards must not have to parse "not supports".
    pub stance_rebuts_verb: String,
    /// How an answer-only card reads in the Proof row. Carries `{number}`,
    /// `{request}` and `{answer}`.
    pub rfa_template: String,
    /// The same line when no request number can be derived. Carries `{request}`
    /// and `{answer}`.
    pub rfa_unnumbered_template: String,
    /// The line above the deck. Carries `{shown}`, `{total}` and `{collapsed}`.
    pub deck_template: String,
    /// The control that opens a field for editing.
    pub edit_label: String,
    /// The control that stores one edited field.
    pub save_label: String,
    /// The control that closes an editor without storing.
    pub cancel_label: String,
    /// Shown when a field edit could not be written. Carries `{detail}`.
    pub save_failed_template: String,
    /// Shown on the rehearsal page under an accusation nobody has answered.
    ///
    /// Domain note: the card is still shown. An unanswered accusation is the most
    /// important thing on that page.
    pub no_answer_notice: String,
    /// The heading over the other side's own statements, in date order.
    pub accusation_heading: String,
    /// Shown where a section would otherwise be blank.
    pub no_cards_notice: String,
}

// KEYS: the stable identifiers. Renaming one is a migration, and until it runs
// the boot loader refuses to start.
// STRUCTURAL: these are the names of rows in `app_settings`, not values read from
// them. A key is wire vocabulary shared with the database — the VALUES are what
// Standing Rule 2 governs, and every one of them is a stored row read at boot.
pub(crate) const KEY_PROOF_LABEL: &str = "fact_card_proof_label";
pub(crate) const KEY_BACKS_LABEL: &str = "fact_card_backs_label";
pub(crate) const KEY_SUPPORTS_LABEL: &str = "fact_card_supports_label";
pub(crate) const KEY_WATCH_OUT_LABEL: &str = "fact_card_watch_out_label";
pub(crate) const KEY_ANSWER_LABEL: &str = "fact_card_answer_label";
pub(crate) const KEY_EMPTY_VALUE: &str = "fact_card_empty_value";
pub(crate) const KEY_DRAFT_MARK: &str = "fact_card_draft_mark";
pub(crate) const KEY_CONTEXT_LABEL: &str = "fact_card_context_label";
pub(crate) const KEY_CONTEXT_HIDE_LABEL: &str = "fact_card_context_hide_label";
pub(crate) const KEY_BACKS_TEMPLATE: &str = "fact_card_backs_template";
pub(crate) const KEY_SUPPORTS_TEMPLATE: &str = "fact_card_supports_template";
pub(crate) const KEY_STANCE_SUPPORTS_VERB: &str = "fact_card_stance_supports_verb";
pub(crate) const KEY_STANCE_REBUTS_VERB: &str = "fact_card_stance_rebuts_verb";
pub(crate) const KEY_RFA_TEMPLATE: &str = "fact_card_rfa_template";
pub(crate) const KEY_RFA_UNNUMBERED_TEMPLATE: &str = "fact_card_rfa_unnumbered_template";
pub(crate) const KEY_DECK_TEMPLATE: &str = "fact_card_deck_template";
pub(crate) const KEY_EDIT_LABEL: &str = "fact_card_edit_label";
pub(crate) const KEY_SAVE_LABEL: &str = "fact_card_save_label";
pub(crate) const KEY_CANCEL_LABEL: &str = "fact_card_cancel_label";
pub(crate) const KEY_SAVE_FAILED_TEMPLATE: &str = "fact_card_save_failed_template";
pub(crate) const KEY_NO_ANSWER_NOTICE: &str = "fact_card_no_answer_notice";
pub(crate) const KEY_ACCUSATION_HEADING: &str = "fact_card_accusation_heading";
pub(crate) const KEY_NO_CARDS_NOTICE: &str = "fact_card_no_cards_notice";

/// Every fact-card key this build reads, so a missing one is caught at boot BY
/// NAME rather than as a blank label on a witness's card.
pub const FACT_CARD_WORDING_KEYS: &[&str] = &[
    KEY_PROOF_LABEL,
    KEY_BACKS_LABEL,
    KEY_SUPPORTS_LABEL,
    KEY_WATCH_OUT_LABEL,
    KEY_ANSWER_LABEL,
    KEY_EMPTY_VALUE,
    KEY_DRAFT_MARK,
    KEY_CONTEXT_LABEL,
    KEY_CONTEXT_HIDE_LABEL,
    KEY_BACKS_TEMPLATE,
    KEY_SUPPORTS_TEMPLATE,
    KEY_STANCE_SUPPORTS_VERB,
    KEY_STANCE_REBUTS_VERB,
    KEY_RFA_TEMPLATE,
    KEY_RFA_UNNUMBERED_TEMPLATE,
    KEY_DECK_TEMPLATE,
    KEY_EDIT_LABEL,
    KEY_SAVE_LABEL,
    KEY_CANCEL_LABEL,
    KEY_SAVE_FAILED_TEMPLATE,
    KEY_NO_ANSWER_NOTICE,
    KEY_ACCUSATION_HEADING,
    KEY_NO_CARDS_NOTICE,
];

/// Build a [`FactCardWording`] from the stored rows, or say which key is wrong.
///
/// Same generic-closure shape as the eleven sibling builders — see
/// [`crate::domain::wording_model_params::build_model_params_wording`] for why
/// `read` is a closure over a generic error type rather than a database handle.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_fact_card_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<FactCardWording, E> {
    Ok(FactCardWording {
        proof_label: read(KEY_PROOF_LABEL)?,
        backs_label: read(KEY_BACKS_LABEL)?,
        supports_label: read(KEY_SUPPORTS_LABEL)?,
        watch_out_label: read(KEY_WATCH_OUT_LABEL)?,
        answer_label: read(KEY_ANSWER_LABEL)?,
        empty_value: read(KEY_EMPTY_VALUE)?,
        draft_mark: read(KEY_DRAFT_MARK)?,
        context_label: read(KEY_CONTEXT_LABEL)?,
        context_hide_label: read(KEY_CONTEXT_HIDE_LABEL)?,
        backs_template: read(KEY_BACKS_TEMPLATE)?,
        supports_template: read(KEY_SUPPORTS_TEMPLATE)?,
        stance_supports_verb: read(KEY_STANCE_SUPPORTS_VERB)?,
        stance_rebuts_verb: read(KEY_STANCE_REBUTS_VERB)?,
        rfa_template: read(KEY_RFA_TEMPLATE)?,
        rfa_unnumbered_template: read(KEY_RFA_UNNUMBERED_TEMPLATE)?,
        deck_template: read(KEY_DECK_TEMPLATE)?,
        edit_label: read(KEY_EDIT_LABEL)?,
        save_label: read(KEY_SAVE_LABEL)?,
        cancel_label: read(KEY_CANCEL_LABEL)?,
        save_failed_template: read(KEY_SAVE_FAILED_TEMPLATE)?,
        no_answer_notice: read(KEY_NO_ANSWER_NOTICE)?,
        accusation_heading: read(KEY_ACCUSATION_HEADING)?,
        no_cards_notice: read(KEY_NO_CARDS_NOTICE)?,
    })
}

#[cfg(test)]
#[path = "wording_fact_card_tests.rs"]
pub(crate) mod seed_tests;

#[cfg(test)]
mod tests {
    use super::*;

    /// A `read` that echoes the key, so the builder's field→key wiring is visible.
    fn echo(key: &str) -> Result<String, std::convert::Infallible> {
        Ok(key.to_string())
    }

    /// Every field reads the key it claims to. A copy-paste that pointed two
    /// fields at one row would put the same sentence in two places on screen with
    /// nothing failing — the defect this echo test exists to catch.
    #[test]
    fn every_field_reads_its_own_key() {
        let w = build_fact_card_wording(echo).expect("infallible read");
        assert_eq!(w.proof_label, KEY_PROOF_LABEL);
        assert_eq!(w.backs_label, KEY_BACKS_LABEL);
        assert_eq!(w.supports_label, KEY_SUPPORTS_LABEL);
        assert_eq!(w.watch_out_label, KEY_WATCH_OUT_LABEL);
        assert_eq!(w.answer_label, KEY_ANSWER_LABEL);
        assert_eq!(w.empty_value, KEY_EMPTY_VALUE);
        assert_eq!(w.draft_mark, KEY_DRAFT_MARK);
        assert_eq!(w.context_label, KEY_CONTEXT_LABEL);
        assert_eq!(w.context_hide_label, KEY_CONTEXT_HIDE_LABEL);
        assert_eq!(w.backs_template, KEY_BACKS_TEMPLATE);
        assert_eq!(w.supports_template, KEY_SUPPORTS_TEMPLATE);
        assert_eq!(w.stance_supports_verb, KEY_STANCE_SUPPORTS_VERB);
        assert_eq!(w.stance_rebuts_verb, KEY_STANCE_REBUTS_VERB);
        assert_eq!(w.rfa_template, KEY_RFA_TEMPLATE);
        assert_eq!(w.rfa_unnumbered_template, KEY_RFA_UNNUMBERED_TEMPLATE);
        assert_eq!(w.deck_template, KEY_DECK_TEMPLATE);
        assert_eq!(w.edit_label, KEY_EDIT_LABEL);
        assert_eq!(w.save_label, KEY_SAVE_LABEL);
        assert_eq!(w.cancel_label, KEY_CANCEL_LABEL);
        assert_eq!(w.save_failed_template, KEY_SAVE_FAILED_TEMPLATE);
        assert_eq!(w.no_answer_notice, KEY_NO_ANSWER_NOTICE);
        assert_eq!(w.accusation_heading, KEY_ACCUSATION_HEADING);
        assert_eq!(w.no_cards_notice, KEY_NO_CARDS_NOTICE);
    }

    /// `FACT_CARD_WORDING_KEYS` is what the boot check enumerates. A key the
    /// builder reads but the list omits would be missing from `REQUIRED_KEYS` and
    /// would surface as a blank on screen instead of a named boot refusal.
    #[test]
    fn the_key_list_covers_every_field() {
        let w = build_fact_card_wording(echo).expect("infallible read");
        let read_keys = [
            w.proof_label,
            w.backs_label,
            w.supports_label,
            w.watch_out_label,
            w.answer_label,
            w.empty_value,
            w.draft_mark,
            w.context_label,
            w.context_hide_label,
            w.backs_template,
            w.supports_template,
            w.stance_supports_verb,
            w.stance_rebuts_verb,
            w.rfa_template,
            w.rfa_unnumbered_template,
            w.deck_template,
            w.edit_label,
            w.save_label,
            w.cancel_label,
            w.save_failed_template,
            w.no_answer_notice,
            w.accusation_heading,
            w.no_cards_notice,
        ];
        assert_eq!(read_keys.len(), FACT_CARD_WORDING_KEYS.len());
        for key in read_keys {
            assert!(
                FACT_CARD_WORDING_KEYS.contains(&key.as_str()),
                "{key} is read by the builder but missing from FACT_CARD_WORDING_KEYS",
            );
        }
    }

    /// Every key is unique — two fields sharing a row is the same defect as
    /// above, seen from the list's side.
    #[test]
    fn the_keys_are_distinct() {
        let mut sorted = FACT_CARD_WORDING_KEYS.to_vec();
        sorted.sort_unstable();
        let count = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), count, "two fact-card wording keys collide");
    }

    /// There is exactly one verb row per stance this build defines.
    ///
    /// A third stance added to `CardStance` without a verb row would render an
    /// unlabelled Supports line; this says so at `cargo test` time rather than on
    /// a witness's card.
    #[test]
    fn there_is_one_verb_row_per_stance() {
        use crate::domain::fact_card::CardStance;
        let verbs = [KEY_STANCE_SUPPORTS_VERB, KEY_STANCE_REBUTS_VERB];
        assert_eq!(
            verbs.len(),
            CardStance::ALL.len(),
            "every card stance needs exactly one verb wording row",
        );
    }

    /// There is exactly one label row per editable field.
    ///
    /// Same argument as the verbs: a sixth field would otherwise render with no
    /// label at all.
    #[test]
    fn there_is_one_label_row_per_editable_field() {
        use crate::domain::fact_card::CardField;
        // Proof labels the QUOTE, which is not an editable field — it is the
        // record's own words and no human writes it. So the five labels cover the
        // four editable fields that have one, plus Proof.
        let labels = [
            KEY_PROOF_LABEL,
            KEY_BACKS_LABEL,
            KEY_SUPPORTS_LABEL,
            KEY_WATCH_OUT_LABEL,
            KEY_ANSWER_LABEL,
        ];
        assert_eq!(
            labels.len(),
            CardField::EDITABLE.len(),
            "the row labels and the editable fields have diverged: Proof labels \
             the quote, and the other four label an editable field each — a fifth \
             editable field needs a row label of its own",
        );
    }
}
