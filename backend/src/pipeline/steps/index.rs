//! backend/src/pipeline/steps/index.rs
//!
//! Index step — embeds grounded entities and upserts to Qdrant.
//! Stub declaration landed in P4-2. Full `Step<DocProcessing>` impl lands in P4-6.

use serde::{Deserialize, Serialize};

/// Index step state. Carries the document id through the pipeline.
/// Additional fields (batch size, embedding provider overrides, etc.) will be added in P4-6.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Index {
    pub document_id: String,
}
