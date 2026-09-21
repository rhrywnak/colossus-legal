//! Tests for the wire mapping.

use chrono::TimeZone;
use serde_json::json;

use super::*;
use crate::repositories::pipeline_repository::chat_discussions::CorpusDocument;
use crate::services::chat_question_text::package_documents;

fn card(block_index: usize, quote: &str) -> StoredCard {
    StoredCard {
        block_index,
        document_id: "d".into(),
        document_title: "LETTER".into(),
        document_date: Some("November 5, 2009".into()),
        page: Some(1),
        start_char: 0,
        end_char: quote.chars().count(),
        quoted_text: quote.into(),
        relocated: false,
    }
}

#[test]
fn a_card_ends_its_segment_and_prose_after_it_starts_a_new_one() {
    let content = json!([
        {"type": "thinking", "thinking": "", "signature": "s"},
        {"type": "text", "text": "Your record puts it in "},
        {"type": "text", "text": "November", "citations": [{}]},
        {"type": "text", "text": ". If you say December…"}
    ]);
    let segs = segments(&content, &[card(2, "The letter is dated November.")]);
    assert_eq!(segs.len(), 2);
    assert_eq!(segs[0].text, "Your record puts it in November");
    assert_eq!(
        segs[0].cards[0].quoted_text,
        "The letter is dated November."
    );
    assert_eq!(segs[1].text, ". If you say December…");
    assert!(segs[1].cards.is_empty());
}

#[test]
fn cards_come_from_the_stored_document_with_its_page() {
    let docs = package_documents(
        &[CorpusDocument {
            id: "doc-l".into(),
            title: "LETTER".into(),
            document_date: chrono::NaiveDate::from_ymd_opt(2009, 11, 5),
            pages: vec![(1, "Page one.".into()), (2, "Page two.".into())],
        }],
        None,
    );
    let verified = vec![(
        3,
        colossus_chat::VerifiedCitation {
            document_index: 0,
            start_char: 11,
            end_char: 20,
            quoted_text: "Page two.".into(),
            relocated: false,
        },
    )];
    let cards = cards_for(&verified, &docs);
    assert_eq!(cards[0].page, Some(2));
    assert_eq!(cards[0].document_date.as_deref(), Some("November 5, 2009"));
    assert_eq!(cards[0].block_index, 3);
}

fn record(role: &str, rendered: Option<&str>, failure: Option<&str>) -> MessageRecord {
    MessageRecord {
        seq: 1,
        role: role.into(),
        content: json!([{"type": "text", "text": rendered.unwrap_or("")}]),
        rendered_text: rendered.map(str::to_string),
        citations: None,
        model: None,
        stop_reason: None,
        failure: failure.map(str::to_string),
        created_at: chrono::Utc.with_ymd_and_hms(2026, 9, 19, 12, 0, 0).unwrap(),
    }
}

#[test]
fn tool_turns_are_hidden_but_named_failures_are_shown() {
    assert!(message_dto(&record("user", None, None), "Marie", "UTC").is_none());
    let failed = message_dto(&record("assistant", None, Some("stalled")), "The AI", "UTC").unwrap();
    assert_eq!(failed.failure.as_deref(), Some("stalled"));
    assert!(failed.segments.is_empty());
    let user = message_dto(&record("user", Some("why?"), None), "Marie", "UTC").unwrap();
    assert_eq!(user.segments[0].text, "why?");
}

#[test]
fn short_day_uses_the_case_timezone() {
    let at = chrono::Utc.with_ymd_and_hms(2026, 9, 20, 2, 0, 0).unwrap();
    assert_eq!(short_day(at, "America/Detroit"), "Sep 19");
}
