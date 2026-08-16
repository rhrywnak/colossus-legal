//! Tests for the shared one-shot plumbing.
//!
//! ## Why no test mutates an environment variable
//!
//! `std::env::set_var` is process-global and `cargo test` runs tests on threads
//! in one process, so a test that sets `PIPELINE_DATABASE_URL` can be observed by
//! any other test running at that moment. This repo has already paid for that
//! lesson once. The flag path is fully testable without touching the
//! environment, and the env path is a single `std::env::var` call whose behaviour
//! is std's, not ours — so what is tested here is the part that is ours.

use std::process::ExitCode;

use super::*;

/// `ExitCode` implements neither `PartialEq` nor `Debug`, so a test cannot
/// compare one directly. Formatting is not available either. What a test CAN do
/// is assert on the branch taken, which is what these do.
fn is_err(result: &Result<String, ExitCode>) -> bool {
    result.is_err()
}

#[test]
fn an_explicit_flag_wins_and_is_returned_verbatim() {
    let url = "postgres://someone@somewhere/colossus_legal_v2";
    let got = pipeline_database_url(Some(url));
    assert_eq!(
        got.ok().as_deref(),
        Some(url),
        "the flag must be passed through unchanged — a tool that rewrote the \
         operator's URL would connect somewhere they did not name"
    );
}

#[test]
fn the_flag_is_preferred_even_when_it_looks_wrong() {
    // Deliberately not a Postgres URL. Validating the SHAPE here would be the
    // tool second-guessing the operator; sqlx reports a bad URL clearly enough,
    // and it reports the operator's actual string rather than a guess about it.
    let got = pipeline_database_url(Some("not-a-url"));
    assert_eq!(got.ok().as_deref(), Some("not-a-url"));
}

#[test]
fn a_missing_url_is_refused_rather_than_defaulted() {
    // If PIPELINE_DATABASE_URL happens to be set in this environment there is
    // nothing to assert — the point of the test is that NO default is invented
    // when neither source has a value, and that case only exists when the var is
    // absent. Reading the var is the only thing this test does with the
    // environment; it never writes one.
    if std::env::var("PIPELINE_DATABASE_URL").is_ok() {
        return;
    }
    assert!(
        is_err(&pipeline_database_url(None)),
        "with no flag and no env var the tool must refuse; a default URL would \
         point a merge at whatever database happened to be reachable"
    );
}

#[test]
fn a_report_is_written_verbatim_to_the_named_path() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("proof.txt");
    let body = "=== PROOF ===\n  rows updated: 42\n";

    emit_report(body, &path).expect("writing into a temp dir succeeds");

    let written = std::fs::read_to_string(&path).expect("the report exists");
    assert_eq!(
        written, body,
        "the file must be byte-identical to what was printed, or the two copies \
         of the proof can disagree"
    );
}

#[test]
fn an_unwritable_report_path_is_an_error_not_a_shrug() {
    let dir = tempfile::tempdir().expect("a temp dir");
    // A path whose parent directory does not exist. The run may have already
    // written the database; losing the proof silently is the failure this
    // guards against.
    let path = dir.path().join("no-such-directory").join("proof.txt");

    assert!(
        emit_report("body", &path).is_err(),
        "an unwritable report path must fail loudly — the proof is the only \
         record of what --apply did"
    );
}
