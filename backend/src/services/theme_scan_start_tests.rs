//! Unit tests for [`crate::services::theme_scan_start`].
//!
//! Split into a sibling file (via `#[path]`) so the start module stays within the
//! module-size limit — the house pattern (`theme_scan_run_tests.rs`,
//! `scan_runs_tests.rs`). Two kinds of test live here: the pure params-snapshot
//! mapping, and an ORDER-OF-OPERATIONS guard read off the shipped source (there
//! is no live-DB harness in this repo, and the order is the whole feature).

use super::*;

/// A pre-filter snapshot, as a run would freeze it.
fn prefilter(min_chars: usize) -> PrefilterSnapshot {
    PrefilterSnapshot {
        min_chars,
        statement_types: vec!["referral".to_string()],
    }
}

/// The `resolved_params` JSONB snapshot must carry the resolved prompt
/// filename (run→prompt provenance) alongside the existing param fields.
#[test]
fn params_snapshot_records_prompt_file_alongside_params() {
    let params = ResolvedLlmParams {
        temperature: Some(0.0),
        timeout_secs: 90,
        max_tokens: 512,
    };
    let snapshot = params_snapshot(&params, "theme_scan_prompt_v2.md", &prefilter(60));

    assert_eq!(snapshot["prompt_file"], "theme_scan_prompt_v2.md");
    // The pre-existing fields must survive the addition.
    assert_eq!(snapshot["timeout_secs"], 90);
    assert_eq!(snapshot["max_tokens"], 512);
    assert_eq!(snapshot["temperature"], 0.0);
}

/// The snapshot also freezes WHICH candidates were put in front of the judge.
///
/// Without this, two runs a week apart can differ in their admitted set and the
/// audit trail cannot say whether the prompt changed or a pre-filter dial did —
/// the conservation block records that fifteen quotes were set aside for length,
/// but not the threshold that set them aside (task 2.15, observability gate).
#[test]
fn params_snapshot_freezes_the_prefilter_dials_that_chose_the_pool() {
    let params = ResolvedLlmParams {
        temperature: Some(0.0),
        timeout_secs: 90,
        max_tokens: 512,
    };

    let strict = params_snapshot(&params, "theme_scan_prompt_v3.md", &prefilter(60));
    assert_eq!(strict["prefilter_min_chars"], 60);
    assert_eq!(strict["prefilter_statement_types"][0], "referral");

    // A run started after the dial was turned records the NEW value, so the two
    // runs are distinguishable in the audit trail rather than merely different.
    let loose = params_snapshot(&params, "theme_scan_prompt_v3.md", &prefilter(0));
    assert_eq!(loose["prefilter_min_chars"], 0);
}

/// A non-default (overridden) prompt filename is recorded verbatim, so a run
/// judged with a bumped prompt version is distinguishable in the audit trail.
#[test]
fn params_snapshot_records_an_overridden_prompt_file() {
    let params = ResolvedLlmParams {
        temperature: None,
        timeout_secs: 30,
        max_tokens: 256,
    };
    let snapshot = params_snapshot(&params, "theme_scan_prompt_v3.md", &prefilter(60));
    assert_eq!(snapshot["prompt_file"], "theme_scan_prompt_v3.md");
}

// ─── The order of operations, read off the shipped source ────────────────────
//
// `start_theme_scan` is an ordering contract, and the ordering is what the change
// exists to establish: the prompt check must precede the stub row (nothing is
// recorded for a scan that could never run), and the stub row must precede
// preparation (a scan that dies in preparation still leaves a failed row). Both
// are invisible to the compiler — a refactor that hoists the preparation above
// the stub compiles and passes every other test while silently restoring the
// blind spot. So they are pinned textually, the same discipline as the SQL-shape
// tests in `scan_runs_tests.rs`.

/// The body of `start_theme_scan`, from its signature to the closing brace at
/// column 0.
fn start_fn_body() -> String {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services/theme_scan_start.rs");
    let text = std::fs::read_to_string(path).expect("theme_scan_start.rs is readable");
    let start = text
        .find("pub async fn start_theme_scan")
        .expect("start_theme_scan is present");
    let rest = &text[start..];
    let end = rest.find("\n}").map(|i| i + 2).unwrap_or(rest.len());
    rest[..end].to_string()
}

/// The position of a call in `start_theme_scan`'s body, or a failing assertion
/// naming what was not found.
fn call_position(body: &str, needle: &str) -> usize {
    body.find(needle)
        .unwrap_or_else(|| panic!("`{needle}` is no longer called in start_theme_scan"))
}

/// The whole start sequence, pinned in order.
///
/// One test, because it is one contract with a single hinge — the stub INSERT.
/// Everything before it can fail without leaving a trace, and each of those steps
/// is there for its own reason: the prompt check because a scan that cannot run
/// must not be recorded as one, the fence because `scan_runs.scenario_id` is a
/// foreign key, and validation because a request the caller can fix announces
/// itself in the panel and does not need a row (the 2026-07-28 ruling). From the
/// stub onward every failure has somewhere to be written.
#[test]
fn start_validates_before_it_records_and_records_before_it_works() {
    let body = start_fn_body();

    let prompt = call_position(&body, "load_scan_prompt(");
    let fence = call_position(&body, "load_scenario_fenced(");
    let validate = call_position(&body, "validate_scan_request(");
    let stub = call_position(&body, "insert_scan_run_stub(");
    let prepare = call_position(&body, "prepare_or_record(");
    let promote = call_position(&body, "promote_run(");
    let spawn = call_position(&body, "spawn_scan_job(");

    assert!(
        prompt < stub,
        "the judging prompt must be checked BEFORE the run stub is written — a \
         scan that cannot possibly run must not leave a run record"
    );
    assert!(
        fence < stub,
        "the scenario must be fenced BEFORE the stub is written: scan_runs.scenario_id \
         is a foreign key, and a row keyed to another case's scenario is a cross-case write"
    );
    assert!(
        validate < stub,
        "request validation must run BEFORE the stub is written — a missing \
         attack_meaning or a retired model pick is the caller's to fix, announces \
         itself in the panel, and must not leave a failed row diluting the history"
    );
    assert!(
        stub < prepare,
        "the stub row must be written BEFORE the work preparation does — otherwise a \
         scan that dies in the vLLM gate or the candidate read leaves no record, which \
         is the defect this order fixes"
    );
    assert!(
        prepare < promote && promote < spawn,
        "preparation must succeed before promotion, and promotion before the judging \
         task is spawned (nothing may spend LLM budget on an unreportable run)"
    );
}

/// The validating phase and the working phase stay DISJOINT.
///
/// The ordering test above proves validation runs first; this proves the right
/// checks are still IN it. Folding the criterion, subject, or model checks back
/// into `prepare_scan` — the shape this split undid — would keep every position
/// assertion true while quietly restoring a failed row for every mis-click.
///
/// The two phases are whole modules, so the test is a containment check per file
/// rather than a slice of one: each check must appear in its own phase and be
/// absent from the other, which is what makes "disjoint" mean something.
#[test]
fn the_caller_fixable_checks_live_in_the_validating_phase() {
    let read = |name: &str| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name} is readable: {e}"))
    };
    let validating = read("src/services/theme_scan_validate.rs");
    let working = read("src/services/theme_scan.rs");

    // The caller's to fix: answered with a 4xx and no run row.
    for check in [
        "EmptyAttackMeaning",
        "resolve_subject(",
        "resolve_scan_provider(",
    ] {
        assert!(
            validating.contains(check),
            "`{check}` must stay in the validating phase — its failure is the \
             caller's to fix and must not record a run"
        );
    }

    // The mirror image: the phase that runs after the row exists keeps the
    // failures that DO deserve a record, and the validating phase must not
    // acquire them (which would move a system failure out of Run History).
    for work in ["assert_vllm_model_loaded(", "read_candidates("] {
        assert!(
            working.contains(work),
            "`{work}` must stay in the working phase — its failure is the system's, \
             and the run row is the only place it becomes visible"
        );
        assert!(
            !validating.contains(work),
            "`{work}` moved into the validating phase, where a failure records \
             nothing — a system failure would go invisible again"
        );
    }
}

/// Preparation failures are recorded on the run row rather than only returned.
///
/// Guards the half of the contract the ordering test cannot see: a `prepare_scan`
/// call moved back inline (dropping the `fail_scan_run` write in the error arm)
/// would keep the correct ORDER while restoring the blank-history symptom, since
/// the stub would keep its placeholder reason forever.
#[test]
fn a_failed_preparation_writes_its_reason_onto_the_run() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services/theme_scan_start.rs");
    let text = std::fs::read_to_string(path).expect("theme_scan_start.rs is readable");
    let start = text
        .find("async fn prepare_or_record")
        .expect("prepare_or_record is present");
    let rest = &text[start..];
    let end = rest.find("\n}").map(|i| i + 2).unwrap_or(rest.len());
    let body = &rest[..end];

    assert!(
        body.contains("fail_scan_run("),
        "a failed preparation must write its reason onto the stub row, not merely \
         return it to the caller"
    );
    assert!(
        body.contains("e.to_string()"),
        "the recorded reason must be the typed error's own message — a generic \
         string would strip the diagnosis the row exists to carry"
    );
}
