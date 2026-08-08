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
use crate::services::settings_template_file::TemplateDir;
use crate::services::theme_scan::{ScanPrompt, ThemeScanError, ValidatedScan};
use crate::services::theme_scan_provider::resolve_scan_provider;
use crate::state::AppState;
use uuid::Uuid;

/// Read the configured judging prompt — the TEMPLATE PRESENCE CHECK that runs
/// before a scan records anything at all.
///
/// The prompt filename is a SETTINGS ROW (`theme_scan_prompt_file`, task 2.15
/// Tier 2 — Roman's amendment of 2026-08-08), read from the live snapshot here at
/// scan start; `template_path` resolves it against the registry's env-driven
/// template dir. It was an env var with a compiled-in default until this task, and
/// the change is what makes "which prompt judges my scans" both visible and
/// editable without a deploy.
///
/// ## Why the row is read HERE and not cached anywhere
///
/// The settings write path swaps the whole snapshot before its response returns
/// (the freshness law), so a prompt changed on the Settings page is in force for
/// the NEXT scan with no restart. Reading it at the top of the start sequence is
/// what makes that true — and it is also why the boot assertion in `main.rs` is
/// not the last word: the row can change after boot, so the read still has to
/// fail loudly on its own.
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
    read_scan_prompt(
        state.settings.current().theme_scan_prompt_file.clone(),
        &TemplateDir::new(state.registry.template_dir()),
    )
}

/// The read itself, with the two inputs handed in.
///
/// Split from [`load_scan_prompt`] so the one behaviour that matters — **the scan
/// reads the file the settings row names** — is testable without an `AppState`.
/// A function taking `&AppState` needs a database, a graph and a registry before
/// it can be called at all; this one needs a string and a directory, so a test can
/// put two prompts on disk, name each in turn, and prove the scan follows the row.
///
/// # Errors
/// Returns [`ThemeScanError::PromptFileMissing`] naming the full path it tried.
pub(crate) fn read_scan_prompt(
    file: String,
    templates: &TemplateDir,
) -> Result<ScanPrompt, ThemeScanError> {
    let path = templates.path_of(&file).display().to_string();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// A scratch template directory holding two prompt versions.
    struct Prompts(PathBuf);

    impl Prompts {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("colossus-scan-prompt-{name}"));
            let _ = fs::remove_dir_all(&path); // best-effort: a leftover run
            fs::create_dir_all(&path).expect("scratch dir is creatable");
            fs::write(path.join("theme_scan_prompt_v2.md"), "JUDGE, v2 rules")
                .expect("v2 is writable");
            fs::write(path.join("theme_scan_prompt_v3.md"), "JUDGE, v3 rules")
                .expect("v3 is writable");
            Prompts(path)
        }

        fn dir(&self) -> TemplateDir {
            TemplateDir::new(self.0.to_string_lossy().to_string())
        }
    }

    impl Drop for Prompts {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0); // best-effort: test cleanup
        }
    }

    /// Changing the settings row changes which prompt the NEXT scan judges with.
    ///
    /// This is the whole of item 1d as a behaviour: before this task the filename
    /// was a compiled-in default behind an env var nobody had set, so the only way
    /// to change it was a rebuild. The two reads below are the same code path a
    /// scan takes at start; only the stored value differs.
    #[test]
    fn the_scan_reads_the_prompt_file_the_settings_row_names() {
        let prompts = Prompts::new("switch");

        let v3 = read_scan_prompt("theme_scan_prompt_v3.md".to_string(), &prompts.dir())
            .expect("v3 is deployed");
        assert_eq!(v3.text, "JUDGE, v3 rules");
        assert_eq!(
            v3.file, "theme_scan_prompt_v3.md",
            "the filename rides along as the run's prompt provenance"
        );

        // The row is edited on the Settings page; the next scan reads the other
        // file, with no restart and no rebuild.
        let v2 = read_scan_prompt("theme_scan_prompt_v2.md".to_string(), &prompts.dir())
            .expect("v2 is deployed");
        assert_eq!(v2.text, "JUDGE, v2 rules");
    }

    /// A named prompt that is not deployed refuses BY PATH, before any spend.
    #[test]
    fn a_missing_prompt_names_the_path_it_looked_for() {
        let prompts = Prompts::new("missing");

        let error = read_scan_prompt("theme_scan_prompt_v9.md".to_string(), &prompts.dir())
            .expect_err("v9 was never deployed");

        let message = error.to_string();
        assert!(
            message.contains("theme_scan_prompt_v9.md"),
            "the refusal must name the file: {message}"
        );
        assert!(
            message.contains(&prompts.0.to_string_lossy().to_string()),
            "and the directory it looked in: {message}"
        );
    }
}
