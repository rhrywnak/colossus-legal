//! What the three 500s and the one 409 in `support` actually SAY.
//!
//! These are pure mapping tests: no pool, no `AppState`, no request. They
//! construct each error the subset write path can produce and assert the
//! message an operator would read, because the whole point of Standing Rule 1
//! is that two different failures do not look alike in the log.
//!
//! The sibling `api::timeline_write::support_tests` proves the same property for
//! the EVENT write path; this is that proof for subsets, which grew its own
//! error types in T1.

use uuid::Uuid;

use super::{seal_failure, write_error};
use crate::error::AppError;
use crate::repositories::pipeline_repository::PipelineRepoError;
use crate::services::chronology_subset_guard::SubsetSealError;
use crate::services::chronology_subset_write::SubsetWriteError;

/// Pull the message out of a 500, or fail loudly saying what arrived instead.
fn internal_message(error: AppError) -> String {
    match error {
        AppError::Internal { message } => message,
        other => panic!("expected a 500, got {other:?}"),
    }
}

// ── the two 500s are different events ───────────────────────────────────────

#[test]
fn a_subset_seal_failure_says_the_change_was_not_made() {
    // ⚑ A write failure means the mutation did not happen. A SEAL failure means
    // it DID and its history row did not, so the whole transaction rolled back.
    // An operator reading "could not be written" for the second would go hunting
    // for a half-applied change that is not there.
    let write = internal_message(super::subset_failure(
        PipelineRepoError::Database("connection reset".to_string()),
        "deleting a subset",
    ));
    let seal = internal_message(seal_failure(
        &SubsetSealError::Vanished {
            subset_id: Uuid::from_u128(1),
        },
        "deleting a subset",
    ));

    assert_ne!(
        write, seal,
        "the two 500s must not read alike; they mean opposite things about the database"
    );
    assert!(
        seal.contains("so it was not made"),
        "the seal 500 must say the change did not survive, got: {seal}"
    );
    assert!(
        seal.contains("deleting a subset"),
        "the seal 500 must name the operation, got: {seal}"
    );
    assert!(
        write.contains("deleting a subset"),
        "the write 500 must name the operation, got: {write}"
    );
}

#[test]
fn a_vanished_subset_names_itself_and_its_own_transaction() {
    // The message an operator gets in the log line behind the 500. It has to say
    // WHICH subset and that the read happened inside the write's own
    // transaction, because that is the detail separating "somebody deleted it"
    // from "our own transaction cannot see our own row".
    let id = Uuid::from_u128(0x2c);
    let message = SubsetSealError::Vanished { subset_id: id }.to_string();

    assert!(
        message.contains(&id.to_string()),
        "a seal failure must name the subset, got: {message}"
    );
    assert!(
        message.contains("write transaction"),
        "a seal failure must say where the read-back happened, got: {message}"
    );
}

// ── the refusal that is a 409, not a 404 ────────────────────────────────────

#[test]
fn writing_to_a_deleted_subset_is_a_conflict_that_names_undo() {
    // ⚑ 409 and not 404 on purpose: the subset is there, in a state that cannot
    // take an edit, and it is one press of Undo away. Answering 404 would tell
    // somebody their story is gone when it is recoverable — the exact collapse
    // the history table exists to prevent.
    let id = Uuid::from_u128(0x5e);
    let error = write_error(
        SubsetWriteError::Deleted { subset_id: id },
        "renaming a subset",
    );

    let AppError::Conflict { message, details } = error else {
        panic!("a write onto a deleted subset must be a 409, not a 404 and not a 500");
    };
    assert!(
        message.contains(&id.to_string()),
        "the 409 must name the subset, got: {message}"
    );
    assert!(
        message.contains("Undo"),
        "the 409 must name the way out, got: {message}"
    );
    assert_eq!(
        details.get("subset_id").and_then(|v| v.as_str()),
        Some(id.to_string().as_str()),
        "the subset id must reach the body's details, not only the prose"
    );
}

#[test]
fn a_repo_failure_under_a_write_is_still_a_500() {
    // The third arm of `write_error`. Proved because a mapping table with an arm
    // nothing exercises is how a database outage starts answering 409.
    let error = write_error(
        SubsetWriteError::Repo {
            source: PipelineRepoError::Database("deadlock detected".to_string()),
        },
        "replacing a subset's events",
    );
    let message = internal_message(error);
    assert!(
        message.contains("replacing a subset's events"),
        "the 500 must name the operation, got: {message}"
    );
}
