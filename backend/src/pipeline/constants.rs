//! Application-level constants for the pipeline.
//!
//! These are the named values referenced throughout colossus-legal's pipeline
//! code. Defining them here eliminates magic strings and numbers from step
//! implementations, API handlers, and migration scripts.
//!
//! Per v5_2 Part 8 spec.

/// Job type identifier for document-processing jobs in `pipeline_jobs`.
pub const JOB_TYPE_DOCUMENT_PROCESSING: &str = "document_processing";

/// Priority value for complaint documents — processed before others.
pub const PRIORITY_COMPLAINT: i32 = 10;

/// Default priority for non-complaint documents.
pub const PRIORITY_DEFAULT: i32 = 0;

/// Qdrant payload field name that holds the document ID for each chunk.
pub const QDRANT_DOCUMENT_ID_FIELD: &str = "document_id";

/// Qdrant collection name for the evidence corpus.
pub const QDRANT_COLLECTION_NAME: &str = "colossus_evidence";

/// Neo4j node property name that holds the source-document identifier.
pub const NEO4J_SOURCE_DOCUMENT_PROP: &str = "source_document";

/// Neo4j node property name that holds the source-document ID (UUID).
pub const NEO4J_SOURCE_DOCUMENT_ID_PROP: &str = "source_document_id";

/// Maximum accepted upload size in bytes (50 MB).
pub const MAX_UPLOAD_SIZE_BYTES: u64 = 50 * 1024 * 1024;

// ── Document statuses ───────────────────────────────────────────
//
// The authoritative definitions live in `crate::models::document_status`.
// These `DOC_STATUS_*` aliases preserve the names existing pipeline
// callers compile against; new code should import the `STATUS_*` names
// directly from `models::document_status`.

pub use crate::models::document_status::STATUS_CANCELLED as DOC_STATUS_CANCELLED;
pub use crate::models::document_status::STATUS_COMPLETED as DOC_STATUS_COMPLETED;
pub use crate::models::document_status::STATUS_FAILED as DOC_STATUS_FAILED;
pub use crate::models::document_status::STATUS_NEW as DOC_STATUS_NEW;
pub use crate::models::document_status::STATUS_PROCESSING as DOC_STATUS_PROCESSING;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_values_have_correct_casing() {
        // These strings are persisted in the documents table and referenced
        // by frontend rendering logic. Changing casing silently would break
        // queries. Lock it in.
        assert_eq!(DOC_STATUS_NEW, "NEW");
        assert_eq!(DOC_STATUS_PROCESSING, "PROCESSING");
        assert_eq!(DOC_STATUS_COMPLETED, "COMPLETED");
        assert_eq!(DOC_STATUS_FAILED, "FAILED");
        assert_eq!(DOC_STATUS_CANCELLED, "CANCELLED");
    }

    #[test]
    fn priority_ordering_puts_complaints_first() {
        // Higher priority is processed first. Complaints must outrank default.
        // Const block enforces the invariant at compile time — stricter than runtime.
        const { assert!(PRIORITY_COMPLAINT > PRIORITY_DEFAULT) };
    }

    #[test]
    fn max_upload_size_is_fifty_megabytes() {
        assert_eq!(MAX_UPLOAD_SIZE_BYTES, 52_428_800);
    }
}
