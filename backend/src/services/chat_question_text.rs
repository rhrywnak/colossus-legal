//! The question chat's context package, rendered — pure functions, no I/O.
//!
//! [`crate::services::chat_question_gather`] collects the rows; this module turns
//! them into what the model reads: the document blocks (every stored document,
//! the question's own source first) and one text block describing the question,
//! her answers and their reads, the team's notes, the other threads on this
//! question, and the earlier shared discussion.
//!
//! ## Domain note: no internal key reaches the model
//!
//! The one-shot read speaks in keys — `P3`, `R1`, `S2` — and its stored reads
//! still carry them (the 2026-09-21 audit). Roman's ruling that day: internal
//! labels never reach a user's screen. The model echoes what it is shown, so the
//! keys are translated HERE, before it ever sees them ([`translate_keys`]), and a
//! test scans the rendered package for any survivor.

use chrono::NaiveDate;
use colossus_chat::ChatDocument;

use crate::repositories::pipeline_repository::chat_discussions::CorpusDocument;

/// A document's display date, e.g. `November 5, 2009`.
///
/// STRUCTURAL: a chrono format string — how a date is spelled, not a setting.
const DOCUMENT_DATE_FORMAT: &str = "%B %-d, %Y";

/// Between pages in a document block. Two newlines: a paragraph break the
/// sentence chunker respects, with no marker text a citation could swallow.
///
/// STRUCTURAL: a separator inside the text, not wording.
const PAGE_JOIN: &str = "\n\n";

/// A document block and the page each character range came from.
#[derive(Debug, Clone, PartialEq)]
pub struct PackagedDocument {
    pub id: String,
    pub title: String,
    pub date: Option<NaiveDate>,
    pub block: ChatDocument,
    /// `(first char of the page, page number)`, ascending.
    pub page_starts: Vec<(usize, i32)>,
}

impl PackagedDocument {
    /// The page a character position falls on.
    pub fn page_at(&self, char_pos: usize) -> Option<i32> {
        self.page_starts
            .iter()
            .rev()
            .find(|(start, _)| *start <= char_pos)
            .map(|(_, page)| *page)
    }
}

/// `November 5, 2009`, or `None` for an undated document.
pub fn document_date_label(date: Option<NaiveDate>) -> Option<String> {
    date.map(|d| d.format(DOCUMENT_DATE_FORMAT).to_string())
}

/// Every stored document as a citation-enabled block, `primary` first.
///
/// `context` (read by the model, never citable) carries the date, the page count,
/// the id the `get_document` tool takes, and — for the question's own source —
/// that it is the document this question was built from.
pub fn package_documents(
    corpus: &[CorpusDocument],
    primary: Option<&str>,
) -> Vec<PackagedDocument> {
    let mut ordered: Vec<&CorpusDocument> = corpus.iter().collect();
    // Stable sort: the primary document moves to the front, the rest keep order.
    ordered.sort_by_key(|d| Some(d.id.as_str()) != primary);
    ordered
        .into_iter()
        .map(|d| {
            let mut text = String::new();
            let mut page_starts = Vec::with_capacity(d.pages.len());
            for (i, (page, page_text)) in d.pages.iter().enumerate() {
                if i > 0 {
                    text.push_str(PAGE_JOIN);
                }
                page_starts.push((text.chars().count(), *page));
                text.push_str(page_text);
            }
            let date = document_date_label(d.document_date)
                .map_or_else(|| "Date not recorded".to_string(), |s| format!("Dated {s}"));
            let mut context = format!("{date}. {} pages. Document id: {}.", d.pages.len(), d.id);
            if Some(d.id.as_str()) == primary {
                context.push_str(" This question was built from this document.");
            }
            PackagedDocument {
                id: d.id.clone(),
                title: d.title.clone(),
                date: d.document_date,
                block: ChatDocument {
                    title: d.title.clone(),
                    context: Some(context),
                    text,
                },
                page_starts,
            }
        })
        .collect()
}

/// Replace the read's internal keys with the words they stand for.
///
/// `P3` → `talking point 3`, `R2` → `the receipt for talking point 2`,
/// `S1` → `what they said`, `S2` → `what they admitted under oath`. A key is a
/// whole token: `P3` inside `EP3` or `P3x` is left alone.
pub fn translate_keys(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if let Some((words, len)) = key_at(&chars, i) {
            out.push_str(&words);
            i += len;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// A key starting at `i` — its translation and length — if one is there.
fn key_at(chars: &[char], i: usize) -> Option<(String, usize)> {
    let letter = *chars.get(i)?;
    if !matches!(letter, 'P' | 'R' | 'S') {
        return None;
    }
    if i > 0 && chars[i - 1].is_alphanumeric() {
        return None;
    }
    let digits: String = chars[i + 1..]
        .iter()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    let end = i + 1 + digits.len();
    if chars.get(end).is_some_and(|c| c.is_alphanumeric()) {
        return None;
    }
    let words = match (letter, digits.as_str()) {
        ('P', n) => format!("talking point {n}"),
        ('R', n) => format!("the receipt for talking point {n}"),
        ('S', "1") => "what they said".to_string(),
        ('S', "2") => "what they admitted under oath".to_string(),
        _ => return None,
    };
    Some((words, end - i))
}

/// Whether any internal key survives in `text` — the test the package must pass.
pub fn has_internal_key(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    (0..chars.len()).any(|i| key_at(&chars, i).is_some())
}

/// A rough token count for the size guard, at the configured
/// `question_chat_chars_per_token` (a zero is read as 1 — never a division by zero).
pub fn estimate_tokens(parts: &[&str], chars_per_token: usize) -> usize {
    parts.iter().map(|p| p.chars().count()).sum::<usize>() / chars_per_token.max(1)
}

#[cfg(test)]
#[path = "chat_question_text_tests.rs"]
mod tests;
