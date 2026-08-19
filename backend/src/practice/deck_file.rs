//! The deck file a human writes, and what makes one valid.
//!
//! One YAML file per scenario (`practice_decks/S-5.yaml`) holding the questions
//! Marie will be asked, with everything her reveal screen renders. Pure: nothing
//! here opens a file or a connection, so every refusal below is a unit test
//! rather than a deployment.
//!
//! ## Why a file and not rows in a migration, or literals in this crate
//!
//! Rule 2, twice over. The text is case-specific ("at each other's throats" is
//! about a named person in one lawsuit), so a shared crate carrying it could not
//! be reused by another Colossus project; and Roman edits these sentences after
//! watching Marie use the tool, which must not be a rebuild. A migration would
//! be worse than either — it would make the deck un-editable without a new
//! migration file.
//!
//! ## Why the file names a POSITION and not a node id
//!
//! `source_index: 1` means "the first of this scenario's ruled instances". The
//! seed resolves it to the real graph node id at write time. An id written into
//! the repo would be a DEV-shaped constant that rots the day a re-ingest re-keys
//! the node — which is the stale-pointer defect of 2026-08-14, where the join
//! key was fine and all 26 refs pointed at nothing.

use serde::Deserialize;

pub use super::deck_valid::DeckError;

/// Which side asks the question.
///
/// ## Rust Learning: `#[serde(rename_all = "snake_case")]` on a unit enum
///
/// The YAML says `side: george`; the Rust says `DeckSide::George`. The attribute
/// is what bridges the two naming conventions, and using an ENUM rather than a
/// `String` means an unknown side is refused BY THE PARSER, naming the line — not
/// discovered later by a database CHECK constraint with no line number in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeckSide {
    /// Cross — the attack, turned into the question their lawyer would ask.
    George,
    /// Direct — the question Chuck asks so she can tell it in her own words.
    Chuck,
}

impl DeckSide {
    /// The value the `practice_questions.side` column stores.
    pub fn as_column(self) -> &'static str {
        match self {
            DeckSide::George => "george",
            DeckSide::Chuck => "chuck",
        }
    }
}

/// What the question DOES, as the file says it.
///
/// ## Domain note: why this is not derivable from `side`
///
/// Chuck asks two kinds. A DIRECT question opens a subject; a REDIRECT repairs
/// one George has just damaged, and it exists only because of the George
/// question it follows. The read judges them by different rules (prompt v2), the
/// mixed queue deals them differently, and the screen tags them differently.
/// `side` cannot carry any of that.
///
/// Absent in the file means `cross` on George's side and `direct` on Chuck's —
/// which is what every deck written before 2026-08-19 meant, and what
/// [`DeckQuestion::resolved_kind`] returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeckKind {
    /// George's attack, turned into the question their lawyer would ask.
    Cross,
    /// Chuck opening a subject so she can tell it in her own words.
    Direct,
    /// Chuck repairing the subject George just damaged.
    Redirect,
}

impl DeckKind {
    /// The value the `practice_questions.kind` column stores.
    pub fn as_column(self) -> &'static str {
        match self {
            DeckKind::Cross => "cross",
            DeckKind::Direct => "direct",
            DeckKind::Redirect => "redirect",
        }
    }
}

/// Where a question traces to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeckSourceKind {
    /// One of the scenario's ruled accusation instances.
    Instance,
    /// One of the scenario's talking points.
    Point,
    /// Typed by a human with nothing behind it. Carries no ref, and the screen
    /// says "no receipt" in words.
    Manual,
}

impl DeckSourceKind {
    /// The value the `practice_questions.source_kind` column stores.
    pub fn as_column(self) -> &'static str {
        match self {
            DeckSourceKind::Instance => "instance",
            DeckSourceKind::Point => "point",
            DeckSourceKind::Manual => "manual",
        }
    }
}

/// One question, as the file writes it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckQuestion {
    /// The stable handle this question keeps for the life of the deck — `g1`,
    /// `c3`, `r2`. Optional so a deck written before 2026-08-19 still parses;
    /// the seed's `--update` path REFUSES a file whose questions have none,
    /// because matching on text is what it exists to stop doing.
    #[serde(default)]
    pub key: Option<String>,
    pub side: DeckSide,
    /// What the question does. Absent means `cross` on George's side and
    /// `direct` on Chuck's — see [`DeckKind`].
    #[serde(default)]
    pub kind: Option<DeckKind>,
    /// The `key` of the George question a redirect answers. Required on a
    /// redirect, refused on anything else.
    #[serde(default)]
    pub follows: Option<String>,
    /// The exhibit this question stands on, as Marie would name it aloud — the
    /// handle the "I'd point to…" picker offers. Absent on every question that
    /// stands on no document of its own.
    #[serde(default)]
    pub source_line: Option<String>,
    /// Who drafted this row, when nobody has reviewed it yet (`architect`). The
    /// deck editor shows a `draft` mark on such rows until they are edited.
    /// Absent on every question a human has settled.
    #[serde(default)]
    pub draft_by: Option<String>,
    pub source_kind: DeckSourceKind,
    /// 1-based position among the scenario's instances or points. Absent on a
    /// `manual` question, required on the other two.
    #[serde(default)]
    pub source_index: Option<usize>,
    /// TACTIC_DECK_v1 card 1–7, or absent when the question carries none.
    #[serde(default)]
    pub tactic: Option<i16>,
    #[serde(default)]
    pub braid_rows: Option<String>,
    pub text: String,
    #[serde(default)]
    pub receipt: Option<String>,
    #[serde(default)]
    pub pair_said: Option<String>,
    #[serde(default)]
    pub pair_admitted: Option<String>,
    #[serde(default)]
    pub watch_for: Option<String>,
    #[serde(default)]
    pub stronger: Option<String>,
    #[serde(default)]
    pub stronger_lean: Option<String>,
}

/// One receipt, under one of her talking points.
///
/// ## Domain note: a STAND-IN, ruled 2026-08-17
///
/// The record of which exhibit backs a point is the PAIRING
/// (`response_item_fact_refs.note`), authored in an editor that is v1 work. Until
/// that exists the reveal would print a named absence under every point, so Roman
/// ruled these seeded with the deck. They lose to a real pairing wherever one
/// appears, which is what stops them becoming a second truth.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckPoint {
    /// The point's PRINTED number — `response_items.item_index + 1`.
    pub position: usize,
    /// Her phrasing of the exhibit, WITHOUT the stored "Backed by:" prefix.
    pub text: String,
}

/// One scenario's whole deck.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckFile {
    /// The scenario this deck is about, as a human writes it: `S-5`.
    pub scenario_code: String,
    /// The receipts under her talking points. Absent or empty is legitimate — a
    /// deck may simply have none, and every point then shows the stored
    /// named-absence line.
    #[serde(default)]
    pub points: Vec<DeckPoint>,
    pub questions: Vec<DeckQuestion>,
}

#[cfg(test)]
#[path = "deck_file_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "deck_shipped_tests.rs"]
mod shipped_tests;
