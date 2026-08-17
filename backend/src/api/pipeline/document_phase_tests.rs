// Tests for `api::pipeline::document_phase`.
//
// The handler needs a live pool, so what is asserted here is the WIRE CONTRACT —
// the shapes a browser actually sends and receives. That is where this endpoint
// can drift silently: a renamed field would compile fine and clear every phase
// it touched.

use super::*;

#[test]
fn a_phase_body_deserializes() {
    let body: SetDocumentPhaseRequest =
        serde_json::from_str(r#"{"phase":"probate"}"#).expect("a slug body must parse");
    assert_eq!(body.phase.as_deref(), Some("probate"));
}

/// The dropdown's "no selection" post. `#[serde(default)]` is what makes an
/// absent field legal, and an empty string is what a `<select>` with no
/// selection actually sends — both mean "clear it".
#[test]
fn an_absent_null_or_empty_phase_all_parse_as_clear() {
    for raw in [r#"{}"#, r#"{"phase":null}"#, r#"{"phase":""}"#] {
        let body: SetDocumentPhaseRequest =
            serde_json::from_str(raw).unwrap_or_else(|e| panic!("{raw} should parse: {e}"));
        // `validate` is what turns all three into `None`; the DTO only has to
        // carry them without failing.
        assert!(
            crate::domain::case_phase::validate(body.phase.as_deref())
                .expect("clearing is always valid")
                .is_none(),
            "{raw} must clear the phase",
        );
    }
}

/// A drifted client is a 422, not a silently ignored field. Without
/// `deny_unknown_fields` a frontend posting `{"document_phase": "..."}` would
/// parse as "clear the phase" and wipe it.
#[test]
fn an_unknown_field_is_refused() {
    let parsed: Result<SetDocumentPhaseRequest, _> =
        serde_json::from_str(r#"{"phase":"probate","document_phase":"probate"}"#);
    assert!(parsed.is_err(), "an unknown field must not be ignored");
}

/// The response carries the slug and NO label — the ruling's whole point.
#[test]
fn the_response_carries_the_slug_and_never_a_label() {
    let json = serde_json::to_string(&DocumentPhaseResponse {
        document_id: "doc-x".to_string(),
        phase: Some("civil_lawsuit".to_string()),
    })
    .expect("the response must serialize");

    assert!(json.contains(r#""phase":"civil_lawsuit""#), "{json}");
    for label in ["PRE-PROBATE", "PROBATE", "COA", "COMPLAINT", "label"] {
        assert!(
            !json.contains(label),
            "the backend must never render a display label: {json}",
        );
    }
}

/// A cleared phase is omitted rather than sent as null, matching every other
/// optional field in these DTOs.
#[test]
fn a_cleared_phase_is_omitted_from_the_response() {
    let json = serde_json::to_string(&DocumentPhaseResponse {
        document_id: "doc-x".to_string(),
        phase: None,
    })
    .expect("the response must serialize");
    assert_eq!(json, r#"{"document_id":"doc-x"}"#);
}
