use neo4rs::{query, Graph};
use std::collections::HashMap;

use crate::dto::{MotionClaimDto, MotionClaimsResponse};

#[derive(Clone)]
pub struct MotionClaimRepository {
    graph: Graph,
}

#[derive(Debug)]
pub enum MotionClaimRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
}

impl From<neo4rs::Error> for MotionClaimRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        MotionClaimRepositoryError::Neo4j(value)
    }
}

impl From<neo4rs::DeError> for MotionClaimRepositoryError {
    fn from(value: neo4rs::DeError) -> Self {
        MotionClaimRepositoryError::Value(value)
    }
}

impl MotionClaimRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch all MotionClaim nodes with their relationships:
    /// - PROVES -> ComplaintAllegation
    /// - RELIES_ON -> Evidence
    /// - APPEARS_IN -> Document
    pub async fn list_motion_claims(
        &self,
    ) -> Result<MotionClaimsResponse, MotionClaimRepositoryError> {
        let mut motion_claims: Vec<MotionClaimDto> = Vec::new();
        let mut by_category: HashMap<String, usize> = HashMap::new();

        let mut result = self
            .graph
            .execute(query(
                "MATCH (m:MotionClaim)
                 OPTIONAL MATCH (m)-[:PROVES]->(a:ComplaintAllegation)
                 OPTIONAL MATCH (m)-[:RELIES_ON]->(e:Evidence)
                 OPTIONAL MATCH (m)-[:APPEARS_IN]->(d:Document)
                 WITH m,
                      collect(DISTINCT a.id) AS allegation_ids,
                      collect(DISTINCT e.id) AS evidence_ids,
                      d
                 RETURN m.id AS id,
                        m.title AS title,
                        m.claim_text AS claim_text,
                        m.category AS category,
                        m.significance AS significance,
                        allegation_ids,
                        evidence_ids,
                        d.id AS document_id,
                        d.title AS document_title
                 ORDER BY m.id",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            let id: String = row.get("id").unwrap_or_default();
            let title: String = row.get("title").unwrap_or_default();
            let claim_text: Option<String> = row.get("claim_text").ok();
            let category: Option<String> = row.get("category").ok();
            let significance: Option<String> = row.get("significance").ok();
            let source_document_id: Option<String> = row.get("document_id").ok();
            let source_document_title: Option<String> = row.get("document_title").ok();

            // Get arrays, filtering out nulls
            let allegation_ids_raw: Vec<Option<String>> =
                row.get("allegation_ids").unwrap_or_default();
            let proves_allegations: Vec<String> =
                allegation_ids_raw.into_iter().flatten().collect();

            let evidence_ids_raw: Vec<Option<String>> = row.get("evidence_ids").unwrap_or_default();
            let relies_on_evidence: Vec<String> = evidence_ids_raw.into_iter().flatten().collect();

            // Count by category
            if let Some(ref cat) = category {
                *by_category.entry(cat.clone()).or_insert(0) += 1;
            }

            motion_claims.push(MotionClaimDto {
                id,
                title,
                claim_text,
                category,
                significance,
                proves_allegations,
                relies_on_evidence,
                source_document_id,
                source_document_title,
            });
        }

        let total = motion_claims.len();

        Ok(MotionClaimsResponse {
            motion_claims,
            total,
            by_category,
        })
    }
}
