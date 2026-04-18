//! backend/src/pipeline/steps/completeness.rs
//!
//! Completeness step — verifies Neo4j + Qdrant counts and finalizes status.
//! Stub declaration landed in P4-2. Full `Step<DocProcessing>` impl lands in P4-7.

use serde::{Deserialize, Serialize};

/// Completeness step state. Carries the document id through the pipeline.
/// Additional fields (tolerance thresholds, verification mode, etc.) will be added in P4-7.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Completeness {
    pub document_id: String,
}
