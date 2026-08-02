//! The rehearsal payload (task 1.5, v2 §10).
//!
//! ## The exclusion law is enforced by CONSTRUCTION
//!
//! §10 excludes from rehearsal mode — and from every rehearsal export — strategy
//! and motivation notes, pinpoint impeachment sourcing, readiness verdicts, all
//! internal vocabulary, and all percentages.
//!
//! These types cannot carry any of it, because they have no field for it. There
//! is no `motivation`, no `confidence`, no `status`, no `graph_node_id`, no page,
//! no document id. A future edit that wanted to leak one would have to ADD a
//! field, which is a visible change to this file rather than a line slipped into
//! a mapper. `RehearsalPayloadTests` then asserts the serialized bytes over a
//! banned-substring list, so even a leak smuggled inside a string is caught.
//!
//! ## Why exclusion is not a restriction on Marie
//!
//! Marie's ACCESS is the full working view — evidence, pinpoints, rulings,
//! verdicts, motivation, everything (v2 §10). The mode is slim; her rights are
//! not. What is excluded here is excluded because a witness rehearsing testimony
//! should be working from themes rather than from the machinery — over-scripting
//! is an ethics hazard (ABA Formal Opinion 508) and produces worse testimony.
//!
//! ## Everything arrives composed
//!
//! Same discipline as every payload since 1.2. The frontend renders and builds
//! nothing.

use serde::{Deserialize, Serialize};

/// One of Marie's talking points, optionally paired with the exhibit behind it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalPoint {
    /// Her words, verbatim as authored.
    pub text: String,
    /// The one exhibit that backs this point, in HER phrasing — "My certified
    /// letter".
    ///
    /// ## Domain note: authored, never derived
    ///
    /// This is the human's label from `response_item_fact_refs.note`, never a
    /// document title assembled from the record. Deriving it would put words in
    /// the witness's mouth, and it would drag pinpoint sourcing into a payload
    /// the exclusion law keeps it out of.
    ///
    /// `None` until the pairing editor exists (tracker task 3.9) — honest, where
    /// an invented label would not be.
    pub exhibit: Option<String>,
}

/// One ready scenario, as Marie rehearses it. The four §10 blocks, and nothing
/// else.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalScenario {
    /// The human handle — "S-2". Speakable, and the only identifier here: there
    /// is deliberately no scenario UUID and no node id on this payload.
    pub code: String,
    /// Block 1 — the theme, C1's one plain sentence. `None` when nobody has
    /// written it; the block is then absent rather than filled with a stand-in.
    pub theme: Option<String>,
    /// Block 2 — what they claim, in plain words.
    pub attack: Option<String>,
    /// Block 3 — her talking points, ordered, at most the configured cap.
    pub points: Vec<RehearsalPoint>,
    /// Block 4 — what the other side will wave around.
    pub watch_list: Vec<String>,
}

/// The rehearsal list: every ready scenario, plus the standing card.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehearsalPayload {
    pub scenarios: Vec<RehearsalScenario>,
    /// The standing card, shown on every screen (v2 §10). Served rather than
    /// compiled into the page so one wording serves the mode and the exports.
    pub standing_card: Vec<String>,
}
