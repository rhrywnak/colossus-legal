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
use crate::domain::fact_tier::FactTier;
use crate::domain::settings::Settings;
use crate::dto::scenario_card::{
    CardBearsOn, CardConfidence, CardGrounding, CardPinpoint, CardQuote, CardSpeaker, CardStance,
    ScenarioCard,
};
use crate::repositories::pipeline_repository::EvidenceSummaryOverrideRecord;
use crate::repositories::scenario_card_repository::CardExtrasRow;
use crate::services::scenario_card_context::{assemble as assemble_context, QuoteContext};
use crate::services::scenario_human_links::{link_summary, resolve_question, HumanTouches};

// The quote-in-context window is a STORED parameter (task 1.6, v2 §2b).
//
// RULED 2026-08-01: this is a TUNABLE, not a structural constant. "How much
// context flanks the quote" is exactly the kind of number that may want turning up
// without a rebuild, so the Configuration law governs it. The display-contract
// argument was rejected on a good ground: if the card's layout really does
// constrain the width, that constraint belongs to the SAME setting — one number,
// one store, both consumers reading it — not to a second config source.
//
// Until 1.6 it was an in-code default behind a `context_window_chars()` seam,
// marked `TODO(1.6)`. The seam is now the `&Settings` this module's builders take;
// the const and the seam function are gone.

/// Everything about one candidate that is not in the graph pool.
///
/// Assembled by the caller from the fact-ref row so the pure builder takes plain
/// data and no repository types.
///
/// ## Why a PROJECTED proposal arrives in the same struct (2026-08-08)
///
/// A card's non-graph state has exactly two possible sources and they are mutually
/// exclusive: a stored reference row (a human has touched it) or the latest
/// completed scan's verdict (nobody has). Precedence R-a is what makes them
/// exclusive — a ref row always wins — so one struct with one filled shape per
/// card is the honest model, and a second parallel argument would have invited a
/// caller to supply both and leave the tie-break to whoever read the fields last.
///
/// For a projected card `status` and `tier` stay `None` (there IS no row), and
/// `confidence` carries the VERDICT's score — which is the same number the retired
/// merge used to copy into the column, banded by the same cutoffs.
#[derive(Debug, Clone, Default)]
pub(crate) struct CardRefState {
    pub status: Option<FactStatus>,
    pub confidence: Option<f32>,
    /// What the latest completed scan proposed, display-ready, or `None` when
    /// nothing proposed this card. Composed by
    /// `services::scenario_card_projection` — the assembler holds the ordinals and
    /// the wording the badge needs, and this builder holds neither.
    pub proposal: Option<crate::dto::scenario_card::CardProposal>,
    /// The judging reason behind this card, whether it is proposed or ruled
    /// (ONE_CARD_GRAMMAR, ruling R3).
    ///
    /// Deliberately NOT inside `proposal`, which is where it used to live: a
    /// proposal exists only while nobody has ruled, so the reason vanished on
    /// Include — and the reason is exactly what an included fact has to keep
    /// saying. For a proposed card the projection supplies it; for a ruled one
    /// the caller recovers it by joining the reference row's `source_run_id`
    /// back to that run's verdict.
    pub scan_reason: Option<String>,
    pub defer_reason: Option<String>,
    /// The decoded weight tier, or `None` when there is no reference row at all
    /// (an unruled candidate has no weight in this scenario).
    ///
    /// Decoded — a `FactTier`, not the raw token — because this is where the
    /// value is BRANCHED on: the card must never show a human a tier this build
    /// cannot name. The decode happens in `build_ref_states`, which refuses the
    /// whole request on an unknown token rather than defaulting it (Standing
    /// Rule 1). That is the same loud boundary the sibling `status` decode uses.
    pub tier: Option<FactTier>,
    /// The human's stored position, or `None` when they have never placed it.
    pub sort_ordinal: Option<i32>,
    /// Where this fact renders in the list — the ONE number the client sorts on.
    /// Computed by `services::scenario_fact_order::effective_order` over the whole
    /// included set, so it accounts for the unplaced tail as well as the stored
    /// positions. `None` for anything that is not an included fact.
    pub display_ordinal: Option<i32>,
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

/// The accusation in complaint language, code-prefixed when the paragraph is known.
///
/// Prefers the summary (the complaint's own sentence) over the title, and falls
/// back to the id only when the node carries neither — a card that says
/// "Accusation alleg-7" is poor, but it is honest, and it beats an empty line the
/// human cannot act on.
///
/// ## The handle is `A-45`, not `¶45` (ONE_CARD_GRAMMAR §3.3, 2026-08-09)
///
/// Same paragraph, same number, typeable prefix — see
/// [`crate::domain::scenario_code::allegation_code`] for why that last property
/// is what forced the change. The spelling lives THERE beside `S-` and `C-`
/// rather than in this `format!`, because the sibling composer in
/// `scenario_link_options` renders the same handle for the type-ahead and two
/// sites spelling one human handle is one too many.
fn accusation_text(link: &ExtrasLink) -> String {
    let body = link
        .allegation_summary
        .as_deref()
        .or(link.allegation_title.as_deref())
        .unwrap_or(&link.allegation_id);

    match link.allegation_paragraph.as_deref() {
        Some(paragraph) if !paragraph.trim().is_empty() => {
            format!(
                "{} — {body}",
                crate::domain::scenario_code::allegation_code(paragraph)
            )
        }
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
///
/// ## The linking promise was withdrawn (task 1.7E, item 6)
///
/// Until 1.7E the unlinked reason said "Link it to an accusation, or defer it".
/// Nothing in the product links an item to an accusation — that arrives with task
/// 2.10 — so the sentence offered a way forward that does not exist, and a human
/// reading it could only conclude the control was hiding somewhere they had not
/// looked. Roman's ruling: the copy says what is true today, which is that Defer
/// is the only door, and says the item will come back when linking lands.
///
/// The two openers survive the rewrite because the distinction they carry is
/// real: "a scan looked at this and scored it" is why C-222 LOOKED rulable, and
/// collapsing it into the unscored wording would erase the reason the card is
/// confusing in the first place.
///
/// ## Linking has now landed (task 2.10), and `has_human_link` is how it lands here
///
/// A human supplying the link the extraction never made removes class 1 entirely:
/// there IS something for the item to bear on, because a person said so. So the
/// unlinked branch asks about BOTH kinds of link, and this one function is the
/// whole of "linking provably clears the defer flag" — nothing else in the payload
/// decides it.
///
/// Class 2 is untouched, deliberately: a human can link a quoteless item all day
/// and it still cannot be cited, so the no-quote refusal survives a link. The two
/// classes were always independent and this keeps them so.
///
/// The unlinked SENTENCE is left exactly as 1.7E wrote it, including "returns when
/// linking arrives" — it is only ever shown on a card with no human link, where it
/// remains true. Rewriting the copy of a still-correct message is task 3.16's
/// sweep, not this one's.
fn defer_reason_for(
    quote: &str,
    stance: Option<&CardStance>,
    has_human_link: bool,
    has_scan_score: bool,
) -> Option<String> {
    if quote.trim().is_empty() {
        return Some(
            "This item has no verbatim quote, so it cannot be cited or matched back \
             to the record after re-processing. Defer it; it stays in the queue."
                .to_string(),
        );
    }
    if stance.is_none() && !has_human_link {
        let opener = if has_scan_score {
            "A scan scored this item, but it is not linked to any accusation yet"
        } else {
            "This item is not linked to any accusation yet"
        };
        return Some(format!(
            "{opener}, so there is nothing for it to support or dispute. It can only \
             be deferred for now — it stays in the queue and returns when linking \
             arrives."
        ));
    }
    None
}

/// The §7.4 grounding row: whether the quote was found in its own page, in the
/// record's own words.
///
/// Split out of [`build_card`] for the function-size limit (Rule 18).
///
/// `None` when the extras row carries no grounding state, and ALSO when it carries a
/// state this build has no label for. The second case is deliberate: a card that
/// showed a raw pipeline token where a sentence belongs would be leaking internals
/// onto a surface a lawyer reads, and `grounding_label` returning `None` is how the
/// vocabulary says "I do not know how to say this one out loud".
///
/// The pair travels together — the machine `state` beside the composed `label` — so
/// a client branches on the token and never parses the prose.
fn build_grounding(extras: Option<&CollapsedExtras>) -> Option<CardGrounding> {
    extras
        .and_then(|e| e.grounding_status.as_deref())
        .and_then(|state| {
            grounding_label(state).map(|label| CardGrounding {
                state: state.to_string(),
                label: label.to_string(),
            })
        })
}

/// The §7.2 pinpoint: where the quote is, and how to get there.
///
/// Split out of [`build_card`] for the function-size limit (Rule 18). The document
/// title is read twice — once composed into `label`, once on its own — because the
/// card renders the composed line but a client filtering or grouping by document
/// needs the bare title, and re-deriving it by parsing `label` would be the browser
/// taking a display string apart to recover data.
///
/// A document-less instance yields empty strings rather than `None`: these fields
/// are always present on the wire so the card's layout is stable, and an absent
/// title renders as omission (see [`pinpoint_label`]).
fn build_pinpoint(instance: &BiasInstance) -> CardPinpoint {
    let document_title = instance
        .document
        .as_ref()
        .map(|d| d.title.clone())
        .unwrap_or_default();
    let document_id = instance
        .document
        .as_ref()
        .map(|d| d.id.clone())
        .unwrap_or_default();

    CardPinpoint {
        label: pinpoint_label(&document_title, instance.page_number),
        document_title,
        page: instance.page_number,
        viewer_href: crate::domain::card_language::viewer_href(&document_id, instance.page_number),
        document_id,
    }
}

/// The §7.3 speaker: who said it, and on what authority we say so.
///
/// Split out of [`build_card`] for the function-size limit (Rule 18).
fn build_speaker(instance: &BiasInstance, extracted_label: &str) -> CardSpeaker {
    CardSpeaker {
        // An empty speaker name IS absent — `evidence_by_ids` decodes a missing
        // STATED_BY edge to `coalesce(…, '')`. Filtering the blank keeps "nobody is
        // recorded as saying this" distinct from "somebody said it and their name is
        // the empty string", which is the distinction a documentary exhibit needs.
        name: instance
            .stated_by
            .as_ref()
            .map(|a| a.name.clone())
            .filter(|n| !n.trim().is_empty()),
        attribution: extracted_label.to_string(),
    }
}

/// The §7.1 quote block: the anchor, its two flanks, and how each flank ended.
///
/// Split out of [`build_card`] for the function-size limit (Rule 18) — task 1.7C
/// added four fields to this struct literal and `build_card` was already over.
///
/// It earns the split beyond arithmetic: this is the one place the CONTEXT LAW's
/// output is shaped for the wire, and the pairing it establishes is not obvious.
/// Each flank contributes THREE things — the text, a boolean saying whether that
/// text ended at a real sentence boundary, and (only when it did not) a
/// server-composed notice naming the page edge. The boolean is the state a client
/// branches on; the notice is the words, because the language law puts every display
/// decision on this side of the wire. Keeping them adjacent here makes a future
/// change that sets one and forgets the other visible.
fn build_quote(
    text: String,
    context: QuoteContext,
    question: Option<String>,
    override_row: Option<&EvidenceSummaryOverrideRecord>,
    machine_authorship_label: &str,
) -> CardQuote {
    let (question, question_authorship) =
        resolve_question(question, override_row, machine_authorship_label);
    CardQuote {
        text,
        context_before: context.before.text,
        context_after: context.after.text,
        context_before_complete: context.before.complete,
        context_after_complete: context.after.complete,
        context_before_notice: context.before.notice,
        context_after_notice: context.after.notice,
        question,
        question_authorship,
    }
}

/// Build one complete card. Pure — no I/O.
///
/// This function IS the §7 contract: every element is assembled here, and the
/// completeness test asserts against its output.
///
/// `settings` carries the two tunables the card reads — the band cutoffs and the
/// context window (v2 §2b). It is an ARGUMENT rather than a global precisely so
/// this function stays what its first line claims: pure, and testable with a
/// struct literal rather than a booted process.
pub(crate) fn build_card(
    instance: &BiasInstance,
    extras: Option<&CollapsedExtras>,
    ref_state: &CardRefState,
    ordinal: Option<i32>,
    page_text: Option<&str>,
    settings: &Settings,
    human: HumanTouches<'_>,
) -> ScenarioCard {
    let HumanTouches {
        question_override,
        links: human_links,
    } = human;
    let quote_text = instance.verbatim_quote.clone().unwrap_or_default();
    // Task 1.7C (D6): the window is the SEED, sentence boundaries are the law, and
    // gutter numerals do not reach the display. Lives in its own module because
    // the rules are substantial enough to test on their own, and because this file
    // is close to the 300-line limit (Rule 17).
    let context = assemble_context(page_text, &quote_text, settings.quote_context_window_chars);

    let links: &[ExtrasLink] = extras.map(|e| e.links.as_slice()).unwrap_or(&[]);
    let stance = build_stance(links);

    let bears_on = build_bears_on(links);

    let band = band_for_score(ref_state.confidence, settings);
    let status = ref_state.status.unwrap_or(FactStatus::Undecided);
    let defer_required_reason = defer_reason_for(
        &quote_text,
        stance.as_ref(),
        !human_links.is_empty(),
        band != ConfidenceBand::Unscored,
    );

    // Task 2.10, ruling R2: the sentence a human-linked card shows instead of a
    // stance. Composed from the STORED template and the accusation labels — see
    // `scenario_human_links::link_summary` for why no canon verb can reach it.
    let human_link_summary = link_summary(human_links, &settings.wording);

    ScenarioCard {
        // The spelling of the handle moved to `domain::scenario_code` in task
        // 2.11, beside its `S-n` sibling, when a third surface needed to name a
        // fact. Two sites that must agree about a name humans say out loud is one
        // too many.
        code: ordinal.map(crate::domain::scenario_code::candidate_code),
        graph_node_id: instance.evidence_id.clone(),
        quote: build_quote(
            quote_text,
            context,
            instance.question.clone(),
            question_override,
            &settings
                .card_grammar_wording
                .question_machine_authorship_label,
        ),
        pinpoint: build_pinpoint(instance),
        speaker: build_speaker(
            instance,
            &settings.card_grammar_wording.speaker_extracted_label,
        ),
        statement_kind: extras
            .and_then(|e| e.statement_type.as_deref())
            .map(statement_kind_label),
        // Task 2.13: both come straight off the reference row. The card does not
        // invent a tier for a candidate with no row — "not in the scenario" and
        // "in it, unweighed" are different states and stay so on the wire.
        tier: ref_state.tier,
        sort_ordinal: ref_state.sort_ordinal,
        display_ordinal: ref_state.display_ordinal,
        stance,
        bears_on,
        grounding: build_grounding(extras),
        confidence: CardConfidence {
            band,
            label: band.label().to_string(),
        },
        status,
        status_label: status_label(status).to_string(),
        defer_required: defer_required_reason.is_some(),
        defer_required_reason,
        defer_reason: ref_state.defer_reason.clone(),
        // Present exactly when this card is a projection of an admitted verdict no
        // human has touched. The band above was computed from the SAME score, so a
        // proposed card's chip and its judgment can never describe different runs.
        proposed: ref_state.proposal.clone(),
        // Ruling R3: the reason belongs to the CARD, so it survives the ruling
        // that ends the proposal. One field, one render site, both wrappers.
        scan_reason: ref_state.scan_reason.clone(),
        human_links: human_links.to_vec(),
        human_link_summary,
    }
}

#[cfg(test)]
#[path = "scenario_card_tests.rs"]
mod tests;
