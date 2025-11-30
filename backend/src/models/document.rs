use chrono::NaiveDate;
use neo4rs::{DeError, Node};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub doc_type: Option<String>,
    pub description: Option<String>,
    pub file_path: Option<String>,
    pub uploaded_at: Option<NaiveDate>,
    pub related_claim_id: Option<String>,
    pub source_url: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug)]
pub enum DocumentConversionError {
    MissingField(&'static str),
    Value(DeError),
    Neo4j(neo4rs::Error),
}

impl From<neo4rs::Error> for DocumentConversionError {
    fn from(value: neo4rs::Error) -> Self {
        DocumentConversionError::Neo4j(value)
    }
}

impl From<DeError> for DocumentConversionError {
    fn from(value: DeError) -> Self {
        DocumentConversionError::Value(value)
    }
}

impl TryFrom<Node> for Document {
    type Error = DocumentConversionError;

    fn try_from(node: Node) -> Result<Self, Self::Error> {
        let id: String = node.get("id").map_err(DocumentConversionError::from)?;
        let title: String = node
            .get("title")
            .map_err(|_| DocumentConversionError::MissingField("title"))?;

        let doc_type: Option<String> = node.get("doc_type").ok();
        let description: Option<String> = node.get("description").ok();
        let file_path: Option<String> = node.get("file_path").ok();
        let uploaded_at: Option<NaiveDate> = node.get("uploaded_at").ok();
        let related_claim_id: Option<String> = node.get("related_claim_id").ok();
        let source_url: Option<String> = node.get("source_url").ok();
        let created_at: Option<String> = node.get("created_at").ok();

        Ok(Self {
            id,
            title,
            doc_type,
            description,
            file_path,
            uploaded_at,
            related_claim_id,
            source_url,
            created_at,
        })
    }
}
