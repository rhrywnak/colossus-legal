// =============================================================================
// backend/src/domain/matrix_edge.rs — what the linking pass wrote on an edge
// =============================================================================
//
// PROOF_MATRIX_v2 §1. The linking pass of 2026-09-06 stamped every
// `(Evidence)-[:CORROBORATES|REBUTS]->(Allegation)` edge with a role and a
// confidence. This module owns those two vocabularies and — the part that earns
// it a file — the ORDER they impose, because §3 sorts on both.
//
// ## Domain note: read-only vocabularies
//
// Nothing in this build writes either property. They arrive from a pass that ran
// outside the application, so this module is a READER, and its whole discipline
// is what to do with a token it does not recognise. The answer is never "guess":
// an unknown role is shown (never hidden) and an unknown confidence carries no
// mark, both with an operator-visible warning at the call site. Showing more than
// we can label is safe on a proof surface; hiding something we failed to parse is
// not.
//
// ## Why not a database CHECK, or a `#[serde]` enum on the graph read
//
// neo4rs decodes a property into a `String`; there is no schema to constrain it,
// and 286 edges predate the pass and carry neither property at all. So the parse
// is explicit, at one boundary, and "absent" and "unreadable" stay distinct —
// they mean different things (an older edge, versus a newer pass this build has
// not caught up with).

use serde::{Deserialize, Serialize};

/// What the linking pass said an item is DOING under an accusation.
///
/// ## Domain note: only one of these hides anything
///
/// `DoesNotBelong` is the pass's own retraction — it looked at an edge the
/// extraction had drawn and said it should not be there. `DuplicateOf` folds a
/// row into another row but removes nothing. The other three are descriptive and
/// change no behaviour beyond the line printed under the quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeRole {
    /// The item proves the accusation directly.
    DirectProof,
    /// The other side's own words — an admission, a sworn answer.
    TheirOwnWords,
    /// Background that makes the accusation intelligible without proving it.
    Context,
    /// A near-identical restatement of another item, named by
    /// `duplicate_of_card_id`.
    DuplicateOf,
    /// The pass's retraction: this edge should not be under this accusation.
    DoesNotBelong,
}

impl EdgeRole {
    /// The full vocabulary — the "extensible list in code".
    pub const ALL: &'static [EdgeRole] = &[
        EdgeRole::DirectProof,
        EdgeRole::TheirOwnWords,
        EdgeRole::Context,
        EdgeRole::DuplicateOf,
        EdgeRole::DoesNotBelong,
    ];

    /// The token as the graph holds it, in `r.role`.
    pub fn code(self) -> &'static str {
        match self {
            EdgeRole::DirectProof => "direct_proof",
            EdgeRole::TheirOwnWords => "their_own_words",
            EdgeRole::Context => "context",
            EdgeRole::DuplicateOf => "duplicate_of",
            EdgeRole::DoesNotBelong => "does_not_belong",
        }
    }

    /// Whether this role hides the item from the default list (§3).
    ///
    /// Stated once here rather than as a `== DoesNotBelong` test at each of the
    /// three call sites — the ordering, the export and the hidden count must
    /// agree, and three copies of one comparison is how they stop agreeing.
    pub fn hides_the_item(self) -> bool {
        matches!(self, EdgeRole::DoesNotBelong)
    }

    /// Whether this role makes the item a candidate for folding under another.
    pub fn is_duplicate(self) -> bool {
        matches!(self, EdgeRole::DuplicateOf)
    }
}

/// The error produced when a stored role token is not one this build knows.
#[derive(Debug, thiserror::Error, PartialEq)]
#[error("unknown edge role '{token}' — this build knows direct_proof/their_own_words/context/duplicate_of/does_not_belong")]
pub struct EdgeRoleParseError {
    pub token: String,
}

impl TryFrom<&str> for EdgeRole {
    type Error = EdgeRoleParseError;

    fn try_from(token: &str) -> Result<Self, Self::Error> {
        EdgeRole::ALL
            .iter()
            .copied()
            .find(|role| role.code() == token)
            .ok_or_else(|| EdgeRoleParseError {
                token: token.to_string(),
            })
    }
}

/// How sure the linking pass was that the item belongs under the accusation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkConfidence {
    High,
    Medium,
    Low,
}

impl LinkConfidence {
    /// The full vocabulary, STRONGEST FIRST — which is also §3's sort order, so
    /// the list and the ordering cannot drift apart.
    pub const ALL: &'static [LinkConfidence] = &[
        LinkConfidence::High,
        LinkConfidence::Medium,
        LinkConfidence::Low,
    ];

    /// The token as the graph holds it, in `r.confidence`.
    pub fn code(self) -> &'static str {
        match self {
            LinkConfidence::High => "high",
            LinkConfidence::Medium => "medium",
            LinkConfidence::Low => "low",
        }
    }

    /// Sort weight: lower sorts first. §3 orders `high > medium > older-untagged`.
    ///
    /// ## Domain note: why `low` sits between medium and untagged
    ///
    /// §3 names three states and the graph currently holds only two of them
    /// (`high` and `medium`, plus 286 edges with nothing at all). `low` is in the
    /// declared vocabulary and will appear the first time a pass emits one, so it
    /// needs a place now rather than a surprise later. It sorts BELOW `medium`
    /// because it is a weaker claim, and ABOVE untagged because it is still a
    /// claim — an edge nobody rated is not the same as one rated poorly, and the
    /// 250 pre-existing edges must not be promoted above a rating by accident.
    ///
    /// ## Rust Learning: an explicit weight instead of `#[derive(Ord)]`
    ///
    /// Deriving `Ord` would order the variants by DECLARATION POSITION — correct
    /// today, and silently wrong the day someone inserts a variant in the middle
    /// for readability. The weight is a decision, so it is written down as one.
    pub fn sort_weight(self) -> u8 {
        match self {
            LinkConfidence::High => 0,
            LinkConfidence::Medium => 1,
            LinkConfidence::Low => 2,
        }
    }

    /// The sort weight of an item carrying NO confidence — sorts after every
    /// rated one. A free function rather than a fourth variant, because "absent"
    /// is not a rating and giving it a variant would let it be written to a
    /// column.
    pub const UNRATED_SORT_WEIGHT: u8 = 3;
}

/// The error produced when a stored confidence token is not one this build knows.
#[derive(Debug, thiserror::Error, PartialEq)]
#[error("unknown link confidence '{token}' — this build knows high/medium/low")]
pub struct LinkConfidenceParseError {
    pub token: String,
}

impl TryFrom<&str> for LinkConfidence {
    type Error = LinkConfidenceParseError;

    fn try_from(token: &str) -> Result<Self, Self::Error> {
        LinkConfidence::ALL
            .iter()
            .copied()
            .find(|c| c.code() == token)
            .ok_or_else(|| LinkConfidenceParseError {
                token: token.to_string(),
            })
    }
}

#[cfg(test)]
#[path = "matrix_edge_tests.rs"]
mod tests;
