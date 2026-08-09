// =============================================================================
// backend/src/domain/wording_card_grammar.rs — the words ONE EVIDENCE CARD speaks
// =============================================================================
//
// The eighth stored-string block, approved as ruling R6 of the ONE_CARD_GRAMMAR
// Phase B authorization (2026-08-09). It carries every sentence the shared
// evidence card and the queue frame around it introduce — the filter chips, the
// collapsed question, the compressed element chips, the labelled weight picker,
// and the two attributions that used to be compiled in.
//
// ## Why a new module rather than more keys on `wording`
//
// The same test the seven siblings apply: which SURFACE speaks these, and does
// its vocabulary move independently? `domain::wording` is the CURATION block —
// the link panel, the facts table, the ruling acknowledgments — and it is at its
// Rule-17 ceiling. These belong to the card GRAMMAR: the shape a piece of
// evidence takes wherever it appears, which is exactly the thing this task made
// independent of the two wrappers that render it. When the grammar moves, it
// should move without touching a row the link panel renders.
//
// ## Two compiled-in strings die here (Standing Rule 2)
//
// * `SYSTEM_AUTHORSHIP_LABEL = "System"` in `services::scenario_human_links` —
//   the badge under a machine-transcribed question. Roman read it as the
//   SPEAKER of a seven-line interrogatory, which is exactly what it looks like
//   when it sits under one; measured on DEV, no `STATED_BY` actor is named
//   "System", so the speaker chip never said it and never could.
// * `attribution: "extracted"` in `services::scenario_card::build_speaker` — the
//   provenance label beside a raw extracted name, which the §3 sequencing ruling
//   requires until canonical identity lands in B0. It is a sentence a human
//   reads, so it is a row.
//
// ## Domain note: why the filter chips' names are rows and the ruling keys are not
//
// I / E / D / U are baked into button labels, `aria-label`s and `<kbd>` chips —
// a remapping setting would only let the screen lie about its own controls, which
// is the reasoning `CardQueue.RULING_KEYS` already records. A FILTER name is not
// like that: "Proposed" is case-facing vocabulary about what a scan did, and
// Roman renames those without a rebuild.

/// Every stored string the shared evidence card and its queue frame render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardGrammarWording {
    // ── The queue frame (Piece 1) ────────────────────────────────────────────
    /// The chip for candidates a completed run put forward and nobody has ruled.
    pub filter_proposed_label: String,
    /// The chip for candidates parked with a stated reason.
    pub filter_deferred_label: String,
    /// The chip for candidates confirmed as facts of this scenario.
    pub filter_included_label: String,
    /// The chip for candidates set aside for this scenario.
    pub filter_excluded_label: String,
    /// The chip for the whole gathered pool.
    ///
    /// Domain note: it replaces "All", which read as a to-do list. What it names
    /// is the denominator, not the work — see [`Self::full_pool_explainer`].
    pub filter_full_pool_label: String,
    /// The ⓘ popup beside the full-pool chip.
    ///
    /// Domain note: Roman's addition. Marie and Chuck will not know the term, and
    /// a filter whose meaning a reader has to infer from its count is a filter
    /// they will not press.
    pub full_pool_explainer: String,
    /// The progress line under the chips. Carries `{ruled}`, `{total}` and
    /// `{filter}`.
    ///
    /// Domain note: it tracks the ACTIVE filter and never the pool. The line it
    /// replaces read "23 of 148 ruled" over a bar of 125 nobody owes — and
    /// rule-the-promising is the ratified triage model, so the frame now agrees
    /// with it.
    pub filter_progress_template: String,

    // ── The card body (Piece 2) ──────────────────────────────────────────────
    /// The control that unfolds a collapsed question.
    pub question_expand_label: String,
    /// The control that folds it away again.
    pub question_collapse_label: String,
    /// Who wrote a question the MACHINE transcribed — the row that retires the
    /// compiled-in "System".
    ///
    /// Domain note: this badge is not a speaker and must never read as one. It
    /// says where the text of the QUESTION came from, which is a different claim
    /// from who gave the answer beneath it.
    pub question_machine_authorship_label: String,
    /// The provenance label beside a raw extracted speaker name.
    ///
    /// Domain note: required by the §3 sequencing ruling until B0 reconciles one
    /// man from two strings. It is provenance, not clutter, and it retires when
    /// canonical identity arrives and not before.
    pub speaker_extracted_label: String,
    /// The speaker chip on an item whose source recorded nobody.
    ///
    /// Domain note: documentary evidence genuinely has no speaker. This says so
    /// rather than the card inventing "Unknown" — the absent-not-fake law.
    pub speaker_absent_label: String,
    /// The control that reveals the element chips beyond the visible few.
    /// Carries `{count}`.
    pub elements_more_template: String,
    /// The control that folds them away again.
    pub elements_fewer_label: String,
    /// The control that reveals the surrounding source text.
    pub context_show_label: String,
    /// The control that hides it again.
    pub context_hide_label: String,
    /// Introduces the scan's own justification on a card.
    ///
    /// Domain note: the sentence has to name its speaker. A judge's reason
    /// rendered unattributed reads as the record's own position, which is the
    /// C-222 shape §7.5 exists to forbid.
    pub scan_reason_label: String,

    // ── Linking (Pieces 4a, 4b) ──────────────────────────────────────────────
    /// The type-ahead's placeholder.
    pub link_typeahead_placeholder: String,
    /// Said above the type-ahead on a card nothing has linked.
    pub link_typeahead_intro: String,
    /// Said when the type-ahead matches nothing.
    pub link_typeahead_no_match: String,
    /// Said when a link wakes a locked card's ruling buttons. Carries `{code}`.
    pub link_woke_ruling_template: String,

    // ── The fact wrapper (Piece 5) ───────────────────────────────────────────
    /// The label on the weight picker.
    pub weight_picker_label: String,
    /// Said when a weight is stored. Carries `{code}` and `{tier}`.
    pub weight_changed_template: String,
    /// The control that takes a weight change back.
    pub weight_undo_label: String,
    /// The section-header control that forgets every human placement.
    pub reset_order_label: String,
    /// The question asked before it runs.
    ///
    /// Domain note: it discards work — the sequence IS the argument — so it is
    /// confirmed, unlike the per-card un-place it replaces.
    pub reset_order_confirm: String,
    /// The button that confirms it.
    pub reset_order_confirm_yes: String,
    /// The button that abandons it.
    pub reset_order_confirm_cancel: String,
    /// Said when it has run. Carries `{count}`.
    pub reset_order_done_template: String,
    /// Said when it could not run. Carries `{reason}`.
    pub reset_order_failed_template: String,

    // ── Chips as currency (Piece 7) ──────────────────────────────────────────
    /// The tooltip on a clickable chip. Carries `{value}`.
    pub chip_filter_hint_template: String,
    /// The control that drops a chip filter. Carries `{value}`.
    pub chip_filter_clear_template: String,
}

// KEYS: the stable identifiers of the stored strings. Renaming one is a
// migration, and until it runs the boot loader refuses to start.
pub(crate) const KEY_FILTER_PROPOSED: &str = "card_filter_proposed_label";
pub(crate) const KEY_FILTER_DEFERRED: &str = "card_filter_deferred_label";
pub(crate) const KEY_FILTER_INCLUDED: &str = "card_filter_included_label";
pub(crate) const KEY_FILTER_EXCLUDED: &str = "card_filter_excluded_label";
pub(crate) const KEY_FILTER_FULL_POOL: &str = "card_filter_full_pool_label";
pub(crate) const KEY_FULL_POOL_EXPLAINER: &str = "card_filter_full_pool_explainer";
pub(crate) const KEY_FILTER_PROGRESS: &str = "card_filter_progress_template";
pub(crate) const KEY_QUESTION_EXPAND: &str = "card_question_expand_label";
pub(crate) const KEY_QUESTION_COLLAPSE: &str = "card_question_collapse_label";
pub(crate) const KEY_QUESTION_MACHINE_AUTHORSHIP: &str = "card_question_machine_authorship_label";
pub(crate) const KEY_SPEAKER_EXTRACTED: &str = "card_speaker_extracted_label";
pub(crate) const KEY_SPEAKER_ABSENT: &str = "card_speaker_absent_label";
pub(crate) const KEY_ELEMENTS_MORE: &str = "card_elements_more_template";
pub(crate) const KEY_ELEMENTS_FEWER: &str = "card_elements_fewer_label";
pub(crate) const KEY_CONTEXT_SHOW: &str = "card_context_show_label";
pub(crate) const KEY_CONTEXT_HIDE: &str = "card_context_hide_label";
pub(crate) const KEY_SCAN_REASON: &str = "card_scan_reason_label";
pub(crate) const KEY_LINK_TYPEAHEAD_PLACEHOLDER: &str = "card_link_typeahead_placeholder";
pub(crate) const KEY_LINK_TYPEAHEAD_INTRO: &str = "card_link_typeahead_intro";
pub(crate) const KEY_LINK_TYPEAHEAD_NO_MATCH: &str = "card_link_typeahead_no_match";
pub(crate) const KEY_LINK_WOKE_RULING: &str = "card_link_woke_ruling_template";
pub(crate) const KEY_WEIGHT_PICKER: &str = "card_weight_picker_label";
pub(crate) const KEY_WEIGHT_CHANGED: &str = "card_weight_changed_template";
pub(crate) const KEY_WEIGHT_UNDO: &str = "card_weight_undo_label";
pub(crate) const KEY_RESET_ORDER: &str = "card_reset_order_label";
pub(crate) const KEY_RESET_ORDER_CONFIRM: &str = "card_reset_order_confirm";
pub(crate) const KEY_RESET_ORDER_YES: &str = "card_reset_order_confirm_yes";
pub(crate) const KEY_RESET_ORDER_CANCEL: &str = "card_reset_order_confirm_cancel";
pub(crate) const KEY_RESET_ORDER_DONE: &str = "card_reset_order_done_template";
pub(crate) const KEY_RESET_ORDER_FAILED: &str = "card_reset_order_failed_template";
pub(crate) const KEY_CHIP_FILTER_HINT: &str = "card_chip_filter_hint_template";
pub(crate) const KEY_CHIP_FILTER_CLEAR: &str = "card_chip_filter_clear_template";

/// Every card-grammar key this build reads, so a missing one is caught at boot BY
/// NAME rather than as a blank control in front of a human mid-triage.
pub const CARD_GRAMMAR_WORDING_KEYS: &[&str] = &[
    KEY_FILTER_PROPOSED,
    KEY_FILTER_DEFERRED,
    KEY_FILTER_INCLUDED,
    KEY_FILTER_EXCLUDED,
    KEY_FILTER_FULL_POOL,
    KEY_FULL_POOL_EXPLAINER,
    KEY_FILTER_PROGRESS,
    KEY_QUESTION_EXPAND,
    KEY_QUESTION_COLLAPSE,
    KEY_QUESTION_MACHINE_AUTHORSHIP,
    KEY_SPEAKER_EXTRACTED,
    KEY_SPEAKER_ABSENT,
    KEY_ELEMENTS_MORE,
    KEY_ELEMENTS_FEWER,
    KEY_CONTEXT_SHOW,
    KEY_CONTEXT_HIDE,
    KEY_SCAN_REASON,
    KEY_LINK_TYPEAHEAD_PLACEHOLDER,
    KEY_LINK_TYPEAHEAD_INTRO,
    KEY_LINK_TYPEAHEAD_NO_MATCH,
    KEY_LINK_WOKE_RULING,
    KEY_WEIGHT_PICKER,
    KEY_WEIGHT_CHANGED,
    KEY_WEIGHT_UNDO,
    KEY_RESET_ORDER,
    KEY_RESET_ORDER_CONFIRM,
    KEY_RESET_ORDER_YES,
    KEY_RESET_ORDER_CANCEL,
    KEY_RESET_ORDER_DONE,
    KEY_RESET_ORDER_FAILED,
    KEY_CHIP_FILTER_HINT,
    KEY_CHIP_FILTER_CLEAR,
];

/// Build a [`CardGrammarWording`] from the stored rows, or say which key is wrong.
///
/// ## Rust Learning: a closure parameter instead of a database handle
///
/// `read` is `impl Fn(&str) -> Result<String, E>` over a GENERIC error type, the
/// same shape all seven siblings use. Two things fall out of that. The function
/// does no I/O, so it is unit-testable against a `HashMap` with no booted
/// process; and it is agnostic about how a missing key is REPORTED, so the boot
/// loader's `SettingError` and a test's `String` both satisfy it without this
/// module knowing either type exists.
///
/// The `?` on every line is what makes the refusal name the FIRST bad key rather
/// than collecting all of them: at boot there is nothing to do with a list, and
/// the operator fixes them one migration at a time anyway.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_card_grammar_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<CardGrammarWording, E> {
    Ok(CardGrammarWording {
        filter_proposed_label: read(KEY_FILTER_PROPOSED)?,
        filter_deferred_label: read(KEY_FILTER_DEFERRED)?,
        filter_included_label: read(KEY_FILTER_INCLUDED)?,
        filter_excluded_label: read(KEY_FILTER_EXCLUDED)?,
        filter_full_pool_label: read(KEY_FILTER_FULL_POOL)?,
        full_pool_explainer: read(KEY_FULL_POOL_EXPLAINER)?,
        filter_progress_template: read(KEY_FILTER_PROGRESS)?,
        question_expand_label: read(KEY_QUESTION_EXPAND)?,
        question_collapse_label: read(KEY_QUESTION_COLLAPSE)?,
        question_machine_authorship_label: read(KEY_QUESTION_MACHINE_AUTHORSHIP)?,
        speaker_extracted_label: read(KEY_SPEAKER_EXTRACTED)?,
        speaker_absent_label: read(KEY_SPEAKER_ABSENT)?,
        elements_more_template: read(KEY_ELEMENTS_MORE)?,
        elements_fewer_label: read(KEY_ELEMENTS_FEWER)?,
        context_show_label: read(KEY_CONTEXT_SHOW)?,
        context_hide_label: read(KEY_CONTEXT_HIDE)?,
        scan_reason_label: read(KEY_SCAN_REASON)?,
        link_typeahead_placeholder: read(KEY_LINK_TYPEAHEAD_PLACEHOLDER)?,
        link_typeahead_intro: read(KEY_LINK_TYPEAHEAD_INTRO)?,
        link_typeahead_no_match: read(KEY_LINK_TYPEAHEAD_NO_MATCH)?,
        link_woke_ruling_template: read(KEY_LINK_WOKE_RULING)?,
        weight_picker_label: read(KEY_WEIGHT_PICKER)?,
        weight_changed_template: read(KEY_WEIGHT_CHANGED)?,
        weight_undo_label: read(KEY_WEIGHT_UNDO)?,
        reset_order_label: read(KEY_RESET_ORDER)?,
        reset_order_confirm: read(KEY_RESET_ORDER_CONFIRM)?,
        reset_order_confirm_yes: read(KEY_RESET_ORDER_YES)?,
        reset_order_confirm_cancel: read(KEY_RESET_ORDER_CANCEL)?,
        reset_order_done_template: read(KEY_RESET_ORDER_DONE)?,
        reset_order_failed_template: read(KEY_RESET_ORDER_FAILED)?,
        chip_filter_hint_template: read(KEY_CHIP_FILTER_HINT)?,
        chip_filter_clear_template: read(KEY_CHIP_FILTER_CLEAR)?,
    })
}

#[cfg(test)]
#[path = "wording_card_grammar_tests.rs"]
pub(crate) mod tests;
