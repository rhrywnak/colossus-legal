//! What makes a deck file valid, and every way one is refused.
//!
//! Split from [`super::deck_file`] on 2026-08-19 when that module passed Rule
//! 17's 300-line limit. The seam is the honest one: the sibling says what a deck
//! file IS — the shapes serde parses into — and this says what makes one usable.
//!
//! Pure, like its sibling: nothing here opens a file or a connection, so every
//! refusal below is a unit test rather than a deployment.
//!
//! ## Why every one of these is a refusal and not a repair
//!
//! A deck is authored prose that a witness reads the night before she testifies.
//! There is no value this code could substitute for a blank question that would
//! be better than stopping and saying so — and a seed that quietly dropped a
//! malformed row would produce a five-question deck with four questions in it,
//! which nobody would notice until the session.
//!
//! ## Rust Learning: an inherent `impl` in another module
//!
//! `impl DeckFile { … }` below is written HERE while `struct DeckFile` is
//! declared in the sibling. Rust allows that for any type defined in the same
//! CRATE — the coherence rule is about crates, not modules — so a type's data
//! and one coherent family of its methods can be filed separately without a
//! trait, a wrapper or a re-export. It is the plainest way to obey a line limit
//! without inventing a `DeckValidator` nobody would otherwise want.

use super::deck_file::{DeckFile, DeckKind, DeckQuestion, DeckSide, DeckSourceKind};

/// Why a deck file cannot be used.
///
/// Every variant names the row by its 1-based position in the file — the question
/// by its place in `questions`, the receipt by its place in `points` — because
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

    #[error("point receipt {ordinal}: the text is blank")]
    BlankPointReceipt { ordinal: usize },

    #[error("point receipt {ordinal}: position is {position}; points are numbered from 1")]
    ZeroPointPosition { ordinal: usize, position: usize },

    #[error("two point receipts both claim point {position}; a point has one receipt")]
    DuplicatePointPosition { position: usize },

    #[error("question {position}: the key is blank")]
    BlankKey { position: usize },

    #[error("two questions both claim the key '{key}'; a key names one question")]
    DuplicateKey { key: String },

    #[error("question {position}: a {kind} question must carry no `follows`, and this one names '{follows}'")]
    FollowsOnNonRedirect {
        position: usize,
        kind: &'static str,
        follows: String,
    },

    #[error("question {position}: a redirect must say which George question it follows, and this one does not")]
    RedirectWithoutFollows { position: usize },

    #[error("question {position}: `follows: {follows}` names no question in this deck")]
    FollowsUnknownKey { position: usize, follows: String },

    #[error("question {position}: `follows: {follows}` names a {kind} question; a redirect follows a cross")]
    FollowsNotCross {
        position: usize,
        follows: String,
        kind: &'static str,
    },

    #[error("question {position}: the source line is blank — omit it rather than writing nothing")]
    BlankSourceLine { position: usize },

    #[error("question {position}: draft_by is blank — omit it rather than writing nothing")]
    BlankDraftBy { position: usize },
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
        self.validate_keys()?;
        self.validate_points()
    }

    /// Prove the keys, and prove every redirect points at a cross in this file.
    ///
    /// ## Why the `follows` check is HERE and not left to the database
    ///
    /// `follows_key` is deliberately not a foreign key — the file speaks in
    /// keys, not uuids, and the seed writes a George row and its redirect in the
    /// same transaction. So the check an FK would have bought is made here
    /// instead, at the moment a human can still fix the file, and it is stricter
    /// than an FK would have been: it also refuses a redirect that follows
    /// another redirect, which no FK could see.
    fn validate_keys(&self) -> Result<(), DeckError> {
        let mut seen: Vec<&str> = Vec::with_capacity(self.questions.len());
        for question in &self.questions {
            let Some(key) = question.key.as_deref() else {
                continue;
            };
            let key = key.trim();
            if seen.contains(&key) {
                return Err(DeckError::DuplicateKey {
                    key: key.to_string(),
                });
            }
            seen.push(key);
        }

        for (i, question) in self.questions.iter().enumerate() {
            let Some(follows) = question.follows.as_deref().map(str::trim) else {
                continue;
            };
            let position = i + 1;
            let target = self
                .questions
                .iter()
                .find(|q| q.key.as_deref().map(str::trim) == Some(follows))
                .ok_or_else(|| DeckError::FollowsUnknownKey {
                    position,
                    follows: follows.to_string(),
                })?;
            if target.resolved_kind() != DeckKind::Cross {
                return Err(DeckError::FollowsNotCross {
                    position,
                    follows: follows.to_string(),
                    kind: target.resolved_kind().as_column(),
                });
            }
        }
        Ok(())
    }

    /// Prove the point receipts.
    ///
    /// The duplicate check is the one worth having: the column's UNIQUE
    /// constraint would catch it too, but only as a mid-transaction database
    /// error naming a constraint, and only after the questions were written.
    /// Here it is a sentence naming the point, before anything is opened.
    fn validate_points(&self) -> Result<(), DeckError> {
        let mut seen: Vec<usize> = Vec::with_capacity(self.points.len());
        for (i, point) in self.points.iter().enumerate() {
            let ordinal = i + 1;
            if point.text.trim().is_empty() {
                return Err(DeckError::BlankPointReceipt { ordinal });
            }
            if point.position == 0 {
                return Err(DeckError::ZeroPointPosition {
                    ordinal,
                    position: point.position,
                });
            }
            if seen.contains(&point.position) {
                return Err(DeckError::DuplicatePointPosition {
                    position: point.position,
                });
            }
            seen.push(point.position);
        }
        Ok(())
    }
}

impl DeckQuestion {
    /// The kind this question is, resolved against the pre-2026-08-19 default.
    ///
    /// A file that says nothing means what every deck written before the column
    /// existed meant: George cross-examines, Chuck examines directly.
    pub fn resolved_kind(&self) -> DeckKind {
        match (self.kind, self.side) {
            (Some(kind), _) => kind,
            (None, DeckSide::George) => DeckKind::Cross,
            (None, DeckSide::Chuck) => DeckKind::Direct,
        }
    }

    /// Prove one question. See [`DeckFile::validate`] for why each is a refusal.
    /// A blank OPTIONAL field is refused rather than treated as absent.
    ///
    /// The columns behind these carry `btrim(...) <> ''` checks, so a blank one
    /// fails as a mid-transaction constraint error naming no line in the file.
    /// And the intent is different anyway: `key: ""` is somebody who meant to
    /// write a key and did not finish.
    fn validate_optionals(&self, position: usize) -> Result<(), DeckError> {
        for (value, blank) in [
            (self.key.as_deref(), DeckError::BlankKey { position }),
            (
                self.source_line.as_deref(),
                DeckError::BlankSourceLine { position },
            ),
            (
                self.draft_by.as_deref(),
                DeckError::BlankDraftBy { position },
            ),
        ] {
            if value.is_some_and(|v| v.trim().is_empty()) {
                return Err(blank);
            }
        }
        Ok(())
    }

    /// Only a redirect carries `follows`, and every redirect must.
    ///
    /// The column has the same CHECK, but a CHECK violation is a database error
    /// naming a constraint and no line in the file. The other half — that the
    /// key it names is a CROSS question in this deck — is a whole-file question
    /// and lives in [`DeckFile::validate_keys`].
    fn validate_kind(&self, position: usize) -> Result<(), DeckError> {
        match (self.resolved_kind(), self.follows.as_deref()) {
            (DeckKind::Redirect, None) => Err(DeckError::RedirectWithoutFollows { position }),
            (kind, Some(follows)) if kind != DeckKind::Redirect => {
                Err(DeckError::FollowsOnNonRedirect {
                    position,
                    kind: kind.as_column(),
                    follows: follows.to_string(),
                })
            }
            _ => Ok(()),
        }
    }

    /// A manual question carries no source index; the other two must carry one.
    fn validate_source(&self, position: usize) -> Result<(), DeckError> {
        match (self.source_kind, self.source_index) {
            (DeckSourceKind::Manual, Some(index)) => {
                Err(DeckError::ManualWithSourceIndex { position, index })
            }
            (DeckSourceKind::Manual, None) => Ok(()),
            (kind, None) => Err(DeckError::MissingSourceIndex {
                position,
                kind: kind.as_column(),
            }),
            (_, Some(0)) => Err(DeckError::ZeroSourceIndex { position, index: 0 }),
            (_, Some(_)) => Ok(()),
        }
    }

    fn validate(&self, position: usize) -> Result<(), DeckError> {
        if self.text.trim().is_empty() {
            return Err(DeckError::BlankText { position });
        }
        if let Some(tactic) = self.tactic {
            if !(1..=7).contains(&tactic) {
                return Err(DeckError::UnknownTactic { position, tactic });
            }
        }
        self.validate_optionals(position)?;
        self.validate_kind(position)?;
        self.validate_source(position)?;
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
#[path = "deck_valid_tests.rs"]
mod tests;
