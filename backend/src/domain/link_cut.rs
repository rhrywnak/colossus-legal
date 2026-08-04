// =============================================================================
// backend/src/domain/link_cut.rs — which way a human-linked statement cuts
// =============================================================================
//
// Task 2.10. When a human links a statement to an accusation the extraction
// never linked, they must also say which way it runs: does it help us, or will
// the other side wield it? This module owns that two-word vocabulary and the
// three-word vocabulary of the ledger beside it.
//
// ## Domain note: why the cut is REQUIRED, and not merely nice to have
//
// A link with no cut would reach the readiness verdict as a fact that "this
// statement bears on ¶41" with no indication whether that is ammunition or a
// hazard. The verdict would then count a landmine as a weapon. So the write path
// refuses a link without one, the column is NOT NULL, and this enum is what makes
// "a cut is one of exactly two things" a property of the TYPE rather than of a
// convention someone has to remember.
//
// ## Why the vocabulary lives here and not in a database CHECK
//
// Same reasoning as `FactStatus` and `HumanFactKind`: a CHECK constraint has to
// be migrated in lockstep with the code that widens the list, and — the stronger
// argument — a CHECK cannot tell you the PARSER is missing. A token the database
// accepts but this build cannot read is a silent wrong answer; a token refused
// here is a loud one, naming the offending value and the whole accepted
// vocabulary.

use serde::{Deserialize, Serialize};

/// Which way a linked statement cuts for our side.
///
/// ## Rust Learning: `#[serde(rename_all = "snake_case")]` on a closed enum
///
/// Each variant maps to its snake_case token on the wire; the enum is CLOSED, so
/// a token this build does not define fails to parse rather than defaulting to
/// one of the two. There is no flattering default available here — guessing
/// "supports" for an unreadable value would tell a lawyer that a hazard is a
/// weapon, which is the single worst mistake this vocabulary could make.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkCut {
    /// The statement helps our side.
    Supports,
    /// The other side will use it against us.
    Against,
}

impl LinkCut {
    /// The full, ordered vocabulary — the "extensible list in code".
    ///
    /// Ordered as the panel offers them: the favourable reading first, matching
    /// the design's mock-up, so the buttons and this list cannot drift.
    pub const ALL: &'static [LinkCut] = &[LinkCut::Supports, LinkCut::Against];

    /// The stable token stored in `evidence_allegation_links.cut`.
    pub fn code(self) -> &'static str {
        match self {
            LinkCut::Supports => "supports",
            LinkCut::Against => "against",
        }
    }
}

/// The error produced when a stored or submitted cut token is not one this build
/// knows.
#[derive(Debug, thiserror::Error, PartialEq)]
#[error("unknown cut '{token}' — a link cuts one of two ways: supports/against")]
pub struct LinkCutParseError {
    pub token: String,
}

/// ## Rust Learning: `TryFrom<&str>` as the READ boundary
///
/// The database column and the request body both arrive as raw text, so this is
/// the single place either becomes a typed value — or a loud error. Same
/// discipline as `FactStatus::try_from` and `HumanFactKind::try_from`: there is
/// no `unwrap_or(Supports)` anywhere downstream, because downstream never sees a
/// string.
impl TryFrom<&str> for LinkCut {
    type Error = LinkCutParseError;

    fn try_from(token: &str) -> Result<Self, Self::Error> {
        match token {
            "supports" => Ok(LinkCut::Supports),
            "against" => Ok(LinkCut::Against),
            other => Err(LinkCutParseError {
                token: other.to_string(),
            }),
        }
    }
}

/// What a human did to a link, as the append-only ledger records it.
///
/// ## Domain note: why `Recut` is its own act
///
/// Changing a link from "supports us" to "they'll use it against us" is not a new
/// link and it is not an unlink — it is a human reversing a judgment about the
/// same pair, which is exactly the kind of change the ledger exists to preserve.
/// Folding it into `Link` would leave the record saying the pair was linked
/// twice, which is a different (and untrue) story.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkAction {
    /// A pair that had no link now has one.
    Link,
    /// A pair that was already linked now cuts the other way.
    Recut,
    /// The link was withdrawn; no row remains in the state table.
    Unlink,
}

impl LinkAction {
    /// The full vocabulary — the "extensible list in code".
    pub const ALL: &'static [LinkAction] =
        &[LinkAction::Link, LinkAction::Recut, LinkAction::Unlink];

    /// The stable token stored in `evidence_allegation_link_events.action`.
    pub fn code(self) -> &'static str {
        match self {
            LinkAction::Link => "link",
            LinkAction::Recut => "recut",
            LinkAction::Unlink => "unlink",
        }
    }

    /// Whether this act leaves a cut behind to record.
    ///
    /// An unlink does not: there is no cut in force afterwards, and writing the
    /// old one into the ledger would make the withdrawal read as though it had
    /// asserted something. This is the rule the ledger's nullable `cut` column
    /// encodes, expressed once so the writer cannot get it wrong per call site.
    pub fn carries_cut(self) -> bool {
        !matches!(self, LinkAction::Unlink)
    }
}

/// The error produced when a stored action token is not one this build knows.
#[derive(Debug, thiserror::Error, PartialEq)]
#[error("unknown link action '{token}' — not one of link/recut/unlink")]
pub struct LinkActionParseError {
    pub token: String,
}

impl TryFrom<&str> for LinkAction {
    type Error = LinkActionParseError;

    fn try_from(token: &str) -> Result<Self, Self::Error> {
        match token {
            "link" => Ok(LinkAction::Link),
            "recut" => Ok(LinkAction::Recut),
            "unlink" => Ok(LinkAction::Unlink),
            other => Err(LinkActionParseError {
                token: other.to_string(),
            }),
        }
    }
}

#[cfg(test)]
#[path = "link_cut_tests.rs"]
mod tests;
