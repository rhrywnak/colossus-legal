// TODO: B-1 Approach C — this repository queries :Claim nodes which have
// never existed in v1 or v2. The /claims API routes are registered but
// return empty results. The Claim model and these CRUD functions are
// legacy scaffolding. Parameterize or remove when Claim nodes are defined
// in a future schema.

use neo4rs::{query, DeError, Graph, Node};

use crate::models::claim::{Claim, ClaimConversionError};
use chrono::Utc;

#[derive(Clone)]
pub struct ClaimRepository {
    graph: Graph,
}

#[derive(Debug)]
pub enum ClaimRepositoryError {
    Neo4j(neo4rs::Error),
    Mapping(ClaimConversionError),
    Value(DeError),
    NotFound,
    CreationFailed,
}

impl From<neo4rs::Error> for ClaimRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        ClaimRepositoryError::Neo4j(value)
    }
}

impl From<ClaimConversionError> for ClaimRepositoryError {
    fn from(value: ClaimConversionError) -> Self {
        ClaimRepositoryError::Mapping(value)
    }
}

impl From<DeError> for ClaimRepositoryError {
    fn from(value: DeError) -> Self {
        ClaimRepositoryError::Value(value)
    }
}

impl ClaimRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    pub async fn list_claims(&self) -> Result<Vec<Claim>, ClaimRepositoryError> {
        let mut result = self
            .graph
            .execute(query("MATCH (c:Claim) RETURN c"))
            .await?;

        let mut claims = Vec::new();
        while let Some(row) = result.next().await? {
            let node: Node = row.get("c")?;
            let claim = Claim::try_from(node)?;
            claims.push(claim);
        }

        Ok(claims)
    }

    pub async fn get_claim_by_id(&self, id: &str) -> Result<Claim, ClaimRepositoryError> {
        let mut result = self
            .graph
            .execute(query("MATCH (c:Claim {id: $id}) RETURN c").param("id", id))
            .await?;

        if let Some(row) = result.next().await? {
            let node: Node = row.get("c")?;
            let claim = Claim::try_from(node)?;
            return Ok(claim);
        }

        Err(ClaimRepositoryError::NotFound)
    }

    pub async fn create_claim(
        &self,
        title: &str,
        description: Option<&str>,
        status: &str,
    ) -> Result<Claim, ClaimRepositoryError> {
        let id = format!("claim-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));

        let mut result = self
            .graph
            .execute(
                query(
                    "CREATE (c:Claim {id: $id, title: $title, description: $description, status: $status}) RETURN c",
                )
                .param("id", id.clone())
                .param("title", title)
                .param("description", description)
                .param("status", status),
            )
            .await?;

        if let Some(row) = result.next().await? {
            let node: Node = row.get("c")?;
            let claim = Claim::try_from(node)?;
            return Ok(claim);
        }

        Err(ClaimRepositoryError::CreationFailed)
    }

    pub async fn update_claim(
        &self,
        id: &str,
        title: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
    ) -> Result<Claim, ClaimRepositoryError> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (c:Claim {id: $id}) \
                     SET c.title = COALESCE($title, c.title), \
                         c.description = COALESCE($description, c.description), \
                         c.status = COALESCE($status, c.status) \
                     RETURN c",
                )
                .param("id", id)
                .param("title", title)
                .param("description", description)
                .param("status", status),
            )
            .await?;

        if let Some(row) = result.next().await? {
            let node: Node = row.get("c")?;
            let claim = Claim::try_from(node)?;
            return Ok(claim);
        }

        Err(ClaimRepositoryError::NotFound)
    }
}
