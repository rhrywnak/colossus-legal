//! The stable id for an `Evidence` node (task TEMPLATE_BATCH/ID_ARM, P1a).
//!
//! ## What was wrong, measured
//!
//! Evidence had no arm in `stable_entity_id`. It fell to the catch-all, which
//! hashes the ENTIRE `item_data` blob — every LLM-mooded field included:
//! `summary`, `significance`, `weight`, `pattern_tags`. Re-running extraction
//! rephrases those, so the hash moves, so the id moves. Measured id survival
//! across a real reprocess: **0 of 131**. Every curated row pointing at those ids
//! — 947 curated rows today, and 1,472 counting pipeline provenance, across
//! eleven columns — became a dangling reference, and
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
//! ## The arm shipped inert, and why — corrected 2026-08-17
//!
//! The first cut of this module read all three components out of
//! `item_data["properties"]`. Extraction does not put the quote there: the
//! templates say "Copy the EXACT text as verbatim_quote at the TOP LEVEL", the
//! schemas say "verbatim_quote remains a top-level entity field, NOT a schema
//! property", and `store_entities_and_relationships` stores the whole entity JSON
//! with the quote as a SIBLING of `properties`. Measured on DEV the day it was
//! found: `properties.verbatim_quote` present on **0 of 574** live Evidence
//! items. So the guarded `?` returned `None` on every row ever written and every
//! Evidence id came from the whole-blob fallback this module exists to replace —
//! for eleven days, with a green test suite, because every fixture in
//! `evidence_key_tests.rs` had been hand-built in the shape the code assumed
//! rather than the shape the pipeline emits.
//!
//! Two rules came out of it, and both are load-bearing:
//!
//! 1. **Key from what the graph carries**, not from what the model wrote. The
//!    node's quote is `extraction_items.verbatim_quote` and its page is
//!    `grounded_page`; `rekey_evidence` hashed those, so this arm hashes those.
//!    See [`evidence_id_from_item`] for the measured reason the claimed page is
//!    not good enough.
//! 2. **A fixture that is not the wire shape proves nothing.** The tests now pin
//!    a byte-for-byte copy of a live `item_data` row.
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

/// Where the quote was found, so the caller can log a shape it did not expect.
///
/// ## Why this is a return value and not a silent preference
///
/// The predecessor of this function read the quote from ONE place, found nothing
/// there on every row the pipeline has ever written, and returned `None` without
/// a word. Naming the source turns "which shape was this?" from an assumption
/// into an observable (Standing Rule 1) — the caller logs anything but
/// [`QuoteSource::Column`], so a template that starts emitting a different shape
/// announces itself on the first document instead of on the next re-extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteSource {
    /// `extraction_items.verbatim_quote` — the expected source. This is the
    /// column `create_entity_node` writes onto the graph node, so keying from it
    /// is what makes the id reproduce what the graph already carries.
    Column,
    /// `item_data["verbatim_quote"]` — the top-level field the templates specify
    /// and 573 of 574 live Evidence rows carry. Reached only when the column is
    /// empty, which means the insert path and the JSON disagree.
    TopLevel,
    /// `item_data["properties"]["verbatim_quote"]` — the shape the pre-fix arm
    /// assumed. Measured 2026-08-17: **0 of 574** live Evidence rows. Kept
    /// because `store_entities_and_relationships` still accepts it, so a future
    /// template could produce it; it is never silent.
    Properties,
}

/// Read the key's components out of one extraction item.
///
/// ## Why the page comes from the caller and not from the JSON
///
/// `grounded_page` is the page the VERIFIER found the quote on;
/// `properties.page_number` is the page the model CLAIMED. They disagree on
/// **132 of 574** live Evidence items (measured 2026-08-17), and it is
/// `grounded_page` that `create_entity_node` writes onto the node and therefore
/// `grounded_page` that `rekey_evidence` hashed. Keying on the claimed page
/// would produce ids that no longer match the graph for those 132 rows, which is
/// why this function takes the grounded page as a parameter and never reads the
/// claimed one.
///
/// ## Why the question still comes from `properties`
///
/// Because that is the only place the node gets it: `create_entity_node` copies
/// schema properties onto the node, so `n.question` is `properties.question` or
/// nothing. Measured: 238 of 574 carry it there, **0** at the top level. Reading
/// a top-level `question` would key on a value the graph does not have.
///
/// Returns the id and the source the quote came from, or `None` when the item
/// carries no usable quote anywhere — the one component with no honest default.
/// An Evidence node with no words is not a statement, and a key derived from an
/// empty string would MERGE every such item onto one node (the exact failure the
/// allegation arm's `hash-e3b0c442` comment records). The caller falls back and
/// logs, rather than this module inventing an id.
///
/// ## Rust Learning: `Option<&str>` chaining with `.filter()` and `.or_else()`
///
/// Each candidate source is an `Option<&str>` that must also survive a
/// "is it actually words?" test. `.filter(|s| !s.trim().is_empty())` turns a
/// `Some("")` into a `None` so the next `.or_else()` gets its turn — the whole
/// preference order reads top-to-bottom with no `if let` ladder and no early
/// returns, and adding a fourth source later is one more line rather than a
/// restructure.
pub fn evidence_id_from_item(
    doc_slug: &str,
    column_quote: Option<&str>,
    grounded_page: Option<i64>,
    item_data: &serde_json::Value,
) -> Option<(String, QuoteSource)> {
    let usable = |s: &&str| !s.trim().is_empty();

    let (quote, source) = column_quote
        .filter(usable)
        .map(|q| (q, QuoteSource::Column))
        .or_else(|| {
            item_data["verbatim_quote"]
                .as_str()
                .filter(usable)
                .map(|q| (q, QuoteSource::TopLevel))
        })
        .or_else(|| {
            item_data["properties"]["verbatim_quote"]
                .as_str()
                .filter(usable)
                .map(|q| (q, QuoteSource::Properties))
        })?;

    let question = item_data["properties"]["question"].as_str();
    Some((
        evidence_id(doc_slug, grounded_page, quote, question),
        source,
    ))
}

#[cfg(test)]
#[path = "evidence_key_tests.rs"]
mod tests;
