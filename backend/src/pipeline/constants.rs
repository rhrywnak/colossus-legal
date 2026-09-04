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

// ─── The rest of the Qdrant payload's keys ──────────────────────────────────
//
// STRUCTURAL: Qdrant payload vocabulary — the wire names a stored point carries,
// read back by `services::qdrant_service::search_points`. They are the shape of
// the data, not a deployment setting: changing one would orphan every point
// already written under the old name. They live here beside
// `QDRANT_DOCUMENT_ID_FIELD` so `services::qdrant_payload` can build a payload
// without a single string literal, and so a reader can see the whole key set in
// one place rather than inferring it from three call sites.

/// Payload key holding the Neo4j node id the point was built from.
pub const QDRANT_NODE_ID_FIELD: &str = "node_id";

/// Payload key holding the node's label (`Evidence`, `Document`, …).
pub const QDRANT_NODE_TYPE_FIELD: &str = "node_type";

/// Payload key holding the node's display title.
pub const QDRANT_TITLE_FIELD: &str = "title";

/// Payload key holding the source document id.
///
/// Deliberately distinct from [`QDRANT_DOCUMENT_ID_FIELD`] even though both are
/// written with the same value today: they are two keys in the stored payload
/// and `search_points` reads them separately.
pub const QDRANT_SOURCE_DOCUMENT_FIELD: &str = "source_document";

/// Payload key holding the page a quote appears on. **Omitted entirely** when
/// the node has no page — see `services::qdrant_payload`.
pub const QDRANT_PAGE_NUMBER_FIELD: &str = "page_number";

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
