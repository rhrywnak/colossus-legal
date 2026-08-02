//! Tests for [`super`] — the ready toggle and the rehearsal read (task 1.5).
//!
//! The handlers themselves need a live Postgres and are DEV-verified. What is
//! pinned here is what the handlers COMPOSE — the confirmations a human reads
//! before deciding whether to demote a scenario the night before trial — and the
//! two structural laws this route family exists to hold:
//!
//! * the ready gate has exactly ONE recorded path (this route), and
//! * rehearsal never reaches the scan/gather tables.

use super::*;

use crate::repositories::pipeline_repository::PipelineRepoError;

/// Build the confirmation the way the handler does, so the assertions below are
/// about the real composition and not a copy of it.
///
/// ## Rust Learning: extracting a pure core to make a handler testable
///
/// `set_scenario_ready` is `async` and needs a pool, so it cannot be called in a
/// unit test. Its user-visible OUTPUT, though, is pure — a code and a boolean in,
/// a sentence out. Mirroring the handler's two arms here would be a copy that
/// proves nothing (the 1.4 lesson), so the handler calls THIS function and the
/// test calls it too.
#[test]
fn the_promotion_message_names_the_scenario_and_who_can_now_rehearse() {
    let message = readiness_message("S-2", true);
    assert!(message.contains("S-2"), "{message}");
    assert!(
        message.contains("rehearse"),
        "the human needs to know what changed for whom: {message}"
    );
}

/// Demoting says what did NOT happen.
///
/// The hesitation before taking a scenario out of rehearsal is "does this throw
/// away the work". A bare "S-2 is now drafted" leaves that unanswered, and an
/// unanswered version of that question means nobody demotes anything.
#[test]
fn the_demotion_message_promises_nothing_else_changed() {
    let message = readiness_message("S-2", false);
    assert!(message.contains("S-2"), "{message}");
    assert!(message.contains("removed from rehearsal"), "{message}");
    assert!(message.contains("nothing else changed"), "{message}");
}

#[test]
fn the_two_directions_never_read_the_same() {
    assert_ne!(
        readiness_message("S-1", true),
        readiness_message("S-1", false)
    );
}

/// The ready gate has exactly one recorded path.
///
/// ## Why this is a source scan rather than a code review note
///
/// `PUT /scenarios/:id` could set `status` until this task, which meant a rename
/// could carry a readiness change with no actor recorded. It was closed by
/// REFUSING the field. A refusal is one line, and a future change that "just
/// passes status through again" would look entirely reasonable in a diff — so the
/// invariant is asserted here (Standing Rule 21) instead of remembered.
///
/// Structural only. The BEHAVIOUR of the refusal — the 400, its details, and the
/// message naming this route — is pinned by `refuse_status_edit`'s own tests in
/// `api::scenarios`, because a scan cannot tell a `return Err` from a
/// `tracing::warn!`.
#[test]
fn the_generic_update_route_cannot_set_a_status() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/api/scenarios.rs"),
    )
    .expect("the scenarios API is readable");

    let update_fn = source
        .split_once("pub async fn update_scenario(")
        .map(|(_, rest)| rest)
        .expect("update_scenario exists");

    assert!(
        update_fn.contains("refuse_status_edit("),
        "PUT /scenarios/:id must REFUSE `status`. Readiness puts a scenario in \
         front of a witness, and v2 §5/§6 make it a human act with a recorded \
         actor — which that route has no way to record."
    );

    // …and the refused field must not reach the store anyway. Scoped to the store
    // CALL, because the refusal itself legitimately reads `payload.status`.
    let store_call = update_fn
        .split_once("update_scenario_row(")
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(args, _)| args)
        .expect("the update handler calls the store");

    assert!(
        !store_call.contains("payload.status"),
        "PUT /scenarios/:id is passing `status` to the store again — that is the \
         unrecorded readiness change task 1.5 closed. The argument must stay \
         `None`. Call site:\n{store_call}"
    );
}

// ── The error mapping ────────────────────────────────────────────────────────
//
// `readiness_error_to_app_error` is a plain `fn` with no pool, so every branch is
// directly callable. The peer function `map_update_error` in `api::scenarios` is
// tested the same way; this is that pattern applied to the ready gate.

fn a_repo_error() -> PipelineRepoError {
    PipelineRepoError::from(sqlx::Error::RowNotFound)
}

#[test]
fn a_refused_no_op_is_a_400_the_human_can_act_on() {
    let error = readiness_error_to_app_error(ReadinessError::AlreadyInState {
        status: "ready".to_string(),
    });
    let AppError::BadRequest { message, details } = error else {
        panic!("a no-op readiness change is the caller's mistake, not the server's");
    };
    assert_eq!(details, json!({ "reason": "readiness_unchanged" }));
    assert!(message.contains("ready"), "{message}");
}

/// A scenario deleted mid-change is a 404, NOT a 500.
///
/// Nothing is broken: someone had the page open while the scenario was removed.
/// Reporting it as a server fault would page an operator over a race that
/// resolved itself correctly, and would hide the one thing the human needs to
/// know — that the scenario is gone.
#[test]
fn a_vanished_scenario_is_a_404_not_a_server_fault() {
    let error = readiness_error_to_app_error(ReadinessError::Vanished);
    let AppError::NotFound { message } = error else {
        panic!("a deleted scenario is a missing resource, not a broken server");
    };
    assert!(message.contains("no longer exists"), "{message}");
}

/// Both server faults are opaque to the client — and DIFFERENT from each other.
///
/// The 1.4 lesson at the HTTP layer: a failed load told the human "failed to
/// save", a message about an action they never took. The detail stays in the log
/// (both branches `tracing::error!` with the source); the client gets the verb
/// that matches what it asked for.
#[test]
fn a_failed_read_and_a_failed_write_report_different_verbs() {
    let read = readiness_error_to_app_error(ReadinessError::Read {
        source: a_repo_error(),
    });
    let write = readiness_error_to_app_error(ReadinessError::Write {
        source: a_repo_error(),
    });

    let (AppError::Internal { message: read_msg }, AppError::Internal { message: write_msg }) =
        (read, write)
    else {
        panic!("a backend fault is a 500 on both sides");
    };
    assert_eq!(read_msg, "failed to load");
    assert_eq!(write_msg, "failed to save");
    assert_ne!(read_msg, write_msg);
}

/// Neither 500 leaks the database's own words to the client.
///
/// The cause is logged; a caller seeing raw sqlx text learns nothing it can act
/// on and everything about the schema.
#[test]
fn a_server_fault_never_leaks_the_underlying_cause() {
    for error in [
        ReadinessError::Read {
            source: a_repo_error(),
        },
        ReadinessError::Write {
            source: a_repo_error(),
        },
    ] {
        let AppError::Internal { message } = readiness_error_to_app_error(error) else {
            panic!("a backend fault is a 500");
        };
        assert!(
            !message.to_lowercase().contains("row")
                && !message.to_lowercase().contains("sqlx")
                && !message.to_lowercase().contains("postgres"),
            "the client message carries database detail: {message}"
        );
    }
}

/// Rehearsal reads human and authored content only.
///
/// The read-side mirror of §8's write allowlist: the mode assembles a witness's
/// four blocks, and a scan/gather table appearing in this path would mean
/// internal machinery reaching a surface the exclusion law keeps slim.
#[test]
fn the_rehearsal_path_never_touches_the_scan_tables() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services/scenario_readiness.rs"),
    )
    .expect("the readiness service is readable");

    // Strip comments: this module's own doc prose discusses the excluded things
    // at length, and the scan must read CODE.
    let code: String = source
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            !(t.starts_with("//") || t.starts_with('*'))
        })
        .collect::<Vec<_>>()
        .join("\n");

    for forbidden in [
        "scan_runs",
        "scan_run_verdicts",
        "scan_run_merges",
        "gather_scenario_candidates",
        "confidence",
        "motivation",
    ] {
        assert!(
            !code.contains(forbidden),
            "the rehearsal read path references {forbidden} — v2 §10 excludes \
             strategy, scan internals and confidence from this mode entirely."
        );
    }
}
