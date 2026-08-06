//! Composing the accusation section's payload (task 2.11 B1).
//!
//! Pure — no I/O. Takes what the read path gathered and returns the finished DTO,
//! so every sentence this surface speaks is decided in one testable place.
//!
//! ## Why the sentences are composed HERE and not in the browser
//!
//! Two reasons, and both are scars.
//!
//! The language law (v2 §7): a client that concatenated "Said " + n + " times"
//! would be composing prose out of numbers, and the words would then live half in
//! the store and half in a component. Roman edits the store; he cannot edit the
//! component.
//!
//! And §10 exclusion has to be STRUCTURAL. A browser holding the count template
//! would need the instance list and the document map to fill it — and would then
//! be holding exactly the data the rehearsal page is forbidden to render. A
//! browser that receives "Said 3 times, in 2 documents." can render that and
//! nothing else.
//!
//! ## The honest-gap law, restated as this module's contract
//!
//! Exactly one of `count_line` and `no_instances_notice` is present. Never both —
//! that would say two things about one state. Never neither — that would leave the
//! section silent about whether anything is marked, which is the difference
//! between "nobody has started" and "the read failed", and those must never look
//! the same.

use std::collections::HashMap;

use crate::domain::scenario_code::candidate_code;
use crate::domain::wording::render;
use crate::domain::wording_accusation::AccusationWording;
use crate::dto::scenario_accusation::{
    AccusationGapDto, AccusationInstanceDto, AccusationPanelDto, AccusationWordingDto,
};
use crate::services::scenario_accusation::{AccusationGap, AccusationState};

/// The stable tokens a client may branch on, beside each gap's sentence.
///
/// ## Domain note: why the token is not the sentence
///
/// The prep list renders louder and first; the two broken-pairing gaps render as
/// repairs. A client that decided which was which by matching on "NO ANSWER" would
/// silently stop distinguishing them the first time Roman edited that row — and he
/// is invited to edit it, because what a gap is CALLED is the substance of the
/// honest-gap law.
pub const GAP_KIND_NO_ANSWER: &str = "no_answer_prepared";
pub const GAP_KIND_ACCUSATION_REMOVED: &str = "accusation_removed";
pub const GAP_KIND_ANSWER_REMOVED: &str = "answer_removed";

/// Everything the composition needs, gathered by the read path.
///
/// ## Rust Learning: a borrowed input struct instead of eight parameters
///
/// Every field is a reference, so nothing is cloned to call this and the caller
/// keeps ownership of what it read. The struct exists because eight positional
/// parameters of which three are maps is a call site nobody can read — and because
/// two `&HashMap<String, …>` next to each other are trivially swappable at a call
/// site and not swappable at all when they have names.
#[derive(Debug)]
pub struct PanelInput<'a> {
    /// The authored plain-words accusation, or `None` — the named gap.
    pub accusation_text: Option<&'a str>,
    /// The derived instances and gaps.
    pub state: &'a AccusationState,
    /// Candidate ordinals by graph node id, for the handles a human reads.
    pub ordinals: &'a HashMap<String, i32>,
    /// Which document each marked statement sits in.
    pub documents: &'a HashMap<String, String>,
    /// How many facts are included in the scenario — what the "nothing marked yet"
    /// notice counts, because "none marked" and "none marked out of forty-six"
    /// are different states and only the second says what to do next.
    pub included_count: usize,
    pub wording: &'a AccusationWording,
}

/// Compose the whole panel.
pub fn build_panel(input: PanelInput<'_>) -> AccusationPanelDto {
    let PanelInput {
        accusation_text,
        state,
        ordinals,
        documents,
        included_count,
        wording,
    } = input;

    let (count_line, no_instances_notice) =
        summary_lines(state, documents, included_count, wording);

    AccusationPanelDto {
        accusation_text: accusation_text.map(str::to_string),
        count_line,
        no_instances_notice,
        instances: instance_dtos(state, ordinals),
        gaps: state
            .gaps
            .iter()
            .map(|gap| to_gap_dto(gap, ordinals, wording))
            .collect(),
        wording: to_wording_dto(wording),
    }
}

/// The count line and the nothing-marked notice — exactly one of them.
///
/// Split out so the rule has a name and one place to be broken. Returning a
/// TUPLE rather than two calls is deliberate: two independent functions could each
/// decide `Some`, and the invariant would then live in whoever called them.
///
/// ## Rust Learning: `(Option<String>, Option<String>)` and what the type cannot say
///
/// The type permits all four combinations; only two are legal. An enum
/// (`Summary::Count(String) | Summary::Nothing(String)`) WOULD say it — and was
/// rejected because the DTO's two fields are the wire shape a client already
/// reads, so the enum would have to be unpacked back into them one line later. The
/// invariant is held by this function being the only producer, and pinned by
/// `exactly_one_summary_line_is_present_in_every_state`.
fn summary_lines(
    state: &AccusationState,
    documents: &HashMap<String, String>,
    included_count: usize,
    wording: &AccusationWording,
) -> (Option<String>, Option<String>) {
    let times = state.times_said();
    if times == 0 {
        // A count of zero is not a count; it is the notice, and the notice says
        // how many facts are waiting, which is what tells a human what to do next.
        return (
            None,
            Some(render(
                &wording.no_instances_notice,
                &[("included", included_count.to_string().as_str())],
            )),
        );
    }

    (
        Some(render(
            &wording.count_template,
            &[
                ("times", times.to_string().as_str()),
                (
                    "documents",
                    state.documents_spoken_in(documents).to_string().as_str(),
                ),
            ],
        )),
        None,
    )
}

/// The marked instances, with their handles and the answer-present flag.
fn instance_dtos(
    state: &AccusationState,
    ordinals: &HashMap<String, i32>,
) -> Vec<AccusationInstanceDto> {
    // The anchors whose paired ANSWER has left the scenario, taken from the
    // DERIVED gaps rather than re-decided here. One rule, one place: a second "is
    // it still present?" test would be free to disagree with the gap list on the
    // same screen, which is the two-surfaces-one-truth defect task 2.12 spent a
    // build on.
    let answer_missing: Vec<&str> = state
        .gaps
        .iter()
        .filter_map(|gap| match gap {
            AccusationGap::AnswerRemoved {
                anchor_graph_node_id,
                ..
            } => Some(anchor_graph_node_id.as_str()),
            _ => None,
        })
        .collect();

    state
        .instances
        .iter()
        .map(|instance| AccusationInstanceDto {
            code: code_for(&instance.anchor_graph_node_id, ordinals),
            answer_code: instance
                .answers_graph_node_id
                .as_deref()
                .and_then(|answer| code_for(answer, ordinals)),
            // Both halves are required: "there IS an answer" and "it is still
            // here". An unpaired instance has no answer at all and must not be
            // reported as having a present one merely because no gap names it.
            answer_present: instance.answers_graph_node_id.is_some()
                && !answer_missing.contains(&instance.anchor_graph_node_id.as_str()),
            graph_node_id: instance.anchor_graph_node_id.clone(),
            answers_graph_node_id: instance.answers_graph_node_id.clone(),
        })
        .collect()
}

/// The handle a human calls a fact by, or `None` when nothing has numbered it.
///
/// `None` rather than the node id: an id in a slot labelled "code" reads as a
/// handle and gets quoted as one out loud, which is worse than a blank.
fn code_for(graph_node_id: &str, ordinals: &HashMap<String, i32>) -> Option<String> {
    ordinals.get(graph_node_id).copied().map(candidate_code)
}

/// One derived gap as its stable token plus its stored sentence.
///
/// The `{code}` a message names is the ANCHOR's — the instance — in all three
/// cases, including `AnswerRemoved`, where the missing thing is the answer. That
/// is deliberate: the human acts on the instance (it needs a new answer), and
/// naming the departed item would send them looking for something that is no
/// longer in the scenario to find.
fn to_gap_dto(
    gap: &AccusationGap,
    ordinals: &HashMap<String, i32>,
    wording: &AccusationWording,
) -> AccusationGapDto {
    let (kind, anchor, template) = match gap {
        AccusationGap::NoAnswerPrepared {
            anchor_graph_node_id,
        } => (
            GAP_KIND_NO_ANSWER,
            anchor_graph_node_id,
            &wording.gap_no_answer,
        ),
        AccusationGap::AccusationRemoved {
            anchor_graph_node_id,
        } => (
            GAP_KIND_ACCUSATION_REMOVED,
            anchor_graph_node_id,
            &wording.gap_accusation_removed,
        ),
        AccusationGap::AnswerRemoved {
            anchor_graph_node_id,
            ..
        } => (
            GAP_KIND_ANSWER_REMOVED,
            anchor_graph_node_id,
            &wording.gap_answer_removed,
        ),
    };

    AccusationGapDto {
        kind: kind.to_string(),
        // An unnumbered fact falls back to its node id HERE, unlike `code_for`
        // above. A gap that cannot say what it is about is not a gap a human can
        // act on, and an ugly id they can search for beats a sentence with a hole
        // in it.
        message: render(
            template,
            &[(
                "code",
                code_for(anchor, ordinals).as_deref().unwrap_or(anchor),
            )],
        ),
        graph_node_id: anchor.clone(),
    }
}

/// The stored strings that cross the wire, and only those.
///
/// The six templates the server fills are absent by construction — there is no
/// field for them to travel in. See `dto::scenario_accusation`'s header.
fn to_wording_dto(w: &AccusationWording) -> AccusationWordingDto {
    AccusationWordingDto {
        section_heading: w.section_heading.clone(),
        section_hint: w.section_hint.clone(),
        text_label: w.text_label.clone(),
        text_placeholder: w.text_placeholder.clone(),
        text_missing_notice: w.text_missing_notice.clone(),
        text_edit_label: w.text_edit_label.clone(),
        text_save_label: w.text_save_label.clone(),
        text_clear_label: w.text_clear_label.clone(),
        text_cancel_label: w.text_cancel_label.clone(),
        mark_label: w.mark_label.clone(),
        unmark_label: w.unmark_label.clone(),
        pair_label: w.pair_label.clone(),
        repair_label: w.repair_label.clone(),
        unpair_label: w.unpair_label.clone(),
        answer_label: w.answer_label.clone(),
        picker_prompt: w.picker_prompt.clone(),
        picker_cancel_label: w.picker_cancel_label.clone(),
        picker_no_match_notice: w.picker_no_match_notice.clone(),
        picker_empty_notice: w.picker_empty_notice.clone(),
        gaps_heading: w.gaps_heading.clone(),
        no_gaps_notice: w.no_gaps_notice.clone(),
        save_failed_template: w.save_failed_template.clone(),
    }
}

#[cfg(test)]
#[path = "scenario_accusation_panel_tests.rs"]
mod tests;
