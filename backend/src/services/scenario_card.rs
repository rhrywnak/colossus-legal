//! Assemble the candidate card payload (task 1.2, v2 §7).
//!
//! ## What this module is for
//!
//! §7 says a card missing any element is a defect. This is the one place every
//! element is brought together, translated into plain trial language, and checked
//! for the condition that made the July cards unrulable — a stance with no
//! object. The frontend (task 1.3) renders the result and computes nothing.
//!
//! ## The shape of the assembly
//!
//! Four sources, joined by evidence id:
//!
//! | Source | Provides |
//! |---|---|
//! | `bias::repository` pool | quote, speaker, document, page (§7.1–7.3) |
//! | `scenario_card_repository` | statement kind, grounding, stance links, bears-on (§7.3, 7.5–7.7) |
//! | `scenario_fact_refs` | workbench status, the human's defer reason, the scan score (§7.8) |
//! | `document_text` | the surrounding source text for quote-in-context (§7.1) |
//!
//! Everything after the reads is pure, so the §7 completeness contract and the
//! defer detection are unit-tested without a database or a graph.

use std::collections::HashMap;

use crate::bias::dto::BiasInstance;
use crate::domain::card_language::{
    grounding_label, stance_verb_for_edge, statement_kind_label, status_label,
};
use crate::domain::confidence_band::{band_for_score, ConfidenceBand};
use crate::domain::fact_status::FactStatus;
use crate::dto::scenario_card::{
    CardBearsOn, CardConfidence, CardGrounding, CardPinpoint, CardQuote, CardSpeaker, CardStance,
    ScenarioCard,
};
use crate::repositories::scenario_card_repository::CardExtrasRow;

// The default width of the quote-in-context window, in characters.
//
// RULED 2026-08-01: this is a TUNABLE, not a structural constant. "How much
// context flanks the quote" is exactly the kind of number that may want turning up
// without a rebuild, so the Configuration law governs it. The display-contract
// argument was rejected on a good ground: if the card's layout really does
// constrain the width, that constraint belongs to the SAME setting — one number,
// one store, both consumers reading it — not to a second config source.
//
// It therefore gets the identical treatment to the band cutoffs: an in-code
// default behind ONE seam, so task 1.6 changes the source in a single place. NOT
// an env var — that would be the second config source this ruling exists to
// prevent.
//
// TODO(1.6): serve from the settings store. `context_window_chars()` below is the
// seam; 1.6 changes its body and deletes this const. It is on 1.6's seed parameter
// list alongside the confidence-band cutoffs.
const DEFAULT_CONTEXT_WINDOW_CHARS: usize = 240;

/// THE SEAM for the context window. The only reader of the default above.
///
/// ## Why a function for a value that is currently constant
///
/// Identical reasoning to `domain::confidence_band::band_for_score`: a tunable
/// behind one function is a tunable with a known home, so task 1.6 swaps the
/// source here and every card in the product follows. Inlining the const at its
/// two use sites would make 1.6 a hunt through the assembler — which is precisely
/// what the Configuration law is protecting against.
fn context_window_chars() -> usize {
    DEFAULT_CONTEXT_WINDOW_CHARS
}

/// Everything about one candidate that is not in the graph pool.
///
/// Assembled by the caller from the fact-ref row so the pure builder takes plain
/// data and no repository types.
#[derive(Debug, Clone, Default)]
pub(crate) struct CardRefState {
    pub status: Option<FactStatus>,
    pub confidence: Option<f32>,
    pub defer_reason: Option<String>,
}

/// Collapse the fanned-out extras rows into one entry per evidence id.
///
/// The graph returns one row per (evidence, allegation, element, count); this
/// folds them into a single record whose `links` vector holds each distinct
/// allegation. Evidence-level columns are taken from the first row seen — they
/// repeat identically across the fan-out.
///
/// ## Rust Learning: `entry().or_insert_with()` to fold a fan-out
///
/// `HashMap::entry` returns a handle to a slot whether or not it is occupied, so
/// one pass builds the map and appends to it without a contains-then-insert
/// double lookup. `or_insert_with` takes a closure so the default is constructed
/// only when the slot is genuinely empty.
pub(crate) fn collapse_extras(rows: Vec<CardExtrasRow>) -> HashMap<String, CollapsedExtras> {
    let mut by_id: HashMap<String, CollapsedExtras> = HashMap::new();

    for row in rows {
        let entry = by_id
            .entry(row.evidence_id.clone())
            .or_insert_with(|| CollapsedExtras {
                statement_type: row.statement_type.clone(),
                grounding_status: row.grounding_status.clone(),
                links: Vec::new(),
            });

        // A row with no edge class is the "links to nothing" case — the OPTIONAL
        // MATCH kept the evidence in the result, and there is no link to record.
        let (Some(edge_class), Some(allegation_id)) = (row.edge_class, row.allegation_id) else {
            continue;
        };

        // The element/count join fans out further: one accusation bearing on three
        // elements arrives as three rows. Keep one entry per (edge, allegation,
        // element) so every element survives — deduping on the allegation alone
        // would keep whichever row came first and silently drop the others.
        if entry.links.iter().any(|l| {
            l.allegation_id == allegation_id
                && l.edge_class == edge_class
                && l.element_name == row.element_name
        }) {
            continue;
        }

        entry.links.push(ExtrasLink {
            edge_class,
            allegation_id,
            allegation_summary: row.allegation_summary,
            allegation_title: row.allegation_title,
            allegation_paragraph: row.allegation_paragraph,
            element_name: row.element_name,
            count_number: row.count_number,
            count_name: row.count_name,
        });
    }
    by_id
}

/// One candidate's extras, after collapsing.
#[derive(Debug, Clone)]
pub(crate) struct CollapsedExtras {
    pub statement_type: Option<String>,
    pub grounding_status: Option<String>,
    pub links: Vec<ExtrasLink>,
}

/// One Evidence→Allegation link with its bears-on chain.
#[derive(Debug, Clone)]
pub(crate) struct ExtrasLink {
    pub edge_class: String,
    pub allegation_id: String,
    pub allegation_summary: Option<String>,
    pub allegation_title: Option<String>,
    pub allegation_paragraph: Option<String>,
    pub element_name: Option<String>,
    pub count_number: Option<i64>,
    pub count_name: Option<String>,
}

/// The accusation in complaint language, paragraph-numbered when known.
///
/// Prefers the summary (the complaint's own sentence) over the title, and falls
/// back to the id only when the node carries neither — a card that says
/// "Accusation alleg-7" is poor, but it is honest, and it beats an empty line the
/// human cannot act on.
fn accusation_text(link: &ExtrasLink) -> String {
    let body = link
        .allegation_summary
        .as_deref()
        .or(link.allegation_title.as_deref())
        .unwrap_or(&link.allegation_id);

    match link.allegation_paragraph.as_deref() {
        Some(paragraph) if !paragraph.trim().is_empty() => format!("¶{paragraph} — {body}"),
        _ => body.to_string(),
    }
}

/// Build the stance line from the FIRST link that carries a mappable edge.
///
/// ## Domain note: why the first link, and why that is enough
///
/// An item can bear on several accusations, and the card shows every one of them
/// under `bears_on`. The STANCE line is a single sentence, so it takes the first
/// mappable link — which the query orders deterministically by count then
/// allegation, so it is stable across requests rather than whatever the graph
/// happened to return first.
///
/// A link whose edge class this build cannot map yields no stance rather than a
/// guess; if no link maps, the caller treats the card as defer-required.
fn build_stance(links: &[ExtrasLink]) -> Option<CardStance> {
    links.iter().find_map(|link| {
        let verb = stance_verb_for_edge(&link.edge_class)?;
        let object = accusation_text(link);
        Some(CardStance {
            summary: format!("This {verb} {object}"),
            verb: verb.to_string(),
            object,
        })
    })
}

/// Group the links into one entry per accusation, collecting its elements.
///
/// ## Domain note: one accusation, several elements
///
/// The graph fans out — an accusation bearing on three elements of a count
/// produces three links. The card shows the ACCUSATION once with its elements
/// listed, because repeating the same complaint sentence three times makes the
/// human deduplicate it by eye, and dropping two of the three understates what
/// the item does. Order is preserved from the query's deterministic sort.
///
/// ## Rust Learning: `iter().position()` instead of a HashMap
///
/// A map would lose the ordering the query established, and these lists are tiny
/// (an item bears on a handful of accusations at most). A linear scan over a
/// `Vec` keeps the order and is faster at this size than hashing.
fn build_bears_on(links: &[ExtrasLink]) -> Vec<CardBearsOn> {
    let mut out: Vec<CardBearsOn> = Vec::new();

    for link in links {
        let accusation = accusation_text(link);
        let count = match (link.count_number, link.count_name.as_deref()) {
            (Some(number), Some(name)) => Some(format!("Count {number} — {name}")),
            (Some(number), None) => Some(format!("Count {number}")),
            (None, Some(name)) => Some(name.to_string()),
            (None, None) => None,
        };

        // Find-or-append, then index. Computing the INDEX in both branches (rather
        // than returning a `&mut` from one and `last_mut()` from the other) avoids
        // an `.expect()` on a "we just pushed" invariant — the borrow checker is
        // satisfied by an index, and there is no unwrap left to justify.
        let index = match out.iter().position(|b| b.accusation == accusation) {
            Some(index) => index,
            None => {
                out.push(CardBearsOn {
                    accusation,
                    elements: Vec::new(),
                    count,
                });
                out.len() - 1
            }
        };

        if let Some(element) = link.element_name.as_deref() {
            if !out[index].elements.iter().any(|e| e == element) {
                out[index].elements.push(element.to_string());
            }
        }
    }
    out
}

/// Locate the quote in its page text and return the surrounding window.
///
/// Returns empty strings when the page text is unavailable or the quote cannot be
/// found in it — an honest "no context" rather than a fabricated one. A quote that
/// cannot be located is also exactly what `grounding_status` reports separately,
/// so the card does not have to guess why.
///
/// ## Rust Learning: slicing a `&str` on a CHARACTER boundary
///
/// Rust strings are UTF-8, and slicing at a byte index that falls inside a
/// multi-byte character panics. `char_indices` walks real boundaries, so taking
/// the window by counting characters and mapping back to byte offsets is safe for
/// any text — which matters here, because OCR'd legal PDFs are full of curly
/// quotes, dashes and accented names.
fn quote_context(page_text: Option<&str>, quote: &str) -> (String, String) {
    let Some(page) = page_text else {
        return (String::new(), String::new());
    };
    let Some(byte_at) = page.find(quote) else {
        return (String::new(), String::new());
    };

    let before_all = &page[..byte_at];
    let after_all = &page[byte_at + quote.len()..];

    // Take the LAST `window` characters before, and the FIRST that many after —
    // the text nearest the quote is the text that explains it. Read through the
    // seam once, so both edges use the same value even if 1.6 makes it dynamic.
    let window = context_window_chars();
    let before_start = before_all
        .char_indices()
        .rev()
        .nth(window.saturating_sub(1))
        .map(|(i, _)| i)
        .unwrap_or(0);
    let after_end = after_all
        .char_indices()
        .nth(window)
        .map(|(i, _)| i)
        .unwrap_or(after_all.len());

    (
        before_all[before_start..].to_string(),
        after_all[..after_end].to_string(),
    )
}

/// The pinpoint line as the card shows it, composed server-side.
///
/// A page-less item reads as the document alone rather than "… at null" — the
/// absence is rendered by omission, never by a placeholder.
fn pinpoint_label(document_title: &str, page: Option<i64>) -> String {
    match page {
        Some(page) => format!("{document_title} at {page}"),
        None => document_title.to_string(),
    }
}

/// The viewer link for a pinpoint, assembled server-side.
///
/// Matches the route the document workspace already serves (`/documents/:id`)
/// with the page anchor it already reads. Built here rather than in the browser
/// because v2 §7 item 2 puts every display decision on this side of the wire.
fn viewer_href(document_id: &str, page: Option<i64>) -> String {
    match page {
        Some(page) => format!("/documents/{document_id}?page={page}&tab=document"),
        None => format!("/documents/{document_id}?tab=document"),
    }
}

/// Why this card cannot be ruled on as it stands, or `None` if it can.
///
/// ## Domain note: the two unrulable classes
///
/// 1. **No object for a stance** — the C-222 class. The item is in the pool, but
///    nothing links it to an accusation, so there is nothing for it to support or
///    dispute. The message names what is missing AND what would unblock it.
/// 2. **No quote** — an item with no verbatim words cannot be cited, so an
///    include would be a rehearsal liability (the citability law that task 1.1's
///    ruling path already enforces on write). Surfacing it on the card means the
///    human learns it before they click, not after the 400.
///
/// The reasons are deliberately causal sentences rather than codes: the person
/// reading them is deciding what to do next, and "no allegation link" would tell
/// them nothing about how to fix it.
fn defer_reason_for(
    quote: &str,
    stance: Option<&CardStance>,
    has_scan_score: bool,
) -> Option<String> {
    if quote.trim().is_empty() {
        return Some(
            "This item has no verbatim quote, so it cannot be cited or matched back \
             to the record after re-processing. Defer it; it stays in the queue."
                .to_string(),
        );
    }
    if stance.is_none() {
        let opener = if has_scan_score {
            "A scan scored this item, but it is not linked to any accusation"
        } else {
            "This item is not linked to any accusation"
        };
        return Some(format!(
            "{opener}, so there is nothing for it to support or dispute. Link it to \
             an accusation, or defer it; it stays in the queue."
        ));
    }
    None
}

/// Build one complete card. Pure — no I/O.
///
/// This function IS the §7 contract: every element is assembled here, and the
/// completeness test asserts against its output.
pub(crate) fn build_card(
    instance: &BiasInstance,
    extras: Option<&CollapsedExtras>,
    ref_state: &CardRefState,
    ordinal: Option<i32>,
    page_text: Option<&str>,
) -> ScenarioCard {
    let quote_text = instance.verbatim_quote.clone().unwrap_or_default();
    let (context_before, context_after) = quote_context(page_text, &quote_text);

    let links: &[ExtrasLink] = extras.map(|e| e.links.as_slice()).unwrap_or(&[]);
    let stance = build_stance(links);

    let bears_on = build_bears_on(links);

    let band = band_for_score(ref_state.confidence);
    let status = ref_state.status.unwrap_or(FactStatus::Undecided);
    let defer_required_reason = defer_reason_for(
        &quote_text,
        stance.as_ref(),
        band != ConfidenceBand::Unscored,
    );

    let document_id = instance
        .document
        .as_ref()
        .map(|d| d.id.clone())
        .unwrap_or_default();

    ScenarioCard {
        code: ordinal.map(|n| format!("C-{n}")),
        graph_node_id: instance.evidence_id.clone(),
        quote: CardQuote {
            text: quote_text,
            context_before,
            context_after,
            question: instance.question.clone(),
        },
        pinpoint: CardPinpoint {
            label: pinpoint_label(
                &instance
                    .document
                    .as_ref()
                    .map(|d| d.title.clone())
                    .unwrap_or_default(),
                instance.page_number,
            ),
            document_title: instance
                .document
                .as_ref()
                .map(|d| d.title.clone())
                .unwrap_or_default(),
            page: instance.page_number,
            viewer_href: viewer_href(&document_id, instance.page_number),
            document_id,
        },
        speaker: CardSpeaker {
            // An empty speaker name IS absent — `evidence_by_ids` decodes a
            // missing STATED_BY edge to `coalesce(…, '')`.
            name: instance
                .stated_by
                .as_ref()
                .map(|a| a.name.clone())
                .filter(|n| !n.trim().is_empty()),
            attribution: "extracted".to_string(),
        },
        statement_kind: extras
            .and_then(|e| e.statement_type.as_deref())
            .map(statement_kind_label),
        stance,
        bears_on,
        grounding: extras
            .and_then(|e| e.grounding_status.as_deref())
            .and_then(|state| {
                grounding_label(state).map(|label| CardGrounding {
                    state: state.to_string(),
                    label: label.to_string(),
                })
            }),
        confidence: CardConfidence {
            band,
            label: band.label().to_string(),
        },
        status,
        status_label: status_label(status).to_string(),
        defer_required: defer_required_reason.is_some(),
        defer_required_reason,
        defer_reason: ref_state.defer_reason.clone(),
    }
}

#[cfg(test)]
#[path = "scenario_card_tests.rs"]
mod tests;
