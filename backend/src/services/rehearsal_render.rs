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

use std::collections::HashMap;

use crate::domain::settings::Settings;
use crate::domain::wording_rehearsal::RehearsalWording;
use crate::domain::wording_templates::render;
use crate::dto::rehearsal::{
    RehearsalAccusation, RehearsalGap, RehearsalHeaders, RehearsalPoint, RehearsalScenario,
    RehearsalWatchItem,
};
use crate::repositories::scenario_accusation_repository::RehearsalFactRow;
use crate::services::rehearsal_count as count;
use crate::services::rehearsal_instances::walk_instances;
use crate::services::rehearsal_rows::attribution_line;
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
    /// "S-2" — the identifier a reader sees.
    pub code: String,
    /// The address the page's editors write to (ruling C1). Renders nowhere.
    pub scenario_id: String,
    pub title: &'a str,
    /// C1's one plain sentence, block 1.
    pub what_this_is: Option<&'a str>,
    /// Who wrote it and when, as stored. `(None, None)` for every sentence
    /// written before task 2.11 C — never backfilled, never guessed.
    pub what_authored: Authored<'a>,
    /// The authored plain-words accusation. Never `attack_text`.
    pub accusation_text: Option<&'a str>,
    pub accusation_authored: Authored<'a>,
    /// What a human marked and paired, and every gap between them.
    pub state: &'a AccusationState,
    /// The placed statements the record could produce, by graph node id.
    pub facts: &'a HashMap<String, RehearsalFactRow>,
    pub points: Vec<RehearsalPoint>,
    pub watch_for: Vec<RehearsalWatchItem>,
    /// Offense or defense, as the stored word rather than the token (task R3).
    /// The prep page states it in the identity line; a client translating the
    /// token would be a second vocabulary for one fact.
    pub direction_label: String,
    /// The attack in THEIR words, verbatim — `definition->>attack_text`.
    ///
    /// Folded away under the plain-words accusation. `None` when nobody has
    /// written one, which renders no fold control at all rather than an empty one.
    pub attack_text: Option<String>,
    /// The complaint paragraphs this scenario bears on, as A-codes.
    pub bears_on: Vec<String>,
    pub settings: &'a Settings,
}

/// Who wrote a sentence, and when — exactly as the store holds it.
///
/// ## Rust Learning: a two-field struct instead of `(Option<&str>, Option<...>)`
///
/// Both fields are optional and both are "about the author", so a tuple would be
/// two `Option`s at a call site with nothing but position to tell them apart —
/// and swapping them would still compile if the types ever converged. Naming them
/// costs four lines and makes `ScenarioInput`'s two attribution fields readable
/// at a glance.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Authored<'a> {
    pub by: Option<&'a str>,
    pub at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Compose one ready scenario.
pub(crate) fn render_scenario(input: ScenarioInput<'_>) -> RehearsalScenario {
    let w = &input.settings.rehearsal_wording;
    let chrome = &input.settings.rehearsal_chrome_wording;
    let accusation = build_accusation(&input, w);
    let timeline = build_timeline(input.state, input.facts, input.settings);
    let headers = section_headers(&accusation, &timeline, &input);

    RehearsalScenario {
        code: input.code,
        scenario_id: input.scenario_id,
        title: input.title.to_string(),
        direction_label: input.direction_label,
        attack_text: input.attack_text,
        bears_on: input.bears_on,
        // The line is composed only when there IS a sentence to attribute. An
        // authorship line over a named gap would attribute an absence.
        what_this_is_attribution: input.what_this_is.map(|_| {
            attribution_line(
                &chrome.what_attribution_template,
                &chrome.attribution_unknown_notice,
                input.what_authored.by,
                input.what_authored.at,
            )
        }),
        what_this_is: input.what_this_is.map(str::to_string),
        what_this_is_gap: gap_when_absent(input.what_this_is, &w.what_gap),
        // Decided here, once, over the count the page will actually render — see
        // `RehearsalScenario::instances_start_expanded` for why the browser gets
        // the answer rather than the rule.
        instances_start_expanded: accusation.instances.len()
            <= input.settings.rehearsal_instance_rows_expand_max,
        accusation,
        timeline,
        points_gap: gap_when_empty(input.points.is_empty(), &w.points_gap),
        points: input.points,
        watch_for_gap: gap_when_empty(input.watch_for.is_empty(), &w.watch_gap),
        watch_for: input.watch_for,
        headers,
    }
}

/// The four count lines that stay visible when a section is folded.
///
/// Split from [`render_scenario`] for the function-size limit, and the seam is a
/// real one: this is the honest-gap law's answer to the collapsible-section
/// hazard, in one place. Every number here is counted from what the page will
/// ACTUALLY render — never from the pool, never from a list a client might
/// filter.
fn section_headers(
    accusation: &RehearsalAccusation,
    timeline: &crate::dto::rehearsal::RehearsalTimeline,
    input: &ScenarioInput<'_>,
) -> RehearsalHeaders {
    let w = &input.settings.rehearsal_wording;
    RehearsalHeaders {
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
            // No row to jump to: this gap is about a statement that is not in the
            // list at all.
            position: None,
        }),
        _ => None,
    }));

    let (count_line, no_instances_notice) =
        summary(walked.instances.len(), walked.documents, input.settings);

    let chrome = &input.settings.rehearsal_chrome_wording;

    RehearsalAccusation {
        text: input.accusation_text.map(str::to_string),
        text_gap: gap_when_absent(input.accusation_text, &w.accusation_text_gap),
        attribution: input.accusation_text.map(|_| {
            attribution_line(
                &chrome.accusation_attribution_template,
                &chrome.attribution_unknown_notice,
                input.accusation_authored.by,
                input.accusation_authored.at,
            )
        }),
        count_line,
        no_instances_notice,
        // The prep page's opening sentence and its section count. Composed here,
        // beside the numbers they describe, so neither can be re-derived by a
        // client from a count it would have to interpret.
        plain_count_line: count::plain_count_line(
            &walked.instances,
            walked.documents,
            input.settings,
        ),
        answered_line: count::answered_line(&walked.instances, input.settings),
        gap_count: gaps.len(),
        gaps,
        instances: walked.instances,
    }
}

/// Walk what a human marked, building the rows and naming every absence.
///
/// Split from [`build_accusation`] for the function-size limit, and the split
/// earns itself: this is the loop where the honest-gap law is actually applied —
/// three of the four gap kinds are decided here, and so is the rule that an
/// view vocabulary §10 keeps off this surface, so this module has its own.
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
