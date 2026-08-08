//! The candidate card payload (task 1.2, v2 §7).
//!
//! ## The contract these types encode
//!
//! §7 makes a card missing any element a DEFECT. These structs are that contract
//! written as types: the required elements are non-`Option` fields, so a card
//! cannot be constructed without them and "we forgot to serve the pinpoint" is a
//! compile error rather than a blank space on screen.
//!
//! What IS optional is optional for a stated domain reason, never for
//! convenience:
//!
//! * `stance` — `None` when the item links to no allegation. A stance without its
//!   object is the July defect, so the payload serves no stance at all rather than
//!   a bare verb, and sets `defer_required` to say why.
//! * `grounding` — `None` when the node carries a state this build cannot name.
//!   An unrecognized state must not render as verified.
//! * `defer_reason` (on the card) — `None` unless a human deferred it.
//!
//! ## Everything here is display-ready
//!
//! Every string is plain trial language produced by `domain::card_language`. The
//! frontend renders and computes nothing (v2 §7 item 2): no percentages to
//! format, no enums to translate, no URL to assemble. If a value would need a
//! decision to display, that decision was made here.

use serde::{Deserialize, Serialize};

use crate::domain::confidence_band::ConfidenceBand;
use crate::domain::fact_status::FactStatus;
use crate::domain::fact_tier::FactTier;

/// The quote with enough surrounding text to be read in context (§7.1).
///
/// ## Domain note: why context is not optional
///
/// A bare quote is how the July cards failed: "That would be correct." is
/// unrulable on its own. The context fields may be EMPTY (the quote sits at the
/// top of a page, or the page text is not stored), but they are always present on
/// the wire so the card's layout is stable and the client never branches.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardQuote {
    /// The verbatim words, exactly as the record holds them.
    ///
    /// Never groomed. The context around it is expanded to sentence boundaries and
    /// has its gutter numerals stripped (task 1.7C, defect D6); this field is the
    /// anchor, and §12 makes anchors sacred.
    pub text: String,
    /// Source text immediately before the quote. Empty when unavailable.
    ///
    /// Sentence-complete at its left edge unless `context_before_complete` is
    /// false. Gutter numerals stripped for display; the stored page is untouched.
    pub context_before: String,
    /// Source text immediately after the quote. Empty when unavailable.
    pub context_after: String,
    /// Whether `context_before` starts at a real sentence start (task 1.7C, D6).
    ///
    /// `false` means the page itself begins mid-sentence — measured on DEV, 154 of
    /// 499 locatable cards (30.9%). The client must not imply a completeness the
    /// data does not have (Standing Rule 1), which is what the paired notice below
    /// is for.
    pub context_before_complete: bool,
    /// Whether `context_after` ends at a real sentence end. `false` in 65 of 499
    /// cards, measured — the page ends mid-sentence.
    pub context_after_complete: bool,
    /// The page-edge notice for `context_before`, composed server-side, present
    /// exactly when that flank is incomplete AND has text to explain.
    ///
    /// ## Why a state flag AND a composed sentence
    ///
    /// The same pairing as [`CardGrounding`] two structs down, for the same
    /// reason: the boolean is the STATE (the client branches on it and a test
    /// asserts it without parsing prose), the string is the WORDS (v2 §7 item 2
    /// puts every display decision on this side of the wire — the browser does not
    /// get to choose how a page edge reads).
    ///
    /// Present-as-null rather than skipped, matching `question` below: "complete,
    /// so no notice" and "field absent from this build" are different, and a
    /// client that cannot tell them apart will eventually render the wrong one.
    pub context_before_notice: Option<String>,
    /// The page-edge notice for `context_after`. See `context_before_notice`.
    pub context_after_notice: Option<String>,
    /// The interrogatory this answers, when the item is a discovery answer.
    ///
    /// Present-as-null rather than skipped: "this is an answer with no recorded
    /// question" and "this is not an answer at all" are different, and the client
    /// renders the question line only when it is non-null.
    ///
    /// ## Task 1.7F: this is the DISPLAY text, which may be a human's
    ///
    /// When a human has corrected the machine's question, this field carries
    /// THEIR words and `question_authorship` says so. The client renders one
    /// sentence and does not choose between two, which is the same discipline as
    /// every other string on this card. The machine's original is untouched in the
    /// graph; see [`CardQuestionAuthorship`].
    pub question: Option<String>,
    /// Who wrote the question above, and when — present exactly when `question`
    /// is (task 1.7F Part B).
    ///
    /// Skipped when absent rather than sent as null: a card with no question has
    /// no authorship to report, and an authorship object hanging off a card with
    /// nothing to attribute would be a field the client has to interpret.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question_authorship: Option<CardQuestionAuthorship>,
}

/// Who wrote a card's question (task 1.7F Part B, ruling R1/R2).
///
/// ## Why the state and the words travel together
///
/// The same pairing as [`CardGrounding`] and the status/status_label pair: the
/// token is the STATE (the client branches on it — a pencil and a Revert control
/// appear only for `human`) and the string is the WORDS (the backend composes how
/// authorship reads, per the language law). A client that had to build "corrected
/// by Roman on 4 August 2026" from parts would be composing case-facing prose in
/// the browser, which v2 §7 item 2 puts on this side of the wire.
///
/// The ICON is deliberately not here. `⚙` and `👤` are the list's own control
/// vocabulary, the same class of thing as the state chip's `✓` and `✕`, and the
/// frontend has always owned those.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CardQuestionAuthorship {
    /// `system` | `human`. The machine token the client branches on.
    pub source: QuestionSource,
    /// The composed badge, rendered verbatim — "System" or "Roman · 4 Aug 2026".
    pub label: String,
}

/// Whether a card's question is the machine's or a human's correction of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionSource {
    /// Straight from `Evidence.question` in the graph — nobody has corrected it.
    System,
    /// A human replaced it. The machine's words are still in the graph, untouched
    /// (ruling R2), and Revert restores them by deleting the override row.
    Human,
}

/// Where the quote is, and how to get there (§7.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardPinpoint {
    pub document_id: String,
    pub document_title: String,
    /// The full pinpoint line, pre-composed — "CFS responses at 26".
    ///
    /// ## Domain note: why the label ships composed, like the stance summary
    ///
    /// The client must not join `document_title` and `page` itself. That would be
    /// the browser making a presentation decision about case vocabulary, which is
    /// exactly what [`CardStance::summary`] exists to prevent one field over. The
    /// raw parts stay on the payload for a client that wants to lay them out
    /// differently; the label is what gets rendered.
    pub label: String,
    /// `None` when the record carries no page. The card shows the document
    /// without a page rather than inventing one.
    pub page: Option<i64>,
    /// The viewer link, assembled server-side (v2 §7 item 2: the browser builds
    /// no URLs). Includes the page anchor when there is a page.
    pub viewer_href: String,
}

/// Who said it, and on what authority we say so (§7.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardSpeaker {
    /// The speaker string as extracted. `None` for documentary evidence, which
    /// genuinely has no speaker.
    pub name: Option<String>,
    /// Always `"extracted"` today — the label is explicit so the card can say the
    /// attribution is the extraction's, not a human's. B0 canonicalization
    /// (Phase 2) is what will make a second value possible; until then this field
    /// exists so the card never implies more certainty than it has.
    pub attribution: String,
}

/// The item's position toward one accusation — the verb AND its object (§7.5).
///
/// ## Domain note: this type cannot express a bare stance
///
/// Both fields are required. That is the whole point: C-222 showed "contradicts"
/// with nothing after it, and the fix is a type in which the object is not
/// omittable. An item with no object produces `stance: null` and a defer flag,
/// never a `CardStance` with an empty `object`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardStance {
    /// The canon verb — "supports" / "disputes" / "comments on" / "mentions".
    pub verb: String,
    /// What the verb applies to, in complaint language.
    pub object: String,
    /// The full line, pre-composed so the client concatenates nothing.
    pub summary: String,
}

/// One accusation this item bears on, with its place in the case (§7.6).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardBearsOn {
    /// The accusation in complaint language, paragraph-numbered when known.
    pub accusation: String,
    /// The elements this accusation goes to.
    ///
    /// ## Domain note: why a LIST, not one element
    ///
    /// Measured on DEV: a single accusation routinely bears on several elements of
    /// one count — ¶12 feeds "Defendant had a duty to disclose", "The undisclosed
    /// facts were material" AND "Plaintiff was damaged by the reliance". An
    /// `Option<String>` here silently kept whichever the graph returned first and
    /// dropped the rest, which would understate what the item does for the case —
    /// exactly the kind of quiet loss this task exists to remove.
    ///
    /// Empty when the accusation is wired to no element yet; that is a real state
    /// (the complaint paragraph exists, the legal theory is not yet mapped), not a
    /// missing value.
    pub elements: Vec<String>,
    /// The count that element belongs to, when known.
    pub count: Option<String>,
}

/// One accusation a HUMAN linked this statement to (task 2.10).
///
/// ## Domain note: why this is not folded into `bears_on`
///
/// A machine link and a human link are different claims about the world. The
/// first says the extraction found a relationship in the record; the second says
/// a person read the statement and decided it bears on this accusation. They
/// render alike — both are chips — but a payload that collapsed them would make
/// "what did the machine find?" unanswerable, and task 2.5's re-anchoring needs
/// that answer.
///
/// It is also what keeps ruling R2 structural: `build_stance` takes machine links
/// and only machine links, so a human-linked card cannot grow a stance sentence
/// the extraction never supported, no matter what else changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardHumanLink {
    /// The accusation's graph id — what an unlink addresses.
    pub allegation_id: String,
    /// The accusation in complaint language, paragraph-numbered when known.
    /// Composed by the same rule `bears_on` uses, so the two read alike.
    pub label: String,
    /// Which way it cuts, as a token to branch on.
    pub cut: crate::domain::link_cut::LinkCut,
    /// The same choice in words — "They'll use it against us". Paired with the
    /// token exactly as `CardGrounding` pairs `state` with `label`: the client
    /// branches on the token and never parses the prose.
    pub cut_label: String,
}

/// What the latest completed scan PROPOSED about this candidate.
///
/// Present exactly when the card is a read-time projection of an admitted verdict
/// that no human has touched (R-a). Absent — not empty, not zeroed — on every
/// other card, so "the scan proposed this" and "nobody proposed this" are
/// different shapes on the wire and not a flag to interpret.
///
/// ## Domain note: everything here is the MACHINE'S claim, and says so
///
/// The card's other elements describe the record: who said it, on what page, what
/// it bears on. These three describe what a model thought about it last night.
/// `role_label` therefore names the scan as its subject exactly as
/// [`CardConfidence::label`] does — a chip reading "supports" beside a sworn
/// admission would read as the record's own stance, which it is not.
///
/// ## Why there is no confidence NUMBER here
///
/// §7.8 is binding: banded words, never a naked percentage. The verdict's score
/// decides [`ScenarioCard::confidence`]'s band on this side of the wire and is
/// then discarded — a client that never receives the number cannot render it, and
/// the rule stops depending on every future component remembering it.
///
/// ## Why there are no covered node ids here
///
/// Ruling a folded card settles its twins, and WHICH nodes those are is resolved
/// server-side at ruling time from the same projection that built this card
/// (architect ruling R4). Shipping the list would invite a client to send it back,
/// and a client-supplied "these nodes go with it" is a claim the server cannot
/// check — the same reasoning that keeps the ruling ANCHOR a server-side read.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardProposal {
    /// The role the judge assigned, in plain trial language and attributed to the
    /// scan ("Scan: supports"). `None` when the stored verdict carries a role token
    /// this build cannot name — the chip is then absent rather than showing a raw
    /// token, and the decode failure is logged (Standing Rule 1).
    pub role_label: Option<String>,
    /// The judge's own justification, verbatim from `scan_run_verdicts.reason`.
    ///
    /// Not composed and not translated: this is the model's sentence, and it is
    /// the thing the human is being asked to weigh. `None` for a verdict recorded
    /// without one.
    pub reason: Option<String>,
    /// How many pool rows this one card speaks for — `1` ordinarily, `2` for a
    /// byte-identical twin pair. Ruling the card settles all of them.
    pub duplicate_count: usize,
    /// The badge naming what a single ruling here covers ("×2 — covers C-46").
    /// `None` when the card speaks only for itself, because a badge reading "×1"
    /// is noise on every card in the queue.
    pub duplicate_label: Option<String>,
}

/// Which run put the proposals in this payload, and how many there are.
///
/// Response-level rather than per-card: R-b means every proposal in one payload
/// comes from ONE run, so repeating its identity on thirty cards would be thirty
/// chances for them to disagree.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalSource {
    /// The run whose verdicts these proposals are. The scan panel matches its
    /// history rows against this to decide which row may show a proposed count
    /// (every other row projects nothing, and shows an em dash).
    pub run_id: uuid::Uuid,
    /// The model that judged, as its stored id. The panel already maps ids to
    /// display names for the history table and the run controls; sending the
    /// display name here would make this the second place that mapping happens.
    pub model_id: String,
    /// When the run started — the date the attribution line names. Formatted by
    /// the browser, in the reader's locale, exactly as the scan-history delete
    /// confirmation's `{run}` already is.
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// How many cards in this payload carry a proposal.
    ///
    /// ## Domain note: this is the LIVE number (R-e)
    ///
    /// Admitted verdicts minus everything precedence has already ruled, counted
    /// over the payload being served — so it falls as the human rules and can
    /// never disagree with the cards beneath it. The run's own frozen counts are
    /// NOT rewritten; this number is served beside them and labelled live.
    pub proposed_count: usize,
}

/// Whether the quote was located in its source (§7.7).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardGrounding {
    /// The raw state token, carried so an operator can correlate with the
    /// pipeline's own reporting.
    pub state: String,
    /// The plain-trial label. Canon word: "grounded" (§9); "proven" is retired.
    pub label: String,
}

/// How confident the scan was, banded (§7.8).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardConfidence {
    pub band: ConfidenceBand,
    /// The plain-trial label, which names the SCAN as its subject so the reader
    /// cannot mistake it for a claim about the evidence.
    pub label: String,
}

/// One complete candidate card.
///
/// Every §7 element is here, and the required ones are required in the type. See
/// the module doc for why the four optional fields are optional.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioCard {
    /// The human handle — `"C-14"` (task 1.1). `None` for a candidate that has
    /// not been numbered yet; the card renders without a chip rather than
    /// inventing one.
    pub code: Option<String>,
    /// The graph node id. Not shown to the human — carried so the client can
    /// address the ruling routes without a second lookup.
    pub graph_node_id: String,
    pub quote: CardQuote,
    pub pinpoint: CardPinpoint,
    pub speaker: CardSpeaker,
    /// The kind of statement, humanized (e.g. "partial admission"). `None` when
    /// the extraction recorded none.
    pub statement_kind: Option<String>,
    /// How much this fact carries THIS scenario (task 2.13).
    ///
    /// The stored token, not a display name: the three names a human reads live
    /// in the settings store so Roman can rename them without a rebuild, and the
    /// client pairs this token with the wording it was served. Sending the LABEL
    /// here would make the payload the second place a tier is named, and the two
    /// would drift the first time one was edited.
    ///
    /// `None` for a candidate with no reference row — nobody has ruled it in, so
    /// it has no weight in this scenario yet. That is distinct from `backup`,
    /// which is a fact that IS in the scenario and has simply never been weighed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<FactTier>,
    /// The human's stored position for this fact in this scenario, or `None` when
    /// they have never placed it (task 2.13).
    ///
    /// Carried so the client can name a drop's neighbours and render the order the
    /// server computed — never so the browser can re-derive the ordering itself.
    /// `None` and `0` are different states and are never collapsed: an unplaced
    /// fact sorts after every placed one, a fact placed at 0 sorts first.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_ordinal: Option<i32>,
    /// Where this fact sits in the scenario's list, as ONE number the client
    /// sorts on (task 2.13c).
    ///
    /// ## Why the server sends the position rather than the client deriving it
    ///
    /// `sort_ordinal` alone is not enough to order the list: most facts have
    /// none, and the rule for where an unplaced fact goes relative to a placed
    /// one is arithmetic (`services::scenario_fact_order`) that the browser would
    /// have to reimplement — the same constant and the same tie-breaks, in a
    /// second place, drifting from the first. It already went wrong once: a
    /// re-included fact landed fourth because the two sides disagreed about what
    /// "end of list" meant.
    ///
    /// So the ordering is computed once, here, and the client sorts ascending on
    /// this number and nothing else. `None` for a card that is not an included
    /// fact — an unruled candidate has no position in a list it is not in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_ordinal: Option<i32>,
    /// The stance WITH its object, or `None` — never a bare verb. See
    /// [`CardStance`].
    pub stance: Option<CardStance>,
    /// Every accusation this item bears on. Empty is a real answer ("this item is
    /// not connected to the complaint"), not a missing value.
    pub bears_on: Vec<CardBearsOn>,
    pub grounding: Option<CardGrounding>,
    pub confidence: CardConfidence,
    /// The candidate's workbench state as a token the client can branch on.
    ///
    /// ## Domain note: why BOTH a state and a label (added in task 1.4)
    ///
    /// The same state+label idiom as `CardGrounding` and `CardConfidence`. The
    /// working view filters to the included items, and filtering on
    /// `status_label` would mean inferring state from prose — it would break
    /// silently the day the wording changes, and it is the frontend reading
    /// meaning out of a display string, which this payload exists to forbid.
    pub status: FactStatus,
    /// The candidate's workbench state, in plain language.
    pub status_label: String,
    /// True when this item cannot be ruled on as it stands.
    ///
    /// ## Domain note: the flag is the July fix
    ///
    /// A card that cannot support a ruling must SAY so, with a reason a human can
    /// act on. Showing a bare stance and letting them discover the problem
    /// themselves is what produced 26 unusable cards.
    pub defer_required: bool,
    /// Why the card cannot be ruled on, in plain language. Present exactly when
    /// `defer_required` is true.
    pub defer_required_reason: Option<String>,
    /// The reason a human already deferred this item (task 1.1), if they did.
    ///
    /// Distinct from `defer_required_reason`: that one is the SYSTEM saying the
    /// card is unrulable; this one is a HUMAN saying they chose to park it.
    pub defer_reason: Option<String>,
    /// The accusations a HUMAN linked this statement to (task 2.10). Empty is the
    /// ordinary state; a non-empty list is what clears `defer_required` on a card
    /// the extraction never linked.
    pub human_links: Vec<CardHumanLink>,
    /// What the latest completed scan proposed about this card, or `None` when
    /// nothing proposed it.
    ///
    /// ## Domain note: presence is what "PROPOSED" means
    ///
    /// There is no `proposed` status and no third writer. A proposed card is one
    /// with no reference row and a live verdict behind it, and this field is the
    /// whole of that state on the wire — which is why the queue's Proposed facet is
    /// `not_ruled` AND this being present, rather than a status token to decode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposed: Option<CardProposal>,
    /// What the human did, in their own terms — "You linked this to ¶41 · they'll
    /// use it against us." Present exactly when `human_links` is non-empty.
    ///
    /// ## Domain note: this is NOT a stance, and must never become one
    ///
    /// The extraction said nothing about this statement, so the card does not
    /// pretend otherwise: no canon verb, no "This supports…", no sentence in the
    /// machine's voice. It reports an act a person took (ruling R2). `stance`
    /// stays `null` on such a card, and the §7.5 slot is filled by this instead.
    pub human_link_summary: Option<String>,
}

/// Response body for the card endpoint.
///
/// Two lists, mirroring the gather endpoint's split: the working pool and the
/// set-aside items, kept apart so the client partitions nothing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioCardsResponse {
    pub pool: Vec<ScenarioCard>,
    pub set_aside: Vec<ScenarioCard>,
    /// How much of the stuck pile a human has worked through — "38 of 94 linked."
    ///
    /// `None` when nothing in this pool is stuck, because a progress line about an
    /// empty problem is noise on the one screen that should be quiet.
    ///
    /// ## Domain note: counted from the POOL, never from keystrokes (1.7E-a)
    ///
    /// Both numbers are derived from the payload being served, so the sentence and
    /// the cards beside it cannot disagree — the defect that put "0 of 148 ruled"
    /// next to twelve green chips. Composed here rather than in the browser for
    /// the same reason every other sentence is (the language law).
    pub link_progress: Option<String>,
    /// Present ONLY when this scenario names no target, in which case both card
    /// lists are empty and this sentence says why (2026-08-07 fix).
    ///
    /// ## Domain note: this is the card queue's half of the fix
    ///
    /// The gather endpoint carries the same field, but THIS is the surface the
    /// defect was seen on: on 2026-08-07 a target-less scenario served 148 cards
    /// here, gathered over the case-default subject, indistinguishable from the
    /// scenario beside it that had named that subject deliberately. An empty
    /// queue with a reason is the whole remedy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_target_notice: Option<String>,
    /// Present ONLY when no scan run has COMPLETED for this scenario (task 2.15,
    /// piece 3). The cards are served in full alongside it.
    ///
    /// ## Domain note: this replaces a claim, not a list
    ///
    /// The cards are real — they are every statement about the subject. What was
    /// false, measured on 2026-08-07, was the page describing them as "Candidates
    /// awaiting ruling — 148 · from all scans" on a scenario no scan had ever
    /// touched. This sentence is what leads instead; the pool moves behind an
    /// explicit opt-in and keeps every one of its rows.
    ///
    /// Absent (not `false`, not empty) once a scan has completed, so the client
    /// branches on presence exactly as it does for [`Self::no_target_notice`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub never_scanned_notice: Option<String>,
    /// Which completed run put the proposals in this payload, or `None` when
    /// nothing is proposed.
    ///
    /// `None` covers three different situations that need no distinguishing HERE —
    /// no run has completed, the latest completed run admitted nothing, or every
    /// admitted verdict has already been ruled — because the queue's answer is the
    /// same in all three: lead with the pool, not with proposals. The first of the
    /// three is separately visible as [`Self::never_scanned_notice`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal_source: Option<ProposalSource>,
}
