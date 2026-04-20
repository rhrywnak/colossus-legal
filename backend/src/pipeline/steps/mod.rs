//! Pipeline step implementations and teardown helpers.
//!
//! Modules here are called by the colossus-pipeline Worker (step impls) or
//! by the document-deletion flow (teardown helpers). Every entry point in
//! this subtree must be idempotent per `colossus-legal`'s CLAUDE.md rules.

pub mod auto_approve;
pub mod cleanup;
pub mod completeness;
pub mod extract_text;
pub mod index;
pub mod ingest;
pub mod llm_extract;
pub mod llm_extract_helpers;
pub mod verify;
