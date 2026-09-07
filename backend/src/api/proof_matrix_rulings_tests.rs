// Tests for `api::proof_matrix_rulings`.
//
// The two handlers need a live `AppState`, so what is asserted here is the
// BOUNDARY: what the request body will and will not accept, and what the response
// says about what happened. Those are the parts a wrong client would hit first.

use super::*;

fn body(json: serde_json::Value) -> Result<RulingRequest, serde_json::Error> {
    serde_json::from_value(json)
}

/// The documented body decodes.
#[test]
fn the_documented_body_decodes() {
    let request = body(serde_json::json!({
        "evidence_id": "doc-x:evidence:01d3e125",
        "allegation_id": "allegation-41",
        "ruling": "keep"
    }))
    .expect("the §2 body shape decodes");

    assert_eq!(request.evidence_id, "doc-x:evidence:01d3e125");
    assert_eq!(request.ruling, Some(MatrixRuling::Keep));
    assert_eq!(request.note, None);
}

/// A verdict outside the vocabulary is refused by SERDE, before the handler runs.
///
/// This is why `ruling` is a typed enum rather than a `String` validated later:
/// there is no code path in which an unknown verdict reaches a database write.
#[test]
fn an_unknown_verdict_is_refused_at_the_boundary() {
    let err = body(serde_json::json!({
        "evidence_id": "e",
        "allegation_id": "a",
        "ruling": "obliterate"
    }))
    .expect_err("'obliterate' is not a verdict");
    assert!(err.to_string().contains("obliterate"), "{err}");
}

/// A camelCase typo is refused rather than silently defaulted.
///
/// Without `deny_unknown_fields`, `{"evidenceId": …}` would decode with an EMPTY
/// `evidence_id` and write a row nothing can ever read back.
#[test]
fn a_camel_case_key_is_refused() {
    let err = body(serde_json::json!({
        "evidenceId": "e",
        "allegation_id": "a",
        "ruling": "keep"
    }))
    .expect_err("unknown fields are denied");
    assert!(err.to_string().contains("evidenceId"), "{err}");
}

/// The DELETE body may omit the verdict — a withdrawal asserts nothing.
#[test]
fn a_withdrawal_body_may_omit_the_verdict() {
    let request = body(serde_json::json!({
        "evidence_id": "e",
        "allegation_id": "a"
    }))
    .expect("a withdrawal needs no verdict");
    assert_eq!(request.ruling, None);
}

/// An optional note rides along.
#[test]
fn a_note_rides_along_when_there_is_one() {
    let request = body(serde_json::json!({
        "evidence_id": "e",
        "allegation_id": "a",
        "ruling": "remove",
        "note": "Duplicates RFA 8."
    }))
    .expect("a note is accepted");
    assert_eq!(request.note.as_deref(), Some("Duplicates RFA 8."));
}

/// A blank id is refused, and the refusal NAMES the field.
///
/// A row keyed on `''` can never be matched by a read: the write would report
/// success, the ledger would record a human decision, and the page would never
/// show it.
#[test]
fn a_blank_id_is_refused_by_name() {
    for (evidence, allegation, expected) in [
        ("", "allegation-41", "evidence_id"),
        ("   ", "allegation-41", "evidence_id"),
        ("evidence-1", "", "allegation_id"),
    ] {
        let request = RulingRequest {
            evidence_id: evidence.to_string(),
            allegation_id: allegation.to_string(),
            ruling: Some(MatrixRuling::Keep),
            note: None,
        };
        let err = validated_pair(&request).expect_err("a blank id is refused");
        let rendered = format!("{err:?}");
        assert!(
            rendered.contains(expected),
            "the refusal must name {expected}: {rendered}"
        );
    }
}

/// Surrounding whitespace is trimmed rather than stored.
///
/// A client that sends `" evidence-1 "` must write the same row as one that sends
/// `"evidence-1"`, or the same item would carry two independent verdicts.
#[test]
fn ids_are_trimmed_before_they_are_written() {
    let request = RulingRequest {
        evidence_id: "  doc-x:evidence:01d3e125  ".to_string(),
        allegation_id: "\tallegation-41\n".to_string(),
        ruling: Some(MatrixRuling::Remove),
        note: None,
    };
    let (evidence_id, allegation_id) = validated_pair(&request).expect("valid once trimmed");
    assert_eq!(evidence_id, "doc-x:evidence:01d3e125");
    assert_eq!(allegation_id, "allegation-41");
}

/// The response distinguishes a withdrawal that removed something from one that
/// found nothing.
///
/// They look identical on screen otherwise, and only one of them means the page
/// was out of date.
#[test]
fn the_response_tells_a_real_withdrawal_from_an_empty_one() {
    let real = RulingResponse {
        evidence_id: "e".to_string(),
        allegation_id: "a".to_string(),
        action: "withdraw",
        changed: true,
    };
    let empty = RulingResponse {
        action: "none",
        changed: false,
        ..RulingResponse {
            evidence_id: "e".to_string(),
            allegation_id: "a".to_string(),
            action: "withdraw",
            changed: true,
        }
    };
    let real = serde_json::to_value(&real).expect("serializes");
    let empty = serde_json::to_value(&empty).expect("serializes");
    assert_eq!(real["action"], "withdraw");
    assert_eq!(real["changed"], true);
    assert_eq!(empty["action"], "none");
    assert_eq!(empty["changed"], false);
}
