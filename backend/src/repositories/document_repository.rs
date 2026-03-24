use chrono::Utc;
use neo4rs::{query, DeError, Graph, Node};

use crate::dto::document::{DocumentCreateRequest, DocumentUpdateRequest};
use crate::models::document::{Document, DocumentConversionError};
use crate::models::document_status::STATUS_UPLOADED;

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
                        source_url: $source_url,
                        status: $status
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
                .param("source_url", request.source_url)
                .param("status", STATUS_UPLOADED),
            )
            .await?;

        if let Some(row) = result.next().await? {
            let node: Node = row.get("d")?;
            let document = Document::try_from(node)?;
            return Ok(document);
        }

        Err(DocumentRepositoryError::CreationFailed)
    }

    /// Create a document with an explicit ID instead of auto-generating one.
    ///
    /// This is used by the admin registration endpoint when the caller
    /// provides a specific ID. The rest of the logic is identical to
    /// `create_document`.
    pub async fn create_document_with_id(
        &self,
        id: &str,
        request: DocumentCreateRequest,
    ) -> Result<Document, DocumentRepositoryError> {
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
                        source_url: $source_url,
                        status: $status
                    }) RETURN d",
                )
                .param("id", id)
                .param("title", request.title)
                .param("doc_type", request.doc_type)
                .param("created_at", created_at_value)
                .param("description", request.description)
                .param("file_path", request.file_path)
                .param("uploaded_at", request.uploaded_at)
                .param("related_claim_id", request.related_claim_id)
                .param("source_url", request.source_url)
                .param("status", STATUS_UPLOADED),
            )
            .await?;

        if let Some(row) = result.next().await? {
            let node: Node = row.get("d")?;
            let document = Document::try_from(node)?;
            return Ok(document);
        }

        Err(DocumentRepositoryError::CreationFailed)
    }

    /// Find a document by its SHA-256 content hash.
    ///
    /// Returns `Ok(Some(doc))` if found, `Ok(None)` if no match.
    /// Used by the admin endpoint for duplicate detection before registration.
    pub async fn find_by_content_hash(
        &self,
        content_hash: &str,
    ) -> Result<Option<Document>, DocumentRepositoryError> {
        let mut result = self
            .graph
            .execute(
                query("MATCH (d:Document {content_hash: $hash}) RETURN d")
                    .param("hash", content_hash),
            )
            .await?;

        if let Some(row) = result.next().await? {
            let node: Node = row.get("d")?;
            let document = Document::try_from(node)?;
            return Ok(Some(document));
        }

        Ok(None)
    }

    /// List all documents with their evidence counts.
    ///
    /// Uses an OPTIONAL MATCH to count Evidence nodes linked via
    /// CONTAINED_IN relationships. Documents with no evidence get count 0.
    pub async fn list_documents_with_evidence_counts(
        &self,
    ) -> Result<Vec<(Document, i64)>, DocumentRepositoryError> {
        let mut result = self
            .graph
            .execute(query(
                "MATCH (d:Document)
                 OPTIONAL MATCH (e:Evidence)-[:CONTAINED_IN]->(d)
                 RETURN d, count(e) AS evidence_count
                 ORDER BY d.created_at DESC",
            ))
            .await?;

        let mut documents = Vec::new();
        while let Some(row) = result.next().await? {
            let node: Node = row.get("d")?;
            let evidence_count: i64 = row.get("evidence_count").unwrap_or(0);
            let document = Document::try_from(node)?;
            documents.push((document, evidence_count));
        }

        Ok(documents)
    }

    /// Set the content_hash property on an existing Document node.
    ///
    /// Called after SHA-256 computation during admin document registration.
    /// Uses SET (not a CREATE property) so it works on existing nodes too.
    pub async fn set_content_hash(
        &self,
        id: &str,
        content_hash: &str,
    ) -> Result<(), DocumentRepositoryError> {
        self.graph
            .run(
                query("MATCH (d:Document {id: $id}) SET d.content_hash = $hash")
                    .param("id", id)
                    .param("hash", content_hash),
            )
            .await?;

        Ok(())
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
