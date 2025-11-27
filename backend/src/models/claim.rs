use neo4rs::{DeError, Node};
use serde::{Deserialize, Serialize};

/// Domain model for a Claim node in Neo4j.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Claim {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
}

#[derive(Debug)]
pub enum ClaimConversionError {
    MissingField(&'static str),
    Value(DeError),
    Neo4j(neo4rs::Error),
}

impl From<neo4rs::Error> for ClaimConversionError {
    fn from(value: neo4rs::Error) -> Self {
        ClaimConversionError::Neo4j(value)
    }
}

impl From<DeError> for ClaimConversionError {
    fn from(value: DeError) -> Self {
        ClaimConversionError::Value(value)
    }
}

impl TryFrom<Node> for Claim {
    type Error = ClaimConversionError;

    fn try_from(node: Node) -> Result<Self, Self::Error> {
        let id: String = node.get("id").map_err(ClaimConversionError::from)?;
        let title: String = node.get("title").map_err(ClaimConversionError::from)?;
        let status: String = node.get("status").map_err(ClaimConversionError::from)?;
        let description: Option<String> = node.get("description").ok();

        Ok(Self {
            id,
            title,
            description,
            status,
        })
    }
}
