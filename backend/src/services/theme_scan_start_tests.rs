//! Unit tests for [`crate::services::theme_scan_start`].
//!
//! Split into a sibling file (via `#[path]`) so the start module stays within the
//! module-size limit — the house pattern (`theme_scan_run_tests.rs`,
//! `scan_runs_tests.rs`). Two kinds of test live here: the pure params-snapshot
//! mapping, and an ORDER-OF-OPERATIONS guard read off the shipped source (there
//! is no live-DB harness in this repo, and the order is the whole feature).

use super::*;

/// The `resolved_params` JSONB snapshot must carry the resolved prompt
/// filename (run→prompt provenance) alongside the existing param fields.
#[test]
fn params_snapshot_records_prompt_file_alongside_params() {
    let params = ResolvedLlmParams {
        temperature: Some(0.0),
        timeout_secs: 90,
        max_tokens: 512,
    };
    let snapshot = params_snapshot(&params, "theme_scan_prompt_v2.md");

    assert_eq!(snapshot["prompt_file"], "theme_scan_prompt_v2.md");
    // The pre-existing fields must survive the addition.
    assert_eq!(snapshot["timeout_secs"], 90);
    assert_eq!(snapshot["max_tokens"], 512);
    assert_eq!(snapshot["temperature"], 0.0);
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
    let snapshot = params_snapshot(&params, "theme_scan_prompt_v3.md");
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

/// The template presence check runs BEFORE the run stub is written, and the stub
/// is written BEFORE any preparation work.
///
/// Two orderings, one test, because they are one contract: everything that can
/// fail cheaply and leave nothing behind happens first, and from the stub onward
/// every failure has somewhere to be recorded.
#[test]
fn start_checks_the_prompt_then_stubs_the_run_then_prepares() {
    let body = start_fn_body();

    let prompt = call_position(&body, "load_scan_prompt(");
    let fence = call_position(&body, "load_scenario_fenced(");
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
        stub < prepare,
        "the stub row must be written BEFORE preparation — otherwise a scan that \
         dies during preparation leaves no record, which is the defect this order fixes"
    );
    assert!(
        prepare < promote && promote < spawn,
        "preparation must succeed before promotion, and promotion before the judging \
         task is spawned (nothing may spend LLM budget on an unreportable run)"
    );
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
