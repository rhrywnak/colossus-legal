// TODO: DAL Phase 2 — this repository queries v1 labels (:Evidence)
// that no longer exist in the v2 graph. The API endpoint that uses this
// repository will return empty results. Migrate or remove once v1 data
// is fully deprecated.

use neo4rs::{query, Graph};
use std::collections::HashMap;

use crate::dto::{EvidenceDto, EvidenceResponse};

#[derive(Clone)]
pub struct EvidenceRepository {
    graph: Graph,
}

#[derive(Debug)]
pub enum EvidenceRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
}

impl From<neo4rs::Error> for EvidenceRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        EvidenceRepositoryError::Neo4j(value)
    }
}

impl From<neo4rs::DeError> for EvidenceRepositoryError {
    fn from(value: neo4rs::DeError) -> Self {
        EvidenceRepositoryError::Value(value)
    }
}

impl EvidenceRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch all evidence from Neo4j with linked documents
    pub async fn list_evidence(&self) -> Result<EvidenceResponse, EvidenceRepositoryError> {
        let mut evidence_list: Vec<EvidenceDto> = Vec::new();
        let mut by_kind: HashMap<String, usize> = HashMap::new();

        let mut result = self
            .graph
            .execute(query(
                "MATCH (e:Evidence)
                 OPTIONAL MATCH (e)-[:CONTAINED_IN]->(d:Document)
                 OPTIONAL MATCH (e)-[:STATED_BY]->(speaker)
                 RETURN e.id AS id,
                        e.exhibit_number AS exhibit_number,
                        e.title AS title,
                        e.question AS question,
                        e.answer AS answer,
                        e.kind AS kind,
                        e.weight AS weight,
                        e.page_number AS page_number,
                        e.significance AS significance,
                        e.verbatim_quote AS verbatim_quote,
                        speaker.name AS stated_by,
                        d.id AS document_id,
                        d.title AS document_title
                 ORDER BY e.id",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            let id: String = row.get("id").unwrap_or_default();
            let exhibit_number: Option<String> = row.get("exhibit_number").ok();
            let title: Option<String> = row.get("title").ok();
            let question: Option<String> = row.get("question").ok();
            let answer: Option<String> = row.get("answer").ok();
            let kind: Option<String> = row.get("kind").ok();
            let weight: Option<i64> = row.get("weight").ok();
            let page_number: Option<i64> = row.get("page_number").ok();
            let significance: Option<String> = row.get("significance").ok();
            let verbatim_quote: Option<String> = row.get("verbatim_quote").ok();
            let stated_by: Option<String> = row.get("stated_by").ok();
            let document_id: Option<String> = row.get("document_id").ok();
            let document_title: Option<String> = row.get("document_title").ok();

            // Count by kind
            if let Some(ref k) = kind {
                *by_kind.entry(k.clone()).or_insert(0) += 1;
            }

            evidence_list.push(EvidenceDto {
                id,
                exhibit_number,
                title,
                question,
                answer,
                kind,
                weight,
                page_number,
                significance,
                verbatim_quote,
                stated_by,
                document_id,
                document_title,
            });
        }

        let total = evidence_list.len();

        Ok(EvidenceResponse {
            evidence: evidence_list,
            total,
            by_kind,
        })
    }
}
