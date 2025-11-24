use chrono::NaiveDate;
use neo4rs::{Graph, Node, Query};
use crate::models::claim::Claim;

/// A repository struct that will contain real Neo4j-backed operations for Claims.
#[derive(Clone)]
pub struct ClaimRepository {
    pub graph: Graph,
}

impl ClaimRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// List all Claim nodes from Neo4j.
    pub async fn list_claims(&self) -> Result<Vec<Claim>, neo4rs::Error> {
        // Simple Cypher: return all Claim nodes
        let mut result = self
            .graph
            .execute(Query::new("MATCH (c:Claim) RETURN c".into()))
            .await?;

        let mut claims = Vec::new();

        // result.next().await returns Option<Result<Row, Error>>
        while let Some(row_result) = result.next().await {
            let row = row_result?; // propagate Neo4j error if any
            let node: Node = row.get("c")?; // "c" is the alias in RETURN c

            let id: String = node.get("id").unwrap_or_default();
            let text: String = node.get("text").unwrap_or_default();
            let made_by: Option<String> = node.get("made_by").ok();
            let category: Option<String> = node.get("category").ok();
            let verified: Option<bool> = node.get("verified").ok();

            // first_made is stored as string "YYYY-MM-DD", parse into NaiveDate if present
            let first_made_str: Option<String> = node.get("first_made").ok();
            let first_made: Option<NaiveDate> = first_made_str
                .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());

            claims.push(Claim {
                id,
                text,
                made_by,
                first_made,
                category,
                verified,
            });
        }

        Ok(claims)
    }
}

