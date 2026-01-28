use neo4rs::{query, Graph};
use std::collections::{HashMap, HashSet};

use crate::dto::{
    ChainAllegation, ChainDocument, ChainSummary, EvidenceChainResponse,
    EvidenceWithDocument, MotionClaimWithEvidence,
};

#[derive(Clone)]
pub struct EvidenceChainRepository {
    graph: Graph,
}

#[derive(Debug)]
pub enum EvidenceChainRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
}

impl From<neo4rs::Error> for EvidenceChainRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        EvidenceChainRepositoryError::Neo4j(value)
    }
}

impl From<neo4rs::DeError> for EvidenceChainRepositoryError {
    fn from(value: neo4rs::DeError) -> Self {
        EvidenceChainRepositoryError::Value(value)
    }
}

impl EvidenceChainRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch the complete evidence chain for a single allegation
    ///
    /// Returns None if the allegation is not found
    pub async fn get_evidence_chain(
        &self,
        allegation_id: &str,
    ) -> Result<Option<EvidenceChainResponse>, EvidenceChainRepositoryError> {
        let cypher = "
            MATCH (a:ComplaintAllegation {id: $allegation_id})
            OPTIONAL MATCH (a)-[:SUPPORTS]->(c:LegalCount)
            OPTIONAL MATCH (m:MotionClaim)-[:PROVES]->(a)
            OPTIONAL MATCH (m)-[:RELIES_ON]->(e:Evidence)
            OPTIONAL MATCH (e)-[:CONTAINED_IN]->(d:Document)
            RETURN a.id AS a_id, a.title AS a_title, a.paragraph AS a_paragraph,
                   a.evidence_status AS a_status,
                   collect(DISTINCT c.title) AS legal_counts,
                   m.id AS m_id, m.title AS m_title,
                   e.id AS e_id, e.title AS e_title, e.question AS e_question,
                   e.answer AS e_answer, e.page_number AS e_page,
                   d.id AS d_id, d.title AS d_title
        ";

        let q = query(cypher).param("allegation_id", allegation_id);
        let mut result = self.graph.execute(q).await?;

        let mut allegation: Option<ChainAllegation> = None;
        let mut motion_claims_map: HashMap<String, MotionClaimWithEvidence> = HashMap::new();
        let mut evidence_set: HashSet<String> = HashSet::new();
        let mut document_set: HashSet<String> = HashSet::new();

        while let Some(row) = result.next().await? {
            // Extract allegation (only once, from first row)
            if allegation.is_none() {
                if let Ok(a_id) = row.get::<String>("a_id") {
                    let a_title: String = row.get("a_title").unwrap_or_default();
                    let a_paragraph: Option<String> = row.get("a_paragraph").ok();
                    let a_status: Option<String> = row.get("a_status").ok();
                    let legal_counts: Vec<String> = row
                        .get::<Vec<Option<String>>>("legal_counts")
                        .unwrap_or_default()
                        .into_iter()
                        .flatten()
                        .collect();

                    allegation = Some(ChainAllegation {
                        id: a_id,
                        title: a_title,
                        paragraph: a_paragraph,
                        evidence_status: a_status,
                        legal_counts,
                    });
                }
            }

            // Extract motion claim and evidence
            if let Ok(m_id) = row.get::<String>("m_id") {
                let m_title: String = row.get("m_title").unwrap_or_default();

                // Build evidence item if present
                let evidence_item = if let Ok(e_id) = row.get::<String>("e_id") {
                    let e_title: String = row.get("e_title").unwrap_or_default();
                    let e_question: Option<String> = row.get("e_question").ok();
                    let e_answer: Option<String> = row.get("e_answer").ok();
                    let e_page: Option<i64> = row.get("e_page").ok();

                    // Build document if present
                    let document = if let Ok(d_id) = row.get::<String>("d_id") {
                        let d_title: String = row.get("d_title").unwrap_or_default();
                        document_set.insert(d_id.clone());
                        Some(ChainDocument {
                            id: d_id,
                            title: d_title,
                            page_number: e_page,
                        })
                    } else {
                        None
                    };

                    // Track unique evidence
                    if evidence_set.insert(e_id.clone()) {
                        Some(EvidenceWithDocument {
                            id: e_id,
                            title: e_title,
                            question: e_question,
                            answer: e_answer,
                            document,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Add or update motion claim
                motion_claims_map
                    .entry(m_id.clone())
                    .or_insert_with(|| MotionClaimWithEvidence {
                        id: m_id,
                        title: m_title,
                        evidence: Vec::new(),
                    })
                    .evidence
                    .extend(evidence_item);
            }
        }

        // Return None if allegation not found
        let allegation = match allegation {
            Some(a) => a,
            None => return Ok(None),
        };

        let motion_claims: Vec<MotionClaimWithEvidence> =
            motion_claims_map.into_values().collect();

        let summary = ChainSummary {
            motion_claim_count: motion_claims.len(),
            evidence_count: evidence_set.len(),
            document_count: document_set.len(),
        };

        Ok(Some(EvidenceChainResponse {
            allegation,
            motion_claims,
            summary,
        }))
    }
}
