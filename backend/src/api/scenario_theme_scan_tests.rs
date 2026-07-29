//! Unit tests for [`crate::api::scenario_theme_scan`].
//!
//! Split into a sibling file (via `#[path]`) so the route module stays within the
//! module-size limit — the same discipline as `theme_scan_persist_tests.rs` and
//! `scan_runs_tests.rs`. These pin the error taxonomy: every `ThemeScanError`
//! variant must map to the HTTP status a caller can act on, so a refactor cannot
//! silently demote a deliberate refusal into a generic 500.

use super::*;
use uuid::Uuid;

// `map_scan_error` is the one piece of policy in this transport shell: it
// decides which failures are the client's fault (4xx), which are a missing
// dependency (503), and which are server bugs (500). Pin each mapping so a
// future variant added to a wrong arm is caught here, not in production.

#[test]
fn not_found_maps_to_404() {
    let e = ThemeScanError::ScenarioNotFound {
        case_slug: "awad".to_string(),
        scenario_id: Uuid::nil(),
    };
    assert!(matches!(map_scan_error(e), AppError::NotFound { .. }));
}

#[test]
fn empty_attack_meaning_maps_to_400() {
    let e = ThemeScanError::EmptyAttackMeaning {
        scenario_id: Uuid::nil(),
    };
    assert!(matches!(map_scan_error(e), AppError::BadRequest { .. }));
}

#[test]
fn subject_unresolvable_maps_to_400() {
    let e = ThemeScanError::SubjectUnresolvable {
        scenario_id: Uuid::nil(),
    };
    assert!(matches!(map_scan_error(e), AppError::BadRequest { .. }));
}

#[test]
fn empty_selection_maps_to_400() {
    // A merge with nothing checked is user-fixable — a 400, distinct from a
    // not-found run (which is a 404). Pins the arm so a future refactor cannot
    // silently demote it to a 500 or collapse it into the not-found case.
    let e = ThemeScanError::EmptySelection {
        run_id: Uuid::nil(),
    };
    assert!(matches!(map_scan_error(e), AppError::BadRequest { .. }));
}

#[test]
fn merged_run_deletion_maps_to_409_and_explains_why() {
    // Deleting a merged run is refused as a CONFLICT — not a 404 (the run
    // plainly exists), not a 400 (the request is well-formed), not a 500 (the
    // server is fine). Pins the arm so a refactor cannot demote it into the
    // catch-all 500, which would read to the user as a transient glitch worth
    // retrying rather than a deliberate, permanent refusal.
    let e = ThemeScanError::ScanRunMerged {
        run_id: Uuid::nil(),
        merge_events: 2,
        attributed_facts: 7,
    };
    let mapped = map_scan_error(e);
    match mapped {
        AppError::Conflict { message, details } => {
            // The message must say WHY, and carry the counts — a bare "cannot
            // delete" would leave the human guessing what is holding the run.
            assert!(
                message.contains("merged") && message.contains("provenance"),
                "409 must explain the refusal: {message}"
            );
            assert!(
                message.contains('2') && message.contains('7'),
                "409 must name what is holding the run: {message}"
            );
            assert_eq!(details["reason"], "run_merged");
        }
        other => panic!("expected 409 Conflict, got {other:?}"),
    }
}

#[test]
fn provenance_check_failure_is_a_500_not_a_permissive_delete() {
    // Standing Rule 1, in the destructive direction: if the pre-delete check
    // cannot be read, the delete must NOT proceed. This lands in the
    // server-side catch-all (500) rather than being mistaken for "no
    // provenance found, safe to delete".
    let e = ThemeScanError::ScanRunProvenanceCheckFailed {
        run_id: Uuid::nil(),
        source: crate::repositories::pipeline_repository::PipelineRepoError::Database(
            "pool timed out".to_string(),
        ),
    };
    assert!(matches!(map_scan_error(e), AppError::Internal { .. }));
}

#[test]
fn vllm_gate_refusals_map_to_503() {
    let unreachable = ThemeScanError::VllmUnreachable {
        endpoint: "http://x:8000".to_string(),
        detail: "connection refused".to_string(),
    };
    assert!(matches!(
        map_scan_error(unreachable),
        AppError::ServiceUnavailable { .. }
    ));
    let mismatch = ThemeScanError::VllmModelMismatch {
        endpoint: "http://x:8000".to_string(),
        selected: "qwen-14b".to_string(),
        loaded: "qwen-7b".to_string(),
    };
    assert!(matches!(
        map_scan_error(mismatch),
        AppError::ServiceUnavailable { .. }
    ));
}

#[test]
fn scan_run_write_failed_maps_to_500() {
    let e = ThemeScanError::ScanRunWriteFailed {
        run_id: Uuid::nil(),
        source: crate::repositories::pipeline_repository::PipelineRepoError::Database(
            "boom".to_string(),
        ),
    };
    assert!(matches!(map_scan_error(e), AppError::Internal { .. }));
}

#[test]
fn scan_run_read_failed_maps_to_500() {
    let e = ThemeScanError::ScanRunReadFailed {
        run_id: Uuid::nil(),
        source: crate::repositories::pipeline_repository::PipelineRepoError::Database(
            "boom".to_string(),
        ),
    };
    assert!(matches!(map_scan_error(e), AppError::Internal { .. }));
}

#[test]
fn scan_run_not_found_maps_to_404() {
    let e = ThemeScanError::ScanRunNotFound {
        run_id: Uuid::nil(),
    };
    assert!(matches!(map_scan_error(e), AppError::NotFound { .. }));
}

#[test]
fn scan_run_list_failed_maps_to_500() {
    // A DB failure listing a scenario's history is server-side: a generic 500
    // whose cause is logged, never leaked (same policy as ScanRunReadFailed).
    let e = ThemeScanError::ScanRunListFailed {
        scenario_id: Uuid::nil(),
        source: crate::repositories::pipeline_repository::PipelineRepoError::Database(
            "boom".to_string(),
        ),
    };
    assert!(matches!(map_scan_error(e), AppError::Internal { .. }));
}

#[test]
fn scan_run_delete_failed_maps_to_500() {
    // A DB failure DELETING a run is server-side: a generic 500 whose cause is
    // logged, never leaked (same policy as ScanRunReadFailed / ScanRunListFailed).
    // Distinct from ScanRunNotFound (zero rows deleted), which maps to 404.
    let e = ThemeScanError::ScanRunDeleteFailed {
        run_id: Uuid::nil(),
        source: crate::repositories::pipeline_repository::PipelineRepoError::Database(
            "boom".to_string(),
        ),
    };
    assert!(matches!(map_scan_error(e), AppError::Internal { .. }));
}

#[test]
fn scan_run_merge_failed_maps_to_500() {
    // A DB failure MERGING a run's picks is server-side: a generic 500 whose
    // cause is logged, never leaked. Distinct from ScanRunNotFound (run absent
    // / wrong scenario → 404) and from a legitimate merged=0 (200).
    let e = ThemeScanError::ScanRunMergeFailed {
        run_id: Uuid::nil(),
        source: crate::repositories::pipeline_repository::PipelineRepoError::Database(
            "boom".to_string(),
        ),
    };
    assert!(matches!(map_scan_error(e), AppError::Internal { .. }));
}

#[test]
fn bad_model_choice_maps_to_400() {
    let e = ThemeScanError::ModelNotAvailable {
        model_id: "nope".to_string(),
    };
    assert!(matches!(map_scan_error(e), AppError::BadRequest { .. }));
}

#[test]
fn params_invalid_maps_to_400() {
    let e = ThemeScanError::ParamsInvalid {
        model_id: "qwen-14b".to_string(),
        source: crate::domain::llm_params::LlmConfigError::ClearNotAllowed {
            param: "max_tokens",
        },
    };
    assert!(matches!(map_scan_error(e), AppError::BadRequest { .. }));
}

#[test]
fn provider_build_failed_maps_to_400() {
    let e = ThemeScanError::ProviderBuildFailed {
        model_id: "llama-3-8b".to_string(),
        detail: "has no api_endpoint".to_string(),
    };
    assert!(matches!(map_scan_error(e), AppError::BadRequest { .. }));
}

#[test]
fn server_side_variants_map_to_500() {
    // A representative server-side variant (an unparseable stored definition)
    // must be a generic 500, not leak its cause to the client.
    let e = ThemeScanError::DefinitionInvalid {
        scenario_id: Uuid::nil(),
        source: serde_json::from_str::<serde_json::Value>("{oops")
            .expect_err("that is not valid JSON"),
    };
    assert!(matches!(map_scan_error(e), AppError::Internal { .. }));
}

/// A missing judging prompt is a 503 that NAMES THE PATH.
///
/// It used to fall into the catch-all 500 above, which reduced it to "theme scan
/// failed" — true, unactionable, and one of the two causes of the eleven-day
/// silence. It is the same shape as the vLLM gate refusals: a deployment
/// dependency the operator corrects, so it gets their status and, unlike a 500,
/// keeps its message. The path assertion is the point of the test — a future edit
/// that returns a tidy generic message would pass a status-only check.
#[test]
fn missing_prompt_file_maps_to_503_naming_the_path() {
    let e = ThemeScanError::PromptFileMissing {
        path: "/opt/colossus/templates/theme_scan_prompt_v2.md".to_string(),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"),
    };
    match map_scan_error(e) {
        AppError::ServiceUnavailable { message } => {
            assert!(
                message.contains("/opt/colossus/templates/theme_scan_prompt_v2.md"),
                "the operator cannot fix what the message does not name: {message}"
            );
            assert!(
                message.contains("THEME_SCAN_PROMPT_FILE"),
                "the message must name the env var that selects the file: {message}"
            );
        }
        other => panic!("expected a 503 naming the path, got {other:?}"),
    }
}

/// An un-promotable run is a 500, and is NOT collapsed with the write failure
/// beside it: the write succeeded and found nothing to promote, which sends the
/// operator looking somewhere else entirely.
#[test]
fn scan_run_not_promotable_maps_to_500_and_says_what_happened() {
    let e = ThemeScanError::ScanRunNotPromotable {
        run_id: Uuid::nil(),
    };
    let message = e.to_string();
    assert!(
        message.contains("could not be promoted"),
        "the log line must say the promotion is what failed: {message}"
    );
    assert!(matches!(map_scan_error(e), AppError::Internal { .. }));
}
