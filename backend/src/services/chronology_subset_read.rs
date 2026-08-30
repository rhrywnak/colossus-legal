//! What a subset READ composes into. Pure — no database.
//!
//! The handlers fetch; this module arranges. The split is the same one
//! `chronology_read` makes, and it buys the same thing: every ordering rule and
//! every count below is reachable by a unit test that states its world in a few
//! lines, with no Postgres anywhere.
//!
//! ## ⚑ THE EVENT SHAPE IS BORROWED, NEVER REBUILT
//!
//! [`build_subset_detail`] does not construct a `TimelineEventDto`. It calls
//! `chronology_read::event_dto` — the same function `GET /api/timeline` uses —
//! and then adds the two facts the subset owns. That is design §4's "references,
//! never copies" enforced at the composition layer as well as in the schema: a
//! subset cannot render an event differently from the timeline, because it does
//! not know how to render an event at all.

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::domain::scenario_code::scenario_code;
use crate::dto::chronology_subset::{
    ScenarioSubsetDto, SubsetDetailDto, SubsetEventDto, SubsetSummaryDto,
};
use crate::repositories::pipeline_repository::chronology::ChronologyLinkRow;
use crate::repositories::pipeline_repository::chronology_subsets::{
    ChronologySubsetRow, ScenarioSubsetRow, SubsetCarrierRow, SubsetCountsRow, SubsetEventRefRow,
};
use crate::repositories::pipeline_repository::chronology_write::ChronologyEventStateRow;
use crate::services::chronology_read::{event_dto, link_dtos, Composed};
use crate::services::chronology_write_response::as_read_row;

/// Group carrier rows into `subset id → ["S-11", "S-12"]`.
///
/// The rows arrive already ordered by `(position, code_ordinal)`, so this only
/// buckets them — the ORDER is the query's, stated once, rather than re-derived
/// here from a field this function would have to be trusted to remember.
///
/// ## Domain note: the backend spells the handle
///
/// `S-11` is a name humans have said out loud and written in margins.
/// `domain::scenario_code` owns the spelling for every surface, so a screen
/// never learns that the separator is a hyphen — which is the same standing rule
/// that keeps business logic out of the frontend.
pub fn carriers_by_subset(rows: &[SubsetCarrierRow]) -> HashMap<Uuid, Vec<String>> {
    let mut out: HashMap<Uuid, Vec<String>> = HashMap::new();
    for row in rows {
        out.entry(row.subset_id)
            .or_default()
            .push(scenario_code(row.code_ordinal));
    }
    out
}

/// `subset id → (event_count, gap_count)`.
pub fn counts_by_subset(rows: &[SubsetCountsRow]) -> HashMap<Uuid, (i64, i64)> {
    rows.iter()
        .map(|r| (r.subset_id, (r.event_count, r.gap_count)))
        .collect()
}

/// The home section's list.
///
/// A subset missing from either map gets `0, 0` and an empty carrier list rather
/// than being dropped: a story with no events and no scenario is a real state an
/// author passes through on the way to a real one, and dropping it would make it
/// unreachable from the only screen that could fix it.
pub fn build_subset_list(
    subsets: &[ChronologySubsetRow],
    counts: &HashMap<Uuid, (i64, i64)>,
    carriers: &HashMap<Uuid, Vec<String>>,
) -> Vec<SubsetSummaryDto> {
    subsets
        .iter()
        .map(|row| {
            let (event_count, gap_count) = counts.get(&row.id).copied().unwrap_or((0, 0));
            SubsetSummaryDto {
                id: row.id,
                name: row.name.clone(),
                description: row.description.clone(),
                event_count,
                gap_count,
                carried_by: carriers.get(&row.id).cloned().unwrap_or_default(),
                created_by: row.created_by.clone(),
                created_at: row.created_at,
                updated_by: row.updated_by.clone(),
                updated_at: row.updated_at,
            }
        })
        .collect()
}

/// Everything one subset-detail read composes from.
///
/// ## Rust Learning: a parameter struct instead of six arguments
///
/// Three of the six are slices of different row types and one is a `HashMap` —
/// more positions than a reader tracks at a call site, and clippy's argument
/// limit agrees. The same shape `TimelineSources` uses for the timeline read.
#[derive(Debug, Clone, Copy)]
pub struct SubsetDetailSources<'a> {
    pub subset: &'a ChronologySubsetRow,
    /// The references, in `position` order as the repository read them.
    pub refs: &'a [SubsetEventRefRow],
    /// The events those references point at, in ANY state — a removed event is
    /// here too, and is what becomes a marked gap.
    pub events: &'a [ChronologyEventStateRow],
    /// Every link for those events, in one flat list.
    pub links: &'a [ChronologyLinkRow],
    /// Only events that HAVE notes appear; a missing event has zero.
    pub note_counts: &'a HashMap<Uuid, i64>,
    /// Which linked document ids exist, from one query.
    pub resolved_documents: &'a HashSet<String>,
    /// The scenario codes carrying this subset, in attachment order.
    pub carried_by: &'a [String],
}

/// The whole `GET /api/timeline/subsets/:id` payload.
///
/// ## ⚑ A REFERENCE WHOSE EVENT IS MISSING ENTIRELY
///
/// It is reported as a warning and DROPPED from the list, and that is the one
/// place this read loses a row. It is unreachable while the foreign key holds:
/// `chronology_subset_events.event_id` references `chronology_events(id)`, and a
/// soft-deleted event is still a row. Reaching it means the constraint is gone,
/// which is an operator's problem and is logged as one — never a silent short
/// list.
pub fn build_subset_detail(sources: SubsetDetailSources<'_>) -> Composed<SubsetDetailDto> {
    // Only the two fields THIS function reads are destructured; the rest are
    // [`compose_events`]'s and are passed to it whole. Naming them all here and
    // using two would be five bindings a reader has to check the fate of.
    let SubsetDetailSources {
        subset, carried_by, ..
    } = sources;
    let mut warnings = Vec::new();
    let out = compose_events(&sources, &mut warnings);
    let gap_count = out.iter().filter(|e| e.removed).count() as i64;
    Composed {
        payload: SubsetDetailDto {
            id: subset.id,
            name: subset.name.clone(),
            description: subset.description.clone(),
            event_count: out.len() as i64,
            gap_count,
            events: out,
            carried_by: carried_by.to_vec(),
            created_by: subset.created_by.clone(),
            created_at: subset.created_at,
            updated_by: subset.updated_by.clone(),
            updated_at: subset.updated_at,
            deleted_at: subset.deleted_at,
        },
        warnings,
    }
}

/// Every reference, composed, in the order the repository read them.
///
/// The two indexes it builds are why this is one pass and not one query per
/// event: `by_id` turns "which event does this reference point at" into a
/// lookup, and `links_by_event` does the same for the flat link list. At fifteen
/// events that is arithmetic rather than a performance argument — but the shape
/// is what keeps the composer honest about NOT going back to the database, which
/// it has no pool to do anyway.
///
/// ## ⚑ A REFERENCE WHOSE EVENT IS MISSING is warned about, not swallowed
///
/// See [`build_subset_detail`]'s header for why that is unreachable and why it
/// is still reported.
fn compose_events(
    sources: &SubsetDetailSources<'_>,
    warnings: &mut Vec<String>,
) -> Vec<SubsetEventDto> {
    let by_id: HashMap<Uuid, &ChronologyEventStateRow> =
        sources.events.iter().map(|e| (e.id, e)).collect();
    let mut links_by_event: HashMap<Uuid, Vec<ChronologyLinkRow>> = HashMap::new();
    for link in sources.links {
        links_by_event
            .entry(link.event_id)
            .or_default()
            .push(link.clone());
    }

    let mut out: Vec<SubsetEventDto> = Vec::with_capacity(sources.refs.len());
    for reference in sources.refs {
        let Some(state) = by_id.get(&reference.event_id) else {
            warnings.push(format!(
                "subset {}: reference to event {} has no row in chronology_events; \
                 the foreign key that makes this impossible is missing",
                sources.subset.id, reference.event_id
            ));
            continue;
        };
        let links = links_by_event
            .get(&state.id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        out.push(subset_event_dto(
            state,
            reference,
            links,
            sources.resolved_documents,
            sources.note_counts.get(&state.id).copied().unwrap_or(0),
            warnings,
        ));
    }
    out
}

/// One reference, composed: the timeline's event shape plus what the subset owns.
///
/// Split out of [`build_subset_detail`] for Rule 18, and it reads better for it —
/// the loop above is now "for each reference, find its event or report it", and
/// this is "here is what one line of the story looks like".
///
/// ## Rust Learning: six arguments is the ceiling, and this is at it
///
/// Clippy refuses a seventh. If this ever needs one more input, the answer is a
/// parameter struct like [`SubsetDetailSources`] — not a longer signature, which
/// is where two `&HashMap`s eventually get transposed at a call site with no
/// compiler complaint.
fn subset_event_dto(
    state: &ChronologyEventStateRow,
    reference: &SubsetEventRefRow,
    links: &[ChronologyLinkRow],
    resolved_documents: &HashSet<String>,
    note_count: i64,
    warnings: &mut Vec<String>,
) -> SubsetEventDto {
    let mut event = event_dto(
        &as_read_row(state),
        link_dtos(links, resolved_documents),
        note_count,
        warnings,
    );
    // Set AFTER composition, for the reason `chronology_write_response` sets it
    // after composition: the read path has no use for the field, and giving it
    // one would mean every read had to remember to clear it.
    event.deleted_at = state.deleted_at;
    SubsetEventDto {
        event,
        subset_note: reference.note.clone(),
        // Derived from the same field the line above carries, so a surface that
        // reads either one is reading the same fact. Two independent sources for
        // "is this event gone" is how one of them goes stale.
        removed: state.deleted_at.is_some(),
    }
}

/// The `GET /api/cases/:slug/scenarios/:id/subsets` payload — the button's data.
///
/// Returns `[]` for a scenario carrying none, which is what hides the button.
/// An empty list and a 404 are deliberately different answers: the first says
/// "this scenario has no stories yet", the second says "there is no such
/// scenario", and a surface that collapsed them would show a working page for a
/// scenario that does not exist.
pub fn build_scenario_subsets(rows: &[ScenarioSubsetRow]) -> Vec<ScenarioSubsetDto> {
    rows.iter()
        .map(|row| ScenarioSubsetDto {
            id: row.subset_id,
            name: row.name.clone(),
            event_count: row.event_count,
            gap_count: row.gap_count,
            position: row.position,
        })
        .collect()
}

#[cfg(test)]
#[path = "chronology_subset_read_tests.rs"]
mod tests;
