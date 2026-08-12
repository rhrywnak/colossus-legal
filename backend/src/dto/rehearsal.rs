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
//! a status, a motivation, a strategy note, or a §2d annotation. A future edit
//! that wanted to leak one would have to ADD a field — a visible change to this
//! file rather than a line slipped into a mapper — and
//! `the_payload_carries_nothing_it_is_excluded_from` then scans the serialized
//! bytes, so a leak smuggled inside a string is caught too.
//!
//! ## What CHANGED again, and the ruling behind it (task 2.11 C, ruling C1)
//!
//! The banned list used to include "a database id", and this payload carried
//! none. Roman's editing ruling of 2026-08-06 turned this surface into a CALLER
//! of the guarded write routes — the accusation sentence, "What this is", the
//! talking points and the watch items are all edited here now — and a route needs
//! an address. `/cases/:slug/scenarios/:scenario_id/…` is the address, so
//! [`RehearsalScenario`] carries `scenario_id` and [`RehearsalWatchItem`] carries
//! its row's `id`.
//!
//! The distinction that keeps this coherent is the same one §10 has always drawn:
//! it exists to keep CONTENT off a witness surface — verdicts, confidences,
//! tiers, strategy, candidate handles. A UUID renders nothing, says nothing about
//! the evidence, and cannot be read aloud. `graph_node_id` and the `C-n` handles
//! stay banned, and the test still asserts it: marking and pairing remain working-
//! view work, so this page never needs to address a statement.
//!
//! A talking point is addressed by its PRINTED POSITION rather than by an id,
//! for the reason [`RehearsalInstance::position`] gives — `item_index` is
//! server-owned and stable, so the row number on screen is a real address.
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
    /// The row number a human reads — 1, 2, 3 — AND this point's address.
    ///
    /// It is `response_items.item_index + 1`: server-owned, stable across edits,
    /// and the same number printed in the pill beside the text. Editing point 2
    /// therefore addresses point 2, with no internal id on the wire. See the
    /// module header's note on ruling C1.
    pub position: usize,
    /// Her words, verbatim as authored.
    pub text: String,
    /// The one exhibit that backs this point, in HER phrasing — "My certified
    /// letter". Authored (`response_item_fact_refs.note`), never a document title
    /// assembled from the record: deriving it would put words in the witness's
    /// mouth. `None` until the pairing editor exists (tracker task 3.9).
    pub exhibit: Option<String>,
    /// The stored sentence shown when `exhibit` is `None`.
    ///
    /// Sent rather than left to the client because this page holds no literals —
    /// and it is a NAMED absence, not a blank, which is the honest-gap law
    /// applied to a block whose pairing editor does not exist yet.
    pub exhibit_notice: String,
}

/// One thing to watch for, and its address.
///
/// A bare `String` until task 2.11 C. It gained an id when the page gained the
/// ability to edit one: `PUT …/human-facts/:fact_id` needs to know WHICH note,
/// and a list position would shift under a concurrent add. See the module
/// header's note on ruling C1 for why an address is not excluded content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalWatchItem {
    pub id: String,
    pub text: String,
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
    /// The question this answer answers, verbatim — `None` when there is none.
    ///
    /// See [`RehearsalInstance::question`] for why a Q/A statement is unreadable
    /// without it. Our OWN answers are the half most often affected: five of the
    /// nine pairings on DEV point at a discovery response, and four of those
    /// quote a bare affirmation.
    pub question: Option<String>,
    /// Our answer's own handle — "C-14". See [`RehearsalInstance::code`].
    ///
    /// The answer carries one for the same reason the instance does: in the room
    /// it is the thing being referred to ("that's answered at C-14"), and a card
    /// that named the accusation but not the rebuttal would make the half Marie
    /// speaks the half she cannot cite.
    pub code: Option<String>,
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
    /// The handle a human SAYS OUT LOUD — "C-91". `None` when nothing has
    /// numbered this candidate.
    ///
    /// ## Why a code is not an internal id, and belongs on this page (task R4)
    ///
    /// §10 keeps stored internal numbers off the rehearsal surface, and the
    /// `position` field above exists precisely so an ordinal cannot creep back
    /// in. A candidate code is a different animal: it is the name the working
    /// page prints, the name the prep list's gap sentences already use, and the
    /// name Marie and Chuck use to refer to one statement in a room. Withholding
    /// it did not protect anything — it meant the two pages called the same
    /// statement different things, and the person least able to know they were
    /// the same thing was the one reading this page.
    ///
    /// `None` rather than the node id, on the same rule the accusation panel
    /// states: an id in a slot labelled "code" reads as a handle and gets quoted
    /// as one out loud, which is worse than a blank.
    pub code: Option<String>,
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
    /// The interrogatory or deposition question this statement answers.
    ///
    /// `None` for documentary evidence — a court finding answers nobody, and a
    /// blank "Q:" line over one would claim a question the record does not hold.
    ///
    /// ## Domain note: why a sworn answer is worthless without its question
    ///
    /// S-6's card for `…response-to-discovery:evidence:0fd1a748` reads, in full,
    /// `Yes.` On the page Marie rehearses from, that is not evidence: it is a
    /// syllable. The question — *"Did George Phillips on behalf of Catholic
    /// Family Services make the argument…"* — is what turns it into an admission
    /// she can quote back. The pair is the unit; the answer alone is a fragment.
    ///
    /// ## Why this is not excluded content (§10)
    ///
    /// §10 keeps impeachment MACHINERY off this surface — verdicts, confidences,
    /// tiers, strategy. A question put to a witness under oath is the record's
    /// own words, the same class as [`quote`](Self::quote) itself, and it is the
    /// half of the record that makes the other half mean anything.
    pub question: Option<String>,
    /// The quote's opening, for the collapsed row.
    ///
    /// Composed HERE so the truncation rule is tested in one place. A browser
    /// slicing prose would produce a different cut for the same quote the moment
    /// two components disagreed about the length.
    pub quote_first_line: String,
    /// What we said back, when a human has paired something.
    pub answer: Option<RehearsalAnswer>,
    /// The small tag on the row — the stored ANSWERED or NO ANSWER word.
    ///
    /// Always present, and composed here rather than chosen in the browser: a
    /// client picking between two labels is a client that can pick wrong, and
    /// the wrong pick puts a green ANSWERED on a row nobody has answered.
    pub answer_tag: String,
    /// The red banner inside an OPENED row with no answer. `None` when there is
    /// one.
    ///
    /// ## Why this is a short banner and not the gap sentence (ruling C5)
    ///
    /// Until beta.381 this field carried the full "NO ANSWER PREPARED — who,
    /// when, where" sentence, which the prep list ALSO carried — the same
    /// sentence twice on one screen, filed as a defect. The sentence now lives
    /// once, in the prep list, where a human works through it. This is the same
    /// fact at a different zoom: three words, no who, no when, no where, so it
    /// can never grow back into the duplicate.
    pub answer_banner: Option<String>,
    /// Which case phase this statement belongs to — the chip, and the value the
    /// phase filter matches on. Always present: a statement with no date and no
    /// forum carries the stored undated label rather than nothing, because a card
    /// with no chip in a filtered list looks like a rendering fault.
    pub phase: String,
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
    /// The instance row this gap is about, by its printed position.
    ///
    /// `Some` for the two gaps that ARE about a rendered row, so the prep-list
    /// entry can carry a link that opens it — the design's point being that the
    /// prep list is the one thing a human can act on today, and making them hunt
    /// for the row wastes the act. `None` for `accusation_removed` (about a
    /// statement no longer in the list) and `instance_unavailable` (about one the
    /// record store cannot produce): neither has a row to jump to, and inventing
    /// one would scroll a reader to somebody else's statement.
    pub position: Option<usize>,
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
    /// Who wrote the sentence above, and when — as a FINISHED line.
    ///
    /// ## Why the line is composed here and not the template sent
    ///
    /// A client holding "Written in plain words by {who} · {when}" would be
    /// composing prose out of two values, and the sentence would live half in the
    /// store and half in a component. Roman edits the store.
    ///
    /// `None` when there is no sentence to attribute. When there IS one but
    /// nobody recorded who wrote it — every sentence written before task 2.11 C —
    /// this carries the stored `rehearsal_attribution_unknown_notice` rather than
    /// a blank or an invented name (ruling C2).
    pub attribution: Option<String>,
    /// "Said 5 times, in 5 documents." — `None` when nothing is marked.
    pub count_line: Option<String>,
    /// The count line in PLAIN WORDS, grammatical at one, with the date span it
    /// actually has (task R3). `None` when nothing is marked.
    ///
    /// Beside `count_line` rather than replacing it: the two serve different
    /// surfaces — the working view's "Said 5 times, in 3 documents." is a
    /// heading, and this is the prep page's opening sentence.
    pub plain_count_line: Option<String>,
    /// "5 of 5 answered" / "3 of 5 answered — 2 to prepare", composed server-side.
    pub answered_line: Option<String>,
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
    /// The stored word for that token — "THEY SAY" or "OUR ANSWER".
    ///
    /// Sent beside the token rather than instead of it: the token decides the
    /// COLOUR and the label is what a human reads, and a client matching on the
    /// prose would stop telling the sides apart the day Roman reworded one.
    pub side_label: String,
    pub quote: String,
    /// Where it was said, and how to open it (task 2.11 C, ruling C7).
    ///
    /// ## Domain note: why the pinpoint ruling reaches this row
    ///
    /// The 2026-08-06 ruling permitted a citation on "instance and answer rows
    /// only". A timeline row IS an instance or an answer, chronologically
    /// arranged — the same statement, one zoom level out — and the research the
    /// ruling rests on is about a witness producing the source on the spot, which
    /// does not care which block she was reading when asked. Confirmed by the
    /// architect as ruling C7.
    pub source: RehearsalSource,
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
    /// Task 2.11 C: "What this is" folds like the others as of the signed
    /// mockup. B2 had it fixed open.
    pub what_open: bool,
    pub accusation_open: bool,
    pub timeline_open: bool,
    pub points_open: bool,
    pub watch_for_open: bool,
}

/// One ready scenario, as Marie rehearses it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalScenario {
    /// The human handle — "S-2". Speakable, and the one identifier a reader sees.
    pub code: String,
    /// The scenario's address on the guarded write routes (ruling C1).
    ///
    /// Renders nowhere. It exists so the page's editors can call
    /// `/cases/:slug/scenarios/:scenario_id/…` — the SAME routes the working view
    /// calls, which is what "reuse, never fork" required once this surface became
    /// editable. See the module header.
    pub scenario_id: String,
    /// The scenario's plain name.
    pub title: String,
    /// Block 1 — one sentence on what this fight is about.
    pub what_this_is: Option<String>,
    pub what_this_is_gap: Option<String>,
    /// Who wrote that sentence, and when — a finished line, or the stored
    /// unknown notice. `None` when there is no sentence. Same construction and
    /// same reasoning as [`RehearsalAccusation::attribution`].
    pub what_this_is_attribution: Option<String>,
    /// Blocks 2 and 3.
    pub accusation: RehearsalAccusation,
    /// Block 4.
    pub timeline: RehearsalTimeline,
    /// Block 5 — at most the configured cap.
    pub points: Vec<RehearsalPoint>,
    pub points_gap: Option<String>,
    /// Block 6.
    pub watch_for: Vec<RehearsalWatchItem>,
    pub watch_for_gap: Option<String>,
    pub headers: RehearsalHeaders,
    /// Whether this scenario's instance rows arrive OPEN or one line tall.
    ///
    /// ## Why the server decides rather than sending the cap
    ///
    /// The same reason [`RehearsalCollapse`] is booleans rather than tokens: the
    /// rule is "at or under `rehearsal_instance_rows_expand_max`, expanded", it is
    /// evaluated against a count only the server has already computed, and a
    /// client re-deriving it is a second implementation of one rule that can
    /// disagree with the first. The browser receives the decided answer.
    ///
    /// Note what this is NOT: a limit on what is shown. Every instance is
    /// rendered and every one is reachable; the list is not paginated at any
    /// size.
    pub instances_start_expanded: bool,
    /// Offense or defense, as the stored word (task R3).
    pub direction_label: String,
    /// The attack in their own words, verbatim. Folded away on the prep page
    /// beneath the plain-words accusation; `None` renders no fold control.
    pub attack_text: Option<String>,
    /// The complaint paragraphs this scenario bears on, as A-codes — the same
    /// handles the working page's chips use.
    ///
    /// ## Why these were wrong until task R4 (P6a)
    ///
    /// They were built by taking the last `:`-segment of an anchor id. An id is
    /// `doc-…:allegation:<hash>`, so that segment is the HASH, and every chip on
    /// the prep page read `A-<hash>`: a node-id fragment wearing the prefix of a
    /// paragraph number, on the surface where a reader is least equipped to
    /// notice. The paragraph is a property of the Allegation node, so the
    /// assembly now reads it — scoped to the handful of ids one scenario
    /// anchors, never the whole complaint.
    ///
    /// An id the graph cannot resolve travels AS the id rather than being
    /// dropped, so the chip row can never be quietly shorter than the scenario's
    /// actual anchors.
    pub bears_on: Vec<String>,
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
    /// The heading over the Ready-scenario list at the bare rehearsal address
    /// (task R1 Piece 1d). The list itself is composed from `scenarios`, which
    /// this payload already carries — no second read, no second ordering.
    pub picker_heading: String,
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

    // ── Task 2.11 C: the rebuilt page's controls and markers ────────────────
    /// The breadcrumb link back to Trial Prep. Was a literal until 2.11 C.
    pub crumb_trial_prep_label: String,
    /// The header control that leaves rehearsal for the scenario's working page.
    pub scenario_page_label: String,
    /// The link on a prep-list entry that opens the row it names.
    pub go_to_row_label: String,
    /// Heads the prep list under the instances.
    pub prep_list_heading: String,
    /// Follows the count line: rows open on click, and a feature nobody can see
    /// is a feature nobody finds.
    pub row_open_hint: String,
    pub add_point_label: String,
    pub add_watch_label: String,
    /// Sits beside the add control on the points block.
    pub points_authoring_note: String,
    /// The words the two sentence editors and the two list editors share.
    pub editor: RehearsalEditorWordingDto,
}

/// The words the page's editors speak.
///
/// ## Why these are BORROWED from the working view's rows (ruling C3)
///
/// Every one of them is already stored for the accusation section — "Edit",
/// "Save", "Withdraw it", "Cancel" — and they are already rehearsal-voiced.
/// Seeding a second set would be four more sentences Roman has to keep in step by
/// hand, which is the reason `answer_label` and the count template were borrowed
/// in task 2.11 B2. Words are not excluded content; the DTO widens and no row
/// duplicates.
///
/// Two of the section's seven keys are deliberately NOT here. `text_label` has no
/// renderer — the mockup shows no field label over the box — and
/// `text_missing_notice` would be a second sentence for the absence this page
/// already names with `accusation.text_gap`. A field nothing renders is a field
/// that rots.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalEditorWordingDto {
    /// Opens a sentence, a talking point or a watch item for editing.
    pub edit_label: String,
    pub save_label: String,
    pub cancel_label: String,
    /// Clears the accusation sentence — offered only where there is one.
    pub withdraw_label: String,
    /// The empty-box hint for the accusation sentence.
    pub accusation_placeholder: String,
    /// The empty-box hint for "What this is".
    pub what_placeholder: String,
    /// Carries `{detail}` — the failure's own words, the one value that exists
    /// only on the browser's side. Filled by the client, never composed by it.
    pub save_failed_template: String,
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
