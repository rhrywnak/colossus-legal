//! Scenario fact-curation wire DTOs (task 1.x — Phase A).
//!
//! These are the request/response shapes for the three curation routes
//! (`POST` / `DELETE` / `GET` under `/cases/:slug/scenarios/:id/facts`). They
//! sit on top of the existing storage (`scenario_fact_refs`, task 1.2) and the
//! existing graph card content (`bias::dto::BiasInstance`) — Phase A is wiring,
//! not new storage, so there is no new record type here.
//!
//! ## Why reuse `BiasInstance` for the content
//!
//! A `scenario_fact_refs` row stores only a `graph_node_id`; the human-readable
//! content (quote, speaker, ABOUT subjects, document, pattern tags) is read live
//! from the graph. The Bias Explorer already assembles exactly that content into
//! `BiasInstance`. Carrying the saved fact's content *as* a `BiasInstance` means
//! one frontend card renders both a bias candidate and a saved fact, and the
//! backend has one graph→content mapping, not two that can drift (Standing
//! Rule: no duplication / no tech debt).

use serde::{Deserialize, Serialize};

use crate::bias::dto::BiasInstance;
use crate::domain::fact_status::FactStatus;

/// Request body for `POST /cases/:slug/scenarios/:scenario_id/facts`.
///
/// `graph_node_id` is the Neo4j node id of the Evidence being curated in
/// (a bias instance's `evidence_id` is exactly this value). `role` and `note`
/// are accepted by the storage layer but are **not surfaced in Phase A**: the
/// `scenario_fact_refs.role_in_this_scenario` / `note` columns exist and round-
/// trip, but no UI authors them yet (role + rebuttal-pairing arrive in a later
/// phase). They are optional here so the column stays ready without forcing the
/// client to send a value.
/// `deny_unknown_fields` so a client typo (e.g. `graphNodeId` instead of
/// `graph_node_id`) is rejected as a 400 rather than silently accepted and the
/// field dropped — matching `ScenarioCreateRequest`'s precedent (Standing
/// Rule 1: a malformed request must be observable, not quietly half-applied).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddFactRequest {
    pub graph_node_id: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

/// One saved fact on a scenario, returned by
/// `GET /cases/:slug/scenarios/:scenario_id/facts`.
///
/// `graph_node_id`, `role`, and `note` come from the `scenario_fact_refs` row
/// (the persisted reference). `content` is the live graph card content for that
/// node id.
///
/// ## Why `status` is intentionally NOT surfaced here (yet)
///
/// The `scenario_fact_refs.status` column (Phase 1a.1 — `undecided` / `included`
/// / `dropped`) is deliberately absent from this DTO. 1a.1 is the data-layer
/// replacement only; surfacing the three-state status on the read path is
/// 1a.2/1a.3 work, gated on the workbench UI that actually consumes it. Exposing
/// it now would ship a field nothing reads (zero frontend consumers today) — the
/// premature exposure the scope fence exists to prevent. When the workbench lands
/// it is added here (as a raw token or a decoded `FactStatus`, decided then).
///
/// ## Why `content` is `Option` (and is NOT skipped when `None`)
///
/// A reference can outlive the graph node it points at — if the Evidence node
/// is later deleted or re-ingested under a new id, the saved `graph_node_id`
/// resolves to nothing. Phase A must never let that fact *silently vanish* from
/// the curated set (Standing Rule 1): the count of saved facts is meaningful, so
/// every reference yields exactly one `ScenarioFactDto`. A missing node is
/// represented by `content: None` — serialized explicitly as `null` (we do NOT
/// `skip_serializing_if`) so the frontend can render a "content unavailable"
/// card carrying the `graph_node_id`, distinct from a fact whose content loaded.
#[derive(Debug, Clone, Serialize)]
pub struct ScenarioFactDto {
    pub graph_node_id: String,
    pub role: Option<String>,
    pub note: Option<String>,
    /// Live graph card content; `None` (serialized as `null`) when the node id
    /// resolves to no live Evidence node — a stale reference, surfaced rather
    /// than dropped.
    pub content: Option<BiasInstance>,
}

/// One candidate in the scenario workbench pool (1a.2 gather endpoint).
///
/// A candidate is a live Evidence node ABOUT the scenario's subject, tagged with
/// its derived workbench `status` for THIS scenario (`undecided` when no human
/// has ruled on it, `included`/`dropped` when one has).
///
/// ## Rust Learning: the `Option` is the invariant, made visible in the type
///
/// Contrast `content: BiasInstance` here (NON-optional) with
/// [`ScenarioFactDto::content`] above (`Option<BiasInstance>`). That older DTO
/// is driven by SAVED references, and a saved reference can outlive the graph
/// node it points at — so its content may be absent (`null`), and the type says
/// so. This DTO is driven by the LIVE graph pool itself: every candidate exists
/// because the graph just returned it, so its content is present BY
/// CONSTRUCTION. There is no "ref outlived its node" case to represent, so there
/// is no `Option`. The question "can this be absent?" is answered in the type,
/// not deferred to a runtime `null` check — the two DTOs' shapes encode their
/// two different provenances.
#[derive(Debug, Clone, Serialize)]
pub struct CandidateDto {
    /// The live graph card content — present by construction (the pool drives
    /// output; every entry is a node the graph just returned).
    pub content: BiasInstance,
    /// This candidate's derived workbench state for this scenario.
    pub status: FactStatus,
    /// The role recorded on the fact-ref, if one exists for this node. `None`
    /// for an undecided candidate (no ref row) or a ref that recorded no role.
    pub role: Option<String>,
    /// The note recorded on the fact-ref, if any.
    pub note: Option<String>,
}

/// Response body for `GET /cases/:slug/scenarios/:scenario_id/facts/gather`.
///
/// Two lists, deliberately separate (not one list the client must partition):
/// `pool` holds the working candidates (undecided + included), `dropped` holds
/// the scenario-scoped exclusions on their own so a later "un-drop" tray has its
/// data ready without re-deriving anything.
#[derive(Debug, Clone, Serialize)]
pub struct GatherCandidatesResponse {
    /// Undecided + included candidates — the working pool.
    pub pool: Vec<CandidateDto>,
    /// Dropped candidates, kept in their own list (not omitted, not mixed in).
    pub dropped: Vec<CandidateDto>,
}
