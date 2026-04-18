//! Pipeline step implementations and teardown helpers.
//!
//! Modules here are called by the colossus-pipeline Worker (step impls) or
//! by the document-deletion flow (teardown helpers). Every entry point in
//! this subtree must be idempotent per `colossus-legal`'s CLAUDE.md rules.

pub mod cleanup;
