// =============================================================================
// backend/src/domain/wording_model_params.rs — the words the MODELS ADMIN speaks
// =============================================================================
//
// The ninth stored-string block, approved as ruling R5 of the
// CC_TASK_MODEL_PARAMS Phase B authorization (2026-08-09). It carries the
// sentences Admin → Models needs to let an operator record what a model does
// with a `temperature` parameter — and nothing else.
//
// ## Why this block exists at all
//
// `llm_models.temperature_mode` is a TOKEN: `zero-ok`, `omit`, or nothing. A
// dropdown offering those three words is a dropdown only the person who wrote
// the resolver can use. Roman is the operator here, and the question he is
// actually answering is "does this model accept a temperature?" — so the screen
// has to ask him that, in those words, and the words are rows (v2 §2b).
//
// ## Why a new module rather than more keys on a sibling
//
// The same test the eight siblings apply: which SURFACE speaks these, and does
// its vocabulary move independently? Every existing block belongs to the CASE
// surfaces — curation, rehearsal, authoring, the scan panel, the card grammar —
// and speaks to somebody working the litigation. This one speaks to somebody
// configuring an LLM registry, on a page no case work ever reaches. It will move
// when a provider changes what it accepts, which has nothing to do with what a
// candidate card says.
//
// ## Domain note: the incident these words exist to prevent
//
// On 2026-08-09 scan run 6a9fad89 sent `temperature: 0` to `claude-opus-5`,
// which no longer accepts the parameter, and all 104 judge calls came back 400
// in five seconds. The row for that model had never had its mode recorded —
// there was no screen on which to record it. This block is that screen's words.

/// The stored strings the models admin renders for the temperature control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelParamsWording {
    /// The label above the mode dropdown.
    pub temperature_mode_label: String,
    /// What the dropdown means, in one sentence, under the control.
    pub temperature_mode_help: String,
    /// The option for a model that REJECTS the parameter.
    pub temperature_mode_omit_label: String,
    /// The option for a model that accepts an explicit number.
    pub temperature_mode_zero_ok_label: String,
    /// What an UNRECORDED row reads as before anybody chooses.
    ///
    /// Domain note: shown, never offered. A mode can be recorded and cannot be
    /// un-recorded from this form, which is deliberate — "nobody has said" is a
    /// gap to close, not a setting to choose. Since 2026-08-09 an unrecorded mode
    /// behaves exactly like `omit` (ruling R1), so the sentence says so rather
    /// than leaving the reader to wonder what is being sent meanwhile.
    pub temperature_mode_unset_label: String,
    /// The label above the numeric value field.
    pub temperature_value_label: String,
    /// Why the numeric field is unavailable while the mode sends nothing.
    pub temperature_value_disabled_help: String,
}

// KEYS: the stable identifiers. Renaming one is a migration, and until it runs
// the boot loader refuses to start.
pub(crate) const KEY_TEMPERATURE_MODE_LABEL: &str = "model_temperature_mode_label";
pub(crate) const KEY_TEMPERATURE_MODE_HELP: &str = "model_temperature_mode_help";
pub(crate) const KEY_TEMPERATURE_MODE_OMIT: &str = "model_temperature_mode_omit_label";
pub(crate) const KEY_TEMPERATURE_MODE_ZERO_OK: &str = "model_temperature_mode_zero_ok_label";
pub(crate) const KEY_TEMPERATURE_MODE_UNSET: &str = "model_temperature_mode_unset_label";
pub(crate) const KEY_TEMPERATURE_VALUE_LABEL: &str = "model_temperature_value_label";
pub(crate) const KEY_TEMPERATURE_VALUE_DISABLED: &str = "model_temperature_value_disabled_help";

/// Every models-admin key this build reads, so a missing one is caught at boot BY
/// NAME rather than as a blank dropdown in front of an operator.
pub const MODEL_PARAMS_WORDING_KEYS: &[&str] = &[
    KEY_TEMPERATURE_MODE_LABEL,
    KEY_TEMPERATURE_MODE_HELP,
    KEY_TEMPERATURE_MODE_OMIT,
    KEY_TEMPERATURE_MODE_ZERO_OK,
    KEY_TEMPERATURE_MODE_UNSET,
    KEY_TEMPERATURE_VALUE_LABEL,
    KEY_TEMPERATURE_VALUE_DISABLED,
];

/// Build a [`ModelParamsWording`] from the stored rows, or say which key is wrong.
///
/// Same generic-closure shape as the eight sibling builders — see
/// [`crate::domain::wording_scenario_authoring::build_scenario_authoring_wording`]
/// for why `read` is a closure over a generic error type rather than a database
/// handle.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_model_params_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<ModelParamsWording, E> {
    Ok(ModelParamsWording {
        temperature_mode_label: read(KEY_TEMPERATURE_MODE_LABEL)?,
        temperature_mode_help: read(KEY_TEMPERATURE_MODE_HELP)?,
        temperature_mode_omit_label: read(KEY_TEMPERATURE_MODE_OMIT)?,
        temperature_mode_zero_ok_label: read(KEY_TEMPERATURE_MODE_ZERO_OK)?,
        temperature_mode_unset_label: read(KEY_TEMPERATURE_MODE_UNSET)?,
        temperature_value_label: read(KEY_TEMPERATURE_VALUE_LABEL)?,
        temperature_value_disabled_help: read(KEY_TEMPERATURE_VALUE_DISABLED)?,
    })
}

#[cfg(test)]
#[path = "wording_model_params_tests.rs"]
pub(crate) mod tests;
