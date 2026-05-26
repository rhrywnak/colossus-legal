//! Extraction-repository public surface (re-export hub).
//!
//! Every extraction-related public function and type that callers use
//! through `pipeline_repository::*` originates in one of the four focused
//! sibling modules below. This file exists to preserve the historical
//! single-import contract: external code continues to write
//! `pipeline_repository::insert_extraction_run` rather than
//! `pipeline_repository::extraction_runs::insert_extraction_run`, and the
//! existing `pub use extraction::*;` line in `mod.rs` keeps that glob
//! working without per-call-site edits.
//!
//! ## Why the split
//!
//! The pre-split `extraction.rs` had grown to >1,000 production lines —
//! well over the project's 300-line module budget — and mixed four
//! distinct concerns whose evolution paths rarely overlap:
//!
//! - [`extraction_runs`] — `extraction_runs` row lifecycle plus the
//!   aggregate per-run chunk statistics and the graph-status writeback
//!   called by Ingest.
//! - [`extraction_items`] — `extraction_items` CRUD and the grounding-
//!   based selection queries Auto-Ingest reads.
//! - [`extraction_items_pass1`] — pass-1 entity loading + the prompt-
//!   shaped `Pass1Entity` projection used to feed pass 2. Logically a
//!   sub-concern of items, split out to keep `extraction_items` under
//!   the 300-line ceiling.
//! - [`extraction_relationships`] — `extraction_relationships` CRUD plus
//!   the LLM-result writers (`store_entities_and_relationships`,
//!   `store_pass2_relationships`) that translate parsed JSON into rows.
//! - [`extraction_context`] — cross-document context loader for pass 2,
//!   carrying the entity-type whitelist + per-type property allowlist
//!   that bound the pass-2 prompt size.
//!
//! ## Adding new extraction-repo functions
//!
//! Place new code in the sibling whose responsibility matches the
//! function, then add a corresponding `pub use` line below so the
//! `pipeline_repository::*` glob continues to surface it. Do NOT add
//! function definitions to this file — its only job is the re-export
//! manifest.

pub use super::extraction_context::{
    load_authored_entities_for_context, load_cross_document_context, CrossDocEntity,
    CROSS_DOC_ENTITY_TYPES, CROSS_DOC_ID_PREFIX,
};
pub use super::extraction_items::{
    batch_update_neo4j_node_ids, count_items_pending_graph_write, get_all_items,
    get_approved_items_for_document, get_existing_item_neo4j_map, get_grounded_items_for_document,
    get_items_for_run, get_items_pending_graph_write, get_items_with_quotes,
    insert_extraction_item, lookup_item_document_ids, lookup_neo4j_node_ids,
    update_item_entity_type, update_item_grounding, ExtractionItemRecord,
};
pub use super::extraction_items_pass1::{load_pass1_entities, Pass1Entity};
pub use super::extraction_relationships::{
    get_all_relationships, get_approved_relationships_for_document,
    get_approved_relationships_for_document_all_passes, get_grounded_relationships_for_document,
    get_relationships_for_run, insert_extraction_relationship, store_entities_and_relationships,
    store_pass2_relationships, ExtractionRelationshipRecord,
};
pub use super::extraction_runs::{
    complete_extraction_run, get_extraction_runs, get_latest_completed_run, insert_extraction_run,
    reset_extraction_run_children, update_graph_status_for_run, update_run_chunk_stats,
    ExtractionRunRecord,
};
