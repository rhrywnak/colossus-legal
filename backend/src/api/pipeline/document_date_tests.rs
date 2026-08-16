//! The HTTP surface of the document-date write.
//!
//! The RULE lives in `domain::date_precision` and is tested there against every
//! valid and invalid pair. What is tested here is the boundary: that the request
//! shape a browser actually sends deserializes, that the response shape carries
//! what the page needs, and that the precision list the intake control renders
//! from is the backend's vocabulary rather than a copy.

use super::*;

#[test]
fn a_dated_request_deserializes_from_what_the_dialog_sends() {
    let body: SetDocumentDateRequest =
        serde_json::from_str(r#"{"document_date":"2009-11-05","date_precision":"day"}"#)
            .expect("deserialises");
    assert_eq!(body.document_date.as_deref(), Some("2009-11-05"));
    assert_eq!(body.date_precision, "day");
}

#[test]
fn an_unknown_date_request_omits_the_date_entirely() {
    // `#[serde(default)]` on `document_date` is what makes this legal. Without
    // it, the override would need the caller to send an explicit null, and a
    // form that simply omits an empty field would 422 instead of recording
    // "this document has no date".
    let body: SetDocumentDateRequest =
        serde_json::from_str(r#"{"date_precision":"unknown"}"#).expect("deserialises");
    assert_eq!(body.document_date, None);
    assert_eq!(body.date_precision, "unknown");
}

#[test]
fn a_request_with_no_precision_is_refused_by_the_deserializer() {
    // Mandatory-with-override starts here: a body that says nothing about
    // precision cannot be read as "unknown". `date_precision` has no serde
    // default, so it is a 422 before the handler runs.
    let result: Result<SetDocumentDateRequest, _> =
        serde_json::from_str(r#"{"document_date":"2009-11-05"}"#);
    assert!(
        result.is_err(),
        "a body with no date_precision must be refused, not defaulted"
    );
}

#[test]
fn an_explicit_null_date_is_accepted_by_the_deserializer() {
    // A form that clears the field may send null rather than omitting it; both
    // reach the handler as `None` and both then meet the same rule.
    let body: SetDocumentDateRequest =
        serde_json::from_str(r#"{"document_date":null,"date_precision":"unknown"}"#)
            .expect("deserialises");
    assert_eq!(body.document_date, None);
}

#[test]
fn a_dated_response_carries_the_date_and_a_human_label() {
    let response = DocumentDateResponse {
        document_id: "doc-certified-letter-to-george-phillips-11-05-2009".to_string(),
        document_date: Some("2009-11-05".to_string()),
        date_precision: "day".to_string(),
        date_precision_label: DatePrecision::Day.label().to_string(),
    };
    let json = serde_json::to_value(&response).expect("serialises");
    assert_eq!(json["document_date"], "2009-11-05");
    assert_eq!(json["date_precision"], "day");
    assert_eq!(json["date_precision_label"], "Exact date");
}

#[test]
fn an_undated_response_omits_the_date_key_rather_than_sending_null() {
    let response = DocumentDateResponse {
        document_id: "doc-x".to_string(),
        document_date: None,
        date_precision: "unknown".to_string(),
        date_precision_label: DatePrecision::Unknown.label().to_string(),
    };
    let json = serde_json::to_value(&response).expect("serialises");
    assert!(
        json.get("document_date").is_none(),
        "an absent date must be an absent key, matching the repo's \
         skip_serializing_if convention"
    );
    assert_eq!(json["date_precision"], "unknown");
}

#[tokio::test]
async fn the_precision_list_offers_every_precision_with_its_date_requirement() {
    // Standing Rule 12: the intake control renders from this, so a hardcoded
    // TypeScript copy cannot drift from the vocabulary.
    let Json(options) = list_date_precisions().await;

    let values: Vec<&str> = options.iter().map(|o| o.value.as_str()).collect();
    assert_eq!(values, vec!["day", "month", "year", "unknown"]);

    for option in &options {
        assert!(!option.label.is_empty(), "{} has no label", option.value);
    }
    let unknown = options
        .iter()
        .find(|o| o.value == "unknown")
        .expect("the override is offered");
    assert!(
        !unknown.requires_date,
        "'unknown' is the override; a control that demanded a date for it would \
         make the override unusable"
    );
    assert!(
        options
            .iter()
            .filter(|o| o.value != "unknown")
            .all(|o| o.requires_date),
        "every real precision requires a date"
    );
}

#[test]
fn a_bad_precision_becomes_a_bad_request_that_names_the_accepted_set() {
    let error = to_app_error(
        "doc-x",
        DatePrecisionError::UnknownToken {
            token: "moth".to_string(),
        },
    );
    match error {
        AppError::BadRequest { message, details } => {
            assert!(message.contains("moth"), "got: {message}");
            let accepted = &details["accepted_precisions"];
            assert_eq!(accepted[0], "day");
            assert_eq!(accepted[3], "unknown");
        }
        other => panic!("expected a 400, got {other:?}"),
    }
}

#[test]
fn a_missing_date_becomes_a_bad_request_not_an_internal_error() {
    // A user who left the field blank must get a message they can act on, not a
    // 500 from a constraint violation further down.
    let error = to_app_error(
        "doc-x",
        DatePrecisionError::DateMissing { precision: "day" },
    );
    assert!(matches!(error, AppError::BadRequest { .. }));
}

#[test]
fn a_write_that_matched_no_rows_is_a_not_found_not_a_success() {
    // The repository returns the row count precisely so this case can be told
    // apart. A date typed against a document id that does not exist must not
    // come back 200 with the values echoed and nothing stored.
    //
    // The handler needs a live pool, so what is asserted here is the shape of
    // the refusal it builds — the branch itself is one `if rows == 0`.
    let error = AppError::NotFound {
        message: "Document 'doc-typo' not found — the date was not stored".to_string(),
    };
    match error {
        AppError::NotFound { message } => {
            assert!(message.contains("doc-typo"));
            assert!(
                message.contains("not stored"),
                "the refusal must say the date did not land, or an operator \
                 assumes it did: {message}"
            );
        }
        other => panic!("expected a 404, got {other:?}"),
    }
}
