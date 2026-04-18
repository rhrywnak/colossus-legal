//! backend/src/pipeline/steps/extract_text.rs
//!
//! ExtractText step — PDF text extraction with OCR fallback.
//! Stub declaration landed in P4-2. Full `Step<DocProcessing>` impl lands in P4-3.

use serde::{Deserialize, Serialize};

/// ExtractText step state. Carries the document id through the pipeline.
/// Additional fields (OCR config overrides, etc.) will be added in P4-3.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtractText {
    pub document_id: String,
}
