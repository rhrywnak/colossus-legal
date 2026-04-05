// TODO: DAL Phase 2 — migrate to colossus_graph once batch relationship
// queries are available. Kept as raw Cypher because the SUPPORTS aggregation
// (collect legal count IDs per allegation) would require N+1 queries.

use neo4rs::{query, Graph};

use crate::dto::{AllegationDto, AllegationSummary, AllegationsResponse};

#[derive(Clone)]
pub struct AllegationRepository {
    graph: Graph,
}

#[derive(Debug)]
pub enum AllegationRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
}

impl From<neo4rs::Error> for AllegationRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        AllegationRepositoryError::Neo4j(value)
    }
}

impl From<neo4rs::DeError> for AllegationRepositoryError {
    fn from(value: neo4rs::DeError) -> Self {
        AllegationRepositoryError::Value(value)
    }
}

impl AllegationRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch all allegations from Neo4j with their linked legal counts
    pub async fn list_allegations(
        &self,
    ) -> Result<AllegationsResponse, AllegationRepositoryError> {
        let mut allegations: Vec<AllegationDto> = Vec::new();

        let mut result = self
            .graph
            .execute(query(
                "MATCH (a:ComplaintAllegation)
                 OPTIONAL MATCH (a)-[:SUPPORTS]->(c:LegalCount)
                 WITH a, collect(DISTINCT c.id) AS legal_count_ids,
                      collect(DISTINCT c.title) AS legal_counts
                 RETURN a.id AS id,
                        a.paragraph AS paragraph,
                        a.title AS title,
                        a.allegation AS allegation,
                        a.evidence_status AS evidence_status,
                        a.category AS category,
                        a.severity AS severity,
                        legal_count_ids,
                        legal_counts
                 ORDER BY a.id",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            let id: String = row.get("id").unwrap_or_default();
            let title: String = row.get("title").unwrap_or_default();
            let paragraph: Option<String> = row.get("paragraph").ok();
            let allegation: Option<String> = row.get("allegation").ok();
            let evidence_status: Option<String> = row.get("evidence_status").ok();
            let category: Option<String> = row.get("category").ok();
            let severity: Option<i64> = row.get("severity").ok();
            // Filter out null values from the collected arrays
            let legal_count_ids: Vec<String> = row
                .get::<Vec<Option<String>>>("legal_count_ids")
                .unwrap_or_default()
                .into_iter()
                .flatten()
                .collect();
            let legal_counts: Vec<String> = row
                .get::<Vec<Option<String>>>("legal_counts")
                .unwrap_or_default()
                .into_iter()
                .flatten()
                .collect();

            allegations.push(AllegationDto {
                id,
                paragraph,
                title,
                allegation,
                evidence_status,
                category,
                severity,
                legal_count_ids,
                legal_counts,
            });
        }

        // Calculate summary
        let proven = allegations
            .iter()
            .filter(|a| a.evidence_status.as_deref() == Some("PROVEN"))
            .count();
        let partial = allegations
            .iter()
            .filter(|a| a.evidence_status.as_deref() == Some("PARTIAL"))
            .count();
        let unproven = allegations
            .iter()
            .filter(|a| a.evidence_status.as_deref() == Some("UNPROVEN"))
            .count();

        let total = allegations.len();
        let summary = AllegationSummary {
            proven,
            partial,
            unproven,
        };

        Ok(AllegationsResponse {
            allegations,
            total,
            summary,
        })
    }
}
