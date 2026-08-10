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
    }
}
