//! Theme Scan request VALIDATION — the phase that runs before a scan records
//! anything, and whose failures are the caller's to fix.
//!
//! Split from `theme_scan.rs` (module-size limit) along the seam the 2026-07-28
//! ruling created. The three start-path modules now mirror the three phases:
//!
//! | module | phase | a failure here… |
//! |---|---|---|
//! | `theme_scan_validate` (this) | validate the request | …is a 4xx and NO run row |
//! | `theme_scan` (`prepare_scan`) | do the implied work | …is recorded on the run row |
//! | `theme_scan_start` | record, promote, judge | …is recorded on the run row |
//!
//! `theme_scan.rs` still owns the error taxonomy every phase returns, and the
//! case-fenced scenario loader they share.

use crate::dto::scenario_crud::ScenarioDefinition;
use crate::repositories::pipeline_repository::ScenarioRecord;
use crate::services::scenario_subject::{resolve_scenario_subject, SubjectResolveError};
use crate::services::theme_scan::{ScanPrompt, ThemeScanError, ValidatedScan};
use crate::services::theme_scan_provider::resolve_scan_provider;
use crate::state::AppState;
use uuid::Uuid;

/// Read the configured judging prompt — the TEMPLATE PRESENCE CHECK that runs
/// before a scan records anything at all.
///
/// The prompt filename is deployment config (env `THEME_SCAN_PROMPT_FILE`,
/// resolved+defaulted in `config.rs`), not a compiled-in const; `template_path`
/// resolves it against the registry's env-driven template dir.
///
/// ## Why this is the FIRST thing a scan start does
///
/// A missing prompt file is the cheapest possible failure — no scenario read, no
/// provider, no LLM budget, no run row — and for eleven days it was also one of
/// the two causes of scans that appeared to do nothing. Checking it at the door
/// means the caller gets a message naming the exact path, and no half-started run
/// is recorded for a scan that could never have run. The read IS the check: an
/// exists()-then-read would race, and would still have to handle the read error.
pub(crate) fn load_scan_prompt(state: &AppState) -> Result<ScanPrompt, ThemeScanError> {
    let file = state.config.theme_scan_prompt_file.clone();
    let path = state.registry.template_path(&file);
    let text = std::fs::read_to_string(&path)
        .map_err(|source| ThemeScanError::PromptFileMissing { path, source })?;
    Ok(ScanPrompt { file, text })
}

/// Validate the request: the checks whose failure is the CALLER'S to fix.
///
/// ## Why this is a separate phase (the 400 split, ruled 2026-07-28)
///
/// A scan that dies is supposed to leave a failed row in Run History — that is
/// the whole point of the stub. But that guarantee was applied to *every* start
/// failure, including the ones the human causes by clicking Scan too early: no
/// `attack_meaning` authored yet, a retired model still selected in the picker.
/// Those already announce themselves — the 400 lands in the panel immediately,
/// next to the control that caused it — so a row adds nothing and dilutes the
/// signal a real failure carries. Run History records the attempts the SYSTEM
/// owed an answer on.
///
/// So everything answerable from the scenario row and the model registry runs
/// HERE, before the stub is written, and returns its error straight to the
/// caller. Everything past this point — the vLLM gate, the graph read, the
/// judging — runs after the stub exists and is recorded when it fails.
///
/// The split is positional, and that is a deliberate simplification with one
/// consequence worth naming in full. THREE failures reachable from this phase are
/// not the caller's fault and still return without a row:
///
/// * [`ThemeScanError::ModelLookupFailed`] — the `llm_models` read failed.
/// * [`ThemeScanError::SubjectResolveFailed`] — the default-subject lookup failed
///   at the graph layer.
/// * [`ThemeScanError::ParamsInvalid`] — the model row was read but carries a
///   value the resolver rejects. It sits with the model-choice family (the route
///   answers 400, and picking another model IS a fix), but a corrupt stored row
///   is the other way to reach it.
///
/// All three are immediate, loud, and identical on every retry, and each now
/// carries its own recovery action in its message. That is what makes the toast a
/// sufficient surface for them — a Run History row would be read later, and later
/// is not when these are diagnosed. Making the rule class-based rather than
/// positional would buy three rare rows at the cost of a second place that writes
/// them, which is the cost this split exists to avoid.
pub(crate) async fn validate_scan_request(
    state: &AppState,
    record: ScenarioRecord,
    requested_model_id: Option<&str>,
) -> Result<ValidatedScan, ThemeScanError> {
    let scenario_id = record.scenario_id;

    let definition: ScenarioDefinition =
        serde_json::from_value(record.definition).map_err(|source| {
            ThemeScanError::DefinitionInvalid {
                scenario_id,
                source,
            }
        })?;

    // A scan with no judgment criteria is meaningless — reject the precondition.
    let attack_meaning = definition
        .attack_meaning
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(ThemeScanError::EmptyAttackMeaning { scenario_id })?
        .to_string();

    let subject_id = resolve_subject(&definition, scenario_id)?;

    // Per-run provider: resolve the model id → row → params → provider via the
    // unified seam (Chunk B), replacing the removed boot-time `theme_scan_provider`.
    let resolved = resolve_scan_provider(state, requested_model_id).await?;

    Ok(ValidatedScan {
        attack_meaning,
        subject_id,
        resolved,
    })
}

/// Resolve WHO the scan is about.
///
/// Delegated to the shared
/// [`crate::services::scenario_subject::resolve_scenario_subject`] so the scan
/// and the 1a.2 gather endpoint read the SAME subject pool by construction (see
/// that module's docs). The shared resolver's own error is mapped back into the
/// scan's existing [`ThemeScanError`] variants here — the scan's error surface
/// is unchanged; only where those variants are *constructed* moved.
///
/// Split from the candidate read (its former caller) by the 400 split: a
/// scenario that names nobody is a request the human can fix, and it belongs on
/// the pre-stub side of the run record for exactly that reason.
///
/// ## What changed on 2026-08-07
///
/// This used to be `async` and take `&AppState`, because the shared resolver
/// could fall back to a graph lookup for the case-default subject. That fallback
/// is gone (see `services::scenario_subject`), so there is no graph call and no
/// `DefaultLookupFailed` to map — one variant in, one variant out. A scan
/// started on a target-less scenario now refuses BY NAME instead of quietly
/// scanning a subject nobody chose and writing its verdicts into that
/// scenario's fact-refs.
fn resolve_subject(
    definition: &ScenarioDefinition,
    scenario_id: Uuid,
) -> Result<String, ThemeScanError> {
    let subject_id = resolve_scenario_subject(definition).map_err(|e| match e {
        SubjectResolveError::NoTarget => ThemeScanError::SubjectUnresolvable { scenario_id },
    })?;
    tracing::debug!(%scenario_id, subject_id = %subject_id, "theme scan: subject resolved");
    Ok(subject_id)
}
