//! The rehearsal payload — seven blocks, rendered honestly (task 2.11 B2).
//!
//! Replaces the four-block payload of task 1.5. What changed and why is in
//! REHEARSAL_VIEW_DESIGN_v2 (signed 2026-08-06): the old page showed one raw
//! quote under "What they say", an empty theme mislabelled "Our answer", and none
//! of the evidence a human had ruled into the scenario. This one is built ON that
//! evidence.
//!
//! ## The exclusion law is still enforced by CONSTRUCTION
//!
//! These types have no field for a verdict, a confidence, a tier, a sort ordinal,
//! a status, a motivation, a strategy note, a §2d annotation, or a database id.
//! A future edit that wanted to leak one would have to ADD a field — a visible
//! change to this file rather than a line slipped into a mapper — and
//! `the_payload_carries_nothing_it_is_excluded_from` then scans the serialized
//! bytes, so a leak smuggled inside a string is caught too.
//!
//! ## What CHANGED in that law, and the ruling behind it
//!
//! Until 2026-08-06 the banned list also held `document_id` and `page`, read out
//! of §10's "pinpoint impeachment sourcing". The architect ruled that
//! REHEARSAL_VIEW_DESIGN_v2 — later, specific, and signed with the
//! "Deposition, p. 42 · [open]" table in it — SUPERSEDES that reading **for
//! instance and answer rows only**.
//!
//! The distinction that makes it coherent: §10 exists to keep impeachment
//! MACHINERY off a witness surface — the grading, the confidence, the verdict,
//! the strategy. A citation that lets Marie produce the document she is about to
//! be asked about is not machinery; the research this design rests on is explicit
//! that a witness or examiner who cannot produce the source on the spot loses
//! credibility. So [`RehearsalInstance`] and [`RehearsalAnswer`] carry a source;
//! [`RehearsalPoint`] deliberately still does not — a talking point is HER words,
//! and attaching a page to it would drag the record into her mouth.
//!
//! FRE 612: sources on a witness-prep surface are discoverable prep material.
//! Roman ruled she gets the context; Chuck reviews when he engages. Recorded, not
//! waiting.
//!
//! ## Everything arrives composed
//!
//! Every sentence, every count line, every gap, every section header. The browser
//! renders and concatenates nothing — and holds none of the templates, so it
//! could not recompose them if it tried.

use serde::{Deserialize, Serialize};

/// One of Marie's talking points, optionally paired with the exhibit behind it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalPoint {
    /// Her words, verbatim as authored.
    pub text: String,
    /// The one exhibit that backs this point, in HER phrasing — "My certified
    /// letter". Authored (`response_item_fact_refs.note`), never a document title
    /// assembled from the record: deriving it would put words in the witness's
    /// mouth. `None` until the pairing editor exists (tracker task 3.9).
    pub exhibit: Option<String>,
}

/// Where a statement was made, and how to open it.
///
/// Present on instances and answers only — see this module's header for the
/// ruling that permits it here and forbids it on a talking point.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalSource {
    /// "Hearing to approve plan, p. 24" — composed from the stored template.
    pub label: String,
    /// The viewer address, already at the right page.
    pub href: String,
    /// The control's word, from the store.
    pub open_label: String,
}

/// What we said back to one instance — paired by a human, never guessed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalAnswer {
    pub who: String,
    /// The date, or `None` with `when_gap` saying the record carries none.
    pub when: Option<String>,
    pub when_gap: Option<String>,
    pub source: RehearsalSource,
    pub quote: String,
}

/// One marked instance of them making the accusation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalInstance {
    /// The row number a human reads — 1, 2, 3.
    ///
    /// ## Why this is `position` and not `ordinal`
    ///
    /// It is the printed position in this list, computed at render. `ordinal` is
    /// this codebase's word for a STORED internal number (`sort_ordinal`,
    /// `code_ordinal`), and §10 keeps those off this surface. Naming it
    /// `position` means the banned word cannot creep back in through a field that
    /// looked innocent.
    pub position: usize,
    /// Who said it, or the stored sentence saying the record does not record it.
    pub who: String,
    pub when: Option<String>,
    /// Named rather than left blank — and never inherited from the document's
    /// title, which on this case carries years that disagree with the statements
    /// inside it.
    pub when_gap: Option<String>,
    pub source: RehearsalSource,
    /// "attorney argument" — humanized, never translated (the set is open).
    pub kind_label: String,
    /// The statement, verbatim.
    pub quote: String,
    /// The quote's opening, for the collapsed row.
    ///
    /// Composed HERE so the truncation rule is tested in one place. A browser
    /// slicing prose would produce a different cut for the same quote the moment
    /// two components disagreed about the length.
    pub quote_first_line: String,
    /// What we said back, when a human has paired something.
    pub answer: Option<RehearsalAnswer>,
    /// The loud line when nobody has. The prep list, per row.
    pub answer_gap: Option<String>,
}

/// One named absence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalGap {
    /// A stable token the client branches on — never the sentence, which Roman
    /// is invited to edit.
    pub kind: String,
    /// The stored sentence, already naming who/when/where.
    pub message: String,
}

/// The accusation and every time they made it — the page's central block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalAccusation {
    /// The standing accusation in a human's plain words.
    pub text: Option<String>,
    /// The stored gap sentence when nobody has written one. Never a quote from
    /// the record standing in for it — that was the defect this task ends.
    pub text_gap: Option<String>,
    /// "Said 5 times, in 5 documents." — `None` when nothing is marked.
    pub count_line: Option<String>,
    /// The nothing-marked notice. Exactly one of these two is ever present.
    pub no_instances_notice: Option<String>,
    /// Chronological — oldest first. The design's force is the repetition over
    /// time, so the order is the story.
    pub instances: Vec<RehearsalInstance>,
    pub gaps: Vec<RehearsalGap>,
    /// How many gaps there are.
    ///
    /// ## Why this is a number of its own beside a list
    ///
    /// It goes into the section header, which stays visible when the section is
    /// FOLDED. That is the whole engineering answer to the known hazard of
    /// collapsible sections — content behind a fold gets missed — and the
    /// honest-gap law's form of it: a gap count is never hidden. Sending the
    /// number separately means a client cannot arrive at it by counting a list it
    /// might have filtered.
    pub gap_count: usize,
}

/// One entry in the interleaved timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalTimelineEntry {
    pub when: String,
    pub who: String,
    /// `their_words` or `our_answer` — a token, so the client can style the two
    /// sides without reading prose.
    pub side: String,
    pub quote: String,
}

/// The timeline, which renders only when the placed items can draw one.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalTimeline {
    /// Empty when the threshold is not met — and then `notice` says so.
    pub entries: Vec<RehearsalTimelineEntry>,
    /// The honest-gap line, naming how many placed items carry no date.
    pub notice: Option<String>,
    /// Everyone who appears, for the filter. Chronological order is the design's
    /// choice; the filter is how all of one person's entries arrive in one click.
    pub people: Vec<String>,
    pub filter_prompt: String,
    pub filter_all_label: String,
}

/// The standing card — the one block that never folds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalAlways {
    pub heading: String,
    pub lines: Vec<String>,
}

/// The count line on each section's header, visible open or folded.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalHeaders {
    pub accusation: String,
    pub timeline: String,
    pub points: String,
    pub watch_for: String,
}

/// Which sections start open, decided once on the server.
///
/// The store holds `open` / `collapsed` tokens because it has no boolean kind;
/// they are parsed at BOOT, where a typo is a named refusal. The client receives
/// the decided answer and has nothing left to get wrong — the same reason
/// `fact_background_starts_collapsed` crosses the wire as a boolean.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalCollapse {
    pub accusation_open: bool,
    pub timeline_open: bool,
    pub points_open: bool,
    pub watch_for_open: bool,
}

/// One ready scenario, as Marie rehearses it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalScenario {
    /// The human handle — "S-2". Speakable, and the only identifier here.
    pub code: String,
    /// The scenario's plain name.
    pub title: String,
    /// Block 1 — one sentence on what this fight is about.
    pub what_this_is: Option<String>,
    pub what_this_is_gap: Option<String>,
    /// Blocks 2 and 3.
    pub accusation: RehearsalAccusation,
    /// Block 4.
    pub timeline: RehearsalTimeline,
    /// Block 5 — at most the configured cap.
    pub points: Vec<RehearsalPoint>,
    pub points_gap: Option<String>,
    /// Block 6.
    pub watch_for: Vec<String>,
    pub watch_for_gap: Option<String>,
    pub headers: RehearsalHeaders,
}

/// Every word the page renders that is not already a composed sentence.
///
/// The templates the server fills are absent by construction — there is no field
/// for `accusation_header_template`, `gap_no_answer` or `count_template` to
/// travel in. A browser cannot recompute a count it has no template for, which is
/// the §10 exclusion made structural rather than promised.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalWordingDto {
    /// Marks the paired answer beneath the instance it answers.
    ///
    /// Borrowed from the working view's rows (`accusation_answer_label`) rather
    /// than seeded again: "Our answer:" is already rehearsal-voiced, and a second
    /// row saying the same thing would be two sentences Roman has to keep in step
    /// by hand. Its gap-message siblings are NOT borrowed — those name a fact
    /// "C-14", which is working-view vocabulary §10 keeps off this surface.
    pub answer_label: String,
    pub page_heading: String,
    pub purpose_line: String,
    pub previous_label: String,
    pub next_label: String,
    pub nothing_ready_notice: String,
    /// Carries  — the ONLY thing this page may say about a scenario it
    /// is not showing, because that scenario is not in the payload at all.
    pub not_ready_notice: String,
    pub expand_all_label: String,
    pub collapse_all_label: String,
    pub block_what_heading: String,
    pub block_accusation_heading: String,
    pub block_timeline_heading: String,
    pub block_points_heading: String,
    pub block_watch_heading: String,
}

/// The rehearsal list: every ready scenario, plus what the page speaks with.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalPayload {
    pub scenarios: Vec<RehearsalScenario>,
    pub always: RehearsalAlways,
    pub wording: RehearsalWordingDto,
    pub collapse: RehearsalCollapse,
    /// "Scenario {n} of {total}", one per scenario, already filled in.
    ///
    /// ## Why the positions are composed rather than the template being sent
    ///
    /// A client holding "Scenario {n} of {total}" would be composing prose out of
    /// two numbers, and the sentence would then live half in the store and half
    /// in a component. Roman edits the store; he cannot edit the component.
    pub positions: Vec<String>,
}
