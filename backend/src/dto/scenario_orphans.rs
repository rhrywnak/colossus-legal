//! Wire shapes for the humane orphan strip (task 1.7C, defect D9).
//!
//! `GET /cases/:slug/scenarios/:scenario_id/facts/orphans` — the saved references
//! whose graph node no longer resolves, grouped by the document they came from.
//!
//! A separate endpoint rather than a new field on the facts list, for two reasons:
//! design §2.8 moves the strip to the bottom of the page (it no longer hangs off
//! the ruling queue, so it no longer wants the queue's payload), and adding an
//! object wrapper to the existing bare-array response would break every caller for
//! no gain.

use serde::Serialize;

// RESPONSE-ONLY, so `Serialize` alone — the same shape as `dto::theme_scan`'s
// `ScanRunHeader`. Deriving `Deserialize` here would add a parse surface nothing
// uses: the backend never reads these back, and the tests construct them directly
// (which is what `PartialEq` is for). A `Deserialize` that no caller exercises is an
// unenforced contract, and the honest way to close that is to not have it rather
// than to bolt `deny_unknown_fields` onto a derive with no reader.

/// How trustworthy a group's label is.
///
/// ## Domain note: three different reasons a label is what it is
///
/// §2.8 forbids inventing titles, which means the client must be able to tell a
/// real document title from a handle derived out of an id. A single `label` string
/// cannot carry that distinction, and a client guessing from the shape of the text
/// would guess wrong. So the state travels beside the label — the same pairing as
/// `CardGrounding { state, label }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OrphanTitleState {
    /// The document still exists and this is its real title.
    Resolved,
    /// The document id parsed, but no such document exists any more — the label is
    /// a slug derived from the id. Measured: 2 of the live 26 refs, pointing at a
    /// `doc-court-of-appeals-ruling-…` id that predates the OCR re-titling to
    /// `…rulling…`.
    Unrecoverable,
    /// The node id did not carry a recognisable document prefix at all. Surfaced as
    /// its own group rather than dropped, so the total never disagrees with the
    /// database.
    Unparseable,
}

/// One document's worth of orphaned references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OrphanGroup {
    /// The document id parsed out of the refs' node ids. Empty for the
    /// `unparseable` group, which has no document to name.
    pub document_id: String,
    /// What the strip shows: the real title, or an id-derived slug. Never invented.
    pub label: String,
    /// Whether `label` is a real title — see [`OrphanTitleState`].
    pub title_state: OrphanTitleState,
    /// How many orphaned references came from this document.
    pub count: usize,
}

/// The whole orphan strip payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScenarioOrphansResponse {
    /// Every orphaned reference, counted once. This is the number the summary line
    /// says out loud ("26 saved references … content unavailable"), and it is the
    /// sum of the groups' counts BY CONSTRUCTION — including the unparseable
    /// group, so a grouping failure can never quietly shrink it.
    pub total: usize,
    /// Ordered biggest-loss-first, then alphabetically. Empty when nothing is
    /// orphaned — a legitimate, and very welcome, empty list.
    pub groups: Vec<OrphanGroup>,
}
