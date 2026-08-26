//! Tests for `api::timeline_write_support`.
//!
//! The handlers themselves need an `AppState` (two pools, a graph, a registry),
//! which this project has no test tier for. What IS reachable is every pure
//! decision the module makes: which targets get asked about, and which HTTP
//! status a refusal becomes.
//!
//! The target-collection tests moved here from `api::timeline_tests` in Phase C,
//! with the function they exercise — the read and the writes now resolve targets
//! by the same code, and the tests followed it rather than being copied.

use super::*;
use crate::services::chronology_validate::ChronologyWriteRefusal;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::{TimeZone, Utc};

fn link(target_type: &str, target_id: &str) -> ChronologyLinkRow {
    ChronologyLinkRow {
        event_id: Uuid::from_u128(1),
        target_type: target_type.to_string(),
        target_id: target_id.to_string(),
        label: None,
        pinpoint: None,
        created_by: None,
        created_at: Utc
            .with_ymd_and_hms(2026, 8, 25, 12, 0, 0)
            .single()
            .expect("a real instant"),
    }
}

// ── which targets are asked about ───────────────────────────────────────────

#[test]
fn only_document_targets_are_asked_about() {
    // Asking whether a `scenario` id "exists in documents" would produce a
    // confident No about a store this build cannot see. Three answers, not two.
    let links = vec![
        link("document", "doc-a"),
        link("scenario", "S-5"),
        link("paperless_document", "42"),
    ];
    assert_eq!(checkable_target_ids(&links), vec!["doc-a".to_string()]);
}

#[test]
fn the_same_document_twice_is_asked_about_once() {
    let links = vec![
        link("document", "doc-a"),
        link("document", "doc-a"),
        link("document", "doc-b"),
    ];
    assert_eq!(
        checkable_target_ids(&links),
        vec!["doc-a".to_string(), "doc-b".to_string()]
    );
}

#[test]
fn no_links_asks_nothing() {
    assert!(checkable_target_ids(&[]).is_empty());
}

// ── the 400 / 422 table ─────────────────────────────────────────────────────

fn status_of(error: AppError) -> StatusCode {
    error.into_response().status()
}

#[test]
fn a_shape_problem_is_a_400() {
    assert_eq!(
        status_of(refusal(ChronologyWriteRefusal::BlankTitle)),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of(refusal(ChronologyWriteRefusal::UnreadableDate {
            supplied: "11/03/2009".to_string()
        })),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        status_of(refusal(ChronologyWriteRefusal::BlankNote)),
        StatusCode::BAD_REQUEST
    );
}

#[test]
fn an_unknown_phase_is_a_422_and_never_a_500() {
    // ⚑ Phase C's instruction, word for word: "an unknown phase is a 422 naming
    // the value, never a 500". Without the refusal the slug reaches Postgres,
    // the foreign key rejects it, and this codebase turns that into a 500 —
    // paging an operator over somebody's typo.
    let error = refusal(ChronologyWriteRefusal::UnknownPhase {
        supplied: "apeals".to_string(),
        known: "estate, probate, appeals, civil_lawsuit".to_string(),
    });
    let status = status_of(error);
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_ne!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn an_unknown_tag_is_a_422_too() {
    assert_eq!(
        status_of(refusal(ChronologyWriteRefusal::UnknownTag {
            supplied: "sanctions".to_string(),
            known: "financial, filing".to_string(),
        })),
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn a_refusal_body_names_the_field_and_quotes_the_value() {
    // A form highlights the box it is told about, and quotes back what was
    // rejected. A 422 whose body said only "invalid" would send an author to
    // guess which of seven fields it meant.
    let response = refusal(ChronologyWriteRefusal::UnknownPhase {
        supplied: "apeals".to_string(),
        known: "estate, probate, appeals, civil_lawsuit".to_string(),
    })
    .into_response();
    let bytes = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .expect("body must be small and readable");
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("valid JSON");

    assert_eq!(body["error"], serde_json::json!("unprocessable_entity"));
    assert_eq!(body["details"]["field"], serde_json::json!("phase"));
    assert_eq!(body["details"]["value"], serde_json::json!("apeals"));
    let message = body["message"].as_str().expect("a message");
    assert!(message.contains("apeals"), "got: {message}");
    assert!(
        message.contains("appeals"),
        "the refusal must also list the real phases: {message}"
    );
}

#[test]
fn a_blank_event_id_is_a_400_that_says_what_an_id_looks_like() {
    let error = parse_event_id("not-a-uuid").expect_err("that is not an id");
    assert_eq!(status_of(error), StatusCode::BAD_REQUEST);
}

#[test]
fn a_real_uuid_parses() {
    let id = parse_event_id("11111111-2222-3333-4444-555555555555").expect("a real id");
    assert_eq!(id.to_string(), "11111111-2222-3333-4444-555555555555");
}

// ── the two failure logs are different events ───────────────────────────────

#[test]
fn a_seal_failure_says_the_change_was_not_made() {
    // ⚑ The two 500s must not read alike. A write failure means the mutation did
    // not happen; a SEAL failure means it did and its record did not, so the
    // whole transaction rolled back. An operator reading "could not be written"
    // for the second would go looking for a half-applied change that is not
    // there.
    let write = write_failure(
        PipelineRepoError::Database("connection reset".to_string()),
        "inserting the event",
    );
    let seal = seal_failure(
        crate::services::chronology_guard::SealError::Vanished {
            event_id: Uuid::from_u128(1),
        },
        "creating an event",
    );
    let (
        AppError::Internal {
            message: write_message,
        },
        AppError::Internal {
            message: seal_message,
        },
    ) = (write, seal)
    else {
        panic!("both must be 500s");
    };
    assert_ne!(write_message, seal_message);
    assert!(
        seal_message.contains("so it was not made"),
        "a seal failure must say the change did not land: {seal_message}"
    );
}
