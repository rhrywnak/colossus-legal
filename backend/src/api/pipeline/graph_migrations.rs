//! Neo4j schema migrations — uniqueness constraints for entity nodes.
//!
//! ## Why uniqueness constraints are required
//!
//! MERGE semantics only provide safe idempotency when a uniqueness constraint
//! exists on the MERGE key. Without a constraint, two concurrent transactions
//! can both check for an existing node, both find none, and both CREATE —
//! producing duplicates even with MERGE syntax.
//!
//! This is documented in the Neo4j Knowledge Base ("Understanding how MERGE
//! works") and is why neo4j-graphrag-python creates an index automatically
//! before any writes (`__entity__tmp_internal_id`).
//!
//! ## Why at startup, not in a migration file
//!
//! Neo4j constraints are not managed by sqlx migrations (those are for
//! PostgreSQL). Running them at startup with IF NOT EXISTS is idempotent
//! and ensures the constraints exist before the first ingest attempt.
//! This pattern is used by production systems that manage Neo4j schema
//! as part of application initialization.

use neo4rs::Graph;

/// Run all Neo4j schema constraints at application startup.
///
/// Safe to call repeatedly — all constraints use IF NOT EXISTS.
/// Logs success or failure for each constraint.
///
/// ## Constraint scope
///
/// We create constraints for every entity type produced by the extraction
/// pipeline. The `id` property is the MERGE key for all entity nodes.
/// Without these constraints, concurrent ingest operations could produce
/// duplicate nodes.
pub async fn run_graph_migrations(graph: &Graph) {
    let constraints = [
        (
            "Document",
            "CREATE CONSTRAINT document_id_unique IF NOT EXISTS \
             FOR (n:Document) REQUIRE (n.id) IS UNIQUE",
        ),
        (
            "Person",
            "CREATE CONSTRAINT person_id_unique IF NOT EXISTS \
             FOR (n:Person) REQUIRE (n.id) IS UNIQUE",
        ),
        (
            "Organization",
            "CREATE CONSTRAINT organization_id_unique IF NOT EXISTS \
             FOR (n:Organization) REQUIRE (n.id) IS UNIQUE",
        ),
        (
            "ComplaintAllegation",
            "CREATE CONSTRAINT complaint_allegation_id_unique IF NOT EXISTS \
             FOR (n:ComplaintAllegation) REQUIRE (n.id) IS UNIQUE",
        ),
        (
            "LegalCount",
            "CREATE CONSTRAINT legal_count_id_unique IF NOT EXISTS \
             FOR (n:LegalCount) REQUIRE (n.id) IS UNIQUE",
        ),
        (
            "Harm",
            "CREATE CONSTRAINT harm_id_unique IF NOT EXISTS \
             FOR (n:Harm) REQUIRE (n.id) IS UNIQUE",
        ),
    ];

    for (label, cypher) in &constraints {
        match graph.run(neo4rs::query(cypher)).await {
            Ok(_) => tracing::info!(
                label = %label,
                "Neo4j constraint created or already exists"
            ),
            Err(e) => tracing::error!(
                label = %label,
                error = %e,
                "Failed to create Neo4j uniqueness constraint — \
                 MERGE operations may not be safe"
            ),
        }
    }
}
