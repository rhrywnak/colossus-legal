// Tests for `api::scenario_card_edit`.
//
// The handler needs a live `AppState`, so what is asserted here is the BOUNDARY:
// which bodies parse, which are refused, and what each field becomes in its
// column. Those are the parts a wrong client hits first — and the column value is
// the part a human never sees until it is wrong.

use super::*;

fn parse(json: serde_json::Value) -> Result<CardFieldUpdate, serde_json::Error> {
    serde_json::from_value(json)
}

/// Each of the five fields decodes from its own documented shape.
#[test]
fn every_editable_field_decodes_from_its_own_shape() {
    let cases = [
        (
            serde_json::json!({"field": "title", "value": "The court ordered it back"}),
            CardField::Title,
        ),
        (
            serde_json::json!({"field": "backs_position", "value": 2}),
            CardField::BacksPosition,
        ),
        (
            serde_json::json!({"field": "supports", "value": []}),
            CardField::Supports,
        ),
        (
            serde_json::json!({"field": "watch_out", "value": "They will say…"}),
            CardField::WatchOut,
        ),
        (
            serde_json::json!({"field": "answer", "value": "The money was Dad's."}),
            CardField::Answer,
        ),
    ];
    for (body, expected) in cases {
        let update = parse(body).expect("the documented shape decodes");
        assert_eq!(update.field(), expected);
    }
    assert_eq!(
        cases_len(),
        CardField::EDITABLE.len(),
        "a sixth editable field needs a case above"
    );
}

fn cases_len() -> usize {
    5
}

/// THE LEDGER-ONLY TOKEN IS UNSENDABLE.
///
/// There is no `card` variant, so a client cannot replace five fields in one
/// write and leave the record saying only that something happened.
#[test]
fn the_whole_card_token_cannot_be_sent() {
    assert!(parse(serde_json::json!({"field": "card", "value": "x"})).is_err());
}

/// An unknown field token is refused at the SERDE boundary, before the handler
/// runs — so no code path exists in which one reaches a column name.
#[test]
fn an_unknown_field_is_refused_at_the_boundary() {
    let err = parse(serde_json::json!({"field": "point", "value": "x"}))
        .expect_err("'point' is not a field");
    assert!(err.to_string().contains("point"), "{err}");
}

/// A camelCase typo is refused rather than dropped.
#[test]
fn a_camel_case_key_is_refused() {
    assert!(parse(serde_json::json!({"field": "answer", "Value": "x"})).is_err());
}

/// A stance outside the vocabulary is refused at the boundary.
#[test]
fn an_unknown_stance_is_refused_at_the_boundary() {
    assert!(parse(serde_json::json!({
        "field": "supports",
        "value": [{"allegation_id": "a-1", "stance": "mentions"}]
    }))
    .is_err());
}

// ─── What reaches the column ─────────────────────────────────────────────────

/// Text fields are trimmed, and a blank one is stored as NULL.
///
/// A human clearing a field with the keyboard and one pressing a Clear control
/// are the same act, and the card renders both as the em dash. Storing `""` would
/// put a value in the row that no reader can see.
#[test]
fn a_blank_text_field_is_stored_as_null() {
    for blank in ["", "   ", "\n\t"] {
        let update = parse(serde_json::json!({"field": "answer", "value": blank})).expect("parses");
        assert_eq!(update.stored_value().expect("valid"), None, "{blank:?}");
    }
    let update =
        parse(serde_json::json!({"field": "answer", "value": "  said it  "})).expect("parses");
    assert_eq!(
        update.stored_value().expect("valid").as_deref(),
        Some("said it")
    );
}

/// An explicit `null` clears the field.
#[test]
fn an_explicit_null_clears_a_field() {
    let update = parse(serde_json::json!({"field": "watch_out", "value": null})).expect("parses");
    assert_eq!(update.stored_value().expect("valid"), None);
}

/// The position is rendered as its own digits, not as JSON.
#[test]
fn a_position_is_stored_as_its_digits() {
    let update = parse(serde_json::json!({"field": "backs_position", "value": 3})).expect("parses");
    assert_eq!(update.stored_value().expect("valid").as_deref(), Some("3"));
}

/// The accusation list is stored as JSON the reader can parse back.
///
/// Round-tripped through the READING type, not merely compared as a string: the
/// write path and the read path have to agree, and a shape that serialized
/// prettily but did not parse would only fail on the next page load.
#[test]
fn the_accusation_list_is_stored_as_json_the_reader_can_parse() {
    use crate::repositories::pipeline_repository::scenario_fact_cards::CardSupport;

    let update = parse(serde_json::json!({
        "field": "supports",
        "value": [
            {"allegation_id": "doc-x:allegation:45984d77", "stance": "supports"},
            {"allegation_id": "doc-x:allegation:08c0731b", "stance": "rebuts"}
        ]
    }))
    .expect("parses");

    let stored = update.stored_value().expect("valid").expect("a value");
    let read: Vec<CardSupport> = serde_json::from_str(&stored).expect("the reader parses it back");
    assert_eq!(read.len(), 2);
    assert_eq!(read[0].allegation_id, "doc-x:allegation:45984d77");
    assert_eq!(read[1].stance, CardStance::Rebuts);
}

/// An EMPTY accusation list clears the column rather than storing `[]`.
///
/// `[]` and NULL both render as the em dash, so storing the first would put a
/// value in the row that reads as nothing — and `supports_list` would then have
/// to treat two shapes as one state.
#[test]
fn an_empty_accusation_list_clears_the_column() {
    let update = parse(serde_json::json!({"field": "supports", "value": []})).expect("parses");
    assert_eq!(update.stored_value().expect("valid"), None);
}

/// A THIRD accusation is REFUSED, not truncated.
///
/// Truncating would store less than the human asked for and say nothing. The
/// renderer truncates on read as a backstop against a row edited around the API;
/// here there is somebody to tell.
#[test]
fn a_third_accusation_is_refused_and_names_the_cap() {
    let update = parse(serde_json::json!({
        "field": "supports",
        "value": [
            {"allegation_id": "a-1", "stance": "supports"},
            {"allegation_id": "a-2", "stance": "supports"},
            {"allegation_id": "a-3", "stance": "supports"}
        ]
    }))
    .expect("parses");
    let err = update.stored_value().expect_err("three exceeds the cap");
    let rendered = format!("{err:?}");
    assert!(rendered.contains("supports"), "{rendered}");
    assert!(rendered.contains('2'), "the cap is named: {rendered}");
}

/// A blank accusation id is refused.
///
/// A row keyed on `""` can never be matched by a read, so the card would claim a
/// link nothing answers — and the Include prefilled from it would write a link
/// row nobody can find.
#[test]
fn a_blank_accusation_id_is_refused() {
    let update = parse(serde_json::json!({
        "field": "supports",
        "value": [{"allegation_id": "   ", "stance": "supports"}]
    }))
    .expect("parses");
    assert!(update.stored_value().is_err());
}

/// Exactly two accusations are accepted — the cap is a maximum, not a refusal of
/// the second.
#[test]
fn two_accusations_are_accepted() {
    let update = parse(serde_json::json!({
        "field": "supports",
        "value": [
            {"allegation_id": "a-1", "stance": "supports"},
            {"allegation_id": "a-2", "stance": "rebuts"}
        ]
    }))
    .expect("parses");
    assert!(update
        .stored_value()
        .expect("two is within the cap")
        .is_some());
}
