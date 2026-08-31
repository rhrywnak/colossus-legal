// =============================================================================
// backend/src/domain/wording_scenario_authoring.rs — the words that define a scenario
// =============================================================================
//
// The thirteen strings the two surfaces that AUTHOR a scenario's definition speak:
// the create form on the Trial Prep dashboard, and the identity modal on the
// working page. Plus the two notices a scenario shows in place of a candidate
// queue — when it has no target and gathers nothing, and (task 2.15) when nothing
// has scanned it yet.
//
// ## Why these are rows and not literals in the React components
//
// The same law every sibling module stands on (v2 §2b, extended from numbers to
// text): a sentence a human reads is configuration. `ScenarioCreateForm.tsx`
// carried its labels as literals because it predates that law and nothing new
// was added to it since. This task adds two fields, and a new literal on a
// surface the law covers is a new violation — so the new words arrive as rows.
//
// The form's PRE-EXISTING literals ("Name", "Direction", "Status") are left
// where they are: moving them is a separate, larger change with its own
// migration, and mixing it into a defect fix would put untested string plumbing
// in the same commit as the fix Roman is waiting on. Recorded, not smuggled.
//
// ## Why one module serves two surfaces here, when `wording_authoring` split them
//
// `wording_authoring`'s header explains why the scenario page and the rehearsal
// page do NOT share rows: they speak to two audiences (Roman curating, Marie
// preparing) whose vocabularies move independently. That reasoning does not
// apply here. The create form and the identity modal have ONE audience — the
// person defining a scenario — and they ask the same two questions minutes
// apart. Two rows reading "Who this scenario is about" would be one label free
// to drift from itself between the dialog that creates a scenario and the dialog
// that fixes it, which is the confusion the split exists to prevent, inverted.
//
// They are still distinct keys rather than shared ones, because the two moments
// genuinely differ: creating asks "choose", editing warns "changing this changes
// what gathers".

/// Every stored string the scenario-definition authoring surfaces render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioAuthoringWording {
    // ── The create form (Trial Prep dashboard) ──────────────────────────────
    pub create_target_label: String,
    /// Says what the target DOES, not what it is. The person filling this in is
    /// the person who will otherwise wonder why their scenario is empty.
    pub create_target_helper: String,
    /// The unselected `<option>`. Never a person's name: a pre-selected human
    /// being reads as a choice somebody made.
    pub create_target_unset_option: String,
    pub create_accusation_label: String,
    /// Names the consumer (the Theme Scan) so the writer knows who reads it.
    pub create_accusation_helper: String,
    /// The refusal when no target was chosen. Names the consequence, because a
    /// refusal that only says "required" does not tell you why it matters.
    pub create_target_required_refusal: String,
    /// The refusal when the accusation is blank.
    pub create_accusation_required_refusal: String,

    // ── The no-target state (working page: card queue and gather) ───────────
    /// Shown IN PLACE OF the candidate queue on a scenario with no target. This
    /// is the sentence that would have prevented 2026-08-07: without it, a
    /// target-less scenario silently borrowed the case-default subject's pool
    /// and rendered as a full one.
    pub no_target_notice: String,
    /// Shown IN PLACE OF the candidate queue on a scenario NOTHING HAS SCANNED
    /// yet (task 2.15, 2026-08-08). Its sibling above answers "there is nobody to
    /// gather about"; this one answers "nobody has judged any of this".
    ///
    /// Domain note: measured 2026-08-07 — a freshly created scenario led with 148
    /// raw pool cards labelled "Candidates awaiting ruling — from all scans", on
    /// a screen that simultaneously reported them as never scanned. The workflow
    /// is create → scan → rule, and this sentence is the first step said out loud.
    pub never_scanned_notice: String,

    // ── The identity modal (working page) ───────────────────────────────────
    pub identity_target_label: String,
    /// Warns that the change has an effect beyond the field, and that rulings
    /// survive it — a human who fears losing two weeks of curation will not
    /// touch the control.
    pub identity_target_helper: String,
    /// The "no target" `<option>`, which names the consequence rather than
    /// reading as a blank.
    pub identity_target_unset_option: String,
    /// Shown when the party vocabulary cannot be read. Says explicitly not to
    /// save, because saving a modal whose target list failed to load would write
    /// back an emptied target (Standing Rule 1: the failure names its own hazard).
    pub target_options_failed_notice: String,
    /// Refuses a save that would choose a target while leaving "what they say"
    /// blank.
    ///
    /// ## Why that combination needs its own sentence
    ///
    /// `attack_text` is required by the definition's parse contract, so a
    /// definition carrying a target and no attack text cannot be stored as a
    /// valid one. Without this refusal the modal would do what it did before —
    /// omit the definition entirely — and the human would watch their chosen
    /// target vanish on reopen with nothing said. Reachable only on legacy rows
    /// created before the create form asked for either.
    pub identity_target_needs_attack_text: String,
    /// Refuses a save that would write a "what that is meant to imply" gloss
    /// while leaving "what they say" blank (task R1 Piece 5a).
    ///
    /// ## Why this is a SECOND sentence and not the one above reworded
    ///
    /// The omission is the same — no `attack_text` means the whole definition is
    /// dropped — but the two humans are not. One picked a person from a list; one
    /// typed a sentence. A single refusal covering both would have to name
    /// neither field, and a refusal that cannot say WHICH of your edits is about
    /// to be discarded is barely better than the silence it replaces.
    pub identity_meaning_needs_attack_text: String,

    // ── The scenario page header (task R1 Piece 1b) ─────────────────────────
    /// Why the "Rehearsal view" control is inert on a scenario that is not Ready.
    ///
    /// Carries `{status}` — the state the scenario is actually in. Rehearsal mode
    /// serves READY scenarios through a human gate (v2 §5/§10); before .390 the
    /// control looked alive on every scenario and, clicked on a Draft one,
    /// silently delivered a different scenario's rehearsal.
    pub rehearsal_link_blocked_reason: String,
    /// The label on the control that opens Marie's practice drill.
    ///
    /// Its neighbour, [`Self::rehearsal_link_blocked_reason`], exists because the
    /// rehearsal link can be INERT. This one has no such row: the drill is never
    /// gated, because it is the surface that reports a scenario has no deck yet,
    /// and a control that hid itself for the un-seeded case would hide the only
    /// screen able to say so.
    pub practice_link_label: String,

    // ── The header strip and the Timeline-subsets section (T5) ─────────────
    /// The SHORT tooltip on the strip's disabled "Rehearsal view" control. No
    /// `{status}`: the segmented control beside it already shows the state.
    pub rehearsal_disabled_tooltip: String,
    /// The heading of the attach/detach section.
    pub edit_subsets_section_title: String,
    /// What the section teaches: where an attached subset shows up, that
    /// attachment is many-to-many, and that Detach is not Delete.
    pub edit_subsets_section_hint: String,
    /// The state word on a row this scenario carries. The ✓ is furniture, in code.
    pub edit_subsets_attached_state: String,
    /// The mirror of [`Self::edit_subsets_attached_state`]. Edit the two together.
    pub edit_subsets_not_attached_state: String,
    /// The button that links a subset. Explicit, not a ✓-toggle — defect D10.
    pub edit_subsets_attach_button: String,
    /// The button that unlinks one. Never "Remove": the subset is untouched.
    pub edit_subsets_detach_button: String,
    /// Opens the floating window on one subset WITHOUT attaching it.
    pub edit_subsets_preview_link: String,
    /// The link out to the timeline, where subsets are made.
    pub edit_subsets_create_link: String,
    /// The muted aside promising a new tab and saying what to do on return.
    pub edit_subsets_create_hint: String,

    // ── The unified identity vocabulary (task R2, Roman 2026-08-10) ─────────
    //
    // ONE row per idea, rendered by BOTH the read-only identity block and the
    // editor. Not two rows seeded alike: the defect being closed is two names for
    // one column, and two rows would be free to drift apart the first time either
    // is edited. The header's names won.
    /// `definition->>attack_text` — what the other side claims.
    pub identity_attack_label: String,
    /// Its stated absence. Never an empty paragraph: that reads as a render fault.
    pub identity_attack_absent: String,
    /// The `theme_statement` column — our answer in one sentence.
    pub identity_theme_label: String,
    pub identity_theme_absent: String,
    /// Says who reads the theme and where, which is what stops it being written
    /// as a case note instead of a line spoken aloud.
    pub identity_theme_helper: String,
    /// The `motivation` column.
    pub identity_motivation_label: String,
    pub identity_motivation_absent: String,
    /// The `anchor_allegation_ids` chips.
    pub identity_bears_on_label: String,
    pub identity_bears_on_absent: String,
}

// KEYS: the stable identifiers of the fourteen stored strings. Renaming one is a
// migration, and until it runs the boot loader refuses to start.
pub(crate) const KEY_CREATE_TARGET_LABEL: &str = "scenario_create_target_label";
pub(crate) const KEY_CREATE_TARGET_HELPER: &str = "scenario_create_target_helper";
pub(crate) const KEY_CREATE_TARGET_UNSET: &str = "scenario_create_target_unset_option";
pub(crate) const KEY_CREATE_ACCUSATION_LABEL: &str = "scenario_create_accusation_label";
pub(crate) const KEY_CREATE_ACCUSATION_HELPER: &str = "scenario_create_accusation_helper";
pub(crate) const KEY_CREATE_TARGET_REQUIRED: &str = "scenario_create_target_required_refusal";
pub(crate) const KEY_CREATE_ACCUSATION_REQUIRED: &str =
    "scenario_create_accusation_required_refusal";
pub(crate) const KEY_NO_TARGET_NOTICE: &str = "scenario_no_target_notice";
pub(crate) const KEY_NEVER_SCANNED_NOTICE: &str = "scenario_never_scanned_notice";
pub(crate) const KEY_IDENTITY_TARGET_LABEL: &str = "scenario_identity_target_label";
pub(crate) const KEY_IDENTITY_TARGET_HELPER: &str = "scenario_identity_target_helper";
pub(crate) const KEY_IDENTITY_TARGET_UNSET: &str = "scenario_identity_target_unset_option";
pub(crate) const KEY_TARGET_OPTIONS_FAILED: &str = "scenario_target_options_failed_notice";
pub(crate) const KEY_TARGET_NEEDS_ATTACK_TEXT: &str = "scenario_identity_target_needs_attack_text";
pub(crate) const KEY_MEANING_NEEDS_ATTACK_TEXT: &str =
    "scenario_identity_meaning_needs_attack_text";
pub(crate) const KEY_REHEARSAL_LINK_BLOCKED: &str = "scenario_rehearsal_link_blocked_reason";
pub(crate) const KEY_IDENTITY_ATTACK_LABEL: &str = "scenario_identity_attack_label";
pub(crate) const KEY_IDENTITY_ATTACK_ABSENT: &str = "scenario_identity_attack_absent";
pub(crate) const KEY_IDENTITY_THEME_LABEL: &str = "scenario_identity_theme_label";
pub(crate) const KEY_IDENTITY_THEME_ABSENT: &str = "scenario_identity_theme_absent";
pub(crate) const KEY_IDENTITY_THEME_HELPER: &str = "scenario_identity_theme_helper";
pub(crate) const KEY_IDENTITY_MOTIVATION_LABEL: &str = "scenario_identity_motivation_label";
pub(crate) const KEY_IDENTITY_MOTIVATION_ABSENT: &str = "scenario_identity_motivation_absent";
pub(crate) const KEY_IDENTITY_BEARS_ON_LABEL: &str = "scenario_identity_bears_on_label";
pub(crate) const KEY_IDENTITY_BEARS_ON_ABSENT: &str = "scenario_identity_bears_on_absent";
/// PRACTICE v0: the control that opens Marie's drill.
///
/// In THIS block and not the practice one because a wording block belongs to the
/// surface that SPEAKS it, and the surface here is the scenario page — which
/// already fetches this block. The alternative measured worse in the obvious
/// way: the page would have fetched a whole practice deck payload to learn one
/// word, with a failure path of its own to surface.
pub(crate) const KEY_PRACTICE_LINK_LABEL: &str = "scenario_practice_link_label";

// ── The header strip and the Timeline-subsets section (T5, 2026-08-31) ──────
//
// One word for Screen 1's disabled Rehearsal control, nine for Screen 4's
// section. See the migration's header for why the long rehearsal sentence stays
// in the store while leaving the page.
pub(crate) const KEY_REHEARSAL_DISABLED_TOOLTIP: &str = "scenario_rehearsal_disabled_tooltip";
pub(crate) const KEY_EDIT_SUBSETS_SECTION_TITLE: &str = "scenario_edit_subsets_section_title";
pub(crate) const KEY_EDIT_SUBSETS_SECTION_HINT: &str = "scenario_edit_subsets_section_hint";
pub(crate) const KEY_EDIT_SUBSETS_ATTACHED_STATE: &str = "scenario_edit_subsets_attached_state";
pub(crate) const KEY_EDIT_SUBSETS_NOT_ATTACHED_STATE: &str =
    "scenario_edit_subsets_not_attached_state";
pub(crate) const KEY_EDIT_SUBSETS_ATTACH_BUTTON: &str = "scenario_edit_subsets_attach_button";
pub(crate) const KEY_EDIT_SUBSETS_DETACH_BUTTON: &str = "scenario_edit_subsets_detach_button";
pub(crate) const KEY_EDIT_SUBSETS_PREVIEW_LINK: &str = "scenario_edit_subsets_preview_link";
pub(crate) const KEY_EDIT_SUBSETS_CREATE_LINK: &str = "scenario_edit_subsets_create_link";
pub(crate) const KEY_EDIT_SUBSETS_CREATE_HINT: &str = "scenario_edit_subsets_create_hint";

/// Every scenario-authoring key this build reads, so a missing one is caught at
/// boot BY NAME rather than as a blank label in front of a human mid-sentence.
pub const SCENARIO_AUTHORING_WORDING_KEYS: &[&str] = &[
    KEY_CREATE_TARGET_LABEL,
    KEY_CREATE_TARGET_HELPER,
    KEY_CREATE_TARGET_UNSET,
    KEY_CREATE_ACCUSATION_LABEL,
    KEY_CREATE_ACCUSATION_HELPER,
    KEY_CREATE_TARGET_REQUIRED,
    KEY_CREATE_ACCUSATION_REQUIRED,
    KEY_NO_TARGET_NOTICE,
    KEY_NEVER_SCANNED_NOTICE,
    KEY_IDENTITY_TARGET_LABEL,
    KEY_IDENTITY_TARGET_HELPER,
    KEY_IDENTITY_TARGET_UNSET,
    KEY_TARGET_OPTIONS_FAILED,
    KEY_TARGET_NEEDS_ATTACK_TEXT,
    KEY_MEANING_NEEDS_ATTACK_TEXT,
    KEY_REHEARSAL_LINK_BLOCKED,
    KEY_PRACTICE_LINK_LABEL,
    // Task 5.
    KEY_REHEARSAL_DISABLED_TOOLTIP,
    KEY_EDIT_SUBSETS_SECTION_TITLE,
    KEY_EDIT_SUBSETS_SECTION_HINT,
    KEY_EDIT_SUBSETS_ATTACHED_STATE,
    KEY_EDIT_SUBSETS_NOT_ATTACHED_STATE,
    KEY_EDIT_SUBSETS_ATTACH_BUTTON,
    KEY_EDIT_SUBSETS_DETACH_BUTTON,
    KEY_EDIT_SUBSETS_PREVIEW_LINK,
    KEY_EDIT_SUBSETS_CREATE_LINK,
    KEY_EDIT_SUBSETS_CREATE_HINT,
    KEY_IDENTITY_ATTACK_LABEL,
    KEY_IDENTITY_ATTACK_ABSENT,
    KEY_IDENTITY_THEME_LABEL,
    KEY_IDENTITY_THEME_ABSENT,
    KEY_IDENTITY_THEME_HELPER,
    KEY_IDENTITY_MOTIVATION_LABEL,
    KEY_IDENTITY_MOTIVATION_ABSENT,
    KEY_IDENTITY_BEARS_ON_LABEL,
    KEY_IDENTITY_BEARS_ON_ABSENT,
];

/// Build a [`ScenarioAuthoringWording`] from the stored rows, or say which key
/// is wrong.
///
/// ## Rust Learning: a generic closure parameter instead of a database handle
///
/// `read` is `impl Fn(&str) -> Result<String, E>` — any callable that turns a
/// key into a value or an error of the caller's choosing. That is what lets ONE
/// builder serve two very different callers: production passes a closure that
/// reads the `app_settings` rows (`E = SettingError`), while the test fixture
/// passes one that reads an in-memory table (`E = String`). The generic `E` is
/// the reason neither caller has to adopt the other's error type. Same shape as
/// all five sibling builders.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_scenario_authoring_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<ScenarioAuthoringWording, E> {
    Ok(ScenarioAuthoringWording {
        create_target_label: read(KEY_CREATE_TARGET_LABEL)?,
        create_target_helper: read(KEY_CREATE_TARGET_HELPER)?,
        create_target_unset_option: read(KEY_CREATE_TARGET_UNSET)?,
        create_accusation_label: read(KEY_CREATE_ACCUSATION_LABEL)?,
        create_accusation_helper: read(KEY_CREATE_ACCUSATION_HELPER)?,
        create_target_required_refusal: read(KEY_CREATE_TARGET_REQUIRED)?,
        create_accusation_required_refusal: read(KEY_CREATE_ACCUSATION_REQUIRED)?,
        no_target_notice: read(KEY_NO_TARGET_NOTICE)?,
        never_scanned_notice: read(KEY_NEVER_SCANNED_NOTICE)?,
        identity_target_label: read(KEY_IDENTITY_TARGET_LABEL)?,
        identity_target_helper: read(KEY_IDENTITY_TARGET_HELPER)?,
        identity_target_unset_option: read(KEY_IDENTITY_TARGET_UNSET)?,
        target_options_failed_notice: read(KEY_TARGET_OPTIONS_FAILED)?,
        identity_target_needs_attack_text: read(KEY_TARGET_NEEDS_ATTACK_TEXT)?,
        identity_meaning_needs_attack_text: read(KEY_MEANING_NEEDS_ATTACK_TEXT)?,
        rehearsal_link_blocked_reason: read(KEY_REHEARSAL_LINK_BLOCKED)?,
        practice_link_label: read(KEY_PRACTICE_LINK_LABEL)?,
        rehearsal_disabled_tooltip: read(KEY_REHEARSAL_DISABLED_TOOLTIP)?,
        edit_subsets_section_title: read(KEY_EDIT_SUBSETS_SECTION_TITLE)?,
        edit_subsets_section_hint: read(KEY_EDIT_SUBSETS_SECTION_HINT)?,
        edit_subsets_attached_state: read(KEY_EDIT_SUBSETS_ATTACHED_STATE)?,
        edit_subsets_not_attached_state: read(KEY_EDIT_SUBSETS_NOT_ATTACHED_STATE)?,
        edit_subsets_attach_button: read(KEY_EDIT_SUBSETS_ATTACH_BUTTON)?,
        edit_subsets_detach_button: read(KEY_EDIT_SUBSETS_DETACH_BUTTON)?,
        edit_subsets_preview_link: read(KEY_EDIT_SUBSETS_PREVIEW_LINK)?,
        edit_subsets_create_link: read(KEY_EDIT_SUBSETS_CREATE_LINK)?,
        edit_subsets_create_hint: read(KEY_EDIT_SUBSETS_CREATE_HINT)?,
        identity_attack_label: read(KEY_IDENTITY_ATTACK_LABEL)?,
        identity_attack_absent: read(KEY_IDENTITY_ATTACK_ABSENT)?,
        identity_theme_label: read(KEY_IDENTITY_THEME_LABEL)?,
        identity_theme_absent: read(KEY_IDENTITY_THEME_ABSENT)?,
        identity_theme_helper: read(KEY_IDENTITY_THEME_HELPER)?,
        identity_motivation_label: read(KEY_IDENTITY_MOTIVATION_LABEL)?,
        identity_motivation_absent: read(KEY_IDENTITY_MOTIVATION_ABSENT)?,
        identity_bears_on_label: read(KEY_IDENTITY_BEARS_ON_LABEL)?,
        identity_bears_on_absent: read(KEY_IDENTITY_BEARS_ON_ABSENT)?,
    })
}

#[cfg(test)]
#[path = "wording_scenario_authoring_tests.rs"]
pub(crate) mod tests;
