//! Request bodies for the two curation writes (task 2.13).
//!
//! Lives in `dto/` with every other wire shape rather than inline in the handler
//! module — the house layout, and what keeps `api::scenario_fact_curation` inside
//! the 300-line limit now that it carries four routes.

use serde::Deserialize;

use crate::domain::fact_tier::FactTier;

/// Body of the tier write.
///
/// `deny_unknown_fields` so a typo'd key is a 400 at the parse boundary rather
/// than a silently ignored field, matching `FactActionRequest`'s stance. The
/// `tier` field is the [`FactTier`] enum itself, so an undefined token fails to
/// deserialize before the handler runs.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetTierRequest {
    pub tier: FactTier,
}

/// Body of the order write: the fact's two new neighbours.
///
/// ## Domain note: neighbours, not an index
///
/// An index describes a position in the list the BROWSER last drew. If anything
/// has changed since — somebody else ruled, a merge landed — index 4 is a
/// different row and the drop lands silently in the wrong place. Naming the two
/// facts it was dropped between lets the server refuse by name when one of them
/// is gone, which is the honest answer to a stale page.
///
/// Both `None` means the list has exactly one fact in that block. `after: None`
/// is a drop at the very top; `before: None` at the very bottom.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveFactRequest {
    /// The fact it now sits BELOW, or `None` when dropped at the top.
    #[serde(default)]
    pub after: Option<String>,
    /// The fact it now sits ABOVE, or `None` when dropped at the bottom.
    #[serde(default)]
    pub before: Option<String>,
}
