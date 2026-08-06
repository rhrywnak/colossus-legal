// =============================================================================
// backend/src/domain/wording_accusation.rs — the accusation section's words
// =============================================================================
//
// Task 2.11 B1, under Roman's 2026-08-04 ruling (the configuration law extended
// from numbers to TEXT) as restated by REHEARSAL_VIEW_DESIGN_v2:
//
//   "every heading, label, gap message, and the Always card text on this page is
//    served from the settings store with a plain description. No literal
//    user-facing string in code."
//
// So there is not one user-facing literal below outside the `cfg(test)` fixture.
// Every field of [`AccusationWording`] is an `app_settings` row read at boot, and
// a missing or blank one is a BOOT REFUSAL rather than a quietly empty button.
//
// ## Why a second module rather than twenty-seven more fields on `Wording`
//
// Two reasons, and the first is the one that would have bitten.
//
// `domain::wording` is already the largest module in `domain/`, and Rule 17 caps
// a module at 300 lines. Twenty-seven more fields, twenty-seven more keys, and
// twenty-seven more arms of `for_test_values` would have added roughly two hundred
// lines to a file that has no room for them.
//
// The second is the argument `Wording` itself makes for nesting inside
// `Settings`: these are the words ONE surface speaks. A reader looking for the
// link panel's refusal should not have to scroll past the accusation section's
// gap messages to find it, and the two sets change for entirely different
// reasons.
//
// What is deliberately NOT duplicated is the placeholder table. `wording::
// REQUIRED_PLACEHOLDERS` stays the one place the write path looks, with this
// module's keys listed in it — a second table would be a second lookup that can
// silently miss, which is the exact failure that table exists to prevent.

// Only the test fixture below returns a map; production code holds the struct.
#[cfg(test)]
use std::collections::HashMap;

/// Every stored string the working view's accusation section renders.
///
/// ## Rust Learning: why this is a struct and not a `HashMap<String, String>`
///
/// The same parse-don't-validate argument `domain::wording` opens with. A map
/// makes every consumer a lookup that can miss — `words.get("accusation_mark_
/// label").unwrap_or("")` is a button with no label, produced at the moment of
/// use, with nothing in the log. Building a struct at boot moves every one of
/// those failures to startup, where the refusal names the key. Once you hold an
/// `AccusationWording` you know all twenty-seven strings are present and
/// non-blank, because the only way to build one checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccusationWording {
    /// The section heading on the working view.
    pub section_heading: String,
    /// The line that tells a reader the marking and the pairing exist.
    pub section_hint: String,
    /// Labels the authored plain-words accusation.
    pub text_label: String,
    /// The grey prompt inside the empty accusation box.
    pub text_placeholder: String,
    /// Shown in place of the accusation when nobody has written one.
    pub text_missing_notice: String,
    /// Opens the accusation for editing.
    pub text_edit_label: String,
    /// Stores the accusation.
    pub text_save_label: String,
    /// Clears it — withdrawing a sentence is a real act with a real control.
    pub text_clear_label: String,
    /// Abandons an edit.
    pub text_cancel_label: String,
    /// The count line. Carries `{times}` and `{documents}`.
    pub count_template: String,
    /// Shown when nothing is marked. Carries `{included}`.
    pub no_instances_notice: String,
    /// Opens the picker that marks an instance.
    pub mark_label: String,
    /// Withdraws a marking.
    pub unmark_label: String,
    /// Opens the picker that pairs an answer.
    pub pair_label: String,
    /// Re-pairs an instance that already has an answer.
    pub repair_label: String,
    /// Removes a pairing.
    pub unpair_label: String,
    /// Marks the paired answer beneath its instance.
    pub answer_label: String,
    /// The prompt over the picker, and a statement of its limit.
    pub picker_prompt: String,
    /// Closes the picker without choosing.
    pub picker_cancel_label: String,
    /// Shown when the picker's filter leaves nothing.
    pub picker_no_match_notice: String,
    /// Shown when the picker has no candidates at all — a different state from
    /// the one above, with a different remedy, so a different sentence.
    pub picker_empty_notice: String,
    /// The heading over the gap list.
    pub gaps_heading: String,
    /// Shown when the gap list is empty — an empty list and an unloaded one must
    /// not look the same.
    pub no_gaps_notice: String,
    /// The prep list's own line. Carries `{code}`.
    pub gap_no_answer: String,
    /// The Remove law, accusation side. Carries `{code}`.
    pub gap_accusation_removed: String,
    /// The Remove law, answer side. Carries `{code}`.
    pub gap_answer_removed: String,
    /// Said when a write on this surface failed. Carries `{detail}`.
    pub save_failed_template: String,
}

// KEYS: the stable identifiers of the twenty-seven stored strings. Not tunables —
// the NAMES of tunables, in the same category as a column name. Renaming one is a
// migration, and until that migration runs the boot loader refuses to start
// rather than guessing.
pub(crate) const KEY_SECTION_HEADING: &str = "accusation_section_heading";
pub(crate) const KEY_SECTION_HINT: &str = "accusation_section_hint";
pub(crate) const KEY_TEXT_LABEL: &str = "accusation_text_label";
pub(crate) const KEY_TEXT_PLACEHOLDER: &str = "accusation_text_placeholder";
pub(crate) const KEY_TEXT_MISSING_NOTICE: &str = "accusation_text_missing_notice";
pub(crate) const KEY_TEXT_EDIT_LABEL: &str = "accusation_text_edit_label";
pub(crate) const KEY_TEXT_SAVE_LABEL: &str = "accusation_text_save_label";
pub(crate) const KEY_TEXT_CLEAR_LABEL: &str = "accusation_text_clear_label";
pub(crate) const KEY_TEXT_CANCEL_LABEL: &str = "accusation_text_cancel_label";
pub const KEY_COUNT_TEMPLATE: &str = "accusation_count_template";
pub const KEY_NO_INSTANCES_NOTICE: &str = "accusation_no_instances_notice";
pub(crate) const KEY_MARK_LABEL: &str = "accusation_mark_label";
pub(crate) const KEY_UNMARK_LABEL: &str = "accusation_unmark_label";
pub(crate) const KEY_PAIR_LABEL: &str = "accusation_pair_label";
pub(crate) const KEY_REPAIR_LABEL: &str = "accusation_repair_label";
pub(crate) const KEY_UNPAIR_LABEL: &str = "accusation_unpair_label";
pub(crate) const KEY_ANSWER_LABEL: &str = "accusation_answer_label";
pub(crate) const KEY_PICKER_PROMPT: &str = "accusation_picker_prompt";
pub(crate) const KEY_PICKER_CANCEL_LABEL: &str = "accusation_picker_cancel_label";
pub(crate) const KEY_PICKER_NO_MATCH: &str = "accusation_picker_no_match_notice";
pub(crate) const KEY_PICKER_EMPTY: &str = "accusation_picker_empty_notice";
pub(crate) const KEY_GAPS_HEADING: &str = "accusation_gaps_heading";
pub(crate) const KEY_NO_GAPS_NOTICE: &str = "accusation_no_gaps_notice";
pub const KEY_GAP_NO_ANSWER: &str = "accusation_gap_no_answer";
pub const KEY_GAP_ACCUSATION_REMOVED: &str = "accusation_gap_accusation_removed";
pub const KEY_GAP_ANSWER_REMOVED: &str = "accusation_gap_answer_removed";
pub const KEY_SAVE_FAILED_TEMPLATE: &str = "accusation_save_failed_template";

/// Every accusation wording key this build reads, so a missing one is caught at
/// boot BY NAME.
///
/// A third counted list beside `settings_store::REQUIRED_KEYS` (the parameters
/// that decide how the system judges) and `wording::WORDING_KEYS` (the words the
/// curation surfaces speak). Three counted lists tell a boot log more than one
/// number of sixty-odd does — `parameters=8 wording=48 accusation=25` names which
/// half of a half-run seed is missing.
pub const ACCUSATION_WORDING_KEYS: &[&str] = &[
    KEY_SECTION_HEADING,
    KEY_SECTION_HINT,
    KEY_TEXT_LABEL,
    KEY_TEXT_PLACEHOLDER,
    KEY_TEXT_MISSING_NOTICE,
    KEY_TEXT_EDIT_LABEL,
    KEY_TEXT_SAVE_LABEL,
    KEY_TEXT_CLEAR_LABEL,
    KEY_TEXT_CANCEL_LABEL,
    KEY_COUNT_TEMPLATE,
    KEY_NO_INSTANCES_NOTICE,
    KEY_MARK_LABEL,
    KEY_UNMARK_LABEL,
    KEY_PAIR_LABEL,
    KEY_REPAIR_LABEL,
    KEY_UNPAIR_LABEL,
    KEY_ANSWER_LABEL,
    KEY_PICKER_PROMPT,
    KEY_PICKER_CANCEL_LABEL,
    KEY_PICKER_NO_MATCH,
    KEY_PICKER_EMPTY,
    KEY_GAPS_HEADING,
    KEY_NO_GAPS_NOTICE,
    KEY_GAP_NO_ANSWER,
    KEY_GAP_ACCUSATION_REMOVED,
    KEY_GAP_ANSWER_REMOVED,
    KEY_SAVE_FAILED_TEMPLATE,
];

/// Build an [`AccusationWording`] from the stored rows, or say which key is wrong.
///
/// `read` is supplied by the settings store, which owns how a row is validated
/// (declared kind, non-blank) and what an error looks like. Passing it in keeps
/// this module free of any dependency on the store — it knows the KEYS and the
/// SHAPE, and nothing about databases. Same signature and same reasoning as
/// `wording::build_wording`; see its `## Rust Learning` note on taking a closure
/// that can fail.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_accusation_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<AccusationWording, E> {
    Ok(AccusationWording {
        section_heading: read(KEY_SECTION_HEADING)?,
        section_hint: read(KEY_SECTION_HINT)?,
        text_label: read(KEY_TEXT_LABEL)?,
        text_placeholder: read(KEY_TEXT_PLACEHOLDER)?,
        text_missing_notice: read(KEY_TEXT_MISSING_NOTICE)?,
        text_edit_label: read(KEY_TEXT_EDIT_LABEL)?,
        text_save_label: read(KEY_TEXT_SAVE_LABEL)?,
        text_clear_label: read(KEY_TEXT_CLEAR_LABEL)?,
        text_cancel_label: read(KEY_TEXT_CANCEL_LABEL)?,
        count_template: read(KEY_COUNT_TEMPLATE)?,
        no_instances_notice: read(KEY_NO_INSTANCES_NOTICE)?,
        mark_label: read(KEY_MARK_LABEL)?,
        unmark_label: read(KEY_UNMARK_LABEL)?,
        pair_label: read(KEY_PAIR_LABEL)?,
        repair_label: read(KEY_REPAIR_LABEL)?,
        unpair_label: read(KEY_UNPAIR_LABEL)?,
        answer_label: read(KEY_ANSWER_LABEL)?,
        picker_prompt: read(KEY_PICKER_PROMPT)?,
        picker_cancel_label: read(KEY_PICKER_CANCEL_LABEL)?,
        picker_no_match_notice: read(KEY_PICKER_NO_MATCH)?,
        picker_empty_notice: read(KEY_PICKER_EMPTY)?,
        gaps_heading: read(KEY_GAPS_HEADING)?,
        no_gaps_notice: read(KEY_NO_GAPS_NOTICE)?,
        gap_no_answer: read(KEY_GAP_NO_ANSWER)?,
        gap_accusation_removed: read(KEY_GAP_ACCUSATION_REMOVED)?,
        gap_answer_removed: read(KEY_GAP_ANSWER_REMOVED)?,
        save_failed_template: read(KEY_SAVE_FAILED_TEMPLATE)?,
    })
}

/// The seeded wording, for TESTS ONLY.
///
/// ## Why this is `#[cfg(test)]`, exactly like `Wording::for_test`
///
/// A production-reachable `Default` would be a compiled-in set of user-facing
/// strings — the precise defect Roman's ruling deletes — and it would be
/// reachable by accident through `unwrap_or_default()`. Gating it on `cfg(test)`
/// means it cannot exist in a release binary at all.
///
/// The values match the migration's seed, and the test below asserts that against
/// the migration FILE, so the two cannot drift silently.
#[cfg(test)]
impl AccusationWording {
    pub fn for_test() -> Self {
        AccusationWording {
            section_heading: "The accusation, and every time they made it".to_string(),
            section_hint: "Write the accusation in plain words. Then mark each included \
                           fact that IS them making it, and pair the fact that answers \
                           it. Whatever is left unpaired is your prep list."
                .to_string(),
            text_label: "In plain words".to_string(),
            text_placeholder: "They say Marie was unreasonable and refused to divide the \
                               property."
                .to_string(),
            text_missing_notice: "Nobody has written this accusation in plain words yet. \
                                  Until somebody does, the rehearsal page names the gap \
                                  rather than standing a quote in for it."
                .to_string(),
            text_edit_label: "Edit".to_string(),
            text_save_label: "Save".to_string(),
            text_clear_label: "Withdraw it".to_string(),
            text_cancel_label: "Cancel".to_string(),
            count_template: "Said {times} times, in {documents} documents.".to_string(),
            no_instances_notice: "No instances marked yet. {included} included facts are \
                                  waiting here."
                .to_string(),
            mark_label: "Mark an instance".to_string(),
            unmark_label: "Not an instance".to_string(),
            pair_label: "Pair our answer".to_string(),
            repair_label: "Change our answer".to_string(),
            unpair_label: "Unpair".to_string(),
            answer_label: "Our answer:".to_string(),
            picker_prompt: "Choose one of this scenario's included facts.".to_string(),
            picker_cancel_label: "Never mind".to_string(),
            picker_no_match_notice: "No included fact matches what you typed.".to_string(),
            picker_empty_notice: "There is nothing left to choose — every included fact \
                                  is already used here."
                .to_string(),
            gaps_heading: "What still needs preparing".to_string(),
            no_gaps_notice: "Every instance marked here has an answer paired to it.".to_string(),
            gap_no_answer: "NO ANSWER PREPARED — {code}".to_string(),
            gap_accusation_removed: "{code} has an answer paired to it but is no longer in \
                                     this scenario. The pairing is kept — nothing a human \
                                     decided disappears quietly."
                .to_string(),
            gap_answer_removed: "The answer paired to {code} is no longer in this scenario. \
                                 The pairing is kept, and it needs a new answer."
                .to_string(),
            save_failed_template: "That did not save: {detail} Reload the page to see what \
                                   is actually stored."
                .to_string(),
        }
    }

    /// The test fixture as a key→value map, in the shape the store reads.
    ///
    /// Lets `settings_store_tests` extend its seeded fixture without holding a
    /// second hand-typed copy of twenty-seven sentences — the copy that matters is
    /// [`AccusationWording::for_test`], and the test below pins THAT to the
    /// migration.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        let w = AccusationWording::for_test();
        ACCUSATION_WORDING_KEYS
            .iter()
            .map(|key| {
                let value = match *key {
                    KEY_SECTION_HEADING => w.section_heading.clone(),
                    KEY_SECTION_HINT => w.section_hint.clone(),
                    KEY_TEXT_LABEL => w.text_label.clone(),
                    KEY_TEXT_PLACEHOLDER => w.text_placeholder.clone(),
                    KEY_TEXT_MISSING_NOTICE => w.text_missing_notice.clone(),
                    KEY_TEXT_EDIT_LABEL => w.text_edit_label.clone(),
                    KEY_TEXT_SAVE_LABEL => w.text_save_label.clone(),
                    KEY_TEXT_CLEAR_LABEL => w.text_clear_label.clone(),
                    KEY_TEXT_CANCEL_LABEL => w.text_cancel_label.clone(),
                    KEY_COUNT_TEMPLATE => w.count_template.clone(),
                    KEY_NO_INSTANCES_NOTICE => w.no_instances_notice.clone(),
                    KEY_MARK_LABEL => w.mark_label.clone(),
                    KEY_UNMARK_LABEL => w.unmark_label.clone(),
                    KEY_PAIR_LABEL => w.pair_label.clone(),
                    KEY_REPAIR_LABEL => w.repair_label.clone(),
                    KEY_UNPAIR_LABEL => w.unpair_label.clone(),
                    KEY_ANSWER_LABEL => w.answer_label.clone(),
                    KEY_PICKER_PROMPT => w.picker_prompt.clone(),
                    KEY_PICKER_CANCEL_LABEL => w.picker_cancel_label.clone(),
                    KEY_PICKER_NO_MATCH => w.picker_no_match_notice.clone(),
                    KEY_PICKER_EMPTY => w.picker_empty_notice.clone(),
                    KEY_GAPS_HEADING => w.gaps_heading.clone(),
                    KEY_NO_GAPS_NOTICE => w.no_gaps_notice.clone(),
                    KEY_GAP_NO_ANSWER => w.gap_no_answer.clone(),
                    KEY_GAP_ACCUSATION_REMOVED => w.gap_accusation_removed.clone(),
                    KEY_GAP_ANSWER_REMOVED => w.gap_answer_removed.clone(),
                    KEY_SAVE_FAILED_TEMPLATE => w.save_failed_template.clone(),
                    // Unreachable by construction: the match covers every entry of
                    // ACCUSATION_WORDING_KEYS, and the test below proves it. A
                    // panic here fires only in a test binary, on a key someone
                    // added to the list and forgot here — exactly when a loud stop
                    // is what you want.
                    other => {
                        panic!("{other} is in ACCUSATION_WORDING_KEYS but not in for_test_values")
                    }
                };
                (*key, value)
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "wording_accusation_tests.rs"]
mod tests;
