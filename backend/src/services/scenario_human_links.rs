//! What a HUMAN did to a card, and how the card says it (tasks 1.7F and 2.10).
//!
//! Everything here is pure, no I/O: which question a card shows and who wrote it,
//! which accusations a person linked it to, the sentence that reports the link,
//! and the count of how much of the stuck pile is cleared.
//!
//! ## Why this is its own module rather than part of `scenario_card`
//!
//! That file builds one card from what the GRAPH and the workbench know; this one
//! is the human's marks on it. Two different reasons to change — and
//! `scenario_card` sits at the 300-line limit (Rule 17), so the seam had to be
//! somewhere. This is where a reader would have drawn it: it is exactly the set
//! [`HumanTouches`] groups.
//!
//! `resolve_question` and `human_authorship_label` moved here from
//! `scenario_card` in task 2.10, unchanged. They were always the same kind of
//! thing as the link functions beside them.
//!
//! ## Standing Rule 1 at the read boundary
//!
//! A `cut` token this build does not define is a data-integrity fault. It is
//! logged with the ids that carry it and the link is DROPPED, never rendered with
//! a guessed cut — telling a lawyer that a landmine is a weapon is the worst thing
//! this vocabulary could do, so "I cannot read this" must not become "supports".
//! Dropping is visible twice over: the log line names it, and the card keeps its
//! defer-only refusal, so nobody rules on a link the system could not read.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::domain::link_cut::LinkCut;
use crate::domain::wording::Wording;
use crate::domain::wording_templates::render;
use crate::dto::scenario_card::{
    CardHumanLink, CardQuestionAuthorship, QuestionSource, ScenarioCard,
};
use crate::repositories::pipeline_repository::{
    EvidenceAllegationLinkRecord, EvidenceSummaryOverrideRecord,
};

/// What HUMANS have done to one candidate, gathered into one argument.
///
/// ## Why a struct and not two more parameters on `build_card`
///
/// That function was at seven arguments before task 2.10 and an eighth trips
/// `clippy::too_many_arguments`. Silencing the lint would have been the cheap
/// move; this is the honest one, because the two fields genuinely belong
/// together: everything in here is something a PERSON did to this card —
/// corrected its question, linked it to an accusation — as opposed to what the
/// extraction found and what the workbench recorded. A third human touch lands
/// here without changing the signature again.
///
/// It lives in THIS module rather than beside `build_card` for the size rule
/// (`scenario_card` sits at the 300-line limit), and the seam is the right one:
/// this file is what humans did, that one is what the card shows.
///
/// ## Rust Learning: `<'_>` — the anonymous lifetime
///
/// Both fields are borrows, so the struct needs a lifetime parameter. Call sites
/// write `HumanTouches<'_>` rather than naming one: the borrow lasts as long as
/// the data the caller already holds, and there is only one lifetime here for the
/// compiler to infer. Borrows rather than owned values because the caller reads
/// both maps ONCE for the whole pool and every card looks into them — copying a
/// link vector per card would allocate 148 times to read data that is not
/// changing.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HumanTouches<'a> {
    /// A human's replacement for the machine's question (task 1.7F Part B).
    pub question_override: Option<&'a EvidenceSummaryOverrideRecord>,
    /// The accusations a human linked this statement to (task 2.10). Empty is the
    /// ordinary state.
    pub links: &'a [CardHumanLink],
}

#[cfg(test)]
impl HumanTouches<'_> {
    /// A card no human has touched — the ordinary case, and most of the fixtures.
    ///
    /// `#[cfg(test)]` rather than a `Default` impl: production always has both
    /// maps in hand, and a reachable "no human touches" constructor is a way for
    /// a call site to accidentally drop the links it was given and re-lock a card
    /// a human had unlocked.
    pub(crate) fn none() -> Self {
        HumanTouches {
            question_override: None,
            links: &[],
        }
    }
}

/// Which question the card shows, and who wrote it (task 1.7F Part B).
///
/// ## Domain note: the human's words win on screen, never in the graph
///
/// An override REPLACES the machine's question for display and does nothing else.
/// `Evidence.question` is in Neo4j and this function only reads its value as an
/// argument — there is no path from here to a graph write, which is what makes
/// ruling R2 ("the machine original is untouchable") structural rather than a
/// promise. Reverting is the caller deleting the override row; this function then
/// composes the machine's words again, unchanged, because they were never edited.
///
/// ## Why an override with no machine question is still shown
///
/// A human can correct a question the extraction never produced — that is a
/// legitimate correction of an omission, not an inconsistency. So the presence of
/// the human's text, not the machine's, decides whether the card has a question
/// line at all.
pub(crate) fn resolve_question(
    machine: Option<String>,
    override_row: Option<&EvidenceSummaryOverrideRecord>,
) -> (Option<String>, Option<CardQuestionAuthorship>) {
    match override_row {
        Some(row) => (
            Some(row.summary_text.clone()),
            Some(CardQuestionAuthorship {
                source: QuestionSource::Human,
                label: human_authorship_label(&row.authored_by, row.updated_at),
            }),
        ),
        None => {
            let authorship = machine.as_ref().map(|_| CardQuestionAuthorship {
                source: QuestionSource::System,
                // The machine has no name and no date worth showing: the
                // extraction's own timestamp would date the RUN, not the sentence,
                // and a card claiming "written 16 July" about a question nobody
                // wrote would be a fact the payload cannot support.
                label: SYSTEM_AUTHORSHIP_LABEL.to_string(),
            });
            (machine, authorship)
        }
    }
}

// CONST: the badge's word for a question nobody has corrected. A control word in
// the same class as the state chip's "Not ruled" — it names the SYSTEM, not
// anything about this case, so another Colossus case renders it unchanged
// (Standing Rule 2's reusability checkpoint).
const SYSTEM_AUTHORSHIP_LABEL: &str = "System";

/// The badge for a human-corrected question: who, and when.
///
/// Composed here rather than in the browser because the language law puts every
/// display decision on this side of the wire (v2 §7 item 2) — and because a date
/// formatted client-side would read differently depending on the reader's locale,
/// which is not a property a legal record should have.
///
/// `4 Aug 2026` rather than an ISO stamp: the badge sits inline beside a sentence
/// a human is reading, not in a log.
pub(crate) fn human_authorship_label(authored_by: &str, updated_at: DateTime<Utc>) -> String {
    format!("{authored_by} · {}", updated_at.format("%-d %b %Y"))
}

/// Resolve the stored rows for a whole pool into display-ready links, by node.
///
/// `labels` maps an accusation id to its composed complaint-language line. A link
/// whose accusation is not in the map is DROPPED with a warning: the row points at
/// a node the graph no longer holds (a re-extraction orphan, the defect that cost
/// 26 rulings on 2026-07-24), and a chip reading "linked to
/// doc-…:allegation:6763a4b1" would be worse than saying nothing.
///
/// ## Rust Learning: why the return is a map of `Vec`, built in one pass
///
/// The caller composes up to 148 cards and needs each one's links. Handing back a
/// flat `Vec` would make that a linear scan per card — 148 × 300 comparisons for
/// a question one `HashMap` answers. `entry().or_default()` appends without a
/// prior lookup, so each row is placed in one hash.
pub(crate) fn resolve_links(
    rows: Vec<EvidenceAllegationLinkRecord>,
    labels: &HashMap<String, String>,
    wording: &Wording,
) -> HashMap<String, Vec<CardHumanLink>> {
    let mut out: HashMap<String, Vec<CardHumanLink>> = HashMap::new();

    for row in rows {
        let Ok(cut) = LinkCut::try_from(row.cut.as_str()) else {
            tracing::error!(
                graph_node_id = %row.graph_node_id,
                allegation_id = %row.allegation_id,
                cut = %row.cut,
                "evidence_allegation_links carries a cut token this build does not \
                 define; the link is not shown and the card stays defer-only \
                 rather than being rendered with a guessed cut"
            );
            continue;
        };

        let Some(label) = labels.get(&row.allegation_id) else {
            tracing::warn!(
                graph_node_id = %row.graph_node_id,
                allegation_id = %row.allegation_id,
                "a human link points at an accusation the graph no longer holds; \
                 it is not shown. A re-extraction orphaned it — task 2.5 re-attaches"
            );
            continue;
        };

        out.entry(row.graph_node_id)
            .or_default()
            .push(CardHumanLink {
                allegation_id: row.allegation_id,
                label: label.clone(),
                cut,
                cut_label: cut_label(wording, cut).to_string(),
            });
    }
    out
}

/// The button-form words for one cut.
///
/// The chip on a card reads as a label ("They'll use it against us"), not as the
/// middle of a sentence — `Wording::cut_phrase` is the other form, and the two are
/// separate stored rows precisely so nothing here has to change a human's case.
fn cut_label(wording: &Wording, cut: LinkCut) -> &str {
    match cut {
        LinkCut::Supports => &wording.link_cut_supports_label,
        LinkCut::Against => &wording.link_cut_against_label,
    }
}

/// The sentence a card shows once a human has linked it.
///
/// `None` for a card with no human links, which is what keeps the §7.5 slot
/// unambiguous: a card carries a stance, OR this sentence, OR a defer reason.
///
/// ## Domain note: no canon verb passes through here (ruling R2)
///
/// Everything in the output comes from two places — the stored template and the
/// accusation labels. There is no branch on an edge class, no call into
/// `stance_verb_for_edge`, and no way for one to be added without deleting this
/// comment: the machine said nothing about this statement, and the page does not
/// pretend otherwise. What it reports is what a PERSON did.
///
/// ## Why the cut of the FIRST link words the sentence
///
/// A human saves one cut at a time, so every link made in one action shares it.
/// A statement linked to ¶41 against and later to ¶92 supporting is a real (if
/// rare) state; the sentence takes the first link's cut because the list is
/// ordered by when they were added, so the sentence describes the same reading
/// the human started from. The chips beside it carry each link's own cut in full,
/// so nothing is hidden — this is a summary, and the detail is one line below it.
pub(crate) fn link_summary(links: &[CardHumanLink], wording: &Wording) -> Option<String> {
    let first = links.first()?;

    // Joined with a comma-space rather than a stored separator: this is
    // punctuation between repeated items, not vocabulary, and a configurable
    // list separator is a knob nobody will ever turn.
    let accusations = links
        .iter()
        .map(|l| l.label.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    Some(render(
        &wording.link_summary_template,
        &[
            ("allegations", accusations.as_str()),
            ("cut", wording.cut_phrase(first.cut)),
        ],
    ))
}

/// The progress line for the stuck pile, or `None` when nothing is stuck.
///
/// ## What the two numbers actually count
///
/// The denominator is every card the EXTRACTION did not link — the pile this
/// feature exists to clear (measured on DEV, 2026-08-04: 104 of S-2's 148). The
/// numerator is how many of those a human has since linked. A card the machine
/// linked itself was never stuck and is in neither number, which is why this is
/// not simply "cards with human links over pool size".
///
/// ## Domain note: pool truth, never keystrokes (the 1.7E-a ruling)
///
/// Both numbers are derived from the payload being served. A count of "links
/// saved this session" would read 0 after a refresh while the chips beside it said
/// otherwise — two surfaces of one screen disagreeing about the same fact.
pub(crate) fn link_progress<'a>(
    cards: impl Iterator<Item = &'a ScenarioCard>,
    wording: &Wording,
) -> Option<String> {
    let mut stuck = 0usize;
    let mut linked = 0usize;

    for card in cards {
        // "The machine did not link this one." `stance` is `Some` exactly when a
        // topical edge was found, so its absence IS the stuck condition — the same
        // test `defer_reason_for` applies, asked of the finished card so the two
        // cannot drift.
        if card.stance.is_some() {
            continue;
        }
        stuck += 1;
        if !card.human_links.is_empty() {
            linked += 1;
        }
    }

    if stuck == 0 {
        return None;
    }

    Some(render(
        &wording.link_progress_template,
        &[
            ("linked", linked.to_string().as_str()),
            ("total", stuck.to_string().as_str()),
        ],
    ))
}

#[cfg(test)]
#[path = "scenario_human_links_tests.rs"]
mod tests;
