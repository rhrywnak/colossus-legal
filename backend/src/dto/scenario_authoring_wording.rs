//! Wire mirrors for the scenario-definition authoring words (2026-08-07).
//!
//! `domain::wording_scenario_authoring::ScenarioAuthoringWording` holds thirteen
//! stored strings. Two different payloads carry subsets of it to two different
//! dialogs, and this module is where each subset is named and mapped.
//!
//! ## Why two DTOs and not one thirteen-field block sent to both
//!
//! Sending a surface only the words it speaks is the same discipline the wording
//! modules themselves follow, one layer out: a string that reaches a client with
//! no rule about when to show it is a string that will eventually be shown at
//! the wrong moment.
//!
//! The clearest case is `no_target_notice`, which is on neither DTO. It rides the
//! gather and cards payloads instead, because there its PRESENCE is the signal
//! that no target is set — a client holding it unconditionally could render it
//! beside a full queue.
//!
//! ## The two create refusals ARE sent, and that was a correction
//!
//! They were withheld at first, on the reasoning that they ride the 400 the
//! create route answers with, and a client holding its own copy could show a
//! different sentence than the server refused with. The architecture review
//! caught what that overlooked: the form validates BEFORE it sends, so when its
//! own check fires no request is made and the server's sentence is never
//! reached. Withholding them did not prevent a second voice — it guaranteed one,
//! and the form fell back to showing HELPER text (which describes what a field is
//! for) where a refusal belonged.
//!
//! Both surfaces now speak the same stored rows. The 400 remains the binding
//! refusal; these are the same words said a moment earlier, beside the control.

use serde::{Deserialize, Serialize};

use crate::domain::wording_scenario_authoring::ScenarioAuthoringWording;

/// The words the create-scenario form speaks (Trial Prep dashboard).
///
/// Only the two NEW fields' words. The form's older labels — "Name",
/// "Direction", "Status" — are still literals in the component; moving them is a
/// separate change with its own migration, recorded rather than smuggled into a
/// defect fix.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScenarioCreateWordingDto {
    pub target_label: String,
    pub target_helper: String,
    /// The unselected `<option>`. The form treats "still on this value" as "no
    /// choice made", so it is a label AND the empty state.
    pub target_unset_option: String,
    pub accusation_label: String,
    pub accusation_helper: String,
    /// Shown when the form is submitted with no target chosen — the same row the
    /// route's 400 carries, so the two surfaces cannot say different things.
    ///
    /// Distinct from `target_helper`: a helper says what a field is FOR, a
    /// refusal says what just went wrong and what to do about it.
    pub target_required: String,
    /// Shown when the form is submitted with a blank accusation. Same rule.
    pub accusation_required: String,
}

/// The words the identity modal speaks about a scenario's target (working page).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScenarioIdentityWordingDto {
    pub target_label: String,
    pub target_helper: String,
    pub target_unset_option: String,
    /// Shown when the party vocabulary cannot be read — and it tells the human
    /// NOT to save, because saving then would clear the target the scenario
    /// already has.
    pub target_options_failed_notice: String,
    /// Refuses a save that names a target while "what they say" is still blank —
    /// the one combination a stored definition has no valid shape for.
    pub target_needs_attack_text: String,
    /// Refuses a save that writes a "meant to imply" gloss while "what they say"
    /// is still blank. Sibling of the field above, and deliberately a different
    /// sentence: the two name different fields and different remedies.
    pub meaning_needs_attack_text: String,
    /// Why the scenario page's "Rehearsal view" control is inert when the
    /// scenario is not Ready. Carries `{status}`, filled by the browser from the
    /// status it is already rendering.
    ///
    /// ## Why the HEADER's word rides the identity payload
    ///
    /// The header has no read of its own — the page loads four payloads and this
    /// is the one that already carries authoring vocabulary for the same
    /// scenario. A fifth request for one string would buy nothing, and a copy on
    /// the detail payload would be a second row to keep in step.
    pub rehearsal_link_blocked_reason: String,
    /// The label on the control that opens Marie's practice drill (PRACTICE v0).
    pub practice_link_label: String,

    // ── The header strip and the Timeline-subsets section (T5) ─────────────
    pub rehearsal_disabled_tooltip: String,
    pub edit_subsets_section_title: String,
    pub edit_subsets_section_hint: String,
    pub edit_subsets_attached_state: String,
    pub edit_subsets_not_attached_state: String,
    pub edit_subsets_attach_button: String,
    pub edit_subsets_detach_button: String,
    pub edit_subsets_preview_link: String,
    pub edit_subsets_create_link: String,
    pub edit_subsets_create_hint: String,

    // ── The header strip's three button labels (T5 round two) ──────────────
    pub header_edit_label: String,
    pub header_rehearsal_view_label: String,
    pub header_delete_label: String,

    // ── The unified identity vocabulary (task R2; SHIPPED in R4) ────────────
    //
    // ## Why these nine arrived a build late
    //
    // Task R2 added them to the domain block, seeded them in a migration, and
    // declared them on the frontend type — and stopped one layer short of THIS
    // struct, which is the only one that crosses the wire. The read-only block
    // therefore rendered `undefined` into each of its four `<div>` labels, so
    // the attack, the theme, the motivation and the bears-on chips came out as
    // unlabelled back-to-back paragraphs (the beta.392 defect P1a). The four
    // `*_absent` rows went with them, which is the worse half: an unwritten text
    // rendered as an EMPTY italic paragraph rather than as its stated absence —
    // a blank where Standing Rule 1 requires a sentence.
    //
    // TypeScript could not catch it. The client type declares `attack_label:
    // string`; a field the server never serialises simply arrives `undefined`
    // and satisfies no check at runtime. `identity_wording_carries_every_field`
    // in this module's test file is what catches it now, by walking the domain
    // block's own field list rather than a list a human maintains here.
    /// `definition->>attack_text` — what the other side claims.
    pub attack_label: String,
    /// Its stated absence. Never an empty paragraph: that reads as a render fault.
    pub attack_absent: String,
    /// The `theme_statement` column — our answer in one sentence.
    pub theme_label: String,
    pub theme_absent: String,
    /// Says who reads the theme and where, which is what stops it being written
    /// as a case note instead of a line spoken aloud. The editor renders it; the
    /// read-only block does not, and both take it from this one row.
    pub theme_helper: String,
    /// The `motivation` column.
    pub motivation_label: String,
    pub motivation_absent: String,
    /// The `anchor_allegation_ids` chips.
    pub bears_on_label: String,
    pub bears_on_absent: String,
}

/// Map the stored block onto the create form's subset.
///
/// ## Rust Learning: a `fn` at the DTO boundary, not a `From` impl
///
/// `From<&ScenarioAuthoringWording>` would read nicely at the call site, but two
/// different DTOs map from the SAME source type here — and a type can only have
/// one `From<T>` for a given `T`. Named functions say which subset is being
/// taken, and adding a third surface later needs no rearranging.
pub fn create_wording(w: &ScenarioAuthoringWording) -> ScenarioCreateWordingDto {
    ScenarioCreateWordingDto {
        target_label: w.create_target_label.clone(),
        target_helper: w.create_target_helper.clone(),
        target_unset_option: w.create_target_unset_option.clone(),
        accusation_label: w.create_accusation_label.clone(),
        accusation_helper: w.create_accusation_helper.clone(),
        target_required: w.create_target_required_refusal.clone(),
        accusation_required: w.create_accusation_required_refusal.clone(),
    }
}

/// Map the stored block onto the identity modal's subset.
pub fn identity_wording(w: &ScenarioAuthoringWording) -> ScenarioIdentityWordingDto {
    ScenarioIdentityWordingDto {
        target_label: w.identity_target_label.clone(),
        target_helper: w.identity_target_helper.clone(),
        target_unset_option: w.identity_target_unset_option.clone(),
        target_options_failed_notice: w.target_options_failed_notice.clone(),
        target_needs_attack_text: w.identity_target_needs_attack_text.clone(),
        meaning_needs_attack_text: w.identity_meaning_needs_attack_text.clone(),
        rehearsal_link_blocked_reason: w.rehearsal_link_blocked_reason.clone(),
        practice_link_label: w.practice_link_label.clone(),
        rehearsal_disabled_tooltip: w.rehearsal_disabled_tooltip.clone(),
        edit_subsets_section_title: w.edit_subsets_section_title.clone(),
        edit_subsets_section_hint: w.edit_subsets_section_hint.clone(),
        edit_subsets_attached_state: w.edit_subsets_attached_state.clone(),
        edit_subsets_not_attached_state: w.edit_subsets_not_attached_state.clone(),
        edit_subsets_attach_button: w.edit_subsets_attach_button.clone(),
        edit_subsets_detach_button: w.edit_subsets_detach_button.clone(),
        edit_subsets_preview_link: w.edit_subsets_preview_link.clone(),
        edit_subsets_create_link: w.edit_subsets_create_link.clone(),
        edit_subsets_create_hint: w.edit_subsets_create_hint.clone(),
        header_edit_label: w.header_edit_label.clone(),
        header_rehearsal_view_label: w.header_rehearsal_view_label.clone(),
        header_delete_label: w.header_delete_label.clone(),
        // The `identity_` prefix is dropped on the wire: over there the whole
        // struct IS the identity vocabulary, so repeating it in every field name
        // would be the payload saying "identity" nine times to one reader.
        attack_label: w.identity_attack_label.clone(),
        attack_absent: w.identity_attack_absent.clone(),
        theme_label: w.identity_theme_label.clone(),
        theme_absent: w.identity_theme_absent.clone(),
        theme_helper: w.identity_theme_helper.clone(),
        motivation_label: w.identity_motivation_label.clone(),
        motivation_absent: w.identity_motivation_absent.clone(),
        bears_on_label: w.identity_bears_on_label.clone(),
        bears_on_absent: w.identity_bears_on_absent.clone(),
    }
}

#[cfg(test)]
#[path = "scenario_authoring_wording_tests.rs"]
mod tests;
