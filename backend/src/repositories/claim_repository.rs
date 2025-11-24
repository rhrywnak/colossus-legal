use std::sync::Arc;

use neo4rs::Graph;


/// Minimal stub for the claim repository.
///
/// The full, Codex-generated implementation is preserved in
/// the `wip/codex-refactor-2025-11` branch. For the current
/// clean skeleton on `main`, we only need a type that compiles
/// and can be constructed from `AppState.graph`.
#[derive(Clone)]
pub struct ClaimRepository {
    graph: Arc<Graph>,
}

impl ClaimRepository {
    pub fn new(graph: Arc<Graph>) -> Self {
        Self { graph }
    }

    /// Placeholder method so the type has *some* behavior.
    /// Real query methods will be reintroduced later in a
    /// dedicated feature branch, once the domain and API
    /// surface are better defined.
    #[allow(dead_code)]
    pub async fn ping(&self) -> bool {
        // In the future, this could do a simple Cypher like:
        // MATCH (n) RETURN n LIMIT 1
        // For now, we just say "it's fine".
        true
    }
}

