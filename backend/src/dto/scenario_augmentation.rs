//! Wire DTOs for the augmentation panel (task 1.4, v2 §8).
//!
//! C1 identity, C4 human facts, C5 talking points — the three human-authored
//! components, as the panel reads and writes them.
//!
//! ## Everything user-visible arrives composed
//!
//! Same discipline as the 1.2 card payload: the authored tag ("Added by Roman"),
//! the qualified date ("Around 2009-04-21") and the person-reference caveat are
//! all built server-side. The browser renders them and composes nothing.
//!
//! ## Why the human content carries a tag at all
//!
//! Every other fact in this system is anchored so it can be defended. These carry
//! an AUTHOR instead — that is the whole shape of C4/C5 — so the tag is not
//! decoration, it is the provenance. A reader must never be in doubt which kind
//! of statement they are looking at (§8: "visibly tagged human-authored, no
//! citation").

use serde::{Deserialize, Serialize};

/// One human fact, ready to render.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanFactDto {
    pub id: String,
    pub text: String,
    /// The date as it should READ, with its qualifier — "Around 2009-04-21".
    /// `None` when the fact carries no date, which is the common case.
    pub date_label: Option<String>,
    /// People this fact mentions, as typed by a human.
    pub person_refs: Vec<String>,
    /// Whether those names are resolved entities. Always `false` before task B0.
    ///
    /// ## Domain note: why the flag ships rather than being assumed
    ///
    /// The panel says "typed, not linked" beside these names. If B0 later
    /// resolves them, the same payload can say so without the client changing —
    /// and until then nobody reads a typed "Phillips" as a resolved reference to
    /// one particular Phillips.
    pub person_refs_are_linked: bool,
    /// "Added by Roman" — the tag §8 requires, composed server-side.
    pub authored_tag: String,
    /// True when the fact has been edited since it was written.
    pub edited: bool,
}

/// One talking point, ready to render.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TalkingPointDto {
    pub text: String,
    /// Its place in the ordered list, server-owned.
    pub position: i32,
    /// "Added by Marie" — present when the row records an author.
    pub authored_tag: Option<String>,
}

/// The C1 identity fields the panel edits.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioIdentityDto {
    /// `"S-3"`, formatted by the backend.
    pub code: String,
    pub name: String,
    pub direction: String,
    /// Our one-sentence answer. `None` until a human writes it.
    pub theme_statement: Option<String>,
    /// What they want the jury to believe. `None` until written.
    pub motivation: Option<String>,
    /// The attack as the OTHER side frames it, read from the definition body.
    ///
    /// Shown beside the theme statement precisely so the two are never confused:
    /// one is what they say, the other is how we answer it.
    pub attack_text: Option<String>,
}

/// The whole augmentation panel in one read.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentationPanelDto {
    pub identity: ScenarioIdentityDto,
    pub human_facts: Vec<HumanFactDto>,
    /// C6 — the human-flagged watch-list notes (task 1.5).
    ///
    /// A SEPARATE list rather than a `kind` field on `HumanFactDto` the client
    /// filters on: the two render in different places (facts in the working
    /// view, watch-list in rehearsal's fourth block), and a client that forgot
    /// the filter would show watch-list notes as facts — the wrong kind of
    /// statement in the wrong place, which is the confusion §8's tagging exists
    /// to prevent. Splitting them server-side makes that miss impossible.
    pub watch_list: Vec<HumanFactDto>,
    pub talking_points: Vec<TalkingPointDto>,
    /// How many talking points this scenario may carry, from the 1.6 seam.
    ///
    /// Served rather than hardcoded in the browser: it is a tunable, and a client
    /// that baked in "3" would show the wrong limit the day 1.6 changes it.
    pub talking_points_cap: usize,
}

/// Request body for adding a human fact.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddHumanFactRequest {
    pub text: String,
    /// `fact` (default) | `watch_list`.
    ///
    /// Defaulted rather than required so the 1.4 clients that predate the
    /// watch-list keep working and keep meaning what they meant — every write
    /// before task 1.5 was a fact, which is exactly what the column's backfill
    /// says about the rows already stored.
    #[serde(default)]
    pub kind: Option<String>,
    /// ISO date (`YYYY-MM-DD`), optional.
    #[serde(default)]
    pub occurred_on: Option<String>,
    /// `exact` | `around` | `range` | `ordered`. Only meaningful with a date.
    #[serde(default)]
    pub date_type: Option<String>,
    #[serde(default)]
    pub person_refs: Vec<String>,
}

/// Request body for replacing a scenario's talking points.
///
/// ## Why the whole list, not one point
///
/// C5 is a short ordered list curated as a whole — reordering, or dropping the
/// middle point, is the normal edit. Sending the list keeps `position`
/// server-owned; an append-only API would make every reorder a delete-then-add
/// dance in the client and put the ordering in the browser's hands.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetTalkingPointsRequest {
    pub points: Vec<String>,
}
