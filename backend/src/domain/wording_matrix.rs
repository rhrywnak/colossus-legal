// =============================================================================
// backend/src/domain/wording_matrix.rs — the words the PROOF MATRIX speaks
// =============================================================================
//
// The tenth stored-string block (task 396, P1). It carries the sentences the
// Proof Matrix needs now that a row leads with a STRONG count instead of a raw
// one — and nothing else.
//
// ## Why a new block rather than more keys on a sibling
//
// The same test the nine siblings apply: which SURFACE speaks these, and does its
// vocabulary move independently? Every other block belongs to the scenario
// surfaces — curation, rehearsal, authoring, the scan panel, the card grammar —
// which speak to somebody working ONE attack. These speak on the case-wide proof
// grid, to Chuck, about how hard a piece of proof is to dispute. They will move
// when the tier map moves, which has nothing to do with what a candidate card
// says.
//
// ## Domain note: the labels and the MAP are separate rows on purpose
//
// `matrix_tier_*_pairs` decides which extraction pairs are Strong; the three chip
// labels below decide what a human READS for each tier. Same split `fact_tier`
// documents — Roman renames "Cannot be disputed" without touching which items
// earn it, and re-maps a pair without renaming anything.

/// The stored strings the Proof Matrix renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixWording {
    /// The column header over the headline number — the strong count.
    ///
    /// Domain note: this column used to be headed with the word for "how many
    /// items corroborate", which is the raw figure. The column now leads with the
    /// count of proof the other side cannot dispute, so the header is a different
    /// claim and gets its own row rather than being reworded in place.
    pub strong_column_label: String,
    /// The depth line beside the headline. Carries `{count}`.
    ///
    /// This is the "· 15 approved" small print: the raw approved figure, demoted
    /// but never removed. Nothing is hidden — the headline is a narrower claim,
    /// and the number it narrows from stays on screen next to it.
    pub raw_approved_template: String,
    /// The hint on the headline number, shown on hover and read aloud, saying
    /// what "strong" means. A number whose definition is invisible is a number a
    /// reader has to trust rather than check.
    pub strong_hint: String,
    /// The chip on a drill-down row whose pair maps to the strong tier.
    pub tier_strong_chip: String,
    /// The chip for the hedged tier.
    pub tier_hedged_chip: String,
    /// The chip for the other tier.
    pub tier_other_chip: String,
    /// The marker on a row that collapsed near-identical statements. Carries
    /// `{count}`.
    ///
    /// Domain note: rendered ONLY above one occurrence. A row reading "×1" on
    /// every line would be noise, and the count of a group of one is not a fact
    /// worth printing.
    pub duplicate_template: String,
    /// The heading over the ranked drill-down list, said once so a reader knows
    /// the order is a claim and not an accident.
    pub ranked_list_note: String,

    // ── v2: the words the READER-FACING matrix speaks ────────────────────────
    //
    // Everything below arrived with PROOF_MATRIX_v2, when this page stopped
    // being a working surface and became something Chuck opens as a reference.
    // The block grew rather than splitting because it is still ONE surface: the
    // Count buttons, the paragraph lists under them, and the Word document that
    // prints the same lists all move together.
    /// The control opening the items past the short list. Carries `{count}`.
    pub more_template: String,
    /// The toggle revealing put-aside and removed items. Carries `{count}`.
    pub show_hidden_template: String,
    /// The same toggle once the hidden items are showing.
    pub hide_hidden_label: String,
    /// The button confirming an item belongs under this paragraph.
    pub keep_label: String,
    /// The button taking an item out of the paragraph's list.
    pub remove_label: String,
    /// The control beside a hidden item that takes its removal back.
    pub undo_label: String,
    /// The grey mark under a quote nobody has confirmed.
    ///
    /// Domain note: this is the load-bearing one. It is what keeps an unread
    /// machine ranking from reading as a reviewed list, which is the single
    /// claim this page must never make falsely.
    pub machine_label: String,
    /// The green mark under a quote a human confirmed. Carries `{actor}`.
    pub kept_template: String,
    /// The amber mark on an item that both supports and disputes one paragraph.
    pub conflict_label: String,
    /// The confidence mark for `high`.
    pub confidence_high_label: String,
    /// The confidence mark for `medium`.
    pub confidence_medium_label: String,
    /// The confidence mark for `low`.
    ///
    /// Domain note: no edge carries `low` today — the 2026-09-06 pass emitted
    /// only `high` and `medium`. The word exists BEFORE a pass emits one, because
    /// the alternative is that the first low-confidence item renders with no mark
    /// at all, which is exactly how an UNRATED item reads. A reader would be told
    /// nothing instead of being told "low".
    pub confidence_low_label: String,
    /// The mark on an item that predates the linking pass and carries no
    /// confidence at all.
    ///
    /// Domain note: deliberately NOT the same word as `medium`. 250 items were
    /// never rated, and printing a rating over them would invent a judgment.
    pub confidence_unrated_label: String,
    /// How a request-for-admission card reads on one line. Carries `{number}`,
    /// `{request}` and `{answer}`.
    pub rfa_template: String,
    /// The same line when no request number can be derived. Carries `{request}`
    /// and `{answer}`.
    pub rfa_unnumbered_template: String,
    /// The one line under the Count buttons explaining the page's colours.
    pub legend_line: String,
    /// The button that downloads the selected Count as a Word document.
    pub export_button_label: String,
    /// The exported document's title. Carries `{number}` and `{name}`.
    pub export_title_template: String,
    /// The heading over the supporting items in the Word export.
    pub export_supporting_heading: String,
    /// The heading over the disputing items in the Word export.
    pub export_disputing_heading: String,
    /// The mark printed in front of a confirmed item in the Word export.
    ///
    /// Stored because the FOOTER names it — "items marked ✓ confirmed by Roman" —
    /// and a mark compiled into the renderer while its explanation lives in a
    /// stored row is two halves of one sentence free to drift apart.
    ///
    /// Carries NO trailing space, and could not: the store trims every text value
    /// on read. The renderer supplies the space that joins it to the quote.
    pub export_confirmed_mark: String,
    /// Printed under a heading with nothing beneath it.
    pub export_empty_line: String,
    /// The footer of every exported Count. Carries `{date}`.
    pub export_footer_template: String,
    /// Shown when a Keep, Remove or Undo could not be written. Carries
    /// `{detail}`.
    pub ruling_failed_template: String,
    /// Shown under a paragraph with no supporting evidence.
    ///
    /// Domain note: worded separately from its disputing twin on purpose. A
    /// paragraph with nothing on either side would otherwise say the same thing
    /// twice, and the two gaps mean genuinely different things — our proof is
    /// missing, versus theirs is.
    pub no_supporting_line: String,
    /// Shown under a paragraph with no disputing evidence.
    pub no_disputing_line: String,
}

// KEYS: the stable identifiers. Renaming one is a migration, and until it runs
// the boot loader refuses to start.
pub(crate) const KEY_STRONG_COLUMN_LABEL: &str = "matrix_strong_column_label";
pub(crate) const KEY_RAW_APPROVED_TEMPLATE: &str = "matrix_raw_approved_template";
pub(crate) const KEY_STRONG_HINT: &str = "matrix_strong_hint";
pub(crate) const KEY_TIER_STRONG_CHIP: &str = "matrix_tier_strong_chip";
pub(crate) const KEY_TIER_HEDGED_CHIP: &str = "matrix_tier_hedged_chip";
pub(crate) const KEY_TIER_OTHER_CHIP: &str = "matrix_tier_other_chip";
pub(crate) const KEY_DUPLICATE_TEMPLATE: &str = "matrix_duplicate_template";
pub(crate) const KEY_RANKED_LIST_NOTE: &str = "matrix_ranked_list_note";
// v2 KEYS: the reader-facing surface (PROOF_MATRIX_v2 §3 and §4).
pub(crate) const KEY_MORE_TEMPLATE: &str = "matrix_more_template";
pub(crate) const KEY_SHOW_HIDDEN_TEMPLATE: &str = "matrix_show_hidden_template";
pub(crate) const KEY_HIDE_HIDDEN_LABEL: &str = "matrix_hide_hidden_label";
pub(crate) const KEY_KEEP_LABEL: &str = "matrix_keep_label";
pub(crate) const KEY_REMOVE_LABEL: &str = "matrix_remove_label";
pub(crate) const KEY_UNDO_LABEL: &str = "matrix_undo_label";
pub(crate) const KEY_MACHINE_LABEL: &str = "matrix_machine_label";
pub(crate) const KEY_KEPT_TEMPLATE: &str = "matrix_kept_template";
pub(crate) const KEY_CONFLICT_LABEL: &str = "matrix_conflict_label";
pub(crate) const KEY_CONFIDENCE_HIGH_LABEL: &str = "matrix_confidence_high_label";
pub(crate) const KEY_CONFIDENCE_MEDIUM_LABEL: &str = "matrix_confidence_medium_label";
pub(crate) const KEY_CONFIDENCE_LOW_LABEL: &str = "matrix_confidence_low_label";
pub(crate) const KEY_CONFIDENCE_UNRATED_LABEL: &str = "matrix_confidence_unrated_label";
pub(crate) const KEY_RFA_TEMPLATE: &str = "matrix_rfa_template";
pub(crate) const KEY_RFA_UNNUMBERED_TEMPLATE: &str = "matrix_rfa_unnumbered_template";
pub(crate) const KEY_LEGEND_LINE: &str = "matrix_legend_line";
pub(crate) const KEY_EXPORT_BUTTON_LABEL: &str = "matrix_export_button_label";
pub(crate) const KEY_EXPORT_TITLE_TEMPLATE: &str = "matrix_export_title_template";
pub(crate) const KEY_EXPORT_SUPPORTING_HEADING: &str = "matrix_export_supporting_heading";
pub(crate) const KEY_EXPORT_DISPUTING_HEADING: &str = "matrix_export_disputing_heading";
pub(crate) const KEY_EXPORT_CONFIRMED_MARK: &str = "matrix_export_confirmed_mark";
pub(crate) const KEY_EXPORT_EMPTY_LINE: &str = "matrix_export_empty_line";
pub(crate) const KEY_EXPORT_FOOTER_TEMPLATE: &str = "matrix_export_footer_template";
pub(crate) const KEY_RULING_FAILED_TEMPLATE: &str = "matrix_ruling_failed_template";
pub(crate) const KEY_NO_SUPPORTING_LINE: &str = "matrix_no_supporting_line";
pub(crate) const KEY_NO_DISPUTING_LINE: &str = "matrix_no_disputing_line";

/// Every Proof-Matrix key this build reads, so a missing one is caught at boot BY
/// NAME rather than as a blank column header in front of Chuck.
pub const MATRIX_WORDING_KEYS: &[&str] = &[
    KEY_STRONG_COLUMN_LABEL,
    KEY_RAW_APPROVED_TEMPLATE,
    KEY_STRONG_HINT,
    KEY_TIER_STRONG_CHIP,
    KEY_TIER_HEDGED_CHIP,
    KEY_TIER_OTHER_CHIP,
    KEY_DUPLICATE_TEMPLATE,
    KEY_RANKED_LIST_NOTE,
    KEY_MORE_TEMPLATE,
    KEY_SHOW_HIDDEN_TEMPLATE,
    KEY_HIDE_HIDDEN_LABEL,
    KEY_KEEP_LABEL,
    KEY_REMOVE_LABEL,
    KEY_UNDO_LABEL,
    KEY_MACHINE_LABEL,
    KEY_KEPT_TEMPLATE,
    KEY_CONFLICT_LABEL,
    KEY_CONFIDENCE_HIGH_LABEL,
    KEY_CONFIDENCE_MEDIUM_LABEL,
    KEY_CONFIDENCE_LOW_LABEL,
    KEY_CONFIDENCE_UNRATED_LABEL,
    KEY_RFA_TEMPLATE,
    KEY_RFA_UNNUMBERED_TEMPLATE,
    KEY_LEGEND_LINE,
    KEY_EXPORT_BUTTON_LABEL,
    KEY_EXPORT_TITLE_TEMPLATE,
    KEY_EXPORT_SUPPORTING_HEADING,
    KEY_EXPORT_DISPUTING_HEADING,
    KEY_EXPORT_CONFIRMED_MARK,
    KEY_EXPORT_EMPTY_LINE,
    KEY_EXPORT_FOOTER_TEMPLATE,
    KEY_RULING_FAILED_TEMPLATE,
    KEY_NO_SUPPORTING_LINE,
    KEY_NO_DISPUTING_LINE,
];

/// Build a [`MatrixWording`] from the stored rows, or say which key is wrong.
///
/// Same generic-closure shape as the nine sibling builders — see
/// [`crate::domain::wording_model_params::build_model_params_wording`] for why
/// `read` is a closure over a generic error type rather than a database handle.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_matrix_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<MatrixWording, E> {
    Ok(MatrixWording {
        strong_column_label: read(KEY_STRONG_COLUMN_LABEL)?,
        raw_approved_template: read(KEY_RAW_APPROVED_TEMPLATE)?,
        strong_hint: read(KEY_STRONG_HINT)?,
        tier_strong_chip: read(KEY_TIER_STRONG_CHIP)?,
        tier_hedged_chip: read(KEY_TIER_HEDGED_CHIP)?,
        tier_other_chip: read(KEY_TIER_OTHER_CHIP)?,
        duplicate_template: read(KEY_DUPLICATE_TEMPLATE)?,
        ranked_list_note: read(KEY_RANKED_LIST_NOTE)?,
        more_template: read(KEY_MORE_TEMPLATE)?,
        show_hidden_template: read(KEY_SHOW_HIDDEN_TEMPLATE)?,
        hide_hidden_label: read(KEY_HIDE_HIDDEN_LABEL)?,
        keep_label: read(KEY_KEEP_LABEL)?,
        remove_label: read(KEY_REMOVE_LABEL)?,
        undo_label: read(KEY_UNDO_LABEL)?,
        machine_label: read(KEY_MACHINE_LABEL)?,
        kept_template: read(KEY_KEPT_TEMPLATE)?,
        conflict_label: read(KEY_CONFLICT_LABEL)?,
        confidence_high_label: read(KEY_CONFIDENCE_HIGH_LABEL)?,
        confidence_medium_label: read(KEY_CONFIDENCE_MEDIUM_LABEL)?,
        confidence_low_label: read(KEY_CONFIDENCE_LOW_LABEL)?,
        confidence_unrated_label: read(KEY_CONFIDENCE_UNRATED_LABEL)?,
        rfa_template: read(KEY_RFA_TEMPLATE)?,
        rfa_unnumbered_template: read(KEY_RFA_UNNUMBERED_TEMPLATE)?,
        legend_line: read(KEY_LEGEND_LINE)?,
        export_button_label: read(KEY_EXPORT_BUTTON_LABEL)?,
        export_title_template: read(KEY_EXPORT_TITLE_TEMPLATE)?,
        export_supporting_heading: read(KEY_EXPORT_SUPPORTING_HEADING)?,
        export_disputing_heading: read(KEY_EXPORT_DISPUTING_HEADING)?,
        export_confirmed_mark: read(KEY_EXPORT_CONFIRMED_MARK)?,
        export_empty_line: read(KEY_EXPORT_EMPTY_LINE)?,
        export_footer_template: read(KEY_EXPORT_FOOTER_TEMPLATE)?,
        ruling_failed_template: read(KEY_RULING_FAILED_TEMPLATE)?,
        no_supporting_line: read(KEY_NO_SUPPORTING_LINE)?,
        no_disputing_line: read(KEY_NO_DISPUTING_LINE)?,
    })
}

#[cfg(test)]
#[path = "wording_matrix_tests.rs"]
pub(crate) mod seed_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::evidence_tier::EvidenceTier;

    /// A `read` that echoes the key, so the builder's field→key wiring is visible.
    fn echo(key: &str) -> Result<String, std::convert::Infallible> {
        Ok(key.to_string())
    }

    /// Every field reads the key it claims to. A copy-paste that pointed two
    /// fields at one row would put the same sentence in two places on screen with
    /// nothing failing — the defect this echo test exists to catch.
    #[test]
    fn every_field_reads_its_own_key() {
        let w = build_matrix_wording(echo).expect("infallible read");
        assert_eq!(w.strong_column_label, KEY_STRONG_COLUMN_LABEL);
        assert_eq!(w.raw_approved_template, KEY_RAW_APPROVED_TEMPLATE);
        assert_eq!(w.strong_hint, KEY_STRONG_HINT);
        assert_eq!(w.tier_strong_chip, KEY_TIER_STRONG_CHIP);
        assert_eq!(w.tier_hedged_chip, KEY_TIER_HEDGED_CHIP);
        assert_eq!(w.tier_other_chip, KEY_TIER_OTHER_CHIP);
        assert_eq!(w.duplicate_template, KEY_DUPLICATE_TEMPLATE);
        assert_eq!(w.ranked_list_note, KEY_RANKED_LIST_NOTE);
        assert_eq!(w.more_template, KEY_MORE_TEMPLATE);
        assert_eq!(w.show_hidden_template, KEY_SHOW_HIDDEN_TEMPLATE);
        assert_eq!(w.hide_hidden_label, KEY_HIDE_HIDDEN_LABEL);
        assert_eq!(w.keep_label, KEY_KEEP_LABEL);
        assert_eq!(w.remove_label, KEY_REMOVE_LABEL);
        assert_eq!(w.undo_label, KEY_UNDO_LABEL);
        assert_eq!(w.machine_label, KEY_MACHINE_LABEL);
        assert_eq!(w.kept_template, KEY_KEPT_TEMPLATE);
        assert_eq!(w.conflict_label, KEY_CONFLICT_LABEL);
        assert_eq!(w.confidence_high_label, KEY_CONFIDENCE_HIGH_LABEL);
        assert_eq!(w.confidence_medium_label, KEY_CONFIDENCE_MEDIUM_LABEL);
        assert_eq!(w.confidence_low_label, KEY_CONFIDENCE_LOW_LABEL);
        assert_eq!(w.confidence_unrated_label, KEY_CONFIDENCE_UNRATED_LABEL);
        assert_eq!(w.rfa_template, KEY_RFA_TEMPLATE);
        assert_eq!(w.rfa_unnumbered_template, KEY_RFA_UNNUMBERED_TEMPLATE);
        assert_eq!(w.legend_line, KEY_LEGEND_LINE);
        assert_eq!(w.export_button_label, KEY_EXPORT_BUTTON_LABEL);
        assert_eq!(w.export_title_template, KEY_EXPORT_TITLE_TEMPLATE);
        assert_eq!(w.export_supporting_heading, KEY_EXPORT_SUPPORTING_HEADING);
        assert_eq!(w.export_disputing_heading, KEY_EXPORT_DISPUTING_HEADING);
        assert_eq!(w.export_confirmed_mark, KEY_EXPORT_CONFIRMED_MARK);
        assert_eq!(w.export_empty_line, KEY_EXPORT_EMPTY_LINE);
        assert_eq!(w.export_footer_template, KEY_EXPORT_FOOTER_TEMPLATE);
        assert_eq!(w.ruling_failed_template, KEY_RULING_FAILED_TEMPLATE);
        assert_eq!(w.no_supporting_line, KEY_NO_SUPPORTING_LINE);
        assert_eq!(w.no_disputing_line, KEY_NO_DISPUTING_LINE);
    }

    /// `MATRIX_WORDING_KEYS` is what the boot check enumerates. A key the builder
    /// reads but the list omits would be missing from `REQUIRED_KEYS` and would
    /// surface as a blank on screen instead of a named boot refusal.
    #[test]
    fn the_key_list_covers_every_field() {
        let w = build_matrix_wording(echo).expect("infallible read");
        let read_keys = [
            w.strong_column_label,
            w.raw_approved_template,
            w.strong_hint,
            w.tier_strong_chip,
            w.tier_hedged_chip,
            w.tier_other_chip,
            w.duplicate_template,
            w.ranked_list_note,
            w.more_template,
            w.show_hidden_template,
            w.hide_hidden_label,
            w.keep_label,
            w.remove_label,
            w.undo_label,
            w.machine_label,
            w.kept_template,
            w.conflict_label,
            w.confidence_high_label,
            w.confidence_medium_label,
            w.confidence_low_label,
            w.confidence_unrated_label,
            w.rfa_template,
            w.rfa_unnumbered_template,
            w.legend_line,
            w.export_button_label,
            w.export_title_template,
            w.export_supporting_heading,
            w.export_disputing_heading,
            w.export_confirmed_mark,
            w.export_empty_line,
            w.export_footer_template,
            w.ruling_failed_template,
            w.no_supporting_line,
            w.no_disputing_line,
        ];
        assert_eq!(read_keys.len(), MATRIX_WORDING_KEYS.len());
        for key in read_keys {
            assert!(
                MATRIX_WORDING_KEYS.contains(&key.as_str()),
                "{key} is read by the builder but missing from MATRIX_WORDING_KEYS",
            );
        }
    }

    /// Every key is unique — two fields sharing a row is the same defect as
    /// above, seen from the list's side.
    #[test]
    fn the_keys_are_distinct() {
        let mut sorted = MATRIX_WORDING_KEYS.to_vec();
        sorted.sort_unstable();
        let count = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), count, "two matrix wording keys collide");
    }

    /// There is exactly one chip row per tier this build defines. A fourth tier
    /// added to `EvidenceTier` without a chip row would render an unlabelled chip;
    /// this is the test that says so at `cargo test` time rather than on screen.
    #[test]
    fn there_is_one_chip_row_per_tier() {
        let chip_keys = [
            KEY_TIER_STRONG_CHIP,
            KEY_TIER_HEDGED_CHIP,
            KEY_TIER_OTHER_CHIP,
        ];
        assert_eq!(
            chip_keys.len(),
            EvidenceTier::ALL.len(),
            "every evidence tier needs exactly one chip wording row",
        );
    }
}
