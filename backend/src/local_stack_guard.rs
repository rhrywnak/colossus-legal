//! The local boot's refusal, asserted by RUNNING it (CC_TASK_FOR_YOU_v1 L3).
//!
//! ## What happened, and why a test rather than a comment
//!
//! On 2026-09-22 a local run script derived its scratch `PIPELINE_DATABASE_URL`
//! with a `sed` that produced an empty string. The backend fell back to
//! `backend/.env`, which points BOTH pool URLs at DEV's `colossus_legal_v2`,
//! and the boot migrator applied eleven pending migrations to a live database.
//!
//! `scripts/assert-scratch-db.sh` is the refusal that was missing. A guard is
//! only worth having if it still refuses a year from now, and the failure mode
//! it exists for — an EMPTY variable — is exactly the one a careless edit
//! reintroduces, because an empty string is the most natural thing for a broken
//! derivation to produce.
//!
//! ## Why this executes the script instead of reading it
//!
//! A scan could prove the word "REFUSING" appears. Only running it proves the
//! exit status, and the exit status is the whole contract: `assert-scratch-db.sh
//! && run-the-backend` is worth nothing if the script exits 0 on a bad URL.
//!
//! Declared at the crate root beside `sql_invariants` and `template_invariants`,
//! for the same reason: it polices a repository artifact rather than a module.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The guard's path, from this crate's own manifest directory.
fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("scripts")
        .join("assert-scratch-db.sh")
}

/// Run the guard with these two URLs. `true` = it allowed the boot.
///
/// Both variables are set explicitly on EVERY call, including to the empty
/// string, so the test's own environment cannot leak a real URL into a case
/// that is supposed to be empty.
fn allows(main: &str, pipeline: &str) -> bool {
    let output = Command::new("bash")
        .arg(script())
        .env("DATABASE_URL", main)
        .env("PIPELINE_DATABASE_URL", pipeline)
        .output()
        .expect("the scratch-database guard is on disk and executable");
    output.status.success()
}

const SCRATCH_MAIN: &str = "postgres://u:p@10.10.100.200:5432/colossus_for_you_l3_main";
const SCRATCH_PIPE: &str = "postgres://u:p@10.10.100.200:5432/colossus_for_you_l3_proof";

/// Two throwaway databases: the boot is allowed.
///
/// The positive half. Without it, a guard that refused EVERYTHING would pass
/// every assertion below and nobody would be able to run the stack at all.
#[test]
fn two_scratch_databases_are_allowed() {
    assert!(allows(SCRATCH_MAIN, SCRATCH_PIPE));
}

/// (M) An EMPTY url is refused — the exact shape of the 2026-09-22 incident.
///
/// This is the case a `sed` that matched nothing produces, and the case the
/// shell hands to a process as "set to nothing" while the application's own
/// `.env` fallback quietly supplies a real one.
#[test]
fn an_empty_url_is_refused() {
    assert!(
        !allows("", SCRATCH_PIPE),
        "an empty DATABASE_URL must refuse"
    );
    assert!(
        !allows(SCRATCH_MAIN, ""),
        "an empty PIPELINE_DATABASE_URL must refuse"
    );
    assert!(!allows("", ""), "two empty URLs must refuse");
}

/// The live databases are refused BY NAME, on either variable.
#[test]
fn the_real_databases_are_refused() {
    let dev_pipeline = "postgres://u:p@10.10.100.200:5432/colossus_legal_v2";
    let dev_main = "postgres://u:p@10.10.100.200:5432/colossus_legal";
    assert!(!allows(dev_main, SCRATCH_PIPE));
    assert!(!allows(SCRATCH_MAIN, dev_pipeline));
    assert!(!allows(dev_main, dev_pipeline));
    // And PROD, which is the same names on another host — the guard reads the
    // database name, so the host it sits on makes no difference.
    assert!(!allows(
        "postgres://u:p@10.10.100.110:5432/colossus_legal",
        "postgres://u:p@10.10.100.110:5432/colossus_legal_v2"
    ));
}

/// A URL with no database segment at all is refused, not read as "fine".
#[test]
fn a_url_naming_no_database_is_refused() {
    assert!(!allows("postgres://u:p@10.10.100.200:5432/", SCRATCH_PIPE));
    assert!(!allows("not-a-url", SCRATCH_PIPE));
}

/// A query string after the database name does not smuggle a live database
/// past the pattern, and does not stop a scratch one from being recognised.
#[test]
fn a_query_string_is_stripped_before_the_name_is_judged() {
    assert!(allows(
        &format!("{SCRATCH_MAIN}?sslmode=disable"),
        &format!("{SCRATCH_PIPE}?sslmode=disable")
    ));
    assert!(!allows(
        "postgres://u:p@h:5432/colossus_legal_v2?sslmode=disable",
        SCRATCH_PIPE
    ));
}

/// (M) A database whose name merely CONTAINS a scratch word is still refused.
///
/// The pattern is anchored. Unanchored, `colossus_legal_v2_proof_of_concept`
/// would pass — and so, more to the point, would a live database somebody
/// renamed with a suffix.
#[test]
fn the_scratch_pattern_is_anchored() {
    assert!(!allows(
        "postgres://u:p@h:5432/proof_colossus_legal_v2",
        SCRATCH_PIPE
    ));
    assert!(!allows(
        "postgres://u:p@h:5432/colossus_legal_v2_prooflike",
        SCRATCH_PIPE
    ));
}
