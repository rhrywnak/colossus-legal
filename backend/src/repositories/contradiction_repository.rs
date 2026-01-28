use neo4rs::{query, Graph};

use crate::dto::{ContradictionDto, ContradictionEvidence, ContradictionsResponse};

#[derive(Clone)]
pub struct ContradictionRepository {
    graph: Graph,
}

#[derive(Debug)]
pub enum ContradictionRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
}

impl From<neo4rs::Error> for ContradictionRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        ContradictionRepositoryError::Neo4j(value)
    }
}

impl From<neo4rs::DeError> for ContradictionRepositoryError {
    fn from(value: neo4rs::DeError) -> Self {
        ContradictionRepositoryError::Value(value)
    }
}

impl ContradictionRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch all contradictions from Neo4j
    pub async fn list_contradictions(
        &self,
    ) -> Result<ContradictionsResponse, ContradictionRepositoryError> {
        let mut contradictions: Vec<ContradictionDto> = Vec::new();

        let mut result = self
            .graph
            .execute(query(
                "MATCH (a:Evidence)-[r:CONTRADICTED_BY]->(b:Evidence)
                 OPTIONAL MATCH (a)-[:CONTAINED_IN]->(da:Document)
                 OPTIONAL MATCH (b)-[:CONTAINED_IN]->(db:Document)
                 RETURN a.id AS evidence_a_id,
                        a.title AS evidence_a_title,
                        a.answer AS evidence_a_answer,
                        da.title AS evidence_a_document,
                        b.id AS evidence_b_id,
                        b.title AS evidence_b_title,
                        b.answer AS evidence_b_answer,
                        db.title AS evidence_b_document,
                        r.description AS description
                 ORDER BY a.id",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            let evidence_a = ContradictionEvidence {
                id: row.get("evidence_a_id").unwrap_or_default(),
                title: row.get("evidence_a_title").ok(),
                answer: row.get("evidence_a_answer").ok(),
                document_title: row.get("evidence_a_document").ok(),
            };

            let evidence_b = ContradictionEvidence {
                id: row.get("evidence_b_id").unwrap_or_default(),
                title: row.get("evidence_b_title").ok(),
                answer: row.get("evidence_b_answer").ok(),
                document_title: row.get("evidence_b_document").ok(),
            };

            let description: Option<String> = row.get("description").ok();

            contradictions.push(ContradictionDto {
                evidence_a,
                evidence_b,
                description,
            });
        }

        let total = contradictions.len();

        Ok(ContradictionsResponse {
            contradictions,
            total,
        })
    }
}
