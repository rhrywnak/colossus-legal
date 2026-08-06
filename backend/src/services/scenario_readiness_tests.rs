//! Unit tests for [`super`] — the ready gate and the readiness transitions.
//!
//! What this module owns is the GATE: which scenarios a witness may be shown at
//! all, and the two human acts that move a scenario across it. Getting the gate
//! wrong puts a drafted scenario in front of a witness, which is the one failure
//! in this area that would actually harm somebody.
//!
//! §10's exclusion law moved to `services::rehearsal_render`'s tests with task
//! 2.11 B2, beside the payload it now guards — the ban list and the payload only
//! mean anything together.
//!
//! The transactional writes need a live Postgres and are DEV-verified, matching
//! the convention of every sibling service.

use super::*;
use uuid::Uuid;

// ── The gate ─────────────────────────────────────────────────────────────────

/// Both drafted spellings read as "not in rehearsal".
///
/// The v2 state machine has two states; the column has three values. This is
/// where the mapping ruled on 2026-08-01 is pinned, so a later reading of
/// `needs_evidence` as "nearly ready" cannot quietly put an unfinished scenario
/// in front of a witness.
#[test]
fn only_the_ready_status_opens_the_gate() {
    for status in ["draft", "needs_evidence"] {
        assert!(
            !is_ready(&record_with_status(status)),
            "'{status}' is v2 'drafted' — it must NOT reach rehearsal"
        );
    }
    assert!(is_ready(&record_with_status(READY_STATUS)));
}

/// A status this build has never heard of is excluded, not included.
///
/// Testing FOR `ready` rather than AGAINST the drafted values means a fourth
/// status added later defaults to invisible. That is the safe direction: a
/// scenario reaches a witness only when someone deliberately said it should.
#[test]
fn an_unknown_status_stays_out_of_rehearsal() {
    assert!(!is_ready(&record_with_status("archived")));
}

// ── The exclusion law (v2 §10, as amended 2026-08-06) ───────────────────────
//
// The banned list MOVED with task 2.11 B2, and the move was ruled rather than
// assumed. `document_id` and `page` are gone from it: REHEARSAL_VIEW_DESIGN_v2 is
// later, specific, and Roman-signed with the "Deposition, p. 42 · [open]" table in
// it, and the research it rests on says a witness who cannot produce the source on
// the spot loses credibility. §10 exists to keep impeachment MACHINERY off this
// surface — the grading, the confidence, the verdict, the strategy — and every one
// of those is still banned below.
//
// The list lives in `rehearsal_exclusion_tests.rs` with the payload-shaped test
// that uses it, because the two only mean anything together.

// ── The refusals ─────────────────────────────────────────────────────────────

#[test]
fn declaring_ready_twice_is_refused_rather_than_recorded() {
    // A transition row from `ready` to `ready` would make the history read as
    // though someone did something. The table's CHECK refuses it too — this turns
    // that 500 into a sentence the human can act on.
    let Err(error) = target_status(READY_STATUS, true) else {
        panic!("promoting an already-ready scenario must be refused");
    };
    let message = error.to_string();
    assert!(message.contains(READY_STATUS), "{message}");
    assert!(message.contains("nothing to change"), "{message}");
}

/// Demoting an already-drafted scenario is a no-op in BOTH spellings.
///
/// `needs_evidence` is not `draft`, but neither is visible to a witness, so
/// "remove from rehearsal" changes nothing either way. Comparing against the
/// target token alone would have recorded a `needs_evidence → draft` transition
/// as though someone had withdrawn something.
#[test]
fn demoting_something_already_out_of_rehearsal_is_refused() {
    for status in ["draft", "needs_evidence"] {
        assert!(
            target_status(status, false).is_err(),
            "'{status}' is already out of rehearsal — there is nothing to withdraw"
        );
    }
}

#[test]
fn the_two_real_transitions_are_allowed_and_land_where_expected() {
    assert_eq!(
        target_status("draft", true).expect("drafted → ready is the promote"),
        READY_STATUS
    );
    assert_eq!(
        target_status("needs_evidence", true).expect("the other drafted spelling promotes too"),
        READY_STATUS
    );
    // A demotion writes the plain drafted token, not a claim about evidence.
    assert_eq!(
        target_status(READY_STATUS, false).expect("ready → drafted is the withdraw"),
        "draft"
    );
}

/// A read failure and a write failure say different things.
///
/// The 1.4 lesson, kept: collapsing the two told a human "failed to save" when a
/// LOAD had failed — a message about an action they never took.
#[test]
fn a_failed_read_and_a_failed_write_are_different_sentences() {
    let source = || PipelineRepoError::from(sqlx::Error::RowNotFound);
    let read = ReadinessError::Read { source: source() }.to_string();
    let write = ReadinessError::Write { source: source() }.to_string();

    assert!(read.contains("read the scenario"), "{read}");
    assert!(write.contains("record the readiness change"), "{write}");
    assert_ne!(read, write);
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// A scenario record carrying nothing but the status under test.
fn record_with_status(status: &str) -> ScenarioRecord {
    ScenarioRecord {
        scenario_id: Uuid::nil(),
        case_slug: "awad-v-cfs".to_string(),
        code_ordinal: 2,
        name: "The missing file".to_string(),
        direction: "defensive".to_string(),
        status: status.to_string(),
        theme_statement: None,
        motivation: None,
        // Task 2.11: nobody has written the plain-words accusation for this
        // fixture, which is the honest default — the page renders its gap.
        accusation_text: None,
        accusation_text_authored_by: None,
        accusation_text_authored_at: None,
        theme_authored_by: None,
        theme_authored_at: None,
        definition: serde_json::json!({}),
        anchor_allegation_ids: None,
        feeds_count_id: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

// ── Which store failed, and therefore which operator it belongs to ──────────

#[test]
fn a_postgres_failure_and_a_record_store_failure_route_to_different_errors() {
    // The two have different operators and different remedies. If the branch
    // condition were wrong, a Neo4j outage would arrive attributed to Postgres —
    // sending somebody to the wrong service with nothing in the failure signature
    // to say so.
    let postgres = assembly_to_readiness(
        AssemblyError::Read {
            source: PipelineRepoError::Database("connection refused".to_string()),
        },
        Uuid::nil(),
    );
    let record = assembly_to_readiness(
        AssemblyError::Record {
            source:
                crate::repositories::scenario_card_repository::ScenarioCardRepoError::RowDecode {
                    operation: "fetch_rehearsal_facts",
                    source: neo4rs::DeError::PropertyMissingButRequired,
                },
        },
        Uuid::nil(),
    );

    assert!(matches!(postgres, ReadinessError::Read { .. }));
    assert!(matches!(record, ReadinessError::Assembly { .. }));
    assert_ne!(postgres.to_string(), record.to_string());
    assert!(postgres.to_string().contains("scenario"), "{postgres}");
    assert!(record.to_string().contains("rehearsal view"), "{record}");
}

#[test]
fn an_undecodable_row_is_an_assembly_failure_and_not_a_read_one() {
    // A stored token this build cannot classify is not an outage. It must not be
    // reported as one, or an operator restarts a healthy database looking for it.
    let error = assembly_to_readiness(
        AssemblyError::Undecodable {
            detail: "unknown human-fact kind 'rehearsal_highlight'".to_string(),
        },
        Uuid::nil(),
    );

    assert!(matches!(error, ReadinessError::Assembly { .. }));
    assert!(
        error.to_string().contains("rehearsal_highlight"),
        "the offending token must survive to the operator: {error}"
    );
}
