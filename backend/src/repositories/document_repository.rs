use neo4rs::{query, DeError, Graph, Node};

use crate::models::document::{Document, DocumentConversionError};

#[derive(Clone)]
pub struct DocumentRepository {
    graph: Graph,
}

#[derive(Debug)]
pub enum DocumentRepositoryError {
    Neo4j(neo4rs::Error),
    Mapping(DocumentConversionError),
    Value(DeError),
}

impl From<neo4rs::Error> for DocumentRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        DocumentRepositoryError::Neo4j(value)
    }
}

impl From<DocumentConversionError> for DocumentRepositoryError {
    fn from(value: DocumentConversionError) -> Self {
        DocumentRepositoryError::Mapping(value)
    }
}

impl From<DeError> for DocumentRepositoryError {
    fn from(value: DeError) -> Self {
        DocumentRepositoryError::Value(value)
    }
}

impl DocumentRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    pub async fn list_documents(&self) -> Result<Vec<Document>, DocumentRepositoryError> {
        let mut result = self
            .graph
            .execute(query(
                "MATCH (d:Document) RETURN d ORDER BY d.created_at DESC",
            ))
            .await?;

        let mut documents = Vec::new();
        while let Some(row) = result.next().await? {
            let node: Node = row.get("d")?;
            let document = Document::try_from(node)?;
            documents.push(document);
        }

        Ok(documents)
    }
}
