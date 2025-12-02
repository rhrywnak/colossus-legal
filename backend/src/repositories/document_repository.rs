use chrono::Utc;
use neo4rs::{query, DeError, Graph, Node};

use crate::dto::document::{DocumentCreateRequest, DocumentUpdateRequest};
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
    NotFound,
    CreationFailed,
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

    pub async fn get_document_by_id(&self, id: &str) -> Result<Document, DocumentRepositoryError> {
        let mut result = self
            .graph
            .execute(query("MATCH (d:Document {id: $id}) RETURN d").param("id", id))
            .await?;

        if let Some(row) = result.next().await? {
            let node: Node = row.get("d")?;
            let document = Document::try_from(node)?;
            return Ok(document);
        }

        Err(DocumentRepositoryError::NotFound)
    }

    pub async fn create_document(
        &self,
        request: DocumentCreateRequest,
    ) -> Result<Document, DocumentRepositoryError> {
        let id = format!("doc-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let created_at_value = request
            .created_at
            .unwrap_or_else(|| Utc::now().to_rfc3339());

        let mut result = self
            .graph
            .execute(
                query(
                    "CREATE (d:Document {
                        id: $id,
                        title: $title,
                        doc_type: $doc_type,
                        created_at: $created_at,
                        description: $description,
                        file_path: $file_path,
                        uploaded_at: $uploaded_at,
                        related_claim_id: $related_claim_id,
                        source_url: $source_url
                    }) RETURN d",
                )
                .param("id", id.clone())
                .param("title", request.title)
                .param("doc_type", request.doc_type)
                .param("created_at", created_at_value)
                .param("description", request.description)
                .param("file_path", request.file_path)
                .param("uploaded_at", request.uploaded_at)
                .param("related_claim_id", request.related_claim_id)
                .param("source_url", request.source_url),
            )
            .await?;

        if let Some(row) = result.next().await? {
            let node: Node = row.get("d")?;
            let document = Document::try_from(node)?;
            return Ok(document);
        }

        Err(DocumentRepositoryError::CreationFailed)
    }

    pub async fn update_document(
        &self,
        id: &str,
        request: DocumentUpdateRequest,
    ) -> Result<Document, DocumentRepositoryError> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (d:Document {id: $id})
                     SET d.title = COALESCE($title, d.title),
                         d.doc_type = COALESCE($doc_type, d.doc_type),
                         d.created_at = COALESCE($created_at, d.created_at),
                         d.description = COALESCE($description, d.description),
                         d.file_path = COALESCE($file_path, d.file_path),
                         d.uploaded_at = COALESCE($uploaded_at, d.uploaded_at),
                         d.related_claim_id = COALESCE($related_claim_id, d.related_claim_id),
                         d.source_url = COALESCE($source_url, d.source_url)
                     RETURN d",
                )
                .param("id", id)
                .param("title", request.title)
                .param("doc_type", request.doc_type)
                .param("created_at", request.created_at)
                .param("description", request.description)
                .param("file_path", request.file_path)
                .param("uploaded_at", request.uploaded_at)
                .param("related_claim_id", request.related_claim_id)
                .param("source_url", request.source_url),
            )
            .await?;

        if let Some(row) = result.next().await? {
            let node: Node = row.get("d")?;
            let document = Document::try_from(node)?;
            return Ok(document);
        }

        Err(DocumentRepositoryError::NotFound)
    }
}
