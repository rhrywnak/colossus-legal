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
                "MATCH (a:Evidence)-[r:CONTRADICTS]->(b:Evidence)
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
                        r.description AS description,
                        r.topic AS topic,
                        r.impeachment_value AS impeachment_value,
                        r.earlier_claim AS earlier_claim,
                        r.later_admission AS later_admission
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
            let topic: Option<String> = row.get("topic").ok();
            let impeachment_value: Option<String> = row.get("impeachment_value").ok();
            let earlier_claim: Option<String> = row.get("earlier_claim").ok();
            let later_admission: Option<String> = row.get("later_admission").ok();

            contradictions.push(ContradictionDto {
                evidence_a,
                evidence_b,
                description,
                topic,
                impeachment_value,
                earlier_claim,
                later_admission,
            });
        }

        let total = contradictions.len();

        Ok(ContradictionsResponse {
            contradictions,
            total,
        })
    }
}
