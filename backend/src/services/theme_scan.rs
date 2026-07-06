//! Theme Scan service (D2b) — batched LLM judgment of candidate quotes.
//!
//! A scenario's `definition` carries an `attack_meaning` (plain-English prose,
//! authored in D1). Theme Scan:
//!
//! 1. reads EVERY candidate quote about the scenario's subject
//!    (`all_evidence_about_subject` — ungated, so recall is 100% by
//!    construction: nothing is pre-filtered by keyword or embedding);
//! 2. asks the deterministic LLM judge to rate each quote against the
//!    `attack_meaning`, returning `{relevant, proposed_role, reason, confidence}`;
//! 3. writes each RELEVANT verdict to `scenario_fact_refs` as a `confirmed=false`
//!    suggestion (idempotent per-row upsert); irrelevant verdicts are counted and
//!    sampled but never persisted;
//! 4. returns a [`ThemeScanSummary`] with the counts, the written suggestions,
//!    and a rejected sample for the honesty check.
//!
//! This module owns the *orchestration* (load, validate, resolve, drive, log).
//! The per-quote judging and result-persistence helpers live in the sibling
//! [`crate::services::theme_scan_judge`], and the verdict parser in
//! [`crate::services::theme_scan_parse`] — kept apart so no single file exceeds
//! the module-size limit and each piece is independently testable.
//!
//! ## Concurrency (D2b STEP-1 decision)
//!
//! Candidates are judged concurrently with `buffer_unordered`, each call bounded
//! by [`AppState::theme_scan_semaphore`] — a DEDICATED cap, not the pipeline's
//! `llm_semaphore`, so a scan and document extraction never starve each other.
//! The provider is `Send + Sync + 'static` with no interior mutability and each
//! `call_with_rate_limit_retry` owns its own retry loop, so concurrent calls are
//! safe; the retry wrapper absorbs any rate-limit brush from the fan-out.

use std::sync::Arc;

use colossus_extract::LlmProvider;
use uuid::Uuid;

use crate::bias::dto::BiasInstance;
use crate::bias::repository::{BiasRepository, BiasRepositoryError};
use crate::dto::scenario_crud::ScenarioDefinition;
use crate::dto::ThemeScanSummary;
use crate::repositories::pipeline_repository::{get_scenario, PipelineRepoError, ScenarioRecord};
use crate::services::theme_scan_judge::{judge_all, persist_and_summarize};
use crate::state::AppState;

// CONST: the verdict token budget is a fixed protocol shape, not a deployment
// knob. A verdict is a tiny four-key JSON object; 512 is a generous ceiling that
// would only ever change if the verdict SHAPE changes — and that is a code change
// (the `Verdict` struct + the prompt shipped together), never per-environment
// tuning. Roman pinned this as a named constant (no env) in the D2b decision. It
// is `pub` because `main::build_theme_scan_provider` reads it too, so the scan
// provider and the per-call cap agree from one source of truth.
pub const THEME_SCAN_MAX_TOKENS: u32 = 512;

// CONST: the prompt VERSION this build judges with is a code decision, not a
// deployment setting. The file CONTENT is tunable without a rebuild (edit + scp
// to the registry's template dir); selecting a *different* version is a
// deliberate, reviewed bump (`_v1` → `_v2`) that ships with any matching
// `Verdict`/parse changes, so the version token is pinned in code on purpose.
// Only the filename is compiled in; the directory it resolves against is
// env-driven via the registry (Standing Rule 2 satisfied for the path).
const THEME_SCAN_PROMPT: &str = "theme_scan_prompt_v1.md";

/// Top-level, scan-aborting failures.
///
/// These are distinct from per-item verdict failures (a bad LLM reply for one
/// quote), which are COUNTED in the summary rather than returned here. Every
/// variant is a condition under which the whole scan cannot meaningfully proceed.
/// The route handler maps each to an HTTP status.
///
/// ## Rust Learning: `#[source]` on a wrapped cause
///
/// `#[source]` exposes the underlying error in the chain so `{source}` in the
/// message and a structured logger both see the real cause (Standing Rule 1: the
/// failure names *what* failed and *why*), without this enum re-stringifying it.
#[derive(Debug, thiserror::Error)]
pub enum ThemeScanError {
    /// The scenario row could not be read (DB/connection error).
    #[error("failed to load scenario {scenario_id}: {source}")]
    ScenarioLoadFailed {
        scenario_id: Uuid,
        #[source]
        source: PipelineRepoError,
    },

    /// No scenario with that id in that case (absent, or the case-fence rejected
    /// a cross-case id). Same observable for both — a caller must not learn that
    /// an id exists in another case.
    #[error("scenario {scenario_id} not found in case '{case_slug}'")]
    ScenarioNotFound {
        case_slug: String,
        scenario_id: Uuid,
    },

    /// The stored `definition` jsonb did not parse as a `ScenarioDefinition`
    /// (e.g. a retired v1 shape). Loud, not defaulted.
    #[error("scenario {scenario_id} has a definition this build cannot parse: {source}")]
    DefinitionInvalid {
        scenario_id: Uuid,
        #[source]
        source: serde_json::Error,
    },

    /// The scenario has no `attack_meaning`. A scan needs judgment criteria; this
    /// is a user-fixable precondition, surfaced clearly rather than scanning with
    /// empty criteria.
    #[error(
        "scenario {scenario_id} has no attack_meaning — a scan needs judgment \
         criteria; author the accusation meaning before scanning"
    )]
    EmptyAttackMeaning { scenario_id: Uuid },

    /// Resolving the case-default subject failed at the graph layer.
    #[error("failed to resolve the default subject for scenario {scenario_id}: {source}")]
    SubjectResolveFailed {
        scenario_id: Uuid,
        #[source]
        source: BiasRepositoryError,
    },

    /// Neither the scenario definition's `target` nor a configured case-default
    /// subject yielded a subject to scan.
    #[error(
        "scenario {scenario_id}: no subject to scan — the scenario names no target \
         and no case-default subject is configured (CASE_DEFAULT_SUBJECT_NAME)"
    )]
    SubjectUnresolvable { scenario_id: Uuid },

    /// Reading the candidate quote set for the subject failed.
    #[error("failed to read candidate evidence for subject '{subject_id}': {source}")]
    CandidateReadFailed {
        subject_id: String,
        #[source]
        source: BiasRepositoryError,
    },

    /// The versioned prompt file is missing/unreadable. Fail-loud, naming the
    /// path (mirrors the extraction template load).
    #[error("Theme Scan prompt file not readable at '{path}': {source}")]
    PromptFileMissing {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// The scan provider is `None` (ANTHROPIC_API_KEY unset). The route surfaces
    /// this as 503, mirroring `rag_pipeline`'s no-key handling.
    #[error("Theme Scan LLM provider is unavailable (ANTHROPIC_API_KEY unset)")]
    ProviderUnavailable,
}

/// Everything a scan needs to judge, resolved and validated up front.
///
/// Bundling these into one struct lets [`run_theme_scan`] read as a short
/// orchestration (prepare → judge → persist) while [`prepare_scan`] owns the
/// multi-step precondition checks.
struct PreparedScan {
    attack_meaning: Arc<str>,
    scan_prompt: Arc<str>,
    provider: Arc<dyn LlmProvider>,
    candidates: Vec<BiasInstance>,
}

/// Run a Theme Scan for one scenario and return its summary.
///
/// See the module docs for the full shape. Takes `&AppState` because a scan
/// touches four subsystems (pipeline pool, graph, registry, the scan provider +
/// semaphore); it is a domain service, not a reusable pipeline step, so the
/// `AppContext`-only rule does not apply.
pub async fn run_theme_scan(
    state: &AppState,
    case_slug: &str,
    scenario_id: Uuid,
) -> Result<ThemeScanSummary, ThemeScanError> {
    let prepared = prepare_scan(state, case_slug, scenario_id).await?;
    tracing::info!(
        case_slug,
        %scenario_id,
        candidates = prepared.candidates.len(),
        "theme scan: judging candidates"
    );

    let results = judge_all(
        prepared.provider,
        Arc::clone(&state.theme_scan_semaphore),
        state.config.theme_scan_concurrency,
        prepared.scan_prompt,
        prepared.attack_meaning,
        prepared.candidates,
    )
    .await;

    let summary = persist_and_summarize(&state.pipeline_pool, scenario_id, results).await;
    tracing::info!(
        case_slug,
        %scenario_id,
        candidates_read = summary.candidates_read,
        relevant_written = summary.relevant_written,
        irrelevant = summary.irrelevant,
        failed = summary.failed,
        "theme scan: complete"
    );
    Ok(summary)
}

/// Load the scenario, validate its preconditions, and gather the inputs a scan
/// needs: the judgment criterion, the candidate quotes, the provider, and the
/// prompt. Every failure here is a typed, scan-aborting [`ThemeScanError`].
async fn prepare_scan(
    state: &AppState,
    case_slug: &str,
    scenario_id: Uuid,
) -> Result<PreparedScan, ThemeScanError> {
    let record = load_scenario_fenced(&state.pipeline_pool, case_slug, scenario_id).await?;

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

    let candidates = read_candidates(state, &definition, scenario_id).await?;

    let provider = state
        .theme_scan_provider
        .clone()
        .ok_or(ThemeScanError::ProviderUnavailable)?;

    let prompt_path = state.registry.template_path(THEME_SCAN_PROMPT);
    let scan_prompt = std::fs::read_to_string(&prompt_path).map_err(|source| {
        ThemeScanError::PromptFileMissing {
            path: prompt_path,
            source,
        }
    })?;

    Ok(PreparedScan {
        attack_meaning: Arc::from(attack_meaning),
        scan_prompt: Arc::from(scan_prompt),
        provider,
        candidates,
    })
}

/// Resolve the scan subject and read every candidate quote about it (the ungated
/// `all_evidence_about_subject` set — the 100%-recall input to the judge).
async fn read_candidates(
    state: &AppState,
    definition: &ScenarioDefinition,
    scenario_id: Uuid,
) -> Result<Vec<BiasInstance>, ThemeScanError> {
    let subject_id = resolve_subject_id(state, definition, scenario_id).await?;
    tracing::debug!(%scenario_id, subject_id = %subject_id, "theme scan: subject resolved");

    let repo = BiasRepository::new(state.graph.clone());
    repo.all_evidence_about_subject(&subject_id)
        .await
        .map_err(|source| ThemeScanError::CandidateReadFailed { subject_id, source })
}

/// Load one scenario, enforcing the case-isolation fence.
///
/// `get_scenario` is keyed on the globally-unique `scenario_id` alone, so the
/// case-fence is applied here: a row from a different case is reported as
/// `ScenarioNotFound`, identical to a truly-absent id (a caller must not learn
/// that an id exists elsewhere).
async fn load_scenario_fenced(
    pool: &sqlx::PgPool,
    case_slug: &str,
    scenario_id: Uuid,
) -> Result<ScenarioRecord, ThemeScanError> {
    let record = get_scenario(pool, scenario_id)
        .await
        .map_err(|source| ThemeScanError::ScenarioLoadFailed {
            scenario_id,
            source,
        })?
        .ok_or_else(|| ThemeScanError::ScenarioNotFound {
            case_slug: case_slug.to_string(),
            scenario_id,
        })?;

    if record.case_slug != case_slug {
        tracing::warn!(
            actual_case = %record.case_slug,
            requested_case = %case_slug,
            %scenario_id,
            "theme scan: scenario requested through the wrong case path"
        );
        return Err(ThemeScanError::ScenarioNotFound {
            case_slug: case_slug.to_string(),
            scenario_id,
        });
    }
    Ok(record)
}

/// Resolve the subject a scan runs over: the definition's `target` if it names
/// one, else the case-default subject (`CASE_DEFAULT_SUBJECT_NAME` → id via the
/// Bias Explorer's resolver), else a typed `SubjectUnresolvable`.
async fn resolve_subject_id(
    state: &AppState,
    definition: &ScenarioDefinition,
    scenario_id: Uuid,
) -> Result<String, ThemeScanError> {
    if let Some(target) = definition.target.as_deref() {
        if !target.trim().is_empty() {
            return Ok(target.to_string());
        }
    }

    // Fall back to the case default, reusing the Bias Explorer's public resolver
    // so the scan and the "About" filter agree on the default subject.
    let repo = BiasRepository::new(state.graph.clone());
    let filters = repo
        .available_filters(state.config.case_default_subject_name.as_deref())
        .await
        .map_err(|source| ThemeScanError::SubjectResolveFailed {
            scenario_id,
            source,
        })?;

    filters
        .default_subject_id
        .ok_or(ThemeScanError::SubjectUnresolvable { scenario_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    // The Display strings are the operator's window into a failed scan (Standing
    // Rule 1). These tests pin that every variant carrying interpolated context
    // actually surfaces it — a format-string typo (wrong field, dropped
    // `{source}`) is invisible until the error is produced at runtime otherwise.

    #[test]
    fn display_scenario_not_found_names_case_and_id() {
        let id = Uuid::nil();
        let s = ThemeScanError::ScenarioNotFound {
            case_slug: "awad".to_string(),
            scenario_id: id,
        }
        .to_string();
        assert!(s.contains("awad"), "missing case slug: {s}");
        assert!(s.contains(&id.to_string()), "missing scenario id: {s}");
    }

    #[test]
    fn display_empty_attack_meaning_names_id_and_field() {
        let id = Uuid::nil();
        let s = ThemeScanError::EmptyAttackMeaning { scenario_id: id }.to_string();
        assert!(s.contains(&id.to_string()));
        assert!(s.contains("attack_meaning"));
    }

    #[test]
    fn display_scenario_load_failed_surfaces_source() {
        let id = Uuid::nil();
        let s = ThemeScanError::ScenarioLoadFailed {
            scenario_id: id,
            source: PipelineRepoError::Database("connection reset".to_string()),
        }
        .to_string();
        assert!(s.contains(&id.to_string()));
        assert!(s.contains("connection reset"), "source not surfaced: {s}");
    }

    #[test]
    fn display_definition_invalid_surfaces_id() {
        let id = Uuid::nil();
        // A real serde_json error (unterminated object) as the source.
        let source = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
        let s = ThemeScanError::DefinitionInvalid {
            scenario_id: id,
            source,
        }
        .to_string();
        assert!(s.contains(&id.to_string()));
        assert!(s.contains("cannot parse"), "unexpected message: {s}");
    }

    #[test]
    fn display_candidate_read_failed_names_subject_and_source() {
        use serde::de::Error as _;
        // BiasRepositoryError wraps a neo4rs deserialization error; construct one
        // via serde's `custom` so the test needs no live Neo4j connection.
        let source = BiasRepositoryError::Deserialize(neo4rs::DeError::custom("bad row"));
        let s = ThemeScanError::CandidateReadFailed {
            subject_id: "subj-1".to_string(),
            source,
        }
        .to_string();
        assert!(s.contains("subj-1"), "missing subject id: {s}");
        assert!(s.contains("bad row"), "source not surfaced: {s}");
    }

    #[test]
    fn display_subject_unresolvable_names_id_and_config_key() {
        let id = Uuid::nil();
        let s = ThemeScanError::SubjectUnresolvable { scenario_id: id }.to_string();
        assert!(s.contains(&id.to_string()), "missing scenario id: {s}");
        assert!(
            s.contains("CASE_DEFAULT_SUBJECT_NAME"),
            "missing the config key that fixes it: {s}"
        );
    }

    #[test]
    fn display_subject_resolve_failed_names_id_and_source() {
        use serde::de::Error as _;
        let id = Uuid::nil();
        // Same construction as the candidate-read test: a neo4rs deserialization
        // error via serde's `custom`, needing no live Neo4j connection.
        let source = BiasRepositoryError::Deserialize(neo4rs::DeError::custom("subjects query"));
        let s = ThemeScanError::SubjectResolveFailed {
            scenario_id: id,
            source,
        }
        .to_string();
        assert!(s.contains(&id.to_string()), "missing scenario id: {s}");
        assert!(s.contains("subjects query"), "source not surfaced: {s}");
    }

    #[test]
    fn display_prompt_file_missing_names_path_and_source() {
        let s = ThemeScanError::PromptFileMissing {
            path: "/templates/theme_scan_prompt_v1.md".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"),
        }
        .to_string();
        assert!(
            s.contains("/templates/theme_scan_prompt_v1.md"),
            "missing path: {s}"
        );
        assert!(s.contains("no such file"), "source not surfaced: {s}");
    }
}
