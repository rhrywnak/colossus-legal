use neo4rs::Graph;

use crate::dto::{PersonDto, PersonsResponse};

#[derive(Clone)]
pub struct PersonRepository {
    graph: Graph,
}

#[derive(Debug)]
pub enum PersonRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
    GraphAccess(colossus_graph::GraphAccessError),
}

impl From<neo4rs::Error> for PersonRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        PersonRepositoryError::Neo4j(value)
    }
}

impl From<neo4rs::DeError> for PersonRepositoryError {
    fn from(value: neo4rs::DeError) -> Self {
        PersonRepositoryError::Value(value)
    }
}

impl From<colossus_graph::GraphAccessError> for PersonRepositoryError {
    fn from(value: colossus_graph::GraphAccessError) -> Self {
        PersonRepositoryError::GraphAccess(value)
    }
}

impl PersonRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch all persons from Neo4j.
    ///
    /// ## Rust Learning: colossus_graph::get_nodes_by_label
    ///
    /// Instead of raw Cypher `MATCH (p:Person) RETURN ...`, we use the
    /// schema-agnostic `get_nodes_by_label` function. It returns `Vec<GraphNode>`
    /// with all properties as serde_json::Value, which we map to PersonDto.
    pub async fn list_persons(&self) -> Result<PersonsResponse, PersonRepositoryError> {
        let nodes = colossus_graph::get_nodes_by_label(&self.graph, "Person").await?;

        let mut persons: Vec<PersonDto> = nodes
            .iter()
            .map(|node| {
                let id = node.id.clone();
                let name = node
                    .properties
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let role = node
                    .properties
                    .get("role")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let description = node
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                PersonDto {
                    id,
                    name,
                    role,
                    description,
                }
            })
            .collect();

        // Sort by name (colossus_graph doesn't guarantee order)
        persons.sort_by(|a, b| a.name.cmp(&b.name));

        let total = persons.len();

        Ok(PersonsResponse { persons, total })
    }
}
