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
    /// Its place in the ordered list, server-owned — and its ADDRESS.
    ///
    /// 1-based since task 2.11 C: it is the number printed in the pill beside
    /// the point AND the segment `PUT …/talking-points/:position` matches, and
    /// those must be one number. It was the raw 0-based `item_index` until then,
    /// which nothing rendered — the client counted the array instead, so two
    /// numbers described one row and only one of them was on the wire.
    pub position: usize,
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
    /// Every word the two authoring sections speak (task 2.11 C, ruling C4b).
    ///
    /// They moved out of the components when those components became SHARED with
    /// the rehearsal page, whose standing law is that every visible word is a
    /// stored row. A component holding a literal cannot be reused on a surface
    /// that forbids one — so the components now take their words as a prop, and
    /// each surface serves its own.
    pub wording: AuthoringWordingDto,
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

/// The words the talking-points and watch-list sections speak on THIS page.
///
/// Mirrors `domain::wording_authoring::AuthoringWording` field for field. The
/// rehearsal page serves a different set for the same components — see that
/// module's header for why one row cannot serve both voices.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoringWordingDto {
    pub points_section_heading: String,
    /// Carries `{cap}` — filled by the client, the one value only it is composing
    /// a sentence around. (The cap itself is served beside it.)
    pub points_section_meta_template: String,
    pub points_empty_notice: String,
    pub points_no_exhibit_notice: String,
    pub points_add_label: String,
    pub points_edit_label: String,
    pub points_save_label: String,
    pub points_saving_label: String,
    pub points_cancel_label: String,
    /// Carries `{cap}`.
    pub points_cap_reached_notice: String,
    /// Carries `{n}`.
    pub points_field_label_template: String,
    pub points_authoring_note: String,
    pub points_save_failed_notice: String,
    pub watch_section_heading: String,
    pub watch_section_meta: String,
    pub watch_field_label: String,
    pub watch_add_label: String,
    pub watch_save_label: String,
    pub watch_edit_label: String,
    pub watch_cancel_label: String,
    pub watch_remove_label: String,
    pub watch_edited_suffix: String,
    pub watch_save_failed_notice: String,
}

/// Rewrite ONE authored line — a talking point or a watch item (task 2.11 C).
///
/// ## Why one request type serves both
///
/// The body is the same single field, the rule is the same (non-blank, stored
/// verbatim), and the two routes differ only in what the URL addresses. A second
/// identical struct would be a second place for the rule to drift.
///
/// ## Why the whole-list write above still exists
///
/// The two answer different intentions. This one is "fix a typo in point 2" and
/// must leave the row's author and its written-on date alone; the list write is
/// "rearrange or drop one", which is a change to the list itself. Ruling C4b:
/// update = update, never remove-and-re-add.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditAuthoredLineRequest {
    pub text: String,
}
