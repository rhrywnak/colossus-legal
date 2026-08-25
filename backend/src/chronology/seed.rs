//! The one-shot's PLAN: read `timeline.json`, decide every row, decide nothing else.
//!
//! Pure — no database, no clock, no environment. Everything this module returns
//! is a function of the file it was handed, which is what lets the whole
//! mapping (including the six re-pointed document ids) be unit-tested without a
//! Postgres anywhere near it. `seed_execute` does the writing; `seed_report`
//! does the printing.
//!
//! ## Why the re-point map is hardcoded HERE, of all places
//!
//! Standing Rule 2 says domain-specific values do not live in code. This map is
//! the deliberate exception the task names, and the reason is that it is not
//! configuration: it is a one-time statement of fact about a corpus that existed
//! on 2026-08-25, ruled by Roman as design R12. It runs once, it is quoted into
//! the report so a human can eyeball it before `--apply`, and after the seed it
//! is history. Putting it in YAML would imply it is something an operator tunes.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::Deserialize;

use crate::domain::chronology::is_known_tag;

/// The `target_type` every seeded link carries.
///
/// The chronology supports ten target kinds (see the migration); the legacy JSON
/// only ever pointed at documents, so the seed only ever writes this one.
// CONST: a wire token in the schema this migration writes, not a setting. This
// tool runs ONCE over a frozen file whose links are all documents; making it
// configurable would imply an operator could point the seed at another kind of
// target, which would mean a different corpus and a different tool.
pub const SEED_TARGET_TYPE: &str = "document";

/// The precision every seeded event carries.
///
/// All 22 legacy events state a full date. The three flagged `approximate` keep
/// their full stored date AND their flag — precision says which parts are known,
/// the flag says the whole thing is a best estimate (see `domain::chronology`).
// CONST: a fact about the 2026-08-25 corpus — all 22 legacy events state a full
// date — not a knob. An operator who changed it would be asserting something
// untrue about a file they cannot edit either, since it retires after the seed.
pub const SEED_PRECISION: &str = "day";

/// The value stamped into `attributes.source` on every seeded row, so a later
/// reader can tell a migrated event from one a human typed.
// CONST: the provenance stamp this one-shot leaves behind, and the value later
// readers match on to tell a migrated event from a typed one. It identifies THIS
// tool; a configurable provenance would let two runs disagree about who wrote a row.
pub const SEED_SOURCE: &str = "legacy_json";

/// The bag key holding the source document's own event id (`e001`…`e022`).
///
/// Ruled 2026-08-25 (report v1, R-D). It is the only way to reconcile a stored
/// row against the retiring file: two events can share a date and a title, so
/// nothing else in the row identifies which JSON entry it came from.
// CONST: a key name in the attributes schema, read back by the guard and by any
// future reconciliation. Renaming it is a data migration, not a config change.
pub const SEED_SOURCE_ID_KEY: &str = "source_id";

/// Design R12: the six near-miss ids, plus the one that was already real.
///
/// `(id as the JSON states it, the id that actually exists in `documents`)`.
/// The seventh pair maps an id to itself; it is written out rather than special-
/// cased so the map is the complete answer to "what happens to each link", and
/// so the count of link rows equals the length of this table.
pub const REPOINT_MAP: &[(&str, &str)] = &[
    (
        "doc-awad-complaint",
        "doc-awad-v-catholic-family-complaint-11-1-13",
    ),
    (
        "doc-cfs-interrogatory-response",
        "doc-cfs-interrogatory-response-08-08-16",
    ),
    (
        "doc-coa-ruling-011212",
        "doc-court-of-appeals-rulling-01-12-2012",
    ),
    (
        "doc-tighe-opinion-041212",
        "doc-judge-tighe-opinion-and-order-041212",
    ),
    (
        "doc-phillips-discovery-response",
        "doc-george-phillips-response-to-discovery",
    ),
    (
        "doc-phillips-motion-for-default",
        "doc-awad-v-catholic-family-motion-for-default-and-default-judgment-as-to-phillips",
    ),
    (
        "doc-sabrina-morris-affidavit",
        "doc-sabrina-morris-affidavit",
    ),
];

/// Design R12: the four references with no document in the corpus at all.
///
/// These events are seeded WITHOUT a link row. The absence is the signal — the
/// surface marks them "no document yet", and they are the to-scan to-do list.
/// Writing a link to an id that does not exist would recreate the exact defect
/// this design was written after.
pub const NO_DOCUMENT_YET: &[&str] = &[
    "doc-penzien-coa-brief-300891",
    "doc-phillips-coa-response-300891",
    "doc-coa-reconsideration-042513",
    "doc-phillips-summary-disposition-121213",
];

// ─── The source file, as serde sees it ───────────────────────────────────────

/// One phase block of `timeline.json`. Read by the guard, not by the seed —
/// the phases are seeded by migration, never by this tool.
// serde: allows unknown fields because the source file is FROZEN and retires
// after this seed; a key this struct does not name must not stop the load.
#[derive(Debug, Clone, Deserialize)]
pub struct SourcePhase {
    pub id: String,
    pub label: String,
    pub date_range: String,
    pub color: String,
    pub description: Option<String>,
}

/// One event block of `timeline.json`.
// serde: allows unknown fields because the source file is FROZEN and retires
// after this seed; a key this struct does not name must not stop the load.
#[derive(Debug, Clone, Deserialize)]
pub struct SourceEvent {
    pub id: String,
    pub phase: String,
    pub date: String,
    pub approximate: bool,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub document_id: Option<String>,
    pub document_label: Option<String>,
}

/// One category block — the tag vocabulary and its colours.
// serde: allows unknown fields because every category in the source file carries
// an `icon` this build has never rendered, and refusing it would fail the seed.
#[derive(Debug, Clone, Deserialize)]
pub struct SourceCategory {
    pub label: String,
    pub color: String,
}

/// The whole file.
///
/// ## Rust Learning: no `deny_unknown_fields` here, on purpose
///
/// Elsewhere in this repo strict parsing guards against field drift. This is the
/// opposite case: the file is FROZEN and about to be deleted, and a key this
/// struct does not name (`icon`, on every category) must not stop the seed. Read
/// what is needed; ignore the rest.
// serde: allows unknown fields because the source file is FROZEN and about to be
// deleted — read what is needed, ignore the rest. See the doc comment above.
#[derive(Debug, Clone, Deserialize)]
pub struct SourceTimeline {
    pub phases: Vec<SourcePhase>,
    pub events: Vec<SourceEvent>,
    pub categories: BTreeMap<String, SourceCategory>,
}

// ─── The plan ────────────────────────────────────────────────────────────────

/// One link the seed will write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedLink {
    /// What the JSON said, kept for the report so Roman can see both sides.
    pub original_target_id: String,
    /// What will actually be written.
    pub target_id: String,
    pub label: Option<String>,
}

/// One event the seed will write, with its link if it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedEvent {
    /// The JSON's own id (`e001`). Stamped into `attributes.source_id` as well
    /// as used to identify the row in the report and in test assertions.
    pub source_id: String,
    pub event_date: NaiveDate,
    pub approximate: bool,
    /// The phase slug, bound for the real `chronology_events.phase` column.
    /// It is deliberately NOT also in `attributes` — one fact, one home.
    pub phase: String,
    pub title: String,
    pub fact: Option<String>,
    pub attributes: serde_json::Value,
    pub link: Option<PlannedLink>,
    /// Set when the JSON named a document that has no row anywhere: the event is
    /// written, no link is, and the report says which four.
    pub unlinkable_target: Option<String>,
}

/// Everything the tool intends to do.
#[derive(Debug, Clone)]
pub struct SeedPlan {
    pub events: Vec<PlannedEvent>,
}

impl SeedPlan {
    /// How many link rows this plan writes.
    pub fn link_count(&self) -> usize {
        self.events.iter().filter(|e| e.link.is_some()).count()
    }

    /// The events whose document reference has no document to point at.
    pub fn unlinkable(&self) -> Vec<(&str, &str)> {
        self.events
            .iter()
            .filter_map(|e| {
                e.unlinkable_target
                    .as_deref()
                    .map(|t| (e.source_id.as_str(), t))
            })
            .collect()
    }

    /// Every distinct document id this plan will link to.
    pub fn target_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .events
            .iter()
            .filter_map(|e| e.link.as_ref().map(|l| l.target_id.clone()))
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }
}

/// Why a plan could not be built. Every variant names the offending event.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SeedError {
    #[error("{path}: could not be read ({cause})")]
    Unreadable { path: String, cause: String },

    #[error("{path}: is not the timeline document this tool expects ({cause})")]
    Unparseable { path: String, cause: String },

    #[error("event {source_id}: date '{date}' is not an ISO YYYY-MM-DD date")]
    BadDate { source_id: String, date: String },

    #[error("event {source_id}: '{tag}' is not a tag this build knows")]
    UnknownTag { source_id: String, tag: String },

    #[error("event {source_id}: phase '{phase}' is not one of this case's phases")]
    UnknownPhase { source_id: String, phase: String },

    #[error(
        "event {source_id}: document '{document_id}' is in neither the re-point map \
         nor the known no-document list. Add it to one of them in \
         `chronology::seed` — guessing which is not this tool's call"
    )]
    UnmappedDocument {
        source_id: String,
        document_id: String,
    },
}

/// Parse the file's bytes into the source document.
pub fn parse_source(path: &str, bytes: &str) -> Result<SourceTimeline, SeedError> {
    serde_json::from_str(bytes).map_err(|e| SeedError::Unparseable {
        path: path.to_string(),
        cause: e.to_string(),
    })
}

/// Turn the source document into the plan, or refuse and say which event.
///
/// The refusals are the point. A silently skipped event would be a chronology
/// missing a fact nobody noticed, which is the failure mode this whole track
/// exists to prevent — so an unknown tag, an unparseable date, an unknown phase
/// and an unmapped document each stop the run with the event's own id in the
/// message.
pub fn build_plan(source: &SourceTimeline) -> Result<SeedPlan, SeedError> {
    let phases: Vec<&str> = source.phases.iter().map(|p| p.id.as_str()).collect();
    let mut events = Vec::with_capacity(source.events.len());
    for event in &source.events {
        events.push(plan_one(event, &phases)?);
    }
    Ok(SeedPlan { events })
}

/// Plan a single event. Split out to keep `build_plan` readable and short.
fn plan_one(event: &SourceEvent, phases: &[&str]) -> Result<PlannedEvent, SeedError> {
    let event_date =
        NaiveDate::parse_from_str(&event.date, "%Y-%m-%d").map_err(|_| SeedError::BadDate {
            source_id: event.id.clone(),
            date: event.date.clone(),
        })?;

    if !is_known_tag(&event.category) {
        return Err(SeedError::UnknownTag {
            source_id: event.id.clone(),
            tag: event.category.clone(),
        });
    }
    if !phases.contains(&event.phase.as_str()) {
        return Err(SeedError::UnknownPhase {
            source_id: event.id.clone(),
            phase: event.phase.clone(),
        });
    }

    let (link, unlinkable_target) = plan_link(event)?;

    Ok(PlannedEvent {
        source_id: event.id.clone(),
        event_date,
        approximate: event.approximate,
        // The phase goes to the COLUMN, and only to the column. A mirrored
        // `attributes.phase` would be a second home for one fact, and the second
        // home is the one that goes stale.
        phase: event.phase.clone(),
        title: event.title.clone(),
        fact: event.description.clone(),
        attributes: serde_json::json!({
            "tags": [event.category],
            "source": SEED_SOURCE,
            SEED_SOURCE_ID_KEY: event.id,
        }),
        link,
        unlinkable_target,
    })
}

/// Decide one event's link: re-pointed, deliberately absent, or a refusal.
fn plan_link(event: &SourceEvent) -> Result<(Option<PlannedLink>, Option<String>), SeedError> {
    let Some(document_id) = event.document_id.as_deref() else {
        return Ok((None, None));
    };

    if let Some((_, real)) = REPOINT_MAP.iter().find(|(from, _)| *from == document_id) {
        return Ok((
            Some(PlannedLink {
                original_target_id: document_id.to_string(),
                target_id: (*real).to_string(),
                // The original label is kept verbatim (task A2): it is what a
                // human wrote to describe the link, and the re-point changed
                // which file it opens, not what it is.
                label: event.document_label.clone(),
            }),
            None,
        ));
    }

    if NO_DOCUMENT_YET.contains(&document_id) {
        return Ok((None, Some(document_id.to_string())));
    }

    Err(SeedError::UnmappedDocument {
        source_id: event.id.clone(),
        document_id: document_id.to_string(),
    })
}

#[cfg(test)]
#[path = "seed_tests.rs"]
mod tests;
