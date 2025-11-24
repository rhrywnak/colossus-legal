use std::sync::Arc;

use neo4rs::Graph;

/// Minimal stub for the claim repository.
///
/// The full, Codex-generated implementation lives in the
/// `wip/codex-refactor-2025-11` branch. On `main`, we keep a
/// lightweight type that compiles and can be extended later.
#[derive(Clone)]
pub struct ClaimRepository {
    graph: Arc<Graph>,
}

impl ClaimRepository {
    pub fn new(graph: Arc<Graph>) -> Self {
        Self { graph }
    }
}

