//! Neo4j connection and schema management.
//!
//! This module handles:
//! - Creating the Neo4j Graph connection
//! - Verifying connectivity
//! - Ensuring schema constraints and indexes exist

use crate::config::AppConfig;
use neo4rs::{Graph, Query};
use tracing::info;

// =============================================================================
// SCHEMA DEFINITIONS
// =============================================================================

/// Schema statements for v2 models.
/// Uses IF NOT EXISTS for idempotent application.
const SCHEMA_STATEMENTS: &[&str] = &[
    // -------------------------------------------------------------------------
    // CONSTRAINTS — Ensure unique IDs for all node types
    // -------------------------------------------------------------------------
    "CREATE CONSTRAINT claim_id_unique IF NOT EXISTS FOR (c:Claim) REQUIRE c.id IS UNIQUE",
    "CREATE CONSTRAINT document_id_unique IF NOT EXISTS FOR (d:Document) REQUIRE d.id IS UNIQUE",
    "CREATE CONSTRAINT person_id_unique IF NOT EXISTS FOR (p:Person) REQUIRE p.id IS UNIQUE",
    "CREATE CONSTRAINT evidence_id_unique IF NOT EXISTS FOR (e:Evidence) REQUIRE e.id IS UNIQUE",
    "CREATE CONSTRAINT case_id_unique IF NOT EXISTS FOR (c:Case) REQUIRE c.id IS UNIQUE",
    // -------------------------------------------------------------------------
    // INDEXES — Claim
    // -------------------------------------------------------------------------
    "CREATE INDEX claim_category_idx IF NOT EXISTS FOR (c:Claim) ON (c.category)",
    "CREATE INDEX claim_status_idx IF NOT EXISTS FOR (c:Claim) ON (c.status)",
    "CREATE INDEX claim_source_doc_idx IF NOT EXISTS FOR (c:Claim) ON (c.source_document_id)",
    // -------------------------------------------------------------------------
    // INDEXES — Document
    // -------------------------------------------------------------------------
    "CREATE INDEX document_type_idx IF NOT EXISTS FOR (d:Document) ON (d.doc_type)",
    "CREATE INDEX document_court_idx IF NOT EXISTS FOR (d:Document) ON (d.court)",
    "CREATE INDEX document_ingested_at_idx IF NOT EXISTS FOR (d:Document) ON (d.ingested_at)",
    // -------------------------------------------------------------------------
    // INDEXES — Person
    // -------------------------------------------------------------------------
    "CREATE INDEX person_role_idx IF NOT EXISTS FOR (p:Person) ON (p.role)",
    "CREATE INDEX person_name_idx IF NOT EXISTS FOR (p:Person) ON (p.name)",
    // -------------------------------------------------------------------------
    // INDEXES — Evidence
    // -------------------------------------------------------------------------
    "CREATE INDEX evidence_kind_idx IF NOT EXISTS FOR (e:Evidence) ON (e.kind)",
    "CREATE INDEX evidence_exhibit_idx IF NOT EXISTS FOR (e:Evidence) ON (e.exhibit_number)",
];

// =============================================================================
// CONNECTION
// =============================================================================

/// Create the Graph connection from the environment config.
pub async fn create_neo4j_graph(config: &AppConfig) -> Result<Graph, neo4rs::Error> {
    let graph = Graph::new(
        config.neo4j_uri.clone(),
        config.neo4j_user.clone(),
        config.neo4j_password.clone(),
    )
    .await?;

    info!(
        "Connected to Neo4j at {} as {}",
        config.neo4j_uri, config.neo4j_user
    );

    Ok(graph)
}

/// Ping Neo4j by running a trivial query.
pub async fn check_neo4j(graph: &Graph) -> Result<(), neo4rs::Error> {
    let mut result = graph.execute(Query::new("RETURN 1".into())).await?;
    let _ = result.next().await; // ensure Neo4j responded
    Ok(())
}

// =============================================================================
// SCHEMA MANAGEMENT
// =============================================================================

/// Ensure all constraints and indexes exist in Neo4j.
///
/// This function is idempotent — safe to call on every startup.
/// Uses `IF NOT EXISTS` so existing constraints/indexes are not modified.
///
/// # Errors
/// Returns an error if any schema statement fails to execute.
pub async fn ensure_schema(graph: &Graph) -> Result<(), neo4rs::Error> {
    for statement in SCHEMA_STATEMENTS {
        graph.run(Query::new((*statement).to_string())).await?;
    }

    info!(
        "Neo4j schema verified: {} constraints, {} indexes",
        5, // 5 constraints
        SCHEMA_STATEMENTS.len() - 5 // remaining are indexes
    );

    Ok(())
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_statements_count() {
        // 5 constraints + 10 indexes = 15 total
        assert_eq!(SCHEMA_STATEMENTS.len(), 15);
    }

    #[test]
    fn test_schema_statements_are_idempotent() {
        // All statements should contain IF NOT EXISTS
        for statement in SCHEMA_STATEMENTS {
            assert!(
                statement.contains("IF NOT EXISTS"),
                "Statement missing IF NOT EXISTS: {}",
                statement
            );
        }
    }

    #[test]
    fn test_schema_has_all_constraints() {
        let constraints: Vec<&str> = SCHEMA_STATEMENTS
            .iter()
            .filter(|s| s.starts_with("CREATE CONSTRAINT"))
            .copied()
            .collect();

        assert_eq!(constraints.len(), 5);

        // Verify all node types have constraints
        let constraint_text = constraints.join(" ");
        assert!(constraint_text.contains("Claim"));
        assert!(constraint_text.contains("Document"));
        assert!(constraint_text.contains("Person"));
        assert!(constraint_text.contains("Evidence"));
        assert!(constraint_text.contains("Case"));
    }

    #[test]
    fn test_schema_has_all_indexes() {
        let indexes: Vec<&str> = SCHEMA_STATEMENTS
            .iter()
            .filter(|s| s.starts_with("CREATE INDEX"))
            .copied()
            .collect();

        assert_eq!(indexes.len(), 10);

        // Verify key indexes exist
        let index_text = indexes.join(" ");
        assert!(index_text.contains("claim_category"));
        assert!(index_text.contains("claim_status"));
        assert!(index_text.contains("document_type"));
        assert!(index_text.contains("person_role"));
        assert!(index_text.contains("evidence_kind"));
    }
}
