use neo4rs::{query, Graph};
use std::collections::HashMap;

use crate::dto::SchemaResponse;

#[derive(Clone)]
pub struct SchemaRepository {
    graph: Graph,
}

#[derive(Debug)]
pub enum SchemaRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
}

impl From<neo4rs::Error> for SchemaRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        SchemaRepositoryError::Neo4j(value)
    }
}

impl From<neo4rs::DeError> for SchemaRepositoryError {
    fn from(value: neo4rs::DeError) -> Self {
        SchemaRepositoryError::Value(value)
    }
}

impl SchemaRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch schema statistics from Neo4j
    pub async fn get_schema_stats(&self) -> Result<SchemaResponse, SchemaRepositoryError> {
        // Query node counts by label
        let mut node_counts: HashMap<String, i64> = HashMap::new();
        let mut total_nodes: i64 = 0;

        let mut result = self
            .graph
            .execute(query(
                "MATCH (n) RETURN labels(n)[0] AS label, count(*) AS count",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            let label: String = row.get("label")?;
            let count: i64 = row.get("count")?;
            total_nodes += count;
            node_counts.insert(label, count);
        }

        // Query relationship counts by type
        let mut relationship_counts: HashMap<String, i64> = HashMap::new();
        let mut total_relationships: i64 = 0;

        let mut result = self
            .graph
            .execute(query(
                "MATCH ()-[r]->() RETURN type(r) AS rel_type, count(*) AS count",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            let rel_type: String = row.get("rel_type")?;
            let count: i64 = row.get("count")?;
            total_relationships += count;
            relationship_counts.insert(rel_type, count);
        }

        Ok(SchemaResponse {
            total_nodes,
            total_relationships,
            node_counts,
            relationship_counts,
        })
    }
}
