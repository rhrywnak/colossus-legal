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
