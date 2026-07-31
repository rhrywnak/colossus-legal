// =============================================================================
// backend/src/repositories/decomposition_repository.rs
// =============================================================================
//
// Home of the shared error type used by all three decomposition repositories
// (this file, allegation_detail_repository, rebuttals_repository).
//
// The overview read that gave this file its name — `GET /decomposition` — was
// retired in the 2026-07-27 honesty batch. Its summary reported `proven_count`
// and `all_proven` derived from `status == "PROVEN"`, but the v5.1 migration had
// dropped the allegation `status` property and the query returned a literal
// NULL in its place, so the comparison could never be true: both figures were
// zero/false by construction rather than by data. The page it fed was
// unreachable from any link or nav item, and nothing else consumed the
// endpoint, so the read was removed with it rather than left computing a
// falsehood for no reader.
//
// The type stays here (rather than moving to a new module) because both
// surviving decomposition repositories import it and a rename would be churn
// with no reader benefit.
// =============================================================================

// ─────────────────────────────────────────────────────────────────────────────
// Shared error type — used by all three decomposition repositories.
//
// RUST LESSON: Why a custom error enum?
// Each repository group defines its own error type wrapping neo4rs errors.
// The `From` impls let us use the `?` operator throughout — when neo4rs
// returns an error, Rust auto-converts it via From. This is called the
// "newtype error pattern" and is standard in Rust projects.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum DecompositionRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
    GraphAccess(colossus_graph::GraphAccessError),
}

impl From<neo4rs::Error> for DecompositionRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        DecompositionRepositoryError::Neo4j(value)
    }
}

impl From<neo4rs::DeError> for DecompositionRepositoryError {
    fn from(value: neo4rs::DeError) -> Self {
        DecompositionRepositoryError::Value(value)
    }
}

impl From<colossus_graph::GraphAccessError> for DecompositionRepositoryError {
    fn from(value: colossus_graph::GraphAccessError) -> Self {
        DecompositionRepositoryError::GraphAccess(value)
    }
}
