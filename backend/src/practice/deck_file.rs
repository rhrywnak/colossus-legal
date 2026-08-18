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
    pub side: DeckSide,
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

/// One scenario's whole deck.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckFile {
    /// The scenario this deck is about, as a human writes it: `S-5`.
    pub scenario_code: String,
    pub questions: Vec<DeckQuestion>,
}

/// Why a deck file cannot be used.
///
/// Every variant names the QUESTION by its 1-based position in the file, because
/// that is the number a human counts to when they open the file to fix it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DeckError {
    #[error("the deck names no scenario code")]
    NoScenarioCode,

    #[error("the deck holds no questions — seeding it would write nothing and report success")]
    NoQuestions,

    #[error("question {position}: the text is blank")]
    BlankText { position: usize },

    #[error("question {position}: tactic {tactic} is not one of the seven cards (1–7)")]
    UnknownTactic { position: usize, tactic: i16 },

    #[error(
        "question {position}: source_kind '{kind}' needs a source_index, and the deck gives none"
    )]
    MissingSourceIndex { position: usize, kind: &'static str },

    #[error("question {position}: a manual question must carry no source_index, and this one has {index}")]
    ManualWithSourceIndex { position: usize, index: usize },

    #[error("question {position}: source_index is {index}; positions are 1-based")]
    ZeroSourceIndex { position: usize, index: usize },

    #[error("question {position}: the pair needs BOTH halves or neither — {half} is missing")]
    HalfAPair { position: usize, half: &'static str },
}

impl DeckFile {
    /// Prove the file before anything is written.
    ///
    /// ## Why every one of these is a refusal and not a repair
    ///
    /// A deck is authored prose that a witness reads the night before she
    /// testifies. There is no value this code could substitute for a blank
    /// question that would be better than stopping and saying so — and a seed
    /// that quietly dropped a malformed row would produce a five-question deck
    /// with four questions in it, which nobody would notice until the session.
    ///
    /// # Errors
    /// Returns the FIRST [`DeckError`] the file earns, naming the question by its
    /// position in the file.
    pub fn validate(&self) -> Result<(), DeckError> {
        if self.scenario_code.trim().is_empty() {
            return Err(DeckError::NoScenarioCode);
        }
        if self.questions.is_empty() {
            return Err(DeckError::NoQuestions);
        }
        for (i, q) in self.questions.iter().enumerate() {
            q.validate(i + 1)?;
        }
        Ok(())
    }
}

impl DeckQuestion {
    /// Prove one question. See [`DeckFile::validate`] for why each is a refusal.
    fn validate(&self, position: usize) -> Result<(), DeckError> {
        if self.text.trim().is_empty() {
            return Err(DeckError::BlankText { position });
        }
        if let Some(tactic) = self.tactic {
            if !(1..=7).contains(&tactic) {
                return Err(DeckError::UnknownTactic { position, tactic });
            }
        }
        match (self.source_kind, self.source_index) {
            (DeckSourceKind::Manual, Some(index)) => {
                return Err(DeckError::ManualWithSourceIndex { position, index })
            }
            (DeckSourceKind::Manual, None) => {}
            (kind, None) => {
                return Err(DeckError::MissingSourceIndex {
                    position,
                    kind: kind.as_column(),
                })
            }
            (_, Some(0)) => return Err(DeckError::ZeroSourceIndex { position, index: 0 }),
            (_, Some(_)) => {}
        }
        match (&self.pair_said, &self.pair_admitted) {
            (Some(_), None) => {
                return Err(DeckError::HalfAPair {
                    position,
                    half: "pair_admitted",
                })
            }
            (None, Some(_)) => {
                return Err(DeckError::HalfAPair {
                    position,
                    half: "pair_said",
                })
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "deck_file_tests.rs"]
mod tests;
