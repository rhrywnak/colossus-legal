//! Citation verification: a quotation is shown only if the STORED document says it.
//!
//! The Citations API promises that `cited_text` comes from the supplied document.
//! This module does not take that on trust. For every `char_location` citation it
//! slices the caller's own copy of the document at the returned range and keeps
//! the citation only when that slice IS the cited text. What the caller renders is
//! the slice of its own document — never text the model produced.
//!
//! ## Why a substring fallback exists
//!
//! The docs say character indices are 0-based with an exclusive end, but not
//! whether a "character" is a Unicode scalar or a UTF-16 unit. The two agree on
//! ASCII and disagree after the first `é` or curly quote. So when the range slice
//! does not match, the cited text is searched for verbatim in the document; if it
//! occurs there, the found range is used (and the correction is recorded). Either
//! way the quote is a byte-exact passage of the stored document. Only a quote that
//! occurs NOWHERE in the document is rejected.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A citation that survived verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedCitation {
    /// Index into the request's documents.
    pub document_index: usize,
    /// Start of the passage, in Unicode scalar values into the stored text.
    pub start_char: usize,
    /// End of the passage (exclusive), in Unicode scalar values.
    pub end_char: usize,
    /// The passage, sliced from the STORED document.
    pub quoted_text: String,
    /// `true` when the provider's range did not slice to the cited text and the
    /// passage was located by search instead.
    pub relocated: bool,
}

/// Why a citation was refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectedCitation {
    /// The provider's `document_index`, when present.
    pub document_index: Option<u64>,
    /// What was wrong, in words for a log line.
    pub reason: String,
}

/// Verify one citation object against the supplied document texts.
///
/// # Errors
/// A [`RejectedCitation`] naming why it cannot be shown.
pub fn verify(citation: &Value, documents: &[&str]) -> Result<VerifiedCitation, RejectedCitation> {
    let doc_index = citation.get("document_index").and_then(Value::as_u64);
    let reject = |reason: String| RejectedCitation {
        document_index: doc_index,
        reason,
    };
    let kind = citation.get("type").and_then(Value::as_str).unwrap_or("");
    if kind != "char_location" {
        return Err(reject(format!(
            "citation type `{kind}` is not char_location"
        )));
    }
    let index = doc_index
        .and_then(|i| usize::try_from(i).ok())
        .ok_or_else(|| reject("no document_index".into()))?;
    let text = *documents
        .get(index)
        .ok_or_else(|| reject(format!("document_index {index} names no supplied document")))?;
    let cited = citation
        .get("cited_text")
        .and_then(Value::as_str)
        .ok_or_else(|| reject("no cited_text".into()))?;
    if cited.trim().is_empty() {
        return Err(reject("cited_text is empty".into()));
    }
    let start = field(citation, "start_char_index");
    let end = field(citation, "end_char_index");
    if let (Some(start), Some(end)) = (start, end) {
        if let Some(slice) = slice_chars(text, start, end) {
            if slice == cited {
                return Ok(VerifiedCitation {
                    document_index: index,
                    start_char: start,
                    end_char: end,
                    quoted_text: slice,
                    relocated: false,
                });
            }
        }
    }
    locate(text, cited, index).ok_or_else(|| {
        reject(format!(
            "cited_text does not occur in document {index} (range {start:?}..{end:?})"
        ))
    })
}

fn field(v: &Value, key: &str) -> Option<usize> {
    v.get(key)
        .and_then(Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
}

/// Slice by Unicode scalar positions; `None` when the range is out of bounds.
fn slice_chars(text: &str, start: usize, end: usize) -> Option<String> {
    if start >= end {
        return None;
    }
    let slice: String = text.chars().skip(start).take(end - start).collect();
    // `take` stops early at the end of text; a short slice means out of range.
    (slice.chars().count() == end - start).then_some(slice)
}

fn locate(text: &str, cited: &str, index: usize) -> Option<VerifiedCitation> {
    let byte_start = text.find(cited)?;
    let start_char = text[..byte_start].chars().count();
    let end_char = start_char + cited.chars().count();
    Some(VerifiedCitation {
        document_index: index,
        start_char,
        end_char,
        // Sliced from the document itself, which by `find` equals `cited`.
        quoted_text: text[byte_start..byte_start + cited.len()].to_string(),
        relocated: true,
    })
}

/// Verify every citation on every `text` block of an assistant message.
///
/// Returns the verified citations, each paired with the index of the text block
/// it annotates, and the rejects.
pub fn verify_message(
    content: &[Value],
    documents: &[&str],
) -> (Vec<(usize, VerifiedCitation)>, Vec<RejectedCitation>) {
    let mut kept = Vec::new();
    let mut rejected = Vec::new();
    for (block_index, block) in content.iter().enumerate() {
        let Some(list) = block.get("citations").and_then(Value::as_array) else {
            continue;
        };
        for c in list {
            match verify(c, documents) {
                Ok(v) => kept.push((block_index, v)),
                Err(r) => rejected.push(r),
            }
        }
    }
    (kept, rejected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const DOC: &str = "The grass is green. The sky is blue.";

    #[test]
    fn a_matching_range_is_kept_and_sliced_from_the_stored_text() {
        let c = json!({"type":"char_location","cited_text":"The grass is green.",
            "document_index":0,"start_char_index":0,"end_char_index":19});
        let v = verify(&c, &[DOC]).unwrap();
        assert_eq!(v.quoted_text, "The grass is green.");
        assert!(!v.relocated);
    }

    #[test]
    fn a_wrong_range_with_real_text_is_relocated() {
        let c = json!({"type":"char_location","cited_text":"The sky is blue.",
            "document_index":0,"start_char_index":3,"end_char_index":9});
        let v = verify(&c, &[DOC]).unwrap();
        assert_eq!((v.start_char, v.end_char), (20, 36));
        assert!(v.relocated);
    }

    #[test]
    fn text_that_is_not_in_the_document_is_rejected() {
        let c = json!({"type":"char_location","cited_text":"The sea is red.",
            "document_index":0,"start_char_index":0,"end_char_index":15});
        assert!(verify(&c, &[DOC]).is_err());
    }

    #[test]
    fn an_index_past_the_documents_is_rejected() {
        let c = json!({"type":"char_location","cited_text":"x","document_index":3,
            "start_char_index":0,"end_char_index":1});
        assert!(verify(&c, &[DOC])
            .unwrap_err()
            .reason
            .contains("names no supplied"));
    }

    #[test]
    fn non_ascii_ranges_count_unicode_scalars() {
        let doc = "Café owner. The letter came.";
        let c = json!({"type":"char_location","cited_text":"The letter came.",
            "document_index":0,"start_char_index":12,"end_char_index":28});
        let v = verify(&c, &[doc]).unwrap();
        assert!(!v.relocated);
        assert_eq!(v.quoted_text, "The letter came.");
    }

    #[test]
    fn verify_message_pairs_citations_with_their_block() {
        let content = vec![
            json!({"type":"text","text":"Per the record, "}),
            json!({"type":"text","text":"grass is green","citations":[
                {"type":"char_location","cited_text":"The grass is green.","document_index":0,
                 "start_char_index":0,"end_char_index":19},
                {"type":"char_location","cited_text":"invented","document_index":0,
                 "start_char_index":0,"end_char_index":8}]}),
        ];
        let (kept, rejected) = verify_message(&content, &[DOC]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].0, 1);
        assert_eq!(rejected.len(), 1);
    }
}
