//! Document-state repository surface (re-export hub).
//!
//! The previous `documents.rs` mixed three concerns (process-endpoint
//! progress writes, read-only state queries, and transactional
//! deletion paths) and had grown past the 300-line module budget. The
//! file is now a `pub use` re-export manifest pointing at three
//! focused siblings:
//!
//! - [`documents_progress`](super::documents_progress) — process-endpoint
//!   progress / failure writes (`update_processing_progress`,
//!   `update_document_failure`).
//! - [`documents_state`](super::documents_state) — small read-only
//!   queries (`is_cancelled`, `count_documents`,
//!   `has_document_of_type`).
//! - [`documents_delete`](super::documents_delete) — the transactional
//!   full-deletion path (`delete_all_document_data`).
//!
//! Pre-existing call sites continue to reach these functions through
//! `super::documents::*` (e.g. `pipeline_repository::documents::is_cancelled`),
//! so no caller required an edit.
//!
//! ## Adding new functions on the `documents` table
//!
//! Choose the sibling that matches the responsibility — write/progress,
//! read/state, or delete/transactional — and add a `pub use` line below.
//! Do NOT add function definitions to this file.

pub use super::documents_delete::delete_all_document_data;
pub use super::documents_progress::{
    mark_document_cancelled, set_restate_invocation_id, update_document_failure,
    update_processing_progress,
};
pub use super::documents_state::{count_documents, has_document_of_type, is_cancelled};
