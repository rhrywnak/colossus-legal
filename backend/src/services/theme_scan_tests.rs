//! Unit tests for `theme_scan.rs` — kept in a sibling file
//! (`#[cfg(test)] #[path = "..."] mod tests;`) so the parent module
//! stays under the 300-line limit (house pattern, see registry_tests.rs).

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
    // The variant kept its name through the one-attack-box ruling (2026-08-10);
    // its CONDITION widened to "neither text is present" and its message now names
    // the box a human can see and fill, rather than the jsonb key behind it. A
    // human meeting this error is being told to go and write something — naming
    // `attack_meaning` sent them looking for a field the modal no longer has.
    let id = Uuid::nil();
    let s = ThemeScanError::EmptyAttackMeaning { scenario_id: id }.to_string();
    assert!(s.contains(&id.to_string()));
    assert!(
        s.contains("attack text"),
        "must name the box, not the column: {s}"
    );
    assert!(
        s.contains("identity"),
        "must say WHERE to fix it — the scenario's identity: {s}"
    );
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
    assert!(
        s.contains("re-save"),
        "must name the recovery action — this fails before a run row exists, so \
         the message is the only surface it gets: {s}"
    );
}

/// The variants that the 400 split left TOAST-ONLY each carry a recovery action
/// in the message itself.
///
/// They fail before the run stub is written, so there is no Run History row to
/// open later and no `error` column to read — the sentence the caller sees is the
/// entire diagnosis. Grouped in one test because it is one property, asserted
/// per-variant so a failure names which message lost its ending.
///
/// `SubjectResolveFailed` was one of these until 2026-08-07 and is now gone; the
/// no-target refusal it sat beside is covered by
/// `display_subject_unresolvable_sends_the_human_to_the_identity_they_must_author`.
#[test]
fn the_toast_only_variants_each_name_a_recovery_action() {
    let model_lookup = ThemeScanError::ModelLookupFailed {
        model_id: "qwen-14b".to_string(),
        source: sqlx::Error::PoolTimedOut,
    }
    .to_string();
    assert!(
        model_lookup.contains("database is reachable"),
        "a failed registry read must say what to check: {model_lookup}"
    );

    let params = ThemeScanError::ParamsInvalid {
        model_id: "qwen-14b".to_string(),
        source: crate::domain::llm_params::LlmConfigError::ClearNotAllowed {
            param: "max_tokens",
        },
    }
    .to_string();
    assert!(
        params.contains("pick another model") && params.contains("llm_models"),
        "both causes need their own recovery (bad choice vs corrupt row): {params}"
    );
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

/// The refusal a human meets when they start a scan on a scenario that names
/// nobody must tell them how to fix it — and since 2026-08-07 the fix is an
/// authoring action, not a deployment one.
///
/// Before that date this message named `CASE_DEFAULT_SUBJECT_NAME`, which sent
/// the reader to an env var that was never their problem: the scenario was
/// missing a target, and the config key existed only to paper over that.
#[test]
fn display_subject_unresolvable_sends_the_human_to_the_identity_they_must_author() {
    let id = Uuid::nil();
    let s = ThemeScanError::SubjectUnresolvable { scenario_id: id }.to_string();
    assert!(s.contains(&id.to_string()), "missing scenario id: {s}");
    assert!(s.contains("target"), "missing what is absent: {s}");
    assert!(
        s.contains("identity"),
        "missing the control that fixes it: {s}"
    );
    assert!(
        !s.contains("CASE_DEFAULT_SUBJECT_NAME"),
        "the case-default fallback is gone; naming its env var sends the human \
         to a setting that cannot help them: {s}"
    );
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

/// An un-promotable run names the run it could not launch.
///
/// The run id is the ONLY handle an operator has here: the row is gone or has
/// moved on, so there is nothing in Run History to click, and the log line is the
/// whole trail. A message without the id would leave nothing to search for.
#[test]
fn display_scan_run_not_promotable_names_the_run_and_the_outcome() {
    let run_id = Uuid::from_u128(7);
    let s = ThemeScanError::ScanRunNotPromotable { run_id }.to_string();
    assert!(s.contains(&run_id.to_string()), "missing run id: {s}");
    assert!(
        s.contains("was not launched"),
        "must say the scan did NOT run — the operator's first question: {s}"
    );
}

// ── Chunk B error variants ────────────────────────────────────────────────

#[test]
fn display_model_lookup_failed_surfaces_model_and_source() {
    let s = ThemeScanError::ModelLookupFailed {
        model_id: "qwen-14b".to_string(),
        source: sqlx::Error::PoolClosed,
    }
    .to_string();
    assert!(s.contains("qwen-14b"), "missing model id: {s}");
    assert!(s.contains("pool"), "source not surfaced: {s}");
}

#[test]
fn display_model_not_available_names_model() {
    let s = ThemeScanError::ModelNotAvailable {
        model_id: "nope".to_string(),
    }
    .to_string();
    assert!(s.contains("nope"), "missing model id: {s}");
    assert!(s.contains("active"), "no operator hint: {s}");
}

#[test]
fn display_params_invalid_surfaces_model_and_source() {
    let s = ThemeScanError::ParamsInvalid {
        model_id: "qwen-14b".to_string(),
        source: crate::domain::llm_params::LlmConfigError::ClearNotAllowed {
            param: "max_tokens",
        },
    }
    .to_string();
    assert!(s.contains("qwen-14b"), "missing model id: {s}");
    assert!(s.contains("max_tokens"), "source not surfaced: {s}");
}

#[test]
fn display_provider_build_failed_surfaces_model_and_detail() {
    let s = ThemeScanError::ProviderBuildFailed {
        model_id: "llama-3-8b".to_string(),
        detail: "has no api_endpoint".to_string(),
    }
    .to_string();
    assert!(s.contains("llama-3-8b"), "missing model id: {s}");
    assert!(
        s.contains("has no api_endpoint"),
        "detail not surfaced: {s}"
    );
}

#[test]
fn display_vllm_unreachable_names_endpoint_and_recovery() {
    let s = ThemeScanError::VllmUnreachable {
        endpoint: "http://10.10.100.200:8000".to_string(),
        detail: "connection refused".to_string(),
    }
    .to_string();
    assert!(
        s.contains("http://10.10.100.200:8000"),
        "missing endpoint: {s}"
    );
    assert!(s.contains("connection refused"), "detail not surfaced: {s}");
    assert!(s.contains("api_endpoint"), "no recovery hint: {s}");
}

#[test]
fn display_vllm_mismatch_names_endpoint_and_both_models() {
    let s = ThemeScanError::VllmModelMismatch {
        endpoint: "http://10.10.100.200:8000".to_string(),
        selected: "qwen-14b".to_string(),
        loaded: "qwen-7b".to_string(),
    }
    .to_string();
    assert!(
        s.contains("http://10.10.100.200:8000"),
        "missing endpoint: {s}"
    );
    assert!(s.contains("qwen-14b"), "missing selected: {s}");
    assert!(s.contains("qwen-7b"), "missing loaded: {s}");
}

// ── Background-job error variants ─────────────────────────────────────────

#[test]
fn display_scan_run_write_failed_names_run_and_source() {
    let s = ThemeScanError::ScanRunWriteFailed {
        run_id: Uuid::nil(),
        source: crate::repositories::pipeline_repository::PipelineRepoError::Database(
            "conn reset".to_string(),
        ),
    }
    .to_string();
    assert!(s.contains(&Uuid::nil().to_string()), "missing run id: {s}");
    assert!(s.contains("conn reset"), "source not surfaced: {s}");
}

#[test]
fn display_scan_run_read_failed_surfaces_source() {
    let s = ThemeScanError::ScanRunReadFailed {
        run_id: Uuid::nil(),
        source: crate::repositories::pipeline_repository::PipelineRepoError::Database(
            "read boom".to_string(),
        ),
    }
    .to_string();
    assert!(s.contains("read boom"), "source not surfaced: {s}");
}

#[test]
fn display_scan_run_not_found_names_run() {
    let s = ThemeScanError::ScanRunNotFound {
        run_id: Uuid::nil(),
    }
    .to_string();
    assert!(s.contains(&Uuid::nil().to_string()), "missing run id: {s}");
    assert!(s.contains("not found"), "unexpected message: {s}");
}

#[test]
fn display_scan_run_delete_failed_names_run_and_source() {
    // A failed DELETE must name BOTH the run it could not delete and the DB cause
    // (Standing Rule 1) — distinct from ScanRunNotFound, which is the legitimate
    // "zero rows, nothing to delete" outcome, not a failure.
    let s = ThemeScanError::ScanRunDeleteFailed {
        run_id: Uuid::nil(),
        source: crate::repositories::pipeline_repository::PipelineRepoError::Database(
            "delete boom".to_string(),
        ),
    }
    .to_string();
    assert!(s.contains(&Uuid::nil().to_string()), "missing run id: {s}");
    assert!(s.contains("delete boom"), "source not surfaced: {s}");
}
