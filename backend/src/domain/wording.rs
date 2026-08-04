// =============================================================================
// backend/src/domain/wording.rs — the words this task puts on screen, from store
// =============================================================================
//
// Task 2.10, and Roman's ruling of 2026-08-04: THE CONFIGURATION LAW, EXTENDED
// FROM NUMBERS TO TEXT.
//
//   "Every heading, prompt, label, button and message this task introduces is
//    served from the settings store with a default and a plain-language
//    description, and is editable on the admin Settings page. A literal
//    user-facing string in this task's code is a defect of the same class as a
//    compiled-in threshold."
//
// So there is not one user-facing literal below. Every field of [`Wording`] is
// read from an `app_settings` row at boot, exactly as the band cutoffs are, and
// a missing or blank one is a BOOT REFUSAL rather than a quietly empty button.
//
// ## Why a struct and not a `HashMap<String, String>` handed to the browser
//
// A map would compile, and it would make every consumer a lookup that can miss —
// `wording.get("link_save_label").unwrap_or("")` is a blank button, produced at
// the moment of use, with nothing in the log. Parsing the store into a struct
// moves every one of those failures to boot, where it names the key. That is the
// same parse-don't-validate argument `domain::settings` opens with, applied to
// text: once you hold a `Wording` you know all twenty strings are present and
// non-blank, because the only way to build one checked.
//
// ## Domain note: sentence forms are STORED, never derived
//
// "It supports us" is a button; "it supports us" is the middle of a sentence.
// They are two rows, not one row and a `to_lowercase()`. Lowercasing a human's
// words in code is the frontend-composing-prose defect wearing a server costume —
// it would also be wrong the first time a label starts with a proper noun.

// Only the test fixture below returns a map; production code holds the struct.
#[cfg(test)]
use std::collections::HashMap;

use crate::domain::settings::{parse_text, SettingError};

/// Every stored string this task's surfaces render.
///
/// ## Rust Learning: why this struct is `Clone` but not `Copy`
///
/// A `String` owns heap memory, so copying one is a real allocation and the
/// compiler will not let a type containing one be `Copy`. `Settings` was `Copy`
/// before this task for exactly the opposite reason — it held only numbers. The
/// derive came off when this struct went in; see the note on [`crate::domain::settings::Settings`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wording {
    /// Why this card cannot be ruled, and what to do about it.
    pub link_panel_intro: String,
    /// That a link is case-wide, said before anyone commits to one.
    pub link_scope_notice: String,
    /// The heading over the accusation checkboxes.
    pub link_allegations_heading: String,
    /// The full-list button. Carries `{count}`.
    pub link_show_all_label: String,
    /// The filter box's grey prompt.
    pub link_filter_placeholder: String,
    /// Shown when the filter leaves nothing.
    pub link_no_match_notice: String,
    /// Shown when the case has no accusations at all — a different state from the
    /// one above, and the two must not share a sentence (Standing Rule 1).
    pub link_empty_options_notice: String,
    /// The heading over the two cut buttons.
    pub link_cut_heading: String,
    /// The favourable cut, as a button.
    pub link_cut_supports_label: String,
    /// The unfavourable cut, as a button.
    pub link_cut_against_label: String,
    /// The favourable cut, as it reads inside a sentence.
    pub link_cut_supports_phrase: String,
    /// The unfavourable cut, as it reads inside a sentence.
    pub link_cut_against_phrase: String,
    /// The save-and-advance button.
    pub link_save_label: String,
    /// The close-without-saving button.
    pub link_cancel_label: String,
    /// The one-click withdrawal.
    pub link_unlink_label: String,
    /// Refusal: no cut chosen.
    pub link_missing_cut_refusal: String,
    /// Refusal: nothing ticked.
    pub link_missing_allegation_refusal: String,
    /// The card's after-the-fact sentence. Carries `{allegations}` and `{cut}`.
    pub link_summary_template: String,
    /// The running count. Carries `{linked}` and `{total}`.
    pub link_progress_template: String,
    /// The control that restores a machine-written question (the 1.7F fold-in).
    pub question_revert_label: String,
    /// Said when Unlink removed nothing because the pair was not linked.
    pub link_unlink_found_nothing: String,
    /// Said when a link or unlink could not be written. Carries `{detail}`.
    pub link_save_failed_template: String,
    /// Why Include and Exclude are greyed while a link panel holds unsaved
    /// choices (task 2.12, item B).
    pub link_save_blocks_ruling: String,
    /// The control that takes a fact back out of a scenario (item G).
    pub fact_remove_label: String,
    /// The question asked before a fact is taken out. Carries `{code}`.
    pub fact_remove_confirm_template: String,
    /// The button that confirms a removal.
    pub fact_remove_confirm_yes: String,
    /// The button that abandons it.
    pub fact_remove_confirm_cancel: String,
    /// Said when a removal could not be written. Carries `{detail}`.
    pub fact_remove_failed_template: String,
}

// KEYS: the stable identifiers of the twenty stored strings. Not tunables — the
// NAMES of tunables, in the same category as a column name. Renaming one is a
// migration, and until that migration runs the boot loader refuses to start
// rather than guessing.
pub(crate) const KEY_PANEL_INTRO: &str = "link_panel_intro";
pub(crate) const KEY_SCOPE_NOTICE: &str = "link_scope_notice";
pub(crate) const KEY_ALLEGATIONS_HEADING: &str = "link_allegations_heading";
pub(crate) const KEY_SHOW_ALL_LABEL: &str = "link_show_all_label";
pub(crate) const KEY_FILTER_PLACEHOLDER: &str = "link_filter_placeholder";
pub(crate) const KEY_NO_MATCH_NOTICE: &str = "link_no_match_notice";
pub(crate) const KEY_EMPTY_OPTIONS_NOTICE: &str = "link_empty_options_notice";
pub(crate) const KEY_CUT_HEADING: &str = "link_cut_heading";
pub(crate) const KEY_CUT_SUPPORTS_LABEL: &str = "link_cut_supports_label";
pub(crate) const KEY_CUT_AGAINST_LABEL: &str = "link_cut_against_label";
pub(crate) const KEY_CUT_SUPPORTS_PHRASE: &str = "link_cut_supports_phrase";
pub(crate) const KEY_CUT_AGAINST_PHRASE: &str = "link_cut_against_phrase";
pub(crate) const KEY_SAVE_LABEL: &str = "link_save_label";
pub(crate) const KEY_CANCEL_LABEL: &str = "link_cancel_label";
pub(crate) const KEY_UNLINK_LABEL: &str = "link_unlink_label";
pub(crate) const KEY_MISSING_CUT_REFUSAL: &str = "link_missing_cut_refusal";
pub(crate) const KEY_MISSING_ALLEGATION_REFUSAL: &str = "link_missing_allegation_refusal";
pub(crate) const KEY_SUMMARY_TEMPLATE: &str = "link_summary_template";
pub(crate) const KEY_PROGRESS_TEMPLATE: &str = "link_progress_template";
pub(crate) const KEY_QUESTION_REVERT_LABEL: &str = "question_revert_label";
pub(crate) const KEY_UNLINK_FOUND_NOTHING: &str = "link_unlink_found_nothing";
pub(crate) const KEY_SAVE_FAILED_TEMPLATE: &str = "link_save_failed_template";
pub(crate) const KEY_SAVE_BLOCKS_RULING: &str = "link_save_blocks_ruling";
pub(crate) const KEY_FACT_REMOVE_LABEL: &str = "fact_remove_label";
pub(crate) const KEY_FACT_REMOVE_CONFIRM: &str = "fact_remove_confirm_template";
pub(crate) const KEY_FACT_REMOVE_YES: &str = "fact_remove_confirm_yes";
pub(crate) const KEY_FACT_REMOVE_CANCEL: &str = "fact_remove_confirm_cancel";
pub(crate) const KEY_FACT_REMOVE_FAILED: &str = "fact_remove_failed_template";

/// Every wording key this build reads, so a missing one is caught at boot by name.
///
/// The text half of `settings_store::REQUIRED_KEYS`, kept separate because the
/// two answer different questions — that list is the parameters that decide how
/// the system JUDGES, this one is the words it SPEAKS — and because one flat list
/// of twenty-seven would tell a boot log less than two counted lists do.
pub const WORDING_KEYS: &[&str] = &[
    KEY_PANEL_INTRO,
    KEY_SCOPE_NOTICE,
    KEY_ALLEGATIONS_HEADING,
    KEY_SHOW_ALL_LABEL,
    KEY_FILTER_PLACEHOLDER,
    KEY_NO_MATCH_NOTICE,
    KEY_EMPTY_OPTIONS_NOTICE,
    KEY_CUT_HEADING,
    KEY_CUT_SUPPORTS_LABEL,
    KEY_CUT_AGAINST_LABEL,
    KEY_CUT_SUPPORTS_PHRASE,
    KEY_CUT_AGAINST_PHRASE,
    KEY_SAVE_LABEL,
    KEY_CANCEL_LABEL,
    KEY_UNLINK_LABEL,
    KEY_MISSING_CUT_REFUSAL,
    KEY_MISSING_ALLEGATION_REFUSAL,
    KEY_SUMMARY_TEMPLATE,
    KEY_PROGRESS_TEMPLATE,
    KEY_QUESTION_REVERT_LABEL,
    KEY_UNLINK_FOUND_NOTHING,
    KEY_SAVE_FAILED_TEMPLATE,
    KEY_SAVE_BLOCKS_RULING,
    KEY_FACT_REMOVE_LABEL,
    KEY_FACT_REMOVE_CONFIRM,
    KEY_FACT_REMOVE_YES,
    KEY_FACT_REMOVE_CANCEL,
    KEY_FACT_REMOVE_FAILED,
];

/// The placeholders a stored string MUST still contain after a human edits it.
///
/// ## Why this is enforced rather than trusted
///
/// `link_summary_template` reads "You linked this to {allegations} · {cut}." An
/// edit that dropped `{allegations}` would produce "You linked this to  · they'll
/// use it against us." — a grammatical sentence with the FACT removed, on a
/// surface a lawyer reads. Nothing downstream could detect that, because a
/// template with no placeholder renders perfectly well.
///
/// So the write path checks this table before storing, and refuses with the
/// missing names. Keys absent from this table have no required placeholders,
/// which is most of them.
pub const REQUIRED_PLACEHOLDERS: &[(&str, &[&str])] = &[
    (KEY_SHOW_ALL_LABEL, &["{count}"]),
    (KEY_SUMMARY_TEMPLATE, &["{allegations}", "{cut}"]),
    (KEY_PROGRESS_TEMPLATE, &["{linked}", "{total}"]),
    (KEY_SAVE_FAILED_TEMPLATE, &["{detail}"]),
    // A confirmation that does not name what it is about is not a confirmation.
    (KEY_FACT_REMOVE_CONFIRM, &["{code}"]),
    (KEY_FACT_REMOVE_FAILED, &["{detail}"]),
];

/// Which required placeholders a candidate value is missing, for one key.
///
/// Returns an empty `Vec` for a key with no requirements and for a value that
/// satisfies all of them — so the caller's check is `is_empty()` either way, with
/// no special case for "this key is unconstrained".
///
/// ## Rust Learning: returning owned `&'static str`s, not a bool
///
/// A `bool` would force the caller to re-derive WHICH placeholder was missing in
/// order to say so, and the refusal has to name them (Standing Rule 1: a failure
/// a reader cannot diagnose is incomplete). The items are `&'static str` because
/// they come from the table above, which lives for the whole program — no
/// lifetime annotation is needed on the signature for the same reason.
pub fn missing_placeholders(key: &str, value: &str) -> Vec<&'static str> {
    REQUIRED_PLACEHOLDERS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, required)| {
            required
                .iter()
                .filter(|token| !value.contains(**token))
                .copied()
                .collect()
        })
        .unwrap_or_default()
}

/// Check a proposed wording value: non-blank, and still carrying its facts.
///
/// The write path's whole rule for a `text` parameter, in one place. Lives here
/// rather than in the settings store because both halves are statements about
/// WORDS — what a label may not be, and what a template must keep — and because
/// the store is at its module-size limit.
///
/// # Errors
/// Returns [`SettingError::Blank`] for an empty value, or
/// [`SettingError::MissingPlaceholders`] naming every placeholder an edit dropped.
pub fn validate_wording_candidate(key: &str, candidate: &str) -> Result<(), SettingError> {
    let text = parse_text(key, candidate)?;

    // The rule a parse cannot catch: "You linked this to {allegations}" edited to
    // "You linked this to it" is well-formed text AND a sentence with the fact
    // removed. Nothing downstream could tell, because a template with no
    // placeholder renders perfectly.
    let missing = missing_placeholders(key, &text);
    if !missing.is_empty() {
        return Err(SettingError::MissingPlaceholders {
            key: key.to_string(),
            missing: missing.join(", "),
        });
    }
    Ok(())
}

/// Fill a stored template's `{placeholders}` from a list of (name, value) pairs.
///
/// ## Why this is a plain scan and not a templating crate
///
/// The whole vocabulary is three placeholders across two templates. A dependency
/// with an expression language would let a stored string reach for data this
/// module never meant to expose, which is a surface no configuration value should
/// have. Substring replacement of an exact `{name}` cannot.
///
/// ## What happens to a placeholder nobody supplied
///
/// Nothing — it stays on screen as `{name}`, visibly wrong. That is deliberate,
/// and it is the honest end of a chain whose other end is
/// [`missing_placeholders`]: a template can only lose a placeholder through an
/// edit the write path refuses, so a `{name}` reaching a screen means the store
/// was edited around the API (a `psql` UPDATE), and showing the token is how a
/// reader finds out. Substituting an empty string instead would produce a
/// confident sentence with a hole in it.
/// ## Rust Learning: why this is ONE scan and not a `replace` per placeholder
///
/// The obvious implementation is `for (name, value) { out = out.replace(…) }`.
/// It has a real bug: a value substituted by an early pass is re-scanned by every
/// later one. An accusation whose own text contains `{cut}` — arbitrary human
/// prose from a complaint — would have that chewed out of it. Walking the
/// TEMPLATE once and emitting values as it goes means a substituted value is
/// never looked at again.
pub fn render(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];

        let Some(close) = after.find('}') else {
            // An unclosed brace is not a placeholder. Emit it and stop scanning —
            // the remainder is literal text.
            out.push_str(&rest[open..]);
            return out;
        };

        let name = &after[..close];
        match values.iter().find(|(key, _)| *key == name) {
            Some((_, value)) => out.push_str(value),
            // Unknown: emit the token verbatim. See the doc above for why this is
            // deliberate rather than substituting an empty string.
            None => out.push_str(&rest[open..open + close + 2]),
        }
        rest = &after[close + 1..];
    }

    out.push_str(rest);
    out
}

/// Build a [`Wording`] from the stored rows, or say precisely which key is wrong.
///
/// `read` is supplied by the settings store, which owns how a row is validated
/// (declared kind, non-blank) and what error it raises. Passing it in keeps this
/// module free of any dependency on the store — it knows the KEYS and the SHAPE,
/// and nothing about databases.
///
/// ## Rust Learning: taking a closure that can fail
///
/// `read: impl Fn(&str) -> Result<String, E>` lets the caller decide both how a
/// value is fetched and what an error looks like, while `?` inside this function
/// still short-circuits on the first bad key. The generic `E` means this compiles
/// against any error type; the store passes its own.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_wording<E>(read: impl Fn(&str) -> Result<String, E>) -> Result<Wording, E> {
    Ok(Wording {
        link_panel_intro: read(KEY_PANEL_INTRO)?,
        link_scope_notice: read(KEY_SCOPE_NOTICE)?,
        link_allegations_heading: read(KEY_ALLEGATIONS_HEADING)?,
        link_show_all_label: read(KEY_SHOW_ALL_LABEL)?,
        link_filter_placeholder: read(KEY_FILTER_PLACEHOLDER)?,
        link_no_match_notice: read(KEY_NO_MATCH_NOTICE)?,
        link_empty_options_notice: read(KEY_EMPTY_OPTIONS_NOTICE)?,
        link_cut_heading: read(KEY_CUT_HEADING)?,
        link_cut_supports_label: read(KEY_CUT_SUPPORTS_LABEL)?,
        link_cut_against_label: read(KEY_CUT_AGAINST_LABEL)?,
        link_cut_supports_phrase: read(KEY_CUT_SUPPORTS_PHRASE)?,
        link_cut_against_phrase: read(KEY_CUT_AGAINST_PHRASE)?,
        link_save_label: read(KEY_SAVE_LABEL)?,
        link_cancel_label: read(KEY_CANCEL_LABEL)?,
        link_unlink_label: read(KEY_UNLINK_LABEL)?,
        link_missing_cut_refusal: read(KEY_MISSING_CUT_REFUSAL)?,
        link_missing_allegation_refusal: read(KEY_MISSING_ALLEGATION_REFUSAL)?,
        link_summary_template: read(KEY_SUMMARY_TEMPLATE)?,
        link_progress_template: read(KEY_PROGRESS_TEMPLATE)?,
        question_revert_label: read(KEY_QUESTION_REVERT_LABEL)?,
        link_unlink_found_nothing: read(KEY_UNLINK_FOUND_NOTHING)?,
        link_save_failed_template: read(KEY_SAVE_FAILED_TEMPLATE)?,
        link_save_blocks_ruling: read(KEY_SAVE_BLOCKS_RULING)?,
        fact_remove_label: read(KEY_FACT_REMOVE_LABEL)?,
        fact_remove_confirm_template: read(KEY_FACT_REMOVE_CONFIRM)?,
        fact_remove_confirm_yes: read(KEY_FACT_REMOVE_YES)?,
        fact_remove_confirm_cancel: read(KEY_FACT_REMOVE_CANCEL)?,
        fact_remove_failed_template: read(KEY_FACT_REMOVE_FAILED)?,
    })
}

impl Wording {
    /// The mid-sentence phrase for one cut.
    ///
    /// The one place the enum meets its two stored sentence forms, so no caller
    /// has to know which field goes with which variant — and so adding a third
    /// cut is a compile error here rather than a wrong phrase somewhere else.
    pub fn cut_phrase(&self, cut: crate::domain::link_cut::LinkCut) -> &str {
        match cut {
            crate::domain::link_cut::LinkCut::Supports => &self.link_cut_supports_phrase,
            crate::domain::link_cut::LinkCut::Against => &self.link_cut_against_phrase,
        }
    }
}

/// The seeded wording, for TESTS ONLY.
///
/// ## Why this is `#[cfg(test)]`, exactly like `Settings::for_test`
///
/// A production-reachable `Default for Wording` would be a compiled-in set of
/// user-facing strings — the precise defect Roman's ruling deletes — and it would
/// be reachable by accident through `unwrap_or_default()`. Gating it on
/// `cfg(test)` means it cannot exist in a release binary at all.
///
/// The values match the migration's seed, and `wording_tests` asserts that
/// against the migration file itself, so the two cannot drift silently.
#[cfg(test)]
impl Wording {
    pub fn for_test() -> Self {
        Wording {
            link_panel_intro: "This statement isn't linked to anything they've accused you of. \
                               Link it and you can rule it."
                .to_string(),
            link_scope_notice: "An accusation link belongs to the statement, not to this \
                                scenario — linking it here links it wherever this item appears."
                .to_string(),
            link_allegations_heading: "What it bears on".to_string(),
            link_show_all_label: "Show all {count}".to_string(),
            link_filter_placeholder: "Type to filter".to_string(),
            link_no_match_notice: "No accusation matches that.".to_string(),
            link_empty_options_notice: "No accusations are stored for this case yet, so there \
                                        is nothing to link to."
                .to_string(),
            link_cut_heading: "How it cuts".to_string(),
            link_cut_supports_label: "It supports us".to_string(),
            link_cut_against_label: "They'll use it against us".to_string(),
            link_cut_supports_phrase: "it supports us".to_string(),
            link_cut_against_phrase: "they'll use it against us".to_string(),
            // Task 2.12 item A: the button no longer moves anyone, so it no
            // longer says it does. The migration UPDATEs the already-seeded row.
            link_save_label: "Save".to_string(),
            link_cancel_label: "Cancel".to_string(),
            link_unlink_label: "Unlink".to_string(),
            link_missing_cut_refusal: "Say which way this cuts before saving — a link with no \
                                       cut can't tell ammunition from hazard."
                .to_string(),
            link_missing_allegation_refusal: "Tick at least one accusation before saving."
                .to_string(),
            link_summary_template: "You linked this to {allegations} · {cut}.".to_string(),
            link_progress_template: "{linked} of {total} linked.".to_string(),
            question_revert_label: "Restore the system's wording".to_string(),
            link_unlink_found_nothing: "That link was already gone — your view was out of \
                                        date, and it has been refreshed."
                .to_string(),
            link_save_failed_template: "That link did not save: {detail} The queue has been \
                                        reloaded from the server, so what you see now is \
                                        what is stored."
                .to_string(),
            link_save_blocks_ruling: "Save the link first.".to_string(),
            fact_remove_label: "Remove".to_string(),
            fact_remove_confirm_template: "Remove {code} from this scenario? It goes back \
                                           to the queue as not ruled."
                .to_string(),
            fact_remove_confirm_yes: "Remove it".to_string(),
            fact_remove_confirm_cancel: "Keep it".to_string(),
            fact_remove_failed_template: "That fact could not be removed: {detail} Reload \
                                          the page to see what is actually stored."
                .to_string(),
        }
    }

    /// The test fixture as a key→value map, in the shape the store reads.
    ///
    /// Lets `settings_store_tests` extend its own seeded fixture without holding a
    /// second hand-typed copy of twenty sentences — the copy that matters is
    /// [`Wording::for_test`], and `wording_tests` pins THAT to the migration.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        let w = Wording::for_test();
        WORDING_KEYS
            .iter()
            .map(|key| {
                let value = match *key {
                    KEY_PANEL_INTRO => w.link_panel_intro.clone(),
                    KEY_SCOPE_NOTICE => w.link_scope_notice.clone(),
                    KEY_ALLEGATIONS_HEADING => w.link_allegations_heading.clone(),
                    KEY_SHOW_ALL_LABEL => w.link_show_all_label.clone(),
                    KEY_FILTER_PLACEHOLDER => w.link_filter_placeholder.clone(),
                    KEY_NO_MATCH_NOTICE => w.link_no_match_notice.clone(),
                    KEY_EMPTY_OPTIONS_NOTICE => w.link_empty_options_notice.clone(),
                    KEY_CUT_HEADING => w.link_cut_heading.clone(),
                    KEY_CUT_SUPPORTS_LABEL => w.link_cut_supports_label.clone(),
                    KEY_CUT_AGAINST_LABEL => w.link_cut_against_label.clone(),
                    KEY_CUT_SUPPORTS_PHRASE => w.link_cut_supports_phrase.clone(),
                    KEY_CUT_AGAINST_PHRASE => w.link_cut_against_phrase.clone(),
                    KEY_SAVE_LABEL => w.link_save_label.clone(),
                    KEY_CANCEL_LABEL => w.link_cancel_label.clone(),
                    KEY_UNLINK_LABEL => w.link_unlink_label.clone(),
                    KEY_MISSING_CUT_REFUSAL => w.link_missing_cut_refusal.clone(),
                    KEY_MISSING_ALLEGATION_REFUSAL => w.link_missing_allegation_refusal.clone(),
                    KEY_SUMMARY_TEMPLATE => w.link_summary_template.clone(),
                    KEY_PROGRESS_TEMPLATE => w.link_progress_template.clone(),
                    KEY_QUESTION_REVERT_LABEL => w.question_revert_label.clone(),
                    KEY_UNLINK_FOUND_NOTHING => w.link_unlink_found_nothing.clone(),
                    KEY_SAVE_FAILED_TEMPLATE => w.link_save_failed_template.clone(),
                    KEY_SAVE_BLOCKS_RULING => w.link_save_blocks_ruling.clone(),
                    KEY_FACT_REMOVE_LABEL => w.fact_remove_label.clone(),
                    KEY_FACT_REMOVE_CONFIRM => w.fact_remove_confirm_template.clone(),
                    KEY_FACT_REMOVE_YES => w.fact_remove_confirm_yes.clone(),
                    KEY_FACT_REMOVE_CANCEL => w.fact_remove_confirm_cancel.clone(),
                    KEY_FACT_REMOVE_FAILED => w.fact_remove_failed_template.clone(),
                    // Unreachable by construction: the match above covers every
                    // entry of WORDING_KEYS, and the test below proves it. A panic
                    // here would only fire in a test binary, on a key someone added
                    // to the list and forgot here — which is exactly when a loud
                    // stop is what you want.
                    other => panic!("{other} is in WORDING_KEYS but not in for_test_values"),
                };
                (*key, value)
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "wording_tests.rs"]
mod tests;
