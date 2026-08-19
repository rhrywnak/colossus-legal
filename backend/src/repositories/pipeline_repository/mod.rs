//! Repository for pipeline tables in the `colossus_legal_v2` database.
//!
//! All functions take a `&PgPool` parameter (the pipeline pool, NOT the
//! main pool). This keeps the repository stateless — the caller decides
//! which pool to pass.
//!
//! ## Module layout
//!
//! This module is split into focused siblings; `mod.rs` itself only
//! declares them, re-exports their public items so callers keep using
//! the `pipeline_repository::*` glob path, and owns the shared
//! [`PipelineRepoError`] type that every sibling raises.
//!
//! - `document_records.rs` — `DocumentRecord` / `DocumentTextRecord`
//!   row types + the canonical CRUD on `documents` and `document_text`.
//! - `documents.rs` — process-endpoint progress writers
//!   (`update_processing_progress`, cancellation flags). Distinct from
//!   `document_records.rs` because the column set it writes is the
//!   Processing-tab UI surface, which evolves on a different cadence
//!   than the canonical CRUD.
//! - `config.rs` — `PipelineConfigInput` / `PipelineConfigRecord` plus
//!   `insert_pipeline_config` and `get_pipeline_config`. The strict-
//!   parsing contract on `PipelineConfigInput` (`deny_unknown_fields`)
//!   guards against silent field drift on any JSON deserialisation.
//! - `config_overrides.rs` — per-document override column read/write
//!   (`get_pipeline_config_overrides`, `patch_pipeline_config_overrides`)
//!   plus the `decode_jsonb_map` no-silent-fail helper.
//! - `extraction.rs` — re-export hub for the five extraction siblings
//!   (`extraction_runs`, `extraction_items`, `extraction_items_pass1`,
//!   `extraction_relationships`, `extraction_context`).
//! - `authored_entities.rs` — CRUD for the Tier-1 `authored_entities` and
//!   Tier-3 `authored_relationships` tables (three-tier architecture,
//!   Option A). Human-authored, not extracted; no FK to pipeline tables.
//! - `scenario_store.rs` — CRUD for the `scenarios` (1.1) and
//!   `scenario_fact_refs` (1.2) tables: a scenario's authored definition and its
//!   per-scenario fact references, no case content. Free fns (not a `Repository`
//!   struct) to avoid colliding with the Neo4j `ScenarioRepository`.
//! - `scenario_responses.rs` — CRUD for the scenario responses model (1.6):
//!   `scenario_responses` / `response_items` / `response_item_fact_refs`. Split
//!   from `scenario_store.rs` to stay under the 300-line limit; same tag-not-copy
//!   discipline (evidence referenced by graph node id, never copied).
//! - `models.rs`, `report_queries.rs`, `review.rs`, `steps.rs`,
//!   `users.rs` — other table-scoped repository modules.

pub mod app_settings;
pub mod authored_entities;
pub mod config;
pub mod config_overrides;
pub mod document_records;
pub mod document_titles;
pub mod documents;
pub mod documents_delete;
pub mod documents_progress;
pub mod documents_state;
pub mod evidence_allegation_links;
pub mod evidence_summary_overrides;
pub mod extraction;
pub mod extraction_context;
pub mod extraction_items;
pub mod extraction_items_pass1;
pub mod extraction_relationships;
pub mod extraction_runs;
pub mod models;
pub mod practice;
pub mod practice_editor;
pub mod practice_flow;
pub mod practice_notes;
pub mod report_queries;
pub mod review;
pub mod review_actions;
pub mod review_edit_history;
pub mod review_grounding;
pub mod review_items;
pub mod scan_run_merges;
pub mod scan_run_projection;
pub mod scan_run_verdicts;
pub mod scan_runs;
pub mod scenario_candidate_ordinals;
pub mod scenario_fact_curation;
pub mod scenario_human_facts;
pub mod scenario_responses;
pub mod scenario_ruling_anchors;
pub mod scenario_status_transitions;
pub mod scenario_store;
pub mod steps;
pub mod users;

pub use app_settings::*;
pub use authored_entities::*;
pub use config::*;
pub use config_overrides::*;
pub use document_records::*;
pub use document_titles::*;
pub use evidence_allegation_links::*;
pub use evidence_summary_overrides::*;
pub use extraction::*;
pub use models::LlmModelRecord;
pub use report_queries::{
    get_extraction_runs_with_processing_config, get_per_pass_entity_breakdown,
    get_per_pass_relationship_breakdown, get_relationship_breakdown_by_type, PerPassRunMetadata,
    RelationshipTypeCount,
};
pub use scan_run_merges::*;
pub use scan_run_projection::*;
pub use scan_run_verdicts::*;
pub use scan_runs::*;
pub use scenario_candidate_ordinals::*;
pub use scenario_fact_curation::*;
pub use scenario_human_facts::*;
pub use scenario_responses::*;
pub use scenario_ruling_anchors::*;
pub use scenario_status_transitions::*;
// NOT a glob, unlike every sibling above — `upsert_fact_ref` and `delete_fact_ref`
// are deliberately withheld from this re-export.
//
// ## Why: the anchor choke point has to be structural, not conventional
//
// Both write a candidate's STATE with no anchor: `upsert_fact_ref` sets it,
// `delete_fact_ref` discards it. Every ruling must go through
// `services::scenario_ruling` (`record_ruling` / `record_removal`), which pairs
// the state write with an anchor ledger row in one transaction. If these were
// re-exported beside those, a future caller reaching for the obvious-looking
// `upsert_fact_ref` would recreate the 2026-07-24 loss — a ruling with no record
// of what was ruled on — with no compile error and no warning. The handler comment
// naming the 1a.4 scan-acceptance seam ("do NOT fold it into this route") is
// exactly the kind of instruction that gets missed.
//
// Withholding it means the wrong path costs a deliberate `scenario_store::`
// import that a reviewer can see, while the right path stays a plain
// `record_ruling` call. Everything else the module exports is re-exported as
// before.
pub use scenario_store::{
    delete_scenario, delete_scenarios_for_case, get_scenario, insert_scenario,
    list_fact_refs_for_scenario, list_scenarios_for_case, set_scenario_accusation, update_scenario,
    ScenarioFactRefRecord, ScenarioRecord,
};

// ── Error type ───────────────────────────────────────────────────

/// Repository error type shared across every sibling module.
///
/// Each variant identifies a distinct failure class so callers can
/// decide whether to retry, surface as 404, or escalate as a data-shape
/// bug. The variants are kept here (rather than in a sibling) because
/// every sibling raises this type — putting it in any one of them would
/// force the others into a forward dependency on that sibling.
#[derive(Debug, thiserror::Error)]
pub enum PipelineRepoError {
    #[error("Database error: {0}")]
    Database(String),
    /// A lookup by stable id returned zero rows. Generic across entity types
    /// — the payload identifies *which* record was missing (e.g. a
    /// `document_id`, an `authored_entities.entity_id`).
    ///
    /// Mapped to `AppError::NotFound` → HTTP 404 by API handlers (see
    /// `api::pipeline::config_handler` for the pattern).
    #[error("Record not found: {0}")]
    NotFound(String),
    /// JSONB column on a `pipeline_config` row decoded from the database
    /// but failed to deserialize into the expected typed shape.
    ///
    /// Reserved for cases where the SQL succeeded (the row exists, the
    /// column is well-formed JSON) but the JSON's *shape* doesn't match
    /// what the application expects — e.g., `chunking_config` is a
    /// JSONB number instead of an object map. The error message names
    /// the offending document_id and column so an auditor can find the
    /// bad row directly.
    ///
    /// Distinct from `Database` so callers can decide whether to retry
    /// (Database errors may be transient; Deserialization errors are
    /// data-shape bugs and a retry won't help) and so audit/alerting
    /// can prioritise this class differently.
    #[error("Deserialization error: {0}")]
    Deserialization(String),
}

impl From<sqlx::Error> for PipelineRepoError {
    fn from(e: sqlx::Error) -> Self {
        PipelineRepoError::Database(e.to_string())
    }
}
