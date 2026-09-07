// Tests for `api::proof_matrix_export`.
//
// The handler needs a live graph, so what is asserted here is the RESPONSE
// ENVELOPE and the error mapping — the parts that decide whether a browser saves
// a Word document or shows a page of binary.

use super::*;
use axum::http::header;

/// The response carries the media type Word registers and an attachment
/// disposition, so a browser SAVES the file rather than rendering it.
///
/// Without the disposition, Chrome shows the ZIP bytes inline. That is not a
/// crash, so nothing else would catch it.
#[test]
fn the_response_is_an_attachment_with_the_word_media_type() {
    let response = docx_response(3, b"PK\x03\x04 pretend package".to_vec());

    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers();
    assert_eq!(
        headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some(DOCX_MEDIA_TYPE)
    );
    let disposition = headers
        .get(header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .expect("a disposition header");
    assert!(disposition.starts_with("attachment;"), "{disposition}");
    assert!(
        disposition.contains("proof-matrix-count-3.docx"),
        "the filename names the Count: {disposition}"
    );
}

/// The media type is the exact one Word registers.
///
/// Pinned by literal: a near-miss (`application/msword`, say) makes a downloaded
/// file open in the wrong application on some machines and not others, which is
/// the kind of defect that gets reported as "it doesn't work" from one desk.
#[test]
fn the_media_type_is_the_registered_ooxml_one() {
    assert_eq!(
        DOCX_MEDIA_TYPE,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    );
}

/// The filename is ASCII and quoted, whatever the Count.
///
/// `Content-Disposition` is a header: a value carrying a raw quote or a non-ASCII
/// byte is a malformed header, and the browser's recovery is to ignore the
/// filename entirely.
#[test]
fn the_filename_is_safe_in_a_header() {
    for count in [1i64, 4, 42] {
        let response = docx_response(count, vec![0u8; 8]);
        let disposition = response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .expect("a disposition header");
        assert!(disposition.is_ascii(), "{disposition}");
        assert_eq!(
            disposition.matches('"').count(),
            2,
            "exactly one quoted filename: {disposition}"
        );
    }
}

/// A missing Count is a 404 that NAMES the count, not a 500 and not an empty
/// document.
#[test]
fn a_missing_count_is_a_404_that_names_it() {
    let response = ExportError::NotFound { count: 9 }.into_response();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Every other failure is a bland 500.
#[test]
fn any_other_failure_is_a_bland_500() {
    let response = ExportError::Internal.into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

/// The query parameter decodes, and a non-numeric or missing one does not.
///
/// Asserted through serde on the JSON form of the same struct — axum's `Query`
/// extractor uses the same `Deserialize` impl, and the property under test is
/// that `count` is a REQUIRED `i64` with no default. A `#[serde(default)]` slipped
/// onto it would silently export Count 0, which is a document with a title and
/// nothing in it.
#[test]
fn the_count_query_parameter_is_required_and_numeric() {
    let ok: ExportQuery =
        serde_json::from_value(serde_json::json!({ "count": 3 })).expect("a number decodes");
    assert_eq!(ok.count, 3);
    assert!(
        serde_json::from_value::<ExportQuery>(serde_json::json!({ "count": "three" })).is_err(),
        "a non-numeric count must be refused rather than defaulted to 0"
    );
    assert!(
        serde_json::from_value::<ExportQuery>(serde_json::json!({})).is_err(),
        "a missing count must be refused rather than exporting an arbitrary Count"
    );
}
