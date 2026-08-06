//! Composing one ready scenario for the rehearsal page (task 2.11 B2).
//!
//! Pure — no I/O. Takes what the assembly gathered and returns the finished
//! blocks, so every sentence a witness reads is decided in one testable place.
//!
//! ## The honest-gap law, as this module's contract
//!
//! * The page renders only what a human PLACED. Nothing here reaches for the
//!   included pool to fill a block out.
//! * Every absence is a NAMED gap, from the store.
//! * The counts never inflate. A marked instance the record can no longer produce
//!   does **not** count toward "said N times" — the page cannot claim what it
//!   cannot load — but it always appears in the gap list and in the header's gap
//!   count (ruled 2026-08-06).
//! * **A collapsed section never hides a gap count.** The header line is composed
//!   here and travels beside the block, so folding is a client-side view of data
//!   that already carries its own honest summary.
//!
//! ## Why the counts are computed here and not from `AccusationState`
//!
//! `AccusationState::times_said` counts instances still INCLUDED in the scenario —
//! the right question for the working view, where a fact that exists but is
//! unreadable is still a fact a human placed. This surface asks a stricter one:
//! how many instances can this page actually PRODUCE? Measured on DEV, six of
//! S-2's fifty-two included refs no longer resolve in the record store, so the two
//! numbers genuinely differ and only the stricter one may be said aloud.

use std::collections::{HashMap, HashSet};

use crate::domain::settings::Settings;
use crate::domain::wording_rehearsal::RehearsalWording;
use crate::domain::wording_templates::render;
use crate::dto::rehearsal::{
    RehearsalAccusation, RehearsalGap, RehearsalHeaders, RehearsalInstance, RehearsalPoint,
    RehearsalScenario,
};
use crate::repositories::scenario_accusation_repository::RehearsalFactRow;
use crate::services::rehearsal_rows::{answer_of, first_line, kind_of, source_of, when_of, who_of};
use crate::services::rehearsal_timeline::build_timeline;
use crate::services::scenario_accusation::{AccusationGap, AccusationState};

/// The gap tokens this surface serves.
///
/// The first three mirror `scenario_accusation_panel`'s so one vocabulary
/// describes one law on both surfaces. The fourth is this surface's own, and it
/// is a different KIND of thing: the other three record a human's act, this one
/// records a system fault. Folding it into `accusation_removed` would blame a
/// human for a substrate failure and bury the signal the re-anchoring work needs
/// (ruled 2026-08-06).
pub const GAP_NO_ANSWER: &str = "no_answer_prepared";
pub const GAP_ACCUSATION_REMOVED: &str = "accusation_removed";
pub const GAP_ANSWER_REMOVED: &str = "answer_removed";
pub const GAP_INSTANCE_UNAVAILABLE: &str = "instance_unavailable";

/// Everything one scenario's render needs, gathered by the assembly.
#[derive(Debug)]
pub(crate) struct ScenarioInput<'a> {
    /// "S-2" — the only identifier this surface carries.
    pub code: String,
    pub title: &'a str,
    /// C1's one plain sentence, block 1.
    pub what_this_is: Option<&'a str>,
    /// The authored plain-words accusation. Never `attack_text`.
    pub accusation_text: Option<&'a str>,
    /// What a human marked and paired, and every gap between them.
    pub state: &'a AccusationState,
    /// The placed statements the record could produce, by graph node id.
    pub facts: &'a HashMap<String, RehearsalFactRow>,
    pub points: Vec<RehearsalPoint>,
    pub watch_for: Vec<String>,
    pub settings: &'a Settings,
}

/// Compose one ready scenario.
pub(crate) fn render_scenario(input: ScenarioInput<'_>) -> RehearsalScenario {
    let w = &input.settings.rehearsal_wording;
    let accusation = build_accusation(&input, w);
    let timeline = build_timeline(input.state, input.facts, input.settings);

    let headers = RehearsalHeaders {
        accusation: render(
            &w.accusation_header_template,
            &[
                ("times", accusation.instances.len().to_string().as_str()),
                ("gaps", accusation.gap_count.to_string().as_str()),
            ],
        ),
        timeline: render(
            &w.timeline_header_template,
            &[("entries", timeline.entries.len().to_string().as_str())],
        ),
        points: render(
            &w.points_header_template,
            &[
                ("shown", input.points.len().to_string().as_str()),
                (
                    "cap",
                    input.settings.talking_points_cap.to_string().as_str(),
                ),
            ],
        ),
        watch_for: render(
            &w.watch_header_template,
            &[("count", input.watch_for.len().to_string().as_str())],
        ),
    };

    RehearsalScenario {
        code: input.code,
        title: input.title.to_string(),
        what_this_is: input.what_this_is.map(str::to_string),
        what_this_is_gap: gap_when_absent(input.what_this_is, &w.what_gap),
        accusation,
        timeline,
        points_gap: gap_when_empty(input.points.is_empty(), &w.points_gap),
        points: input.points,
        watch_for_gap: gap_when_empty(input.watch_for.is_empty(), &w.watch_gap),
        watch_for: input.watch_for,
        headers,
    }
}

/// The named gap for an absent sentence — `None` when one is present.
fn gap_when_absent(value: Option<&str>, gap: &str) -> Option<String> {
    value.is_none().then(|| gap.to_string())
}

/// The named gap for an empty list — `None` when it has anything in it.
fn gap_when_empty(is_empty: bool, gap: &str) -> Option<String> {
    is_empty.then(|| gap.to_string())
}

/// Blocks 2 and 3 — the accusation, its instances, their answers, and the gaps.
fn build_accusation(input: &ScenarioInput<'_>, w: &RehearsalWording) -> RehearsalAccusation {
    let walked = walk_instances(input, w);

    let mut gaps = walked.gaps;
    // The Remove law's other half: a pairing whose ACCUSATION is gone. Derived
    // next door and passed through, because it is about a statement that is not
    // in this list at all — there is no row to hang it from.
    gaps.extend(input.state.gaps.iter().filter_map(|gap| match gap {
        AccusationGap::AccusationRemoved { .. } => Some(RehearsalGap {
            kind: GAP_ACCUSATION_REMOVED.to_string(),
            message: w.gap_accusation_removed.clone(),
        }),
        _ => None,
    }));

    let (count_line, no_instances_notice) =
        summary(walked.instances.len(), walked.documents, input.settings);

    RehearsalAccusation {
        text: input.accusation_text.map(str::to_string),
        text_gap: gap_when_absent(input.accusation_text, &w.accusation_text_gap),
        count_line,
        no_instances_notice,
        gap_count: gaps.len(),
        gaps,
        instances: walked.instances,
    }
}

/// What one walk of the marked instances produced.
///
/// Three results rather than a tuple, because two of them are counts and a
/// `(Vec, Vec, usize)` at a call site is three positions nobody can read.
struct WalkedInstances {
    instances: Vec<RehearsalInstance>,
    gaps: Vec<RehearsalGap>,
    /// How many DISTINCT documents the rendered instances sit in.
    documents: usize,
}

/// Walk what a human marked, building the rows and naming every absence.
///
/// Split from [`build_accusation`] for the function-size limit, and the split
/// earns itself: this is the loop where the honest-gap law is actually applied —
/// three of the four gap kinds are decided here, and so is the rule that an
/// instance the record cannot produce is named but never counted.
fn walk_instances(input: &ScenarioInput<'_>, w: &RehearsalWording) -> WalkedInstances {
    let mut instances = Vec::new();
    let mut gaps = Vec::new();
    // The documents the RENDERED instances sit in — gathered as the rows are
    // built, because the DTO deliberately carries no node id for a later pass to
    // look one up with. A statement whose document the record cannot name adds
    // nothing rather than inventing a source, so the count can only ever
    // undercount, which is the one direction the honest-gap law permits.
    let mut documents: HashSet<&str> = HashSet::new();

    for marked in &input.state.instances {
        let Some(fact) = usable_fact(&marked.anchor_graph_node_id, input.facts) else {
            // A marked instance the record store can no longer produce. It is NOT
            // rendered — the page shows only what it can show — and it is NOT
            // counted, but it is always named. Six of these were measured on S-2.
            gaps.push(RehearsalGap {
                kind: GAP_INSTANCE_UNAVAILABLE.to_string(),
                message: w.gap_instance_unavailable.clone(),
            });
            continue;
        };

        let (row, gap) = answered_row(instances.len() + 1, marked, fact, input.facts, w);
        if let Some(gap) = gap {
            gaps.push(gap);
        }
        if let Some(document_id) = fact.document_id.as_deref() {
            documents.insert(document_id);
        }
        instances.push(row);
    }

    WalkedInstances {
        instances,
        gaps,
        documents: documents.len(),
    }
}

/// One instance row, and the gap it leaves if nobody has answered it.
///
/// ## Why the sentence is composed once and lands in two places
///
/// On the ROW, because the design is explicit that an unanswered accusation says
/// so loudly where it is read. And in the block's GAP LIST, which is what the
/// header counts and what stays visible when the section is folded. One
/// composition, so the two can never disagree about what is missing.
fn answered_row(
    position: usize,
    marked: &crate::services::scenario_accusation::MarkedInstance,
    fact: &RehearsalFactRow,
    facts: &HashMap<String, RehearsalFactRow>,
    w: &RehearsalWording,
) -> (RehearsalInstance, Option<RehearsalGap>) {
    let answer = marked
        .answers_graph_node_id
        .as_deref()
        .and_then(|id| answer_of(id, facts, w));

    let gap = answer
        .is_none()
        .then(|| unanswered_gap(fact, marked.answers_graph_node_id.is_some(), w));

    let row = instance_row(
        position,
        fact,
        answer,
        gap.as_ref().map(|g| g.message.clone()),
        w,
    );
    (row, gap)
}

/// A placed statement the page can actually render, or `None`.
///
/// "Present in the map" is not enough: a node carrying no quote would render as an
/// empty pair of quotation marks attributed to a named person, which is worse than
/// saying the statement could not be loaded.
fn usable_fact<'a>(
    graph_node_id: &str,
    facts: &'a HashMap<String, RehearsalFactRow>,
) -> Option<&'a RehearsalFactRow> {
    let fact = facts.get(graph_node_id)?;
    let has_words = fact.quote.as_deref().is_some_and(|q| !q.trim().is_empty());
    has_words.then_some(fact)
}

/// The gap for an instance with no usable answer.
///
/// Two different absences, two different sentences: nobody paired anything (the
/// prep list), or somebody did and the item has since left (the Remove law). A
/// shared message would send a human to the wrong remedy.
fn unanswered_gap(fact: &RehearsalFactRow, was_paired: bool, w: &RehearsalWording) -> RehearsalGap {
    let (when, _) = when_of(fact, w);
    let when = when.unwrap_or_else(|| w.instance_when_gap.clone());
    let who = who_of(fact, w);

    if was_paired {
        return RehearsalGap {
            kind: GAP_ANSWER_REMOVED.to_string(),
            message: render(
                &w.gap_answer_removed,
                &[("who", who.as_str()), ("when", when.as_str())],
            ),
        };
    }

    RehearsalGap {
        kind: GAP_NO_ANSWER.to_string(),
        message: render(
            &w.gap_no_answer,
            &[
                ("who", who.as_str()),
                ("when", when.as_str()),
                ("where", source_of(fact, w).label.as_str()),
            ],
        ),
    }
}

/// One instance row.
fn instance_row(
    position: usize,
    fact: &RehearsalFactRow,
    answer: Option<crate::dto::rehearsal::RehearsalAnswer>,
    answer_gap: Option<String>,
    w: &RehearsalWording,
) -> RehearsalInstance {
    // Safe by construction: `usable_fact` refused a quote that is absent or blank,
    // and this is only reached through it.
    let quote = fact.quote.as_deref().unwrap_or_default().trim().to_string();
    let (when, when_gap) = when_of(fact, w);

    RehearsalInstance {
        position,
        who: who_of(fact, w),
        when,
        when_gap,
        source: source_of(fact, w),
        kind_label: kind_of(fact),
        quote_first_line: first_line(&quote),
        quote,
        answer_gap,
        answer,
    }
}

/// The count line, or the nothing-marked notice — exactly one of them.
///
/// The same contract `scenario_accusation_panel` holds on the working view, for
/// the same reason: both would say two things about one state, and neither would
/// leave the block silent about whether anything is placed — which is the
/// difference between "nobody has started" and "the read failed".
///
/// ## Why the count template is BORROWED from the working view's rows
///
/// "Said 5 times, in 5 documents." is already rehearsal-voiced — it names a fact
/// about the record and no internal vocabulary — so a second row saying the same
/// thing would be two sentences Roman has to keep in step by hand. Its three
/// SIBLINGS are not borrowed: B1's gap rows name a fact "C-14", which is working-
/// view vocabulary §10 keeps off this surface, so this module has its own.
fn summary(
    instances: usize,
    documents: usize,
    settings: &Settings,
) -> (Option<String>, Option<String>) {
    if instances == 0 {
        return (
            None,
            Some(settings.rehearsal_wording.no_instances_notice.clone()),
        );
    }

    (
        Some(render(
            &settings.accusation_wording.count_template,
            &[
                ("times", instances.to_string().as_str()),
                ("documents", documents.to_string().as_str()),
            ],
        )),
        None,
    )
}

#[cfg(test)]
#[path = "rehearsal_render_tests.rs"]
mod tests;
