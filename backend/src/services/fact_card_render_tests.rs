// Tests for `services::fact_card_render`.
//
// Everything here is a literal in, finished trial language out. The assertions
// are weighted towards what a witness would be told WRONGLY: a stance rendered
// as its opposite, a stale point rendered as half a sentence, a draft mark
// cleared by an edit to a different field.

use super::render_fixtures::*;
use super::*;

// ─── Nothing is hidden ───────────────────────────────────────────────────────

/// THE RULE THIS FILE EXISTS FOR. An empty card renders every slot as absent —
/// and the block is still produced.
///
/// §2: nothing is hidden for lacking a field. A card with no Answer is work
/// somebody still owes, and the row is what says so.
#[test]
fn an_empty_card_renders_every_field_as_absent_and_is_still_a_card() {
    let block = render(&bare_record());
    assert_eq!(block.title, None);
    assert_eq!(block.backs, None);
    assert_eq!(block.answer, None);
    assert_eq!(block.watch_out, None);
    assert!(block.supports.is_empty());
    assert!(block.count_tags.is_empty());
    assert!(!block.drafts.any(), "nobody has written anything to draft");
}

/// A blank string is the same as absent — a human clearing a field leaves the
/// row reading as an em dash, not as an empty quotation.
#[test]
fn a_blank_field_renders_as_absent() {
    let mut record = bare_record();
    record.title = Some("   ".to_string());
    record.answer = Some(String::new());
    let block = render(&record);
    assert_eq!(block.title, None);
    assert_eq!(block.answer, None);
}

/// Values are trimmed, so a stray newline from an editor does not reach a card.
#[test]
fn values_are_trimmed() {
    let mut record = bare_record();
    record.title = Some("  The court ordered it back.\n".to_string());
    assert_eq!(
        render(&record).title.as_deref(),
        Some("The court ordered it back.")
    );
}

// ─── The Supports line ───────────────────────────────────────────────────────

/// A supporting card names the accusation with the supporting verb.
#[test]
fn a_supporting_card_reads_with_the_supporting_verb() {
    let mut record = bare_record();
    record.supports = Some(supports_value(&[("a-21", CardStance::Supports)]));
    let block = render(&record);
    assert_eq!(
        block.supports,
        vec!["Supports A-21 — CFS could have returned the money."]
    );
    assert_eq!(block.supports_refs[0].code, "A-21");
    assert_eq!(block.supports_refs[0].stance, CardStance::Supports);
}

/// A DISPUTING card reads with the disputing verb — never a negation of the
/// other.
///
/// The two words are opposite claims about the same accusation, and a build that
/// rendered both as "Supports" would tell a witness that a statement destroying
/// the accusation helps it.
#[test]
fn a_disputing_card_reads_with_the_disputing_verb() {
    let mut record = bare_record();
    record.supports = Some(supports_value(&[("a-21", CardStance::Rebuts)]));
    let block = render(&record);
    assert_eq!(
        block.supports,
        vec!["Disputes A-21 — CFS could have returned the money."]
    );
}

/// AT MOST TWO accusations reach the card, whatever the column holds.
///
/// The loader and the edit route both refuse a third; this is the read-side
/// backstop, and the one that matters if either is bypassed. Five lines in a slot
/// laid out for one is a broken card rather than a refused write.
#[test]
fn a_card_renders_at_most_two_accusations() {
    let mut record = bare_record();
    record.supports = Some(supports_value(&[
        ("a-21", CardStance::Supports),
        ("a-44", CardStance::Supports),
        ("a-61", CardStance::Rebuts),
    ]));
    let block = render(&record);
    assert_eq!(block.supports.len(), 2);
    assert_eq!(block.supports_refs.len(), 2);
    assert!(block.supports[0].contains("A-21"));
    assert!(block.supports[1].contains("A-44"));
}

/// An accusation this build cannot find still renders its verb and code slot —
/// with an EMPTY text — rather than dropping the line.
///
/// A stale pointer is the 2026-07-24 defect class. Dropping the row would hide
/// that the card claims a link nothing answers; rendering it shows a reader
/// something is wrong.
#[test]
fn an_unknown_accusation_still_renders_a_line() {
    let mut record = bare_record();
    record.supports = Some(supports_value(&[("a-gone", CardStance::Supports)]));
    let block = render(&record);
    assert_eq!(block.supports.len(), 1);
    assert_eq!(
        block.supports_refs[0].code, "",
        "half a citation is worse than none"
    );
}

/// A malformed supports column costs the card its accusations and NOTHING else.
///
/// The title, the answer and the watch-out are still the witness's words, and
/// withholding four good sentences over one bad list would be the wrong trade.
#[test]
fn a_malformed_supports_column_leaves_the_rest_of_the_card_intact() {
    let mut record = bare_record();
    record.title = Some("The court ordered it back.".to_string());
    record.answer = Some("The money was Dad's.".to_string());
    record.supports = Some(serde_json::json!([{"allegation_id": "a-21"}]));
    let block = render(&record);
    assert!(block.supports.is_empty());
    assert_eq!(block.title.as_deref(), Some("The court ordered it back."));
    assert_eq!(block.answer.as_deref(), Some("The money was Dad's."));
}
