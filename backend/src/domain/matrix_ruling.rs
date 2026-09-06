// =============================================================================
// backend/src/domain/matrix_ruling.rs — a human's verdict on one ranked item
// =============================================================================
//
// PROOF_MATRIX_v2 §2. The linking pass ranked 1,194 Evidence→Allegation edges
// and gave each one a role. Everything it wrote is a MACHINE claim. This module
// owns the two-word vocabulary a human uses to overrule it — keep this item, or
// take it out of the list — and the three-word vocabulary of the ledger beside
// it.
//
// ## Domain note: why "keep" is a real ruling and not just "not removed"
//
// The default state of an item is UNRULED: the machine put it there and nobody
// has looked. `Keep` says a human looked and agreed, and that is the difference
// between a paragraph nobody has read and one Roman has been through. The
// ordering in §3 sorts kept items to the top for exactly that reason, so the two
// states cannot be collapsed into one without changing what the reader sees.
//
// ## Why the vocabulary lives here and not in a database CHECK
//
// The same argument `link_cut` makes, and it is worth restating because this
// table carries a stronger claim: a CHECK constraint cannot tell you the PARSER
// is missing. A token Postgres accepts but this build cannot read would render
// as *some* state on a proof surface — and the wrong one of two states here is
// the difference between "Roman confirmed this" and "Roman struck this out".
// Refused loudly at the read boundary, naming the token, is the only safe answer.

use serde::{Deserialize, Serialize};

/// What a human decided about one ranked Evidence→Allegation item.
///
/// ## Rust Learning: `#[serde(rename_all = "snake_case")]` on a closed enum
///
/// Each variant maps to its snake_case token on the wire, and the enum is
/// CLOSED — a token this build does not define fails to parse rather than
/// defaulting to a neighbour. There is no safe default to fall back to: guessing
/// `Keep` would print a struck item in a Word export bound for opposing counsel,
/// and guessing `Remove` would hide proof Roman confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixRuling {
    /// A human confirmed this item belongs. It sorts first and prints in the
    /// export with the confirmed mark.
    Keep,
    /// A human took this item out. Hidden by default, revealed greyed behind the
    /// "show hidden" toggle with an Undo, and never printed in the export.
    Remove,
}

impl MatrixRuling {
    /// The full, ordered vocabulary — the "extensible list in code".
    ///
    /// Ordered as the two buttons appear on the row: Keep first, because the
    /// affirmative act is the common one and the destructive one should never be
    /// the nearer target.
    pub const ALL: &'static [MatrixRuling] = &[MatrixRuling::Keep, MatrixRuling::Remove];

    /// The stable token stored in `evidence_allegation_rulings.ruling`.
    pub fn code(self) -> &'static str {
        match self {
            MatrixRuling::Keep => "keep",
            MatrixRuling::Remove => "remove",
        }
    }

    /// Whether an item bearing this ruling is hidden from the default list.
    ///
    /// Stated once, here, rather than as a `== Remove` test at each of the three
    /// call sites (the ordering, the export, the hidden-count) — because those
    /// three must agree, and three copies of one comparison is how they stop
    /// agreeing.
    pub fn hides_the_item(self) -> bool {
        matches!(self, MatrixRuling::Remove)
    }
}

/// The error produced when a stored or submitted ruling token is not one this
/// build knows.
#[derive(Debug, thiserror::Error, PartialEq)]
#[error("unknown ruling '{token}' — an item is ruled one of two ways: keep/remove")]
pub struct MatrixRulingParseError {
    pub token: String,
}

/// ## Rust Learning: `TryFrom<&str>` as the READ boundary
///
/// The `ruling` column and the request body both arrive as raw text, so this is
/// the single place either becomes a typed value — or a loud error. Nothing
/// downstream ever sees the string, which is why no `unwrap_or` can exist there.
impl TryFrom<&str> for MatrixRuling {
    type Error = MatrixRulingParseError;

    fn try_from(token: &str) -> Result<Self, Self::Error> {
        match token {
            "keep" => Ok(MatrixRuling::Keep),
            "remove" => Ok(MatrixRuling::Remove),
            other => Err(MatrixRulingParseError {
                token: other.to_string(),
            }),
        }
    }
}

/// What a human did to a ruling, as the append-only ledger records it.
///
/// ## Domain note: why `Rerule` is its own act
///
/// Moving an item from `remove` back to `keep` is not a fresh ruling and it is
/// not a withdrawal — it is a human reversing their own judgment about the same
/// item. Folding it into `Rule` would leave the record saying the item was ruled
/// twice, which is a different and untrue story; folding it into `Withdraw`
/// would lose the fact that a decision is still in force.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RulingAction {
    /// An item that had no ruling now has one.
    Rule,
    /// An item that was already ruled now carries the other verdict.
    Rerule,
    /// The ruling was withdrawn; no row remains in the state table, and the item
    /// returns to the machine's own ordering.
    Withdraw,
}

impl RulingAction {
    /// The full vocabulary — the "extensible list in code".
    pub const ALL: &'static [RulingAction] = &[
        RulingAction::Rule,
        RulingAction::Rerule,
        RulingAction::Withdraw,
    ];

    /// The stable token stored in `evidence_allegation_ruling_events.action`.
    pub fn code(self) -> &'static str {
        match self {
            RulingAction::Rule => "rule",
            RulingAction::Rerule => "rerule",
            RulingAction::Withdraw => "withdraw",
        }
    }

    /// Whether this act leaves a ruling behind to record.
    ///
    /// A withdrawal does not: there is no verdict in force afterwards, and
    /// writing the old one into the ledger would make the withdrawal read as
    /// though it had asserted something. Same rule, same reason, as
    /// [`crate::domain::link_cut::LinkAction::carries_cut`].
    pub fn carries_ruling(self) -> bool {
        !matches!(self, RulingAction::Withdraw)
    }
}

/// The error produced when a stored action token is not one this build knows.
#[derive(Debug, thiserror::Error, PartialEq)]
#[error("unknown ruling action '{token}' — not one of rule/rerule/withdraw")]
pub struct RulingActionParseError {
    pub token: String,
}

impl TryFrom<&str> for RulingAction {
    type Error = RulingActionParseError;

    fn try_from(token: &str) -> Result<Self, Self::Error> {
        match token {
            "rule" => Ok(RulingAction::Rule),
            "rerule" => Ok(RulingAction::Rerule),
            "withdraw" => Ok(RulingAction::Withdraw),
            other => Err(RulingActionParseError {
                token: other.to_string(),
            }),
        }
    }
}

#[cfg(test)]
#[path = "matrix_ruling_tests.rs"]
mod tests;
