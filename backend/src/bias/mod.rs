//! Bias Explorer — structured-query interface over the knowledge graph.
//!
//! ## Why this module exists
//!
//! The Chat / Ask pipeline answers questions through vector search → graph
//! expansion → LLM synthesis. That works for "what's the strongest evidence
//! against Phillips?" but it cannot enumerate. Trial preparation needs
//! "show me EVERY instance of disparagement, with quote and page number."
//! That is a deterministic graph traversal, not a question.
//!
//! The Bias Explorer is the first user-facing structured-query feature.
//! It runs parameterised Cypher against `Evidence` nodes filtered by
//! `pattern_tags`, with optional STATED_BY actor filter, and returns
//! every match — no sampling, no LLM, no embeddings.
//!
//! ## Module layout
//!
//! - `dto` — request and response DTOs (no business logic).
//! - `repository` — Cypher queries and result mapping; the only place that
//!   talks to Neo4j for bias-related reads.
//! - `handlers` — Axum HTTP handlers; the only place that talks to clients.
//! - `tests` — pure unit tests for serialization and helpers (no live Neo4j).
//!
//! ## Why a new top-level module instead of extending `graph/`
//!
//! Per the standing rule against premature generalization, the bias-specific
//! Cypher and DTOs live here. When (or if) a second structured-query feature
//! arrives — Harm Explorer, Contradictions Explorer — we can lift the
//! shared bits into a generic helper. Building that helper today would
//! design against a use case we haven't seen.

pub mod dto;
pub mod handlers;
pub mod repository;

#[cfg(test)]
mod tests;
