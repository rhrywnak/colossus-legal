// =============================================================================
// backend/src/dto/scenario_crud.rs
// =============================================================================
//
// Wire DTOs for the scenario CRUD HTTP surface (authored-state store, task 1.1).
//
// Kept DELIBERATELY separate from the Neo4j `dto/scenario.rs` (which carries the
// graph-read fact/page shapes). These are the Postgres `scenarios` authored-state
// shapes — a saved *lens* over the case, not case content. Same domain word,
// different layer; two files so the two never blur.
//
// These are pure wire types — no dependency on the repository record type. The
// `ScenarioRecord` → `ScenarioDto` mapping lives in the handler module
// (`api/scenarios.rs`), the same place `claims.rs` keeps its `to_dto`, so the
// dto layer stays a leaf.
// =============================================================================

use serde::{Deserialize, Serialize};

/// One scenario as the wire sees it.
///
/// Mirrors `ScenarioRecord` (the Postgres row) with two wire adaptations:
/// `scenario_id` is the row's `Uuid` rendered as a string, and
/// `anchor_allegation_ids` is flattened from the row's `Option<Vec<String>>` to a
/// plain `Vec<String>` (`None` → `[]`) so the client never has to distinguish
/// "null" from "empty" for a list it only ever iterates. `created_at` /
/// `updated_at` are intentionally omitted for this chunk — the form does not need
/// them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDto {
    /// The database-minted `Uuid`, rendered as a string for the wire.
    pub scenario_id: String,
    pub name: String,
    pub direction: String,
    pub status: String,
    pub case_slug: String,
    pub feeds_count_id: Option<String>,
    /// Flattened from `Option<Vec<String>>` — `None`/absent both become `[]`.
    pub anchor_allegation_ids: Vec<String>,
    /// The authored definition body, stored and returned as opaque JSON (its
    /// shape is validated at render time, not here — see `scenario_store`).
    pub definition: serde_json::Value,
}

/// The create-scenario request body.
///
/// `name` and `direction` are required; everything else is optional with a
/// server-applied default (`status` → `"draft"`, `definition` → `{}`). Note that
/// `case_slug` is NOT here on purpose — the handler sources it from the URL path,
/// so a request can never write a scenario into a case other than the one its URL
/// names.
///
/// ## Rust Learning: `Option<T>` fields are optional without `#[serde(default)]`
///
/// serde already treats a missing key as `None` for an `Option<T>` field, so
/// these need no `#[serde(default)]`. `deny_unknown_fields` still rejects keys
/// the struct does NOT declare — a typo'd field fails loudly rather than being
/// silently ignored (Standing Rule 1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioCreateRequest {
    pub name: String,
    pub direction: String,
    /// Absent → the handler defaults to `"draft"`.
    pub status: Option<String>,
    pub feeds_count_id: Option<String>,
    pub anchor_allegation_ids: Option<Vec<String>>,
    /// Absent → the handler defaults to an empty JSON object `{}` (the column is
    /// NOT NULL; SQL null is never written).
    pub definition: Option<serde_json::Value>,
}
