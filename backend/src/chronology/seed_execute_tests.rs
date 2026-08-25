//! Tests for the executor's REFUSALS.
//!
//! The `run`/`write_and_verify`/`verify` chain needs a live `PgPool`, and this
//! project has no database test tier — that half was proved by BEGIN/ROLLBACK
//! transcripts against DEV. What needs no database, and is asserted here, is
//! what each refusal SAYS.
//!
//! ## Why a message deserves a test
//!
//! These strings are read by an operator mid-runbook, usually in a hurry. The
//! `Verification` variant interpolates three fields (`what`, `expected`,
//! `counted`); two of them are `i64` and transposing them compiles cleanly and
//! produces a confidently backwards sentence at the worst possible moment. A
//! test that reads the rendered message is the only thing that catches that.

use super::*;

#[test]
fn already_seeded_names_the_case_and_the_count_it_found() {
    let error = SeedExecError::AlreadySeeded {
        case_slug: "awad_v_catholic_family_service".to_string(),
        existing: 22,
    };
    let rendered = error.to_string();

    assert!(
        rendered.contains("awad_v_catholic_family_service"),
        "{rendered}"
    );
    assert!(rendered.contains("22 chronology event(s)"), "{rendered}");
    assert!(
        rendered.contains("seeds a case ONCE"),
        "the message must say WHY it refused, got: {rendered}"
    );
}

#[test]
fn missing_targets_lists_the_ids_and_says_nothing_was_written() {
    let error = SeedExecError::MissingTargets {
        missing: "doc-gone, doc-also-gone".to_string(),
        missing_count: 2,
    };
    let rendered = error.to_string();

    assert!(rendered.contains("2 document(s)"), "{rendered}");
    assert!(rendered.contains("doc-gone, doc-also-gone"), "{rendered}");
    assert!(
        rendered.contains("Nothing was written"),
        "an operator's first question is what survived, got: {rendered}"
    );
    assert!(
        rendered.contains("both need a human, not a retry"),
        "the message must say what NOT to do, got: {rendered}"
    );
}

#[test]
fn verification_reports_expected_and_counted_the_right_way_round() {
    // The transposition guard. `expected` and `counted` are both i64, so
    // swapping them at the call site compiles and lies.
    let error = SeedExecError::Verification {
        what: "link rows",
        expected: 7,
        counted: 6,
    };
    let rendered = error.to_string();

    assert!(
        rendered.contains("expected 7 link rows"),
        "the EXPECTED count must follow the word 'expected', got: {rendered}"
    );
    assert!(
        rendered.contains("counted 6"),
        "the COUNTED value must follow the word 'counted', got: {rendered}"
    );
    assert!(
        !rendered.contains("expected 6"),
        "expected and counted are transposed: {rendered}"
    );
    assert!(
        rendered.contains("rolled back and nothing was written"),
        "{rendered}"
    );
}

#[test]
fn a_database_failure_carries_the_underlying_cause() {
    let error =
        SeedExecError::Database("relation \"chronology_events\" does not exist".to_string());
    let rendered = error.to_string();

    assert!(
        rendered.contains("relation \"chronology_events\" does not exist"),
        "the cause must survive to the operator, got: {rendered}"
    );
    assert!(
        rendered.contains("the chronology seed failed"),
        "{rendered}"
    );
}

#[test]
fn the_four_refusals_are_distinguishable_from_each_other() {
    // Four distinct operator actions — remove the rows, fix the map, investigate
    // the data, investigate the environment — so four distinct sentences. A
    // reader of a log who cannot tell them apart has to open the source.
    let rendered = [
        SeedExecError::AlreadySeeded {
            case_slug: "c".to_string(),
            existing: 1,
        },
        SeedExecError::MissingTargets {
            missing: "d".to_string(),
            missing_count: 1,
        },
        SeedExecError::Verification {
            what: "events",
            expected: 1,
            counted: 0,
        },
        SeedExecError::Database("boom".to_string()),
    ]
    .iter()
    .map(ToString::to_string)
    .collect::<Vec<_>>();

    let mut prefixes: Vec<&str> = rendered.iter().map(|r| &r[..20]).collect();
    prefixes.sort_unstable();
    let before = prefixes.len();
    prefixes.dedup();
    assert_eq!(before, prefixes.len(), "two refusals open the same way");
}

#[test]
fn seed_mode_distinguishes_the_three_acts() {
    // The mode drives whether the transaction commits, rolls back, or is never
    // opened; two modes comparing equal would silently merge two of those.
    assert_ne!(SeedMode::DryRun, SeedMode::ProveInTransaction);
    assert_ne!(SeedMode::ProveInTransaction, SeedMode::Apply);
    assert_ne!(SeedMode::DryRun, SeedMode::Apply);
}
