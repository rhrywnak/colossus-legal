//! Stored rows → the wire (pure). Where cards meet the words they back.

use chrono::{DateTime, Utc};
use colossus_chat::VerifiedCitation;
use serde_json::Value;

use crate::dto::chat_discussion::{CitationCardDto, MessageDto, SegmentDto, StoredCard};
use crate::repositories::pipeline_repository::chat_discussions::MessageRecord;
use crate::repositories::pipeline_repository::practice_discussions::DiscussionTurnRecord;
use crate::services::chat_question_text::PackagedDocument;
use crate::services::practice_clock::local_stamp;

/// STRUCTURAL: `Sep 19` — how the switcher and header spell a day.
const SHORT_DAY_FORMAT: &str = "%b %-d";

/// `Sep 19`, in the case's timezone.
pub fn short_day(at: DateTime<Utc>, timezone: &str) -> String {
    // The zone is the case's settings row; a name chrono_tz does not know is
    // logged and read as UTC, the same fallback `practice_clock` makes.
    let tz: chrono_tz::Tz = timezone.parse().unwrap_or_else(|e| {
        tracing::warn!(timezone, error = %e, "question chat: unknown case timezone, days shown in UTC");
        chrono_tz::UTC
    });
    at.with_timezone(&tz).format(SHORT_DAY_FORMAT).to_string()
}

/// Turn one assistant turn's verified citations into the cards stored with it.
pub fn cards_for(
    verified: &[(usize, VerifiedCitation)],
    documents: &[PackagedDocument],
) -> Vec<StoredCard> {
    verified
        .iter()
        .filter_map(|(block_index, v)| {
            let doc = documents.get(v.document_index)?;
            Some(StoredCard {
                block_index: *block_index,
                document_id: doc.id.clone(),
                document_title: doc.title.clone(),
                document_date: crate::services::chat_question_text::document_date_label(doc.date),
                page: doc.page_at(v.start_char),
                start_char: v.start_char,
                end_char: v.end_char,
                quoted_text: v.quoted_text.clone(),
                relocated: v.relocated,
            })
        })
        .collect()
}

/// Split a message's text blocks into segments: a segment ends after every block
/// that carries a card, so each card renders under the words it backs.
pub fn segments(content: &Value, cards: &[StoredCard]) -> Vec<SegmentDto> {
    let mut out: Vec<SegmentDto> = Vec::new();
    let mut text = String::new();
    let blocks = content.as_array().cloned().unwrap_or_default();
    for (i, block) in blocks.iter().enumerate() {
        if block.get("type").and_then(Value::as_str) != Some("text") {
            continue;
        }
        text.push_str(
            block
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        );
        let mine: Vec<CitationCardDto> = cards
            .iter()
            .filter(|c| c.block_index == i)
            .map(|c| CitationCardDto {
                document_title: c.document_title.clone(),
                document_date: c.document_date.clone(),
                page: c.page,
                quoted_text: c.quoted_text.clone(),
            })
            .collect();
        if !mine.is_empty() {
            out.push(SegmentDto {
                text: std::mem::take(&mut text),
                cards: mine,
            });
        }
    }
    if !text.trim().is_empty() || out.is_empty() {
        out.push(SegmentDto {
            text,
            cards: Vec::new(),
        });
    }
    out
}

/// A stored message as the thread shows it. `None` for tool-result turns and
/// empty turns, which a person has nothing to read in — unless the turn is a
/// named failure, which is always shown.
pub fn message_dto(m: &MessageRecord, author_name: &str, timezone: &str) -> Option<MessageDto> {
    if m.rendered_text.is_none() && m.failure.is_none() {
        return None;
    }
    let cards: Vec<StoredCard> = m
        .citations
        .clone()
        .and_then(|c| serde_json::from_value(c).ok())
        .unwrap_or_default();
    let segments = match &m.rendered_text {
        Some(_) if m.role == "assistant" => segments(&m.content, &cards),
        Some(text) => vec![SegmentDto {
            text: text.clone(),
            cards: Vec::new(),
        }],
        None => Vec::new(),
    };
    Some(MessageDto {
        seq: m.seq,
        role: m.role.clone(),
        author_name: author_name.to_string(),
        segments,
        at: local_stamp(m.created_at, timezone),
        failure: m.failure.clone(),
    })
}

/// An old-dock turn (ADDENDUM_1) in the same shape — no cards, author as stored.
pub fn earlier_dto(t: &DiscussionTurnRecord, index: usize, timezone: &str) -> MessageDto {
    MessageDto {
        seq: i32::try_from(index + 1).unwrap_or(i32::MAX),
        role: if t.role == "model" {
            "assistant"
        } else {
            "user"
        }
        .to_string(),
        author_name: t.author_name.clone(),
        segments: vec![SegmentDto {
            text: t.text.clone(),
            cards: Vec::new(),
        }],
        at: local_stamp(t.created_at, timezone),
        failure: None,
    }
}

#[cfg(test)]
#[path = "chat_question_view_tests.rs"]
mod tests;
