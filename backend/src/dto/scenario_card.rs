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
    pub text: String,
    /// Source text immediately before the quote. Empty when unavailable.
    pub context_before: String,
    /// Source text immediately after the quote. Empty when unavailable.
    pub context_after: String,
    /// The interrogatory this answers, when the item is a discovery answer.
    ///
    /// Present-as-null rather than skipped: "this is an answer with no recorded
    /// question" and "this is not an answer at all" are different, and the client
    /// renders the question line only when it is non-null.
    pub question: Option<String>,
}

/// Where the quote is, and how to get there (§7.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardPinpoint {
    pub document_id: String,
    pub document_title: String,
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
    /// The stance WITH its object, or `None` — never a bare verb. See
    /// [`CardStance`].
    pub stance: Option<CardStance>,
    /// Every accusation this item bears on. Empty is a real answer ("this item is
    /// not connected to the complaint"), not a missing value.
    pub bears_on: Vec<CardBearsOn>,
    pub grounding: Option<CardGrounding>,
    pub confidence: CardConfidence,
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
}
