// Tests for `api::pipeline::evidence_key`.
//
// The property this arm exists for is SURVIVAL: the same statement, re-extracted,
// must produce the same id even though every mooded field around it changed. Most
// of what follows is that one property, approached from the angles that actually
// broke it — 0 of 131 ids survived a real reprocess before this module existed.

use super::*;
use serde_json::json;

const DOC: &str = "doc-george-phillips-response-to-discovery";

#[test]
fn the_id_keeps_the_shape_every_existing_reader_expects() {
    // `{doc_slug}:evidence:{8 hex}` — the same form the catch-all produced, so a
    // reader that splits on ':' is unaffected by the re-key.
    let id = evidence_id(DOC, Some(16), "Yes.", Some("Did you sign it?"));
    let parts: Vec<&str> = id.split(':').collect();
    assert_eq!(parts.len(), 3, "id must be doc:evidence:hash — got {id}");
    assert_eq!(parts[0], DOC);
    assert_eq!(parts[1], "evidence");
    assert_eq!(parts[2].len(), 8, "hash segment is 8 hex chars");
    assert!(parts[2].chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn the_same_statement_produces_the_same_id() {
    let a = evidence_id(DOC, Some(16), "Yes.", Some("Did you sign it?"));
    let b = evidence_id(DOC, Some(16), "Yes.", Some("Did you sign it?"));
    assert_eq!(a, b);
}

/// THE POINT OF THE WHOLE MODULE, as one assertion.
///
/// Re-extraction rephrases `summary`, `significance`, `weight` and
/// `pattern_tags`. None of them is in the key, so none of them can move the id.
/// This is expressed through the properties reader because that is the surface
/// the pipeline actually calls — a key built by hand in a test would not prove
/// the reader ignores the mooded fields.
#[test]
fn rephrasing_every_mooded_field_does_not_move_the_id() {
    let first = json!({
        "verbatim_quote": "Yes, Mr. Milster prepared pleadings.",
        "page_number": 4,
        "question": "Was Richard Milster ever involved?",
        "summary": "Phillips confirms Milster's involvement.",
        "significance": "Phillips admits Richard Milster prepared pleadings.",
        "weight": 8,
        "pattern_tags": "admission,fiduciary",
        "statement_type": "admission",
        "evidence_strength": "sworn_party_admission",
    });
    let second = json!({
        "verbatim_quote": "Yes, Mr. Milster prepared pleadings.",
        "page_number": 4,
        "question": "Was Richard Milster ever involved?",
        // Everything below is rephrased, re-scored, re-tagged, re-classified.
        "summary": "The witness acknowledges the attorney drafted filings.",
        "significance": "Confirms under oath that Milster acted in the estate.",
        "weight": 6,
        "pattern_tags": "concession",
        "statement_type": "partial_admission",
        "evidence_strength": "sworn_party_evasion",
    });

    assert_eq!(
        evidence_id_from_properties(&first, DOC),
        evidence_id_from_properties(&second, DOC),
        "a mooded field moved the id — the arm is compromised",
    );
}

#[test]
fn a_different_quote_produces_a_different_id() {
    let a = evidence_id(DOC, Some(16), "Yes.", Some("Did you sign it?"));
    let b = evidence_id(DOC, Some(16), "No.", Some("Did you sign it?"));
    assert_ne!(a, b);
}

/// The measured reason `question` is in the key: three "Yes." answers on one page
/// of one document are three distinct sworn admissions. Without the question they
/// would be one id and two of them would vanish at MERGE.
#[test]
fn the_same_answer_to_different_questions_stays_distinct() {
    let ids = [
        evidence_id(DOC, Some(16), "Yes.", Some("Did you receive the letter?")),
        evidence_id(DOC, Some(16), "Yes.", Some("Was the auction held?")),
        evidence_id(DOC, Some(16), "Yes.", Some("Did you sign the accounting?")),
    ];
    let mut unique = ids.to_vec();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 3, "three admissions collapsed to {ids:?}");
}

#[test]
fn the_same_quote_on_a_different_page_or_document_is_a_different_statement() {
    let base = evidence_id(DOC, Some(16), "Yes.", Some("Did you sign it?"));
    assert_ne!(
        base,
        evidence_id(DOC, Some(17), "Yes.", Some("Did you sign it?"))
    );
    assert_ne!(
        base,
        evidence_id(
            "doc-cfs-interrogatory-response-08-08-16",
            Some(16),
            "Yes.",
            Some("Did you sign it?")
        ),
    );
}

/// OCR re-runs move line breaks around. That is layout, not content, and it must
/// not change what a statement IS.
#[test]
fn whitespace_and_line_breaks_do_not_move_the_id() {
    let tidy = evidence_id(
        DOC,
        Some(9),
        "Correspondence was received from Marie Awad.",
        None,
    );
    let ragged = evidence_id(
        DOC,
        Some(9),
        "  Correspondence was\n\nreceived   from\tMarie Awad.  ",
        None,
    );
    assert_eq!(tidy, ragged);
}

/// The NFC half of the normalization, which the Phase A Cypher measurement could
/// not apply and this arm must.
///
/// "Awad é" decomposed (e + U+0301 combining acute) and composed (U+00E9) are the
/// same text to a reader and different bytes to a hash. An extraction run that
/// emits one form and a later one that emits the other would otherwise break
/// every curated row pointing at that statement.
#[test]
fn decomposed_and_composed_accents_produce_one_id() {
    let composed = "Le d\u{e9}p\u{f4}t was never opened.";
    let decomposed = "Le de\u{301}po\u{302}t was never opened.";
    assert_ne!(composed, decomposed, "the two forms differ as bytes");
    assert_eq!(
        evidence_id(DOC, Some(3), composed, None),
        evidence_id(DOC, Some(3), decomposed, None),
        "NFC is not being applied",
    );
}

#[test]
fn normalize_composes_trims_and_collapses() {
    assert_eq!(normalize("  a \n b\t\tc  "), "a b c");
    assert_eq!(normalize("e\u{301}"), "\u{e9}");
    assert_eq!(normalize(""), "");
    assert_eq!(normalize("   \n\t "), "");
}

/// A documentary statement answers nobody. Absent and empty `question` mean the
/// same thing here — the one place this module treats them alike, and it is
/// stated in the doc comment because everywhere else in this codebase they must
/// stay distinguishable.
#[test]
fn an_absent_and_an_empty_question_agree() {
    let absent = evidence_id(DOC, Some(3), "The court so found.", None);
    let empty = evidence_id(DOC, Some(3), "The court so found.", Some(""));
    let blank = evidence_id(DOC, Some(3), "The court so found.", Some("   "));
    assert_eq!(absent, empty);
    assert_eq!(absent, blank);
}

/// A quoteless item gets NO id from this arm — the caller falls back and logs.
///
/// Giving it a key derived from an empty string would MERGE every quoteless item
/// in a document onto ONE node. That exact failure is on the record in the
/// allegation arm's `hash-e3b0c442` comment, and it must not be repeated here.
#[test]
fn an_item_with_no_usable_quote_is_refused_rather_than_keyed() {
    for props in [
        json!({"page_number": 3}),
        json!({"verbatim_quote": null, "page_number": 3}),
        json!({"verbatim_quote": "", "page_number": 3}),
        json!({"verbatim_quote": "   \n ", "page_number": 3}),
    ] {
        assert_eq!(
            evidence_id_from_properties(&props, DOC),
            None,
            "a quoteless item must not receive a key: {props}",
        );
    }
}

/// `page_number` has been seen as both an integer and a string on the wire.
/// Accepting only one would quietly key half a document as "no page".
#[test]
fn a_string_page_number_reads_the_same_as_an_integer_one() {
    let as_int = json!({"verbatim_quote": "Yes.", "page_number": 16});
    let as_str = json!({"verbatim_quote": "Yes.", "page_number": "16"});
    assert_eq!(
        evidence_id_from_properties(&as_int, DOC),
        evidence_id_from_properties(&as_str, DOC),
    );
}

/// A missing page contributes an empty component rather than being dropped, so
/// the components cannot slide into each other's positions.
#[test]
fn a_missing_page_cannot_be_confused_with_a_shifted_component() {
    let no_page = evidence_id(DOC, None, "16", None);
    let page_16_no_quote_text = evidence_id(DOC, Some(16), "x", None);
    assert_ne!(no_page, page_16_no_quote_text);
}

/// The separator is a control character precisely so a quote cannot forge a
/// component boundary. A printable one (`|`, `:`) could be typed into a document.
#[test]
fn a_quote_cannot_forge_a_component_boundary() {
    // If the separator were '|', these two would hash identical material.
    let a = evidence_id(DOC, Some(1), "alpha|beta", None);
    let b = evidence_id(DOC, Some(1), "alpha", Some("beta"));
    assert_ne!(
        a, b,
        "a printable separator would let a quote forge a boundary"
    );
}
