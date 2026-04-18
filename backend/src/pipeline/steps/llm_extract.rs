//! backend/src/pipeline/steps/llm_extract.rs
//!
//! LlmExtract step — chunk-level LLM entity extraction.
//! Stub declaration landed in P4-2. Full `Step<DocProcessing>` impl lands in P4-4.

use serde::{Deserialize, Serialize};

/// LlmExtract step state. Carries the document id through the pipeline.
/// Additional fields (chunk strategy, model overrides, etc.) will be added in P4-4.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmExtract {
    pub document_id: String,
}
