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
use crate::domain::wording_rehearsal_chrome::RehearsalChromeWording;
use crate::domain::wording_templates::render;
use crate::dto::rehearsal::{
    RehearsalAccusation, RehearsalGap, RehearsalHeaders, RehearsalInstance, RehearsalPoint,
    RehearsalScenario, RehearsalWatchItem,
};
use crate::repositories::scenario_accusation_repository::RehearsalFactRow;
use crate::services::rehearsal_rows::{
    answer_of, attribution_line, first_line, kind_of, source_of, when_of, who_of,
};
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
        plain_count_line: plain_count_line(&walked.instances, walked.documents, input.settings),
        answered_line: answered_line(&walked.instances, input.settings),
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

    // CHRONOLOGY (task R3): oldest first, undated LAST.
    //
    // This section IS the timeline now — the separate TIMELINE block is gone — so
    // the order a witness reads these in has to be the order they happened. The
    // marked list arrives in the order a human clicked, which is not that.
    //
    // ## Domain note: why sorting on the STORED string is correct here
    //
    // Dates in this record are variable precision — measured on DEV, 228 of 525
    // evidence nodes carry one and they range from `"2005"` to `"2015-10"` to a
    // full day. ISO-shaped prefixes sort as time (`"2009-12" < "2011-01"`), which
    // is the same property `rehearsal_timeline` already relied on. Formatting
    // first and sorting after would order them alphabetically — `"1 Dec 2009"`
    // before `"15 Nov 2009"` — which is the defect that module documents.
    //
    // Undated instances sort last rather than first: 57% of this case's evidence
    // has no date, and a block of undated statements at the TOP would bury the
    // chronology the section exists to show. At the bottom they read as what they
    // are — the ones still needing a date, which the working page is where to add.
    let mut marked_in_order: Vec<_> = input.state.instances.iter().collect();
    marked_in_order.sort_by(|a, b| {
        let key = |m: &&crate::services::scenario_accusation::MarkedInstance| {
            usable_fact(&m.anchor_graph_node_id, input.facts)
                .and_then(|f| f.occurred_on.clone())
                .filter(|d| !d.trim().is_empty())
        };
        match (key(a), key(b)) {
            (Some(x), Some(y)) => x.cmp(&y),
            // `None` is "no date yet" and goes after every dated row.
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            // Two undated rows keep the order a human placed them in.
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    for marked in marked_in_order {
        let Some(fact) = usable_fact(&marked.anchor_graph_node_id, input.facts) else {
            // A marked instance the record store can no longer produce. It is NOT
            // rendered — the page shows only what it can show — and it is NOT
            // counted, but it is always named. Six of these were measured on S-2.
            gaps.push(RehearsalGap {
                kind: GAP_INSTANCE_UNAVAILABLE.to_string(),
                message: w.gap_instance_unavailable.clone(),
                // Not rendered, so there is nothing to jump to. A position here
                // would scroll a reader to somebody else's statement.
                position: None,
            });
            continue;
        };

        let (row, gap) = answered_row(
            instances.len() + 1,
            marked,
            fact,
            input.facts,
            w,
            &input.settings.rehearsal_chrome_wording,
            input.settings,
        );
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
/// ## Why the SENTENCE lands in one place and a SHORT form in the other
///
/// Until beta.381 the composed gap sentence went onto the row AND into the gap
/// list — the same "NO ANSWER PREPARED — who, when, where" twice on one screen,
/// filed as a defect and killed by ruling C5. The sentence now lands only in the
/// list, which is what the header counts and what stays visible when the section
/// is folded. The row carries three words that cannot grow back into it.
fn answered_row(
    position: usize,
    marked: &crate::services::scenario_accusation::MarkedInstance,
    fact: &RehearsalFactRow,
    facts: &HashMap<String, RehearsalFactRow>,
    w: &RehearsalWording,
    chrome: &RehearsalChromeWording,
    settings: &Settings,
) -> (RehearsalInstance, Option<RehearsalGap>) {
    let answer = marked
        .answers_graph_node_id
        .as_deref()
        .and_then(|id| answer_of(id, facts, w));

    let gap = answer.is_none().then(|| {
        let mut gap = unanswered_gap(fact, marked.answers_graph_node_id.is_some(), w);
        // The row this entry is about, so the prep list can carry a link to it.
        gap.position = Some(position);
        gap
    });

    let row = instance_row(position, fact, answer, w, chrome, settings);
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
            // Filled by the caller, which is the only place that knows the row
            // number. Never left as `None` for these two kinds — see the caller.
            position: None,
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
        position: None,
    }
}

/// One instance row.
fn instance_row(
    position: usize,
    fact: &RehearsalFactRow,
    answer: Option<crate::dto::rehearsal::RehearsalAnswer>,
    w: &RehearsalWording,
    chrome: &RehearsalChromeWording,
    settings: &Settings,
) -> RehearsalInstance {
    // Safe by construction: `usable_fact` refused a quote that is absent or blank,
    // and this is only reached through it.
    let quote = fact.quote.as_deref().unwrap_or_default().trim().to_string();
    let (when, when_gap) = when_of(fact, w);
    let answered = answer.is_some();

    RehearsalInstance {
        position,
        // Forum first, then the date — see `rehearsal_phase`. Always a label:
        // a card with no chip in a filtered list reads as a rendering fault.
        phase: crate::services::rehearsal_phase::phase_of(
            fact.document_id.as_deref(),
            fact.occurred_on.as_deref(),
            settings,
        ),
        who: who_of(fact, w),
        when,
        when_gap,
        source: source_of(fact, w),
        kind_label: kind_of(fact),
        quote_first_line: first_line(&quote),
        quote,
        // Chosen HERE rather than in the browser: two labels and a boolean is a
        // choice a client can get backwards, and backwards means a green
        // ANSWERED over a row nobody has answered.
        answer_tag: if answered {
            chrome.answered_tag.clone()
        } else {
            chrome.no_answer_tag.clone()
        },
        answer_banner: (!answered).then(|| chrome.no_answer_banner.clone()),
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

/// The prep page's count line, in plain words and grammatically correct at ONE.
///
/// "They said it 5 times, in 3 documents, from December 2009 through October
/// 2015 — every one is below, with your answer under it."
///
/// ## Domain note: why this is composed here rather than by a template system
///
/// The general singular/plural template system was deferred out of .391, and this
/// page cannot wait for it: it opens with this sentence, and "They said it 1
/// times" on the surface a witness preps from is the kind of thing a reader stops
/// trusting the whole page over. So the two count-bearing clauses carry BOTH forms
/// as their own settings rows and this function picks, which is the narrow version
/// of the same idea — and the rows a general system would eventually read.
///
/// ## Why the date clause can be absent
///
/// 57% of this case's evidence carries no date at all (measured on DEV). A range
/// needs two endpoints; with none, the clause is OMITTED rather than rendered
/// with an invented span or an em dash. A sentence that quietly drops a clause it
/// has no facts for is honest; one that says "from — through —" is not.
///
/// When every dated instance shares one date the clause says "on <date>" rather
/// than "from X through X", which is a sentence nobody would write by hand.
fn plain_count_line(
    instances: &[RehearsalInstance],
    documents: usize,
    settings: &Settings,
) -> Option<String> {
    if instances.is_empty() {
        return None;
    }
    let w = &settings.rehearsal_wording;

    let times = pick_form(instances.len(), &w.count_times_one, &w.count_times_many);
    let docs = pick_form(documents, &w.count_documents_one, &w.count_documents_many);

    // The endpoints come from the list, which `walk_instances` has already sorted
    // oldest-first with the undated at the end — so the first dated row is the
    // earliest and the last dated row is the latest. Re-deriving them here would
    // be a second ordering free to disagree with the one on screen.
    let dated: Vec<&str> = instances
        .iter()
        .filter_map(|i| i.when.as_deref())
        .filter(|d| !d.trim().is_empty())
        .collect();

    let span = match (dated.first(), dated.last()) {
        (Some(first), Some(last)) if first == last => {
            Some(render(&w.count_span_one_date, &[("date", first)]))
        }
        (Some(first), Some(last)) => Some(render(
            &w.count_span_range,
            &[("from", first), ("through", last)],
        )),
        _ => None,
    };

    Some(render(
        &w.count_line_template,
        &[
            ("times", times.as_str()),
            ("documents", docs.as_str()),
            // An absent span leaves the slot empty rather than the sentence
            // half-built; the stored template puts the clause between commas so
            // an empty one closes up cleanly.
            ("span", span.unwrap_or_default().as_str()),
        ],
    ))
}

/// "5 of 5 answered" — or "3 of 5 answered — 2 to prepare" when work remains.
///
/// ## Domain note: the second clause is the point of the line
///
/// A witness reading "3 of 5 answered" has to do the subtraction to learn what is
/// left, and the number they are subtracting toward is the one they came for. The
/// preparing clause says it outright, and it is ABSENT when nothing is
/// outstanding — "5 of 5 answered — 0 to prepare" reads as a to-do list with an
/// empty item on it.
fn answered_line(instances: &[RehearsalInstance], settings: &Settings) -> Option<String> {
    if instances.is_empty() {
        return None;
    }
    let w = &settings.rehearsal_wording;
    let answered = instances.iter().filter(|i| i.answer.is_some()).count();
    let total = instances.len();
    let remaining = total - answered;

    let template = if remaining == 0 {
        &w.answered_line_all
    } else {
        &w.answered_line_some
    };
    Some(render(
        template,
        &[
            ("answered", answered.to_string().as_str()),
            ("total", total.to_string().as_str()),
            ("remaining", remaining.to_string().as_str()),
        ],
    ))
}

/// One or many, with the count already in the stored form.
///
/// Both forms are rows, so "1 time" / "5 times" are Roman's to word — including
/// in a language where the split is not at one. This only chooses.
fn pick_form(n: usize, one: &str, many: &str) -> String {
    let template = if n == 1 { one } else { many };
    render(template, &[("n", n.to_string().as_str())])
}

#[cfg(test)]
#[path = "rehearsal_render_tests.rs"]
mod tests;
