//! Reading the stored-string blocks out of the settings store.
//!
//! Split from `settings_store` on 2026-08-07, when adding a sixth block took
//! that module past the 300-line limit (Rule 17). The seam is the one that
//! module's own header already argued: `build_settings` decides the NUMBERS this
//! system judges by, and this decides the WORDS it speaks.
//!
//! Nothing here does I/O. It takes the rows the store has already read and turns
//! them into eight typed blocks, or names the first key that is wrong.

use std::collections::HashMap;

use crate::domain::settings::SettingError;
use crate::domain::wording::{build_wording, Wording};
use crate::domain::wording_accusation::{build_accusation_wording, AccusationWording};
use crate::domain::wording_authoring::{build_authoring_wording, AuthoringWording};
use crate::domain::wording_card_grammar::{build_card_grammar_wording, CardGrammarWording};
use crate::domain::wording_model_params::{build_model_params_wording, ModelParamsWording};
use crate::domain::wording_rehearsal::{build_rehearsal_wording, RehearsalWording};
use crate::domain::wording_rehearsal_chrome::{
    build_rehearsal_chrome_wording, RehearsalChromeWording,
};
use crate::domain::wording_scan::{build_scan_wording, ScanWording};
use crate::domain::wording_scenario_authoring::{
    build_scenario_authoring_wording, ScenarioAuthoringWording,
};
use crate::repositories::pipeline_repository::AppSettingRecord;
use crate::services::settings_store::{require, text_of};

/// Every stored-string block, read by one rule.
///
/// ## Rust Learning: a named struct instead of a six-tuple
///
/// This started as `(Wording, AccusationWording, RehearsalWording)` and task
/// 2.11 C would have made it five long. A tuple that wide is positions nobody
/// can read at the call site, and — worse — two of them would be
/// `RehearsalWording` and `RehearsalChromeWording`, whose order the compiler
/// WOULD catch but whose neighbours' would not. Named fields cost a few lines
/// and remove the whole class of mistake.
pub(crate) struct AllWording {
    pub(crate) curation: Wording,
    pub(crate) accusation: AccusationWording,
    pub(crate) rehearsal: RehearsalWording,
    pub(crate) chrome: RehearsalChromeWording,
    pub(crate) authoring: AuthoringWording,
    pub(crate) scenario_authoring: ScenarioAuthoringWording,
    /// The scan surface's three strings (task 2.15 Tier 2).
    pub(crate) scan: ScanWording,
    /// The words one evidence card speaks under either wrapper (ruling R6).
    pub(crate) card_grammar: CardGrammarWording,
    /// The models admin's temperature control (ruling R5, 2026-08-09).
    pub(crate) model_params: ModelParamsWording,
}

/// The eight stored-string blocks, read by one rule.
///
/// Each block lives in its own `domain` module (Rule 17 — none of them fits in
/// one), and all eight are read by the same closure, so a row must exist, declare
/// `text`, and carry something non-blank whichever surface it belongs to.
///
/// ## Why the rehearsal block's derived shapes are parsed HERE
///
/// `always_lines` and `section_states` turn stored text into a list and five
/// enums. Doing it at boot means a blank standing card or an unreadable state
/// token stops the service with a named refusal; doing it at render would
/// surprise a witness mid-rehearsal, on the one block §10 says is never scrolled
/// away from.
///
/// # Errors
/// Returns [`SettingError`] naming the first key that is missing, of the wrong
/// declared kind, blank, or — for the two derived shapes — unreadable.
pub(crate) fn build_all_wording(
    rows: &HashMap<String, AppSettingRecord>,
) -> Result<AllWording, SettingError> {
    let curation = build_wording(|key| text_of(require(rows, key)?))?;
    let accusation = build_accusation_wording(|key| text_of(require(rows, key)?))?;
    let rehearsal = build_rehearsal_wording(|key| text_of(require(rows, key)?))?;
    let chrome = build_rehearsal_chrome_wording(|key| text_of(require(rows, key)?))?;
    let authoring = build_authoring_wording(|key| text_of(require(rows, key)?))?;
    let scenario_authoring = build_scenario_authoring_wording(|key| text_of(require(rows, key)?))?;
    let scan = build_scan_wording(|key| text_of(require(rows, key)?))?;
    let card_grammar = build_card_grammar_wording(|key| text_of(require(rows, key)?))?;
    let model_params = build_model_params_wording(|key| text_of(require(rows, key)?))?;

    rehearsal.always_lines()?;
    rehearsal.section_states()?;

    Ok(AllWording {
        curation,
        accusation,
        rehearsal,
        chrome,
        authoring,
        scenario_authoring,
        scan,
        card_grammar,
        model_params,
    })
}
