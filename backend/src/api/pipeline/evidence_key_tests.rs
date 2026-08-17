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

/// Build `item_data` in the shape the pipeline ACTUALLY writes: the quote is a
/// top-level sibling of `properties`, never inside it.
///
/// Every fixture below goes through this helper, so a test cannot accidentally
/// assert against a shape the pipeline does not produce. That is precisely how
/// the inert arm passed its own suite for eleven days — see the module header.
fn wire_item_data(quote: &str, properties: serde_json::Value) -> serde_json::Value {
    json!({
        "id": "evidence-fixture",
        "label": "fixture",
        "entity_type": "Evidence",
        "properties": properties,
        "verbatim_quote": quote,
    })
}

/// THE POINT OF THE WHOLE MODULE, as one assertion.
///
/// Re-extraction rephrases `summary`, `significance`, `weight` and
/// `pattern_tags`. None of them is in the key, so none of them can move the id.
/// This goes through the item reader, not a hand-built key, because the reader
/// is the surface the pipeline calls — and it is the reader, not the hash, that
/// was wrong last time.
#[test]
fn rephrasing_every_mooded_field_does_not_move_the_id() {
    const QUOTE: &str = "Yes, Mr. Milster prepared pleadings.";
    let first = wire_item_data(
        QUOTE,
        json!({
            "page_number": 4,
            "question": "Was Richard Milster ever involved?",
            "summary": "Phillips confirms Milster's involvement.",
            "significance": "Phillips admits Richard Milster prepared pleadings.",
            "weight": 8,
            "pattern_tags": "admission,fiduciary",
            "statement_type": "admission",
            "evidence_strength": "sworn_party_admission",
        }),
    );
    let second = wire_item_data(
        QUOTE,
        json!({
            "page_number": 4,
            "question": "Was Richard Milster ever involved?",
            // Everything below is rephrased, re-scored, re-tagged, re-classified.
            "summary": "The witness acknowledges the attorney drafted filings.",
            "significance": "Confirms under oath that Milster acted in the estate.",
            "weight": 6,
            "pattern_tags": "concession",
            "statement_type": "partial_admission",
            "evidence_strength": "sworn_party_evasion",
        }),
    );

    assert_eq!(
        evidence_id_from_item(DOC, Some(QUOTE), Some(4), &first),
        evidence_id_from_item(DOC, Some(QUOTE), Some(4), &second),
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
    // Every combination of "no words anywhere": absent column, absent top-level,
    // and the blank forms of each. Measured on DEV, exactly 1 of 574 live
    // Evidence items is in this state — it is rare, not impossible.
    for (column, item_data) in [
        (None, json!({"properties": {"page_number": 3}})),
        (Some(""), json!({"properties": {"page_number": 3}})),
        (Some("   \n "), json!({"properties": {"page_number": 3}})),
        (None, json!({"verbatim_quote": null, "properties": {}})),
        (None, json!({"verbatim_quote": "", "properties": {}})),
        (None, json!({"verbatim_quote": "  \t ", "properties": {}})),
        // ...including whitespace in the LAST-resort source, which is the one a
        // future template could start using.
        (None, json!({"properties": {"verbatim_quote": "   "}})),
    ] {
        assert_eq!(
            evidence_id_from_item(DOC, column, Some(3), &item_data),
            None,
            "a quoteless item must not receive a key: {column:?} / {item_data}",
        );
    }
}

/// THE 132-ROW INVARIANT. The key uses the page the VERIFIER grounded the quote
/// to, never the page the model claimed in `properties.page_number`.
///
/// Measured on DEV 2026-08-17: the two disagree on 132 of 574 live Evidence
/// items. `create_entity_node` writes `grounded_page` onto the node, so the
/// graph — and `rekey_evidence`, which hashed the graph — is keyed on the
/// grounded page. An arm that read the claimed page would produce ids that
/// silently fail to match the graph for those 132 rows, which is a re-run of the
/// exact class of defect this module was built to end.
#[test]
fn the_claimed_page_is_ignored_and_the_grounded_page_decides_the_id() {
    let item_data = wire_item_data("Yes.", json!({"page_number": 99}));

    // Claimed page 99, grounded page 16 → the id is the page-16 id.
    let (id, _) = evidence_id_from_item(DOC, Some("Yes."), Some(16), &item_data)
        .expect("a quoted item must receive a key");
    assert_eq!(
        id,
        evidence_id(DOC, Some(16), "Yes.", None),
        "the claimed page_number leaked into the key",
    );
    assert_ne!(
        id,
        evidence_id(DOC, Some(99), "Yes.", None),
        "the key followed the model's claim instead of the verifier",
    );

    // And an ungrounded item keys as "no page" rather than borrowing the claim —
    // which is what the node itself does, since `page_number` is only SET when
    // `grounded_page` is `Some`.
    let (ungrounded, _) = evidence_id_from_item(DOC, Some("Yes."), None, &item_data)
        .expect("a quoted item must receive a key");
    assert_eq!(ungrounded, evidence_id(DOC, None, "Yes.", None));
}

/// The quote is taken from the column first, then the top level, then
/// `properties` — and the source is reported so the caller can log the two that
/// are not normal.
#[test]
fn the_quote_source_order_is_column_then_top_level_then_properties() {
    // All three present and DIFFERENT, so the winner is unambiguous.
    let all_three = json!({
        "verbatim_quote": "top-level text",
        "properties": {"verbatim_quote": "properties text"},
    });
    assert_eq!(
        evidence_id_from_item(DOC, Some("column text"), Some(1), &all_three),
        Some((
            evidence_id(DOC, Some(1), "column text", None),
            QuoteSource::Column
        )),
    );

    // Column empty → top level, and the source says so.
    assert_eq!(
        evidence_id_from_item(DOC, None, Some(1), &all_three),
        Some((
            evidence_id(DOC, Some(1), "top-level text", None),
            QuoteSource::TopLevel
        )),
    );

    // Neither → properties, the shape the pre-fix arm assumed and 0 of 574 live
    // rows actually use. Still read, never silently.
    let properties_only = json!({"properties": {"verbatim_quote": "properties text"}});
    assert_eq!(
        evidence_id_from_item(DOC, None, Some(1), &properties_only),
        Some((
            evidence_id(DOC, Some(1), "properties text", None),
            QuoteSource::Properties
        )),
    );
}

/// The question is read from `properties` only, because that is the only place
/// the graph node gets it — `create_entity_node` copies schema properties onto
/// the node, so a top-level `question` would key on a value the node does not
/// carry. Measured: 238 of 574 live items carry it in `properties`, 0 at the top
/// level.
#[test]
fn the_question_is_read_from_properties_and_not_from_the_top_level() {
    let top_level_question = json!({
        "verbatim_quote": "Yes.",
        "question": "Did you sign it?",
        "properties": {},
    });
    let (id, _) = evidence_id_from_item(DOC, Some("Yes."), Some(16), &top_level_question)
        .expect("a quoted item must receive a key");
    assert_eq!(
        id,
        evidence_id(DOC, Some(16), "Yes.", None),
        "a top-level question leaked into the key",
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

// ── THE REGRESSION TEST (red on the pre-fix arm) ────────────────────────────

/// A VERBATIM copy of a live `extraction_items.item_data` row —
/// `doc-sabrina-morris-affidavit` item 8589, read read-only from DEV on
/// 2026-08-17. Nothing is redacted or reshaped; it is a public court affidavit,
/// and the point of copying it byte-for-byte is that shape drift between what
/// the pipeline writes and what the tests assume can never again pass unnoticed.
///
/// Note what is and is NOT here: `verbatim_quote` is a TOP-LEVEL sibling of
/// `properties`, and `properties` carries no quote at all. Measured corpus-wide
/// the same day: 573 of 574 Evidence items carry the quote at the top level,
/// **0 of 574** carry one inside `properties`.
const REAL_MORRIS_ITEM_DATA: &str = r#"{
    "id": "evidence-morris-higgs-visits",
    "label": "Morris: Milton Higgs visited Emil Awad with Nadia Awad at least four times",
    "properties": {
        "kind": "testimonial",
        "title": "Morris: Milton Higgs visited Emil Awad with Nadia Awad at least four times",
        "answer": "Milton Higgs visited Emil Awad with Nadia Awad at least four times",
        "weight": 6,
        "paragraph": "A-1",
        "page_number": 1,
        "pattern_tags": "coordination",
        "significance": "CORROBORATES: coordination between Nadia Awad and Milton Higgs in approaching Emil Awad",
        "statement_date": "2010-02-12",
        "statement_type": "factual_assertion",
        "evidence_strength": "sworn_testimony"
    },
    "entity_type": "Evidence",
    "verbatim_quote": "Milton Higgs visited Emil Awad with Nadia Awad at least four times."
}"#;

/// The live id that row carries in the graph, set by `rekey_evidence --apply`
/// on 2026-08-17 (and read back read-only from `~/morris_before.json`, the
/// snapshot taken before that morning's gate-test reprocess overwrote the
/// column). This is the number the fix has to reproduce.
const REAL_MORRIS_REKEYED_ID: &str = "doc-sabrina-morris-affidavit:evidence:5cda8c01";

fn real_morris_item() -> crate::repositories::pipeline_repository::ExtractionItemRecord {
    let item_data: serde_json::Value =
        serde_json::from_str(REAL_MORRIS_ITEM_DATA).expect("the pinned fixture must parse");
    crate::repositories::pipeline_repository::ExtractionItemRecord {
        id: 8589,
        run_id: 1,
        document_id: "doc-sabrina-morris-affidavit".to_string(),
        entity_type: "Evidence".to_string(),
        // The column, as the live row carries it. `store_entities_and_relationships`
        // copies the top-level quote here at insert; measured corpus-wide, the two
        // never disagree (0 of 574).
        verbatim_quote: Some(
            "Milton Higgs visited Emil Awad with Nadia Awad at least four times.".to_string(),
        ),
        grounding_status: Some("exact".to_string()),
        // The VERIFIER's page, which is what the graph node carries — not the
        // model's claimed `properties.page_number`. They disagree on 132 of 574
        // live Evidence items.
        grounded_page: Some(1),
        item_data,
        review_status: "approved".to_string(),
        reviewed_by: None,
        reviewed_at: None,
        review_notes: None,
        graph_status: "written".to_string(),
        neo4j_node_id: None,
        resolved_entity_type: None,
    }
}

/// THE REGRESSION TEST. Red on the pre-fix arm, by construction.
///
/// Pre-fix, `stable_entity_id` read the quote from `item_data["properties"]`,
/// found nothing, and returned the catch-all digest of the WHOLE `item_data`
/// blob. This test recomputes that exact fallback value inline and asserts the
/// arm did NOT produce it — so it fails on the old code with a diff an operator
/// can read, rather than failing to compile.
#[test]
fn the_evidence_arm_keys_a_real_pipeline_row_and_does_not_fall_back() {
    use crate::api::pipeline::ingest_helpers::{slug, stable_entity_id};
    use sha2::{Digest, Sha256};

    let item = real_morris_item();
    let doc = "doc-sabrina-morris-affidavit";
    let got = stable_entity_id(&item, doc);

    // What the PRE-FIX code path produced for this very row: the blob hash.
    let blob = serde_json::to_string(&item.item_data).unwrap_or_default();
    let pre_fix = format!(
        "{}:{}:{}",
        slug(doc),
        slug("Evidence"),
        &format!("{:x}", Sha256::digest(blob.as_bytes()))[..8]
    );
    assert_ne!(
        got, pre_fix,
        "the arm fell through to the whole-blob hash — it is still inert",
    );

    // And what it must produce: the key, from the quote and the GROUNDED page.
    assert_eq!(
        got,
        evidence_id(
            doc,
            Some(1),
            item.verbatim_quote.as_deref().unwrap_or(""),
            None
        ),
        "the arm did not compute evidence_id from the row's own quote and page",
    );

    // The live number, which is the only assertion that cannot be satisfied by a
    // self-consistent but wrong implementation.
    assert_eq!(
        got, REAL_MORRIS_REKEYED_ID,
        "the id does not match the live re-keyed id for this row",
    );
}

/// Build an Evidence row with the quote and page placed wherever the test needs
/// them, so the three arms of `stable_entity_id`'s Evidence branch can each be
/// driven end-to-end rather than only through `evidence_id_from_item`.
fn evidence_item(
    column_quote: Option<&str>,
    grounded_page: Option<i32>,
    item_data: serde_json::Value,
) -> crate::repositories::pipeline_repository::ExtractionItemRecord {
    let mut item = real_morris_item();
    item.verbatim_quote = column_quote.map(str::to_string);
    item.grounded_page = grounded_page;
    item.item_data = item_data;
    item
}

/// ARM 2, through `stable_entity_id` itself: the column is empty but `item_data`
/// carries the quote at the top level.
///
/// The id must still be the CONTENT key — the whole point is that this item is
/// keyed correctly and merely reported, not diverted to the blob hash. Driving it
/// through `stable_entity_id` (not `evidence_id_from_item`) is what proves the
/// arm returns the reader's id rather than falling through after logging.
#[test]
fn the_evidence_arm_keys_from_item_data_when_the_column_is_empty() {
    use crate::api::pipeline::ingest_helpers::{slug, stable_entity_id};
    use sha2::{Digest, Sha256};

    const DOC: &str = "doc-sabrina-morris-affidavit";
    const QUOTE: &str = "Milton Higgs visited Emil Awad with Nadia Awad at least four times.";
    let item = evidence_item(
        None,
        Some(1),
        wire_item_data(QUOTE, json!({"page_number": 1})),
    );

    let got = stable_entity_id(&item, DOC);
    assert_eq!(
        got,
        evidence_id(DOC, Some(1), QUOTE, None),
        "arm 2 must key on the recovered quote, not divert to the fallback",
    );

    let blob = serde_json::to_string(&item.item_data).unwrap_or_default();
    let fallback = format!(
        "{}:{}:{}",
        slug(DOC),
        slug("Evidence"),
        &format!("{:x}", Sha256::digest(blob.as_bytes()))[..8]
    );
    assert_ne!(got, fallback, "arm 2 fell through to the blob hash");
}

/// ARM 3, through `stable_entity_id` itself: no quote in the column, at the top
/// level, or in `properties`.
///
/// This is the one Evidence item on the live corpus (item 8722, the certified
/// letter) and it must still reach the blob hash — an empty-string key would
/// MERGE every quoteless item in a document onto one node.
#[test]
fn the_evidence_arm_still_falls_back_when_there_is_no_quote_anywhere() {
    use crate::api::pipeline::ingest_helpers::{slug, stable_entity_id};
    use sha2::{Digest, Sha256};

    const DOC: &str = "doc-certified-letter-to-george-phillips-11-05-2009";
    let item = evidence_item(
        None,
        Some(4),
        json!({"label": "no words", "properties": {"page_number": 4}}),
    );

    let blob = serde_json::to_string(&item.item_data).unwrap_or_default();
    let expected = format!(
        "{}:{}:{}",
        slug(DOC),
        slug("Evidence"),
        &format!("{:x}", Sha256::digest(blob.as_bytes()))[..8]
    );
    assert_eq!(
        stable_entity_id(&item, DOC),
        expected,
        "a quoteless Evidence item must still take the blob-hash fallback",
    );
}
