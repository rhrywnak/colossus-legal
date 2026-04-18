//! backend/src/pipeline/steps/ingest.rs
//!
//! Ingest step — Neo4j node and relationship creation via MERGE.
//! Stub declaration landed in P4-2. Full `Step<DocProcessing>` impl lands in P4-5.

use serde::{Deserialize, Serialize};

/// Ingest step state. Carries the document id through the pipeline.
/// Additional fields (MERGE tuning, batch size, etc.) will be added in P4-5.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ingest {
    pub document_id: String,
}
