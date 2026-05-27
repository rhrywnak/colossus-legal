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

use crate::models::document_status::{
    ENTITY_COMPLAINT_ALLEGATION, ENTITY_DOCUMENT, ENTITY_HARM, ENTITY_LEGAL_COUNT,
    ENTITY_ORGANIZATION, ENTITY_PERSON,
};

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
    // (Neo4j label, constraint name). Constraint names are persisted in the
    // database, so changing one is a migration; pair them with the label
    // explicitly rather than deriving from PascalCase to keep this stable.
    let constraints: &[(&str, &str)] = &[
        (ENTITY_DOCUMENT, "document_id_unique"),
        (ENTITY_PERSON, "person_id_unique"),
        (ENTITY_ORGANIZATION, "organization_id_unique"),
        (
            ENTITY_COMPLAINT_ALLEGATION,
            "complaint_allegation_id_unique",
        ),
        (ENTITY_LEGAL_COUNT, "legal_count_id_unique"),
        (ENTITY_HARM, "harm_id_unique"),
    ];

    for (label, constraint_name) in constraints {
        let cypher = format!(
            "CREATE CONSTRAINT {constraint_name} IF NOT EXISTS \
             FOR (n:{label}) REQUIRE (n.id) IS UNIQUE"
        );
        match graph.run(neo4rs::query(&cypher)).await {
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

    // v5.1 relationship-property indexes (LEGAL_DATA_MODEL_v5_1.md §8).
    //
    // These support traceability queries — "all relationships produced by
    // extraction run X" and "all relationships originating from source
    // document Y" — that back the verification UI work in the unified
    // roadmap (step 5). Without an index, those lookups degrade to a full
    // relationship scan; with the index, Neo4j picks a RelationshipIndexSeek.
    //
    // Same idempotency story as the constraints above (IF NOT EXISTS), and
    // the same logging discipline so a startup failure is visible in the
    // logs rather than silently degrading query performance later.
    let rel_indexes: &[(&str, &str)] = &[
        ("rel_extraction_run", "extraction_run_id"),
        ("rel_source_document", "source_document_id"),
    ];

    for (index_name, property) in rel_indexes {
        let cypher = format!(
            "CREATE INDEX {index_name} IF NOT EXISTS \
             FOR ()-[r]-() ON (r.{property})"
        );
        match graph.run(neo4rs::query(&cypher)).await {
            Ok(_) => tracing::info!(
                index = %index_name,
                property = %property,
                "Neo4j relationship index created or already exists"
            ),
            Err(e) => tracing::error!(
                index = %index_name,
                property = %property,
                error = %e,
                "Failed to create Neo4j relationship index — \
                 traceability queries may degrade to relationship scans"
            ),
        }
    }
}
