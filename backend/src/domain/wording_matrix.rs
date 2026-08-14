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
