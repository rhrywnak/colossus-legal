//! The accusation, its marked instances, and every gap between them (task 2.11).
//!
//! ## The honest-gap law, as code
//!
//! REHEARSAL_VIEW_DESIGN_v2, binding: *the page renders only what a human
//! deliberately placed; every absence renders as a named gap; the machine never
//! fills in.* A law expressed only in a component is a law nobody can test, so
//! the gaps are DERIVED here, as data, and the render walks them.
//!
//! Three absences are named, and they are genuinely different things:
//!
//! * an instance nobody has answered — **the prep list**, and the design calls
//!   it the single most useful thing on the page;
//! * a pairing whose accusation has left the scenario;
//! * a pairing whose ANSWER has left the scenario.
//!
//! The last two exist because of the Remove law (ruled 2026-08-06): when either
//! side of a pairing leaves, the row is KEPT and shown as a gap rather than
//! cascade-deleted. A human's pairing must not vanish silently — and this
//! scenario has already been taught that lesson, with 26 saved references
//! pointing at nodes that no longer resolve.
//!
//! ## Why the count is derived here and not in the browser
//!
//! Two reasons, both scars. A count computed client-side is how "102 remaining"
//! came to sit beside "Not ruled (92)" on one screen. And §10 exclusion has to be
//! STRUCTURAL: if the browser computed the count it would need the instances, and
//! then it holds data the rehearsal page is forbidden to render.

use std::collections::{HashMap, HashSet};

use crate::domain::human_authored::HumanFactKind;

/// One marked accusation instance, with whatever answers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkedInstance {
    /// The statement that IS them making the accusation.
    pub anchor_graph_node_id: String,
    /// The record item a human paired as our answer, if one is paired.
    pub answers_graph_node_id: Option<String>,
}

/// An absence the page must NAME rather than hide.
///
/// ## Rust Learning: an enum of absences, not a bag of booleans
///
/// The alternative is flags on the instance — `unanswered`, `anchor_missing`,
/// `answer_missing` — which permits combinations that cannot occur and forces
/// every reader to work out which one to believe. A closed enum says there are
/// exactly three ways this can be incomplete, and a `match` on it is
/// exhaustiveness-checked when a fourth is added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccusationGap {
    /// Marked as an instance; nobody has paired an answer. The prep list.
    NoAnswerPrepared { anchor_graph_node_id: String },
    /// A pairing survives, but the accusation it answers is no longer in the
    /// scenario — the Remove law's visible broken pairing.
    AccusationRemoved { anchor_graph_node_id: String },
    /// A pairing survives, but the item a human paired as the ANSWER is no
    /// longer in the scenario.
    AnswerRemoved {
        anchor_graph_node_id: String,
        answers_graph_node_id: String,
    },
}

/// The authoring state of one scenario's accusation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccusationState {
    /// Instances still present in the scenario, with their answers.
    pub instances: Vec<MarkedInstance>,
    /// Every named absence, in a stable order.
    pub gaps: Vec<AccusationGap>,
}

impl AccusationState {
    /// How many instances the page may claim — never more than are placed.
    ///
    /// The design is explicit that the count never inflates: "said 2 times" with
    /// three gaps showing is the honest render. So this counts INSTANCES THAT
    /// ARE STILL HERE, and an instance whose statement has left the scenario is
    /// a gap rather than a tally mark.
    pub fn times_said(&self) -> usize {
        self.instances.len()
    }

    /// How many instances have no answer — the size of the prep list.
    pub fn unanswered(&self) -> usize {
        self.gaps
            .iter()
            .filter(|gap| matches!(gap, AccusationGap::NoAnswerPrepared { .. }))
            .count()
    }

    /// In how many DISTINCT documents the accusation was said.
    ///
    /// The second half of Roman's sentence — *"5 times in 5 different
    /// documents"* — and the half that carries the force: the same accusation
    /// repeated in one deposition is one witness on one day, and repeated across
    /// a deposition, a brief and an opinion is a pattern.
    ///
    /// `document_of` maps a graph node id to the document it sits in. An instance
    /// the map does not know about contributes NOTHING to this count and is still
    /// counted by [`times_said`](Self::times_said) — the two numbers are allowed
    /// to disagree in that direction, and only that direction.
    ///
    /// ## Why an unknown document undercounts rather than being guessed
    ///
    /// The honest-gap law says the count never inflates. Counting an instance
    /// whose document nobody could resolve as "one more document" would claim a
    /// source the page cannot produce — and the design's first research finding is
    /// that an examiner who cannot produce the source on the spot loses
    /// credibility. Undercounting is the safe direction; over-claiming is not.
    ///
    /// ## Rust Learning: `HashSet` from an iterator, and why not `.count()`
    ///
    /// `.filter_map(…).collect::<HashSet<_>>().len()` is a DISTINCT count.
    /// `.filter_map(…).count()` would count instances, not documents, and would
    /// report "said 5 times, in 5 documents" for five sentences on one page — the
    /// exact overstatement this method exists to avoid.
    pub fn documents_spoken_in(&self, document_of: &HashMap<String, String>) -> usize {
        self.instances
            .iter()
            .filter_map(|instance| document_of.get(&instance.anchor_graph_node_id))
            .collect::<HashSet<&String>>()
            .len()
    }
}

/// One stored judgment row, as this derivation needs to see it.
///
/// Deliberately narrow: the derivation must not be able to reach a human's
/// prose, an author or a timestamp, because none of those may cross into the
/// rehearsal payload (§10).
#[derive(Debug, Clone)]
pub struct StoredJudgment {
    pub kind: HumanFactKind,
    pub anchor_graph_node_id: String,
    pub answers_graph_node_id: Option<String>,
}

/// Derive the instances and every gap, given what is stored and what is present.
///
/// `included` is the set of graph node ids currently INCLUDED in the scenario —
/// the only things that may appear on the page. Everything a judgment points at
/// is checked against it, which is what turns the Remove law from a promise into
/// a rendered gap.
///
/// Order is stable and deliberate: instances by anchor id, then gaps grouped by
/// kind. Two renders of the same data can never disagree, and the prep list
/// (`NoAnswerPrepared`) comes first because it is the list a human acts on.
pub fn derive(judgments: &[StoredJudgment], included: &HashSet<String>) -> AccusationState {
    let pairings = pairings_of(judgments);
    let marked = marked_anchors(judgments);

    let (instances, unanswered, answer_removed) = walk_marked(&marked, &pairings, included);
    let accusation_removed = orphaned_pairings(&pairings, &marked, included);

    // Prep list first — it is the list a human ACTS on, and the design calls it
    // the single most useful thing on the page. The two broken-pairing kinds
    // follow, each already in anchor order.
    let mut gaps = unanswered;
    gaps.extend(answer_removed);
    gaps.extend(accusation_removed);

    AccusationState { instances, gaps }
}

/// The answer paired to each anchor, whatever became of either side.
///
/// Deliberately not filtered by `included`: a pairing whose accusation or answer
/// has left is exactly what the Remove law keeps, and dropping it here would make
/// it vanish before anything could name it.
fn pairings_of(judgments: &[StoredJudgment]) -> HashMap<&str, &str> {
    judgments
        .iter()
        .filter(|j| j.kind == HumanFactKind::AnswerPairing)
        .filter_map(|j| {
            j.answers_graph_node_id
                .as_deref()
                .map(|answer| (j.anchor_graph_node_id.as_str(), answer))
        })
        .collect()
}

/// Every anchor a human marked, sorted and de-duplicated.
///
/// Sorting here is what makes the whole derivation TOTAL: a `HashMap` walk would
/// reshuffle the prep list between two reloads of the same data, and a human
/// working down a list they expect to stay put would lose their place. The dedup
/// is belt and braces — the partial unique index already forbids two marks on one
/// anchor — but a duplicate would silently double the count, which is the one
/// number this surface promises never to inflate.
fn marked_anchors(judgments: &[StoredJudgment]) -> Vec<&str> {
    let mut marked: Vec<&str> = judgments
        .iter()
        .filter(|j| j.kind == HumanFactKind::AccusationInstance)
        .map(|j| j.anchor_graph_node_id.as_str())
        .collect();
    marked.sort_unstable();
    marked.dedup();
    marked
}

/// The instances still present, and the two gap kinds that hang off them.
///
/// Returns the instances, the prep list, and the answers that have left — three
/// lists rather than one, because they land in different places in the final
/// order and merging them here would lose that.
fn walk_marked(
    marked: &[&str],
    pairings: &HashMap<&str, &str>,
    included: &HashSet<String>,
) -> (Vec<MarkedInstance>, Vec<AccusationGap>, Vec<AccusationGap>) {
    let mut instances = Vec::new();
    let mut unanswered = Vec::new();
    let mut answer_removed = Vec::new();

    for anchor in marked {
        if !included.contains(*anchor) {
            // The instance itself is gone. Whether or not it had an answer, the
            // honest thing on screen is that the statement left — reporting the
            // answer as intact would describe a pairing to nothing.
            continue;
        }
        match pairings.get(anchor) {
            None => unanswered.push(AccusationGap::NoAnswerPrepared {
                anchor_graph_node_id: (*anchor).to_string(),
            }),
            Some(answer) if !included.contains(*answer) => {
                // The pairing survives and says so — the Remove law. The instance
                // still counts as said; what is missing is the answer.
                answer_removed.push(AccusationGap::AnswerRemoved {
                    anchor_graph_node_id: (*anchor).to_string(),
                    answers_graph_node_id: (*answer).to_string(),
                });
            }
            Some(_) => {}
        }
        instances.push(MarkedInstance {
            anchor_graph_node_id: (*anchor).to_string(),
            answers_graph_node_id: pairings.get(anchor).map(|a| (*a).to_string()),
        });
    }

    answer_removed.sort_by(|a, b| gap_key(a).cmp(gap_key(b)));
    (instances, unanswered, answer_removed)
}

/// Pairings whose ACCUSATION is not a present, marked instance.
///
/// Two ways to get here, and both are a human's judgment left standing: the
/// statement left the scenario, or it is back but nobody has re-marked it — which
/// a re-include after a removal produces. Either way the pairing is still
/// somebody's decision and still has to be visible.
fn orphaned_pairings(
    pairings: &HashMap<&str, &str>,
    marked: &[&str],
    included: &HashSet<String>,
) -> Vec<AccusationGap> {
    let marked_set: HashSet<&str> = marked.iter().copied().collect();
    let mut out: Vec<AccusationGap> = pairings
        .keys()
        .filter(|anchor| !included.contains(**anchor) || !marked_set.contains(**anchor))
        .map(|anchor| AccusationGap::AccusationRemoved {
            anchor_graph_node_id: (*anchor).to_string(),
        })
        .collect();
    // The source is a `HashMap`'s keys, whose iteration order is randomized per
    // process. Without this sort two backends would serve the same gaps in
    // different orders, and so would one backend across a restart.
    out.sort_by(|a, b| gap_key(a).cmp(gap_key(b)));
    out
}

/// The anchor a gap is about, for a stable sort.
fn gap_key(gap: &AccusationGap) -> &str {
    match gap {
        AccusationGap::NoAnswerPrepared {
            anchor_graph_node_id,
        }
        | AccusationGap::AccusationRemoved {
            anchor_graph_node_id,
        }
        | AccusationGap::AnswerRemoved {
            anchor_graph_node_id,
            ..
        } => anchor_graph_node_id,
    }
}

#[cfg(test)]
#[path = "scenario_accusation_tests.rs"]
mod tests;
