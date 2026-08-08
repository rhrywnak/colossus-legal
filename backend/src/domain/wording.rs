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
// ## Where the TEMPLATE RULES went (task 2.11 B2)
//
// `REQUIRED_PLACEHOLDERS`, `missing_placeholders`, `validate_wording_candidate`
// and `render` now live in `domain::wording_templates`. They stopped being this
// module's business the moment a second and third stored-string module appeared:
// they are the rules for ALL stored text, and there are three surfaces' worth of
// it now. Leaving them here made this module the place every other surface had to
// reach into — and it is why this file went past Rule 17's limit. What remains is
// one surface's words, and nothing else.
//
// ## Domain note: sentence forms are STORED, never derived
//
// "It supports us" is a button; "it supports us" is the middle of a sentence.
// They are two rows, not one row and a `to_lowercase()`. Lowercasing a human's
// words in code is the frontend-composing-prose defect wearing a server costume —
// it would also be wrong the first time a label starts with a proper noun.

use crate::domain::settings::SettingError;

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

    // ── Task 2.13 slice 1: the facts list as a prep surface ──────────────────
    /// Marks the interrogatory question printed above a discovery answer.
    pub fact_question_label: String,
    /// Introduces the kind of statement a fact is ("admission", "evasive", …).
    pub fact_statement_kind_label: String,
    /// The name of the heaviest weight tier.
    pub fact_tier_carries_label: String,
    /// The name of the middle tier — where a newly included fact lands.
    pub fact_tier_backup_label: String,
    /// The name of the lightest tier, folded beneath the list.
    pub fact_tier_background_label: String,
    /// The prompt on a row's weight control.
    pub fact_tier_prompt: String,
    /// Whether the background pile starts folded. Two tokens, parsed by
    /// [`BackgroundDefaultState`] — never rendered to a human.
    pub fact_tier_background_default_state: String,
    /// The folded pile's line. Carries `{count}`.
    pub fact_background_count_template: String,
    /// The control that folds the background pile away again.
    pub fact_background_hide_label: String,
    /// The drag handle's accessible label.
    pub fact_order_drag_hint: String,
    /// Said when a weight could not be stored. Carries `{code}` and `{reason}`.
    pub fact_tier_save_failed_template: String,
    /// Said when a drag could not be stored — including gap exhaustion. Carries
    /// `{code}` and `{reason}`.
    pub fact_order_save_failed_template: String,
    /// The queue's summary when there is no pool at all, including before it has
    /// loaded. Distinct from "all ruled" on purpose (task 2.13, from the beta.376
    /// click-through): a queue that has not counted must not report the work done.
    pub queue_empty_pool_summary: String,
    /// The queue's summary when a real pool exists and none of it is outstanding.
    pub queue_all_ruled_summary: String,
    /// The queue's summary while the counts are NOT KNOWN — before anything has
    /// measured the pool. A third state, distinct from both "no candidates" and
    /// "all ruled": a queue that has not looked must not report either result.
    pub queue_counting_summary: String,

    // ── Task 2.13c ───────────────────────────────────────────────────────────
    /// Marks the answer beneath its question, as the question label's partner.
    pub fact_answer_label: String,
    /// Said when a fact is weighed down into the background pile. Carries `{code}`.
    pub fact_background_move_notice: String,
    /// The line under the section heading naming the weights and the drag.
    pub fact_weights_hint: String,
    /// The count beneath the list. Carries `{shown}` and `{background}`.
    pub fact_footer_template: String,
    /// The control that forgets where a human dragged one fact.
    pub fact_unplace_label: String,

    // ── Task 2.15 Tier 2 (2026-08-08) ────────────────────────────────────────
    /// The opt-in that opens the raw evidence pool on a scenario nothing has
    /// scanned yet. Carries `{count}`.
    ///
    /// Domain note: on a never-scanned scenario the pool is not a queue of
    /// candidates — it is every statement about the subject, which no judgment has
    /// touched. Leading the page with it (measured 2026-08-07: 148 cards under
    /// "Candidates awaiting ruling — from all scans") claims a parentage the rows
    /// do not have. Behind this control they are still one click away, and
    /// honestly named.
    pub queue_raw_pool_toggle_template: String,

    // ── Scan → ruling (2026-08-08) ───────────────────────────────────────────
    /// The queue's heading when a completed scan is proposing candidates. Carries
    /// `{count}` and `{when}`.
    ///
    /// Domain note: the queue LEADS with proposals now, so this sentence is the
    /// first thing a curator reads on a scanned scenario. It names the source
    /// because "30 awaiting ruling" and "30 the Aug 7 scan put in front of you"
    /// are different claims, and only the second is true of a projection.
    pub queue_proposed_heading_template: String,
    /// The attribution line on a proposed card. Carries `{when}`.
    pub card_proposed_attribution_template: String,
    /// The proposed-role chip. Carries `{verb}`, which is the canon stance word.
    ///
    /// Domain note: the template exists so the chip can NAME THE SCAN as the
    /// speaker, exactly as the banded-confidence label does. A chip reading
    /// "supports" beside a sworn admission reads as the record's own stance; this
    /// one cannot be mistaken for it.
    pub card_proposed_role_template: String,
    /// The badge on a card whose one ruling settles a byte-identical twin. Carries
    /// `{count}` and `{codes}`.
    pub card_proposed_covers_template: String,

    // ── Ruling acknowledgment (2026-08-08) ───────────────────────────────────
    //
    // Every ruling action says what it did, in success AND in failure. The
    // measured defect these answer: on beta.385 a defer landed in the database —
    // anchor, reference row and provenance — and the screen said nothing at all,
    // so the architect reported the feature dead. It was not dead; it was silent.
    /// Said when a ruling has been STORED. Carries `{code}` and `{state}`.
    pub card_ruling_saved_template: String,
    /// Said when a stored ruling takes the card out of the list the human is
    /// looking at. Carries `{code}` and `{filter}`.
    ///
    /// Domain note: this is the "vanish". Under the Proposed filter a ruled card
    /// stops being proposed and leaves the list — correct behaviour that reads
    /// exactly like a click doing nothing, unless the queue says so as it goes.
    pub card_ruling_left_filter_template: String,
    /// Said when a LOCKED card's one-press defer records the system's own
    /// sentence. Carries `{reason}`.
    ///
    /// Domain note: the human should be able to read the sentence they just
    /// signed. A one-press commit that shows nothing is a signature nobody saw.
    pub card_defer_recorded_template: String,
    /// Said when a ruling could NOT be stored. Carries `{code}` and `{detail}`.
    pub card_ruling_failed_template: String,
    /// The standing condition on a card whose Include and Exclude are shut.
    ///
    /// Domain note (D3a): the sentence already existed as the disabled buttons'
    /// tooltip. A condition a human can only discover by hovering is a condition
    /// most humans never discover, so it is stated on the card's face.
    pub card_locked_condition_label: String,
}

/// Whether the background pile starts folded away.
///
/// ## Rust Learning: a two-variant enum instead of a `bool` from a string
///
/// The stored value is text (`ValueKind` has no boolean — ruled 2026-08-05), and
/// the tempting decode is `value == "collapsed"`. That silently treats every
/// typo — "Collapsed", "colapsed", "true" — as `expanded`, so a setting Roman
/// edited would appear to do nothing with no error anywhere. Parsing into a
/// closed enum makes an unrecognised token a loud, named failure instead
/// (Standing Rule 1), the same discipline `FactTier` and `FactStatus` use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundDefaultState {
    /// The pile starts folded, showing only its count.
    Collapsed,
    /// The pile starts open.
    Expanded,
}

impl TryFrom<&str> for BackgroundDefaultState {
    type Error = SettingError;

    fn try_from(token: &str) -> Result<Self, Self::Error> {
        match token {
            "collapsed" => Ok(BackgroundDefaultState::Collapsed),
            "expanded" => Ok(BackgroundDefaultState::Expanded),
            other => Err(SettingError::Unreadable {
                key: KEY_FACT_BACKGROUND_DEFAULT_STATE.to_string(),
                value: other.to_string(),
                expected: "either 'collapsed' or 'expanded'",
            }),
        }
    }
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
pub(crate) const KEY_FACT_QUESTION_LABEL: &str = "fact_question_label";
pub(crate) const KEY_FACT_STATEMENT_KIND_LABEL: &str = "fact_statement_kind_label";
pub(crate) const KEY_FACT_TIER_CARRIES: &str = "fact_tier_carries_label";
pub(crate) const KEY_FACT_TIER_BACKUP: &str = "fact_tier_backup_label";
pub(crate) const KEY_FACT_TIER_BACKGROUND: &str = "fact_tier_background_label";
pub(crate) const KEY_FACT_TIER_PROMPT: &str = "fact_tier_prompt";
pub(crate) const KEY_FACT_BACKGROUND_DEFAULT_STATE: &str = "fact_tier_background_default_state";
pub(crate) const KEY_FACT_BACKGROUND_COUNT: &str = "fact_background_count_template";
pub(crate) const KEY_FACT_BACKGROUND_HIDE: &str = "fact_background_hide_label";
pub(crate) const KEY_FACT_ORDER_DRAG_HINT: &str = "fact_order_drag_hint";
pub(crate) const KEY_FACT_TIER_SAVE_FAILED: &str = "fact_tier_save_failed_template";
pub(crate) const KEY_FACT_ORDER_SAVE_FAILED: &str = "fact_order_save_failed_template";
pub(crate) const KEY_QUEUE_EMPTY_POOL: &str = "queue_empty_pool_summary";
pub(crate) const KEY_QUEUE_ALL_RULED: &str = "queue_all_ruled_summary";
pub(crate) const KEY_QUEUE_COUNTING: &str = "queue_counting_summary";
pub(crate) const KEY_FACT_ANSWER_LABEL: &str = "fact_answer_label";
pub(crate) const KEY_FACT_BG_MOVE_NOTICE: &str = "fact_background_move_notice";
pub(crate) const KEY_FACT_WEIGHTS_HINT: &str = "fact_weights_hint";
pub(crate) const KEY_FACT_FOOTER: &str = "fact_footer_template";
pub(crate) const KEY_FACT_UNPLACE_LABEL: &str = "fact_unplace_label";
pub(crate) const KEY_QUEUE_RAW_POOL_TOGGLE: &str = "queue_raw_pool_toggle_template";
pub(crate) const KEY_QUEUE_PROPOSED_HEADING: &str = "queue_proposed_heading_template";
pub(crate) const KEY_CARD_PROPOSED_ATTRIBUTION: &str = "card_proposed_attribution_template";
pub(crate) const KEY_CARD_PROPOSED_ROLE: &str = "card_proposed_role_template";
pub(crate) const KEY_CARD_PROPOSED_COVERS: &str = "card_proposed_covers_template";
pub(crate) const KEY_CARD_RULING_SAVED: &str = "card_ruling_saved_template";
pub(crate) const KEY_CARD_RULING_LEFT_FILTER: &str = "card_ruling_left_filter_template";
pub(crate) const KEY_CARD_DEFER_RECORDED: &str = "card_defer_recorded_template";
pub(crate) const KEY_CARD_RULING_FAILED: &str = "card_ruling_failed_template";
pub(crate) const KEY_CARD_LOCKED_CONDITION: &str = "card_locked_condition_label";

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
    KEY_FACT_QUESTION_LABEL,
    KEY_FACT_STATEMENT_KIND_LABEL,
    KEY_FACT_TIER_CARRIES,
    KEY_FACT_TIER_BACKUP,
    KEY_FACT_TIER_BACKGROUND,
    KEY_FACT_TIER_PROMPT,
    KEY_FACT_BACKGROUND_DEFAULT_STATE,
    KEY_FACT_BACKGROUND_COUNT,
    KEY_FACT_BACKGROUND_HIDE,
    KEY_FACT_ORDER_DRAG_HINT,
    KEY_FACT_TIER_SAVE_FAILED,
    KEY_FACT_ORDER_SAVE_FAILED,
    KEY_QUEUE_EMPTY_POOL,
    KEY_QUEUE_ALL_RULED,
    KEY_QUEUE_COUNTING,
    KEY_FACT_ANSWER_LABEL,
    KEY_FACT_BG_MOVE_NOTICE,
    KEY_FACT_WEIGHTS_HINT,
    KEY_FACT_FOOTER,
    KEY_FACT_UNPLACE_LABEL,
    KEY_QUEUE_RAW_POOL_TOGGLE,
    KEY_QUEUE_PROPOSED_HEADING,
    KEY_CARD_PROPOSED_ATTRIBUTION,
    KEY_CARD_PROPOSED_ROLE,
    KEY_CARD_PROPOSED_COVERS,
    KEY_CARD_RULING_SAVED,
    KEY_CARD_RULING_LEFT_FILTER,
    KEY_CARD_DEFER_RECORDED,
    KEY_CARD_RULING_FAILED,
    KEY_CARD_LOCKED_CONDITION,
];

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
        fact_question_label: read(KEY_FACT_QUESTION_LABEL)?,
        fact_statement_kind_label: read(KEY_FACT_STATEMENT_KIND_LABEL)?,
        fact_tier_carries_label: read(KEY_FACT_TIER_CARRIES)?,
        fact_tier_backup_label: read(KEY_FACT_TIER_BACKUP)?,
        fact_tier_background_label: read(KEY_FACT_TIER_BACKGROUND)?,
        fact_tier_prompt: read(KEY_FACT_TIER_PROMPT)?,
        fact_tier_background_default_state: read(KEY_FACT_BACKGROUND_DEFAULT_STATE)?,
        fact_background_count_template: read(KEY_FACT_BACKGROUND_COUNT)?,
        fact_background_hide_label: read(KEY_FACT_BACKGROUND_HIDE)?,
        fact_order_drag_hint: read(KEY_FACT_ORDER_DRAG_HINT)?,
        fact_tier_save_failed_template: read(KEY_FACT_TIER_SAVE_FAILED)?,
        fact_order_save_failed_template: read(KEY_FACT_ORDER_SAVE_FAILED)?,
        queue_empty_pool_summary: read(KEY_QUEUE_EMPTY_POOL)?,
        queue_all_ruled_summary: read(KEY_QUEUE_ALL_RULED)?,
        queue_counting_summary: read(KEY_QUEUE_COUNTING)?,
        fact_answer_label: read(KEY_FACT_ANSWER_LABEL)?,
        fact_background_move_notice: read(KEY_FACT_BG_MOVE_NOTICE)?,
        fact_weights_hint: read(KEY_FACT_WEIGHTS_HINT)?,
        fact_footer_template: read(KEY_FACT_FOOTER)?,
        fact_unplace_label: read(KEY_FACT_UNPLACE_LABEL)?,
        queue_raw_pool_toggle_template: read(KEY_QUEUE_RAW_POOL_TOGGLE)?,
        queue_proposed_heading_template: read(KEY_QUEUE_PROPOSED_HEADING)?,
        card_proposed_attribution_template: read(KEY_CARD_PROPOSED_ATTRIBUTION)?,
        card_proposed_role_template: read(KEY_CARD_PROPOSED_ROLE)?,
        card_proposed_covers_template: read(KEY_CARD_PROPOSED_COVERS)?,
        card_ruling_saved_template: read(KEY_CARD_RULING_SAVED)?,
        card_ruling_left_filter_template: read(KEY_CARD_RULING_LEFT_FILTER)?,
        card_defer_recorded_template: read(KEY_CARD_DEFER_RECORDED)?,
        card_ruling_failed_template: read(KEY_CARD_RULING_FAILED)?,
        card_locked_condition_label: read(KEY_CARD_LOCKED_CONDITION)?,
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

// `pub(crate)` rather than private (task 2.11): `wording_accusation_tests` needs
// the migration-seed parser this module's tests own, and a second copy of it
// would be a second thing to get wrong — the parser is the only thing standing
// between "the fixture matches the migration" and a false green, so there must be
// exactly one of it, tested once.
#[cfg(test)]
#[path = "wording_tests.rs"]
pub(crate) mod tests;
