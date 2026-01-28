use neo4rs::{query, Graph};

use crate::dto::{PersonDto, PersonsResponse};

#[derive(Clone)]
pub struct PersonRepository {
    graph: Graph,
}

#[derive(Debug)]
pub enum PersonRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
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

impl PersonRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch all persons from Neo4j
    pub async fn list_persons(&self) -> Result<PersonsResponse, PersonRepositoryError> {
        let mut persons: Vec<PersonDto> = Vec::new();

        let mut result = self
            .graph
            .execute(query(
                "MATCH (p:Person)
                 RETURN p.id AS id, p.name AS name, p.role AS role, p.description AS description
                 ORDER BY p.name",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            let id: String = row.get("id").unwrap_or_default();
            let name: String = row.get("name").unwrap_or_default();
            let role: Option<String> = row.get("role").ok();
            let description: Option<String> = row.get("description").ok();

            persons.push(PersonDto {
                id,
                name,
                role,
                description,
            });
        }

        let total = persons.len();

        Ok(PersonsResponse { persons, total })
    }
}
