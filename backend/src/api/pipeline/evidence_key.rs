//! The stable id for an `Evidence` node (task TEMPLATE_BATCH/ID_ARM, P1a).
//!
//! ## What was wrong, measured
//!
//! Evidence had no arm in `stable_entity_id`. It fell to the catch-all, which
//! hashes the ENTIRE `item_data` blob — every LLM-mooded field included:
//! `summary`, `significance`, `weight`, `pattern_tags`. Re-running extraction
//! rephrases those, so the hash moves, so the id moves. Measured id survival
//! across a real reprocess: **0 of 131**. Every curated row pointing at those ids
//! — 947 of them today, across seven tables — became a dangling reference, and
//! the pipeline had no idea any of it existed.
//!
//! ## The key, and why each part of it is there
//!
//! `doc_slug + page_number + normalized verbatim_quote + question (when present)`
//!
//! Every component is a property of what the DOCUMENT says, not of what the model
//! said about it. A re-extraction of the same page produces the same quote on the
//! same page of the same document, so it produces the same id. Nothing mooded
//! enters the key, ever — that is the whole point, and it is the one rule this
//! module must never be "improved" past.
//!
//! `question` is in because measurement demanded it: quote+page alone left
//! 119/131 distinct, quote+page+question 129/131. It is optional because 287 of
//! 525 live Evidence nodes are documentary and answer nobody.
//!
//! ## Normalization: NFC, trim, collapse
//!
//! Unicode NFC first, so a quote whose accents arrive decomposed one run and
//! composed the next does not change identity. Then whitespace: OCR line breaks
//! and double spaces are layout, not content.
//!
//! ## The id SHAPE is deliberately unchanged
//!
//! `{doc_slug}:evidence:{8 hex}` — byte-identical in form to what the catch-all
//! produced. Only the material being hashed changes. That keeps every reader that
//! splits an id on `:` working, and it makes the re-key a pure value change.
//!
//! ## Domain note: this arm does NOT resolve duplicates
//!
//! Measured on DEV: 21 pairs of Evidence nodes share this key exactly — the same
//! statement extracted twice, identical on every stable field, differing only in
//! mooded prose. Seven of those pairs carry curated rows on BOTH twins, and three
//! carry DIFFERENT weights (one `carries`, one `backup`, same scenario). So the
//! twins are not interchangeable and no positional tiebreaker is safe: assigning
//! them by order would swap Roman's weights on the demo-facing scenarios.
//!
//! Ruled 2026-08-14: **no disambiguator, ever.** A key with more than one holder
//! is refused rather than decorated — see `ingest_dedupe` for the door-side cure
//! and the twin-merge script for the existing pairs.

use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

/// The separator between key components.
///
/// U+001F INFORMATION SEPARATOR ONE — a C0 control character that cannot occur in
/// a verbatim quote extracted from a legal document, and would be stripped by the
/// whitespace pass if it somehow did. Using a printable separator (`|`, `:`) would
/// let a quote containing that character shift the component boundaries and
/// collide with a different statement.
const KEY_SEPARATOR: char = '\u{1f}';

/// How many hex characters of the digest the id carries.
///
/// Eight, because that is what every existing Evidence id already carries and the
/// re-key is meant to change the derivation, not the shape. 32 bits over 525
/// nodes is a ~0.003% birthday risk — and unlike the mooded-blob hash it replaces,
/// a collision here is detectable: two nodes with one id fail the uniqueness check
/// the re-key migration runs before it writes anything.
const ID_HASH_CHARS: usize = 8;

/// Normalize one piece of text for the key: NFC, trim, collapse whitespace.
///
/// ## Rust Learning: `.nfc()` comes from a trait
///
/// `UnicodeNormalization` is an extension trait implemented for `&str`; importing
/// it is what puts `.nfc()` in scope. It returns a lazy ITERATOR of `char`, not a
/// `String` — nothing is allocated until it is collected, which is why the
/// collect happens here and the whitespace pass runs over the result.
///
/// ## Rust Learning: `split_whitespace` as the collapser
///
/// It already skips runs of any whitespace — the newlines an OCR'd quote is full
/// of included — so joining its pieces with a single space collapses and trims in
/// one pass, with no regex.
pub fn normalize(text: &str) -> String {
    let composed: String = text.nfc().collect();
    composed.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Build the stable Evidence id from its components.
///
/// `page` is `Option` for defensiveness only — all 525 live Evidence nodes carry
/// one. A missing page contributes an empty component rather than being dropped,
/// so "page 1 / no quote" and "no page / quote 1" cannot produce one key.
///
/// `question` absent and `question` empty produce the SAME key deliberately: a
/// documentary statement has no question, and an extraction that emitted `""` for
/// one means the same thing. That is the only place this module treats absent and
/// empty alike, and it is safe because the distinction carries no meaning here.
pub fn evidence_id(
    doc_slug: &str,
    page: Option<i64>,
    verbatim_quote: &str,
    question: Option<&str>,
) -> String {
    let page_part = page.map(|p| p.to_string()).unwrap_or_default();
    let quote_part = normalize(verbatim_quote);
    let question_part = question.map(normalize).unwrap_or_default();

    let material = format!(
        "{doc_slug}{sep}{page_part}{sep}{quote_part}{sep}{question_part}",
        sep = KEY_SEPARATOR,
    );
    let hash = format!("{:x}", Sha256::digest(material.as_bytes()));
    format!("{doc_slug}:evidence:{}", &hash[..ID_HASH_CHARS])
}

/// Read the key's components out of one extraction item's properties.
///
/// Returns the id, or `None` when the item carries no usable `verbatim_quote` —
/// the one component with no honest default. An Evidence node with no words is
/// not a statement, and giving it a key derived from an empty string would MERGE
/// every such item onto one node (the exact failure the allegation arm's
/// `hash-e3b0c442` comment records). The caller falls back to the previous
/// behaviour and logs, rather than this module inventing an id.
pub fn evidence_id_from_properties(props: &serde_json::Value, doc_slug: &str) -> Option<String> {
    let quote = props["verbatim_quote"].as_str()?;
    if quote.trim().is_empty() {
        return None;
    }
    // The graph stores `page_number` as an integer, but extraction JSON has been
    // seen carrying it as a string; both are accepted rather than one silently
    // becoming "no page".
    let page = props["page_number"]
        .as_i64()
        .or_else(|| props["page_number"].as_str().and_then(|s| s.parse().ok()));
    let question = props["question"].as_str();
    Some(evidence_id(doc_slug, page, quote, question))
}

#[cfg(test)]
#[path = "evidence_key_tests.rs"]
mod tests;
