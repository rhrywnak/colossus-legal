use neo4rs::{query, Graph};

use crate::dto::{
    AllegationStrength, AnalysisResponse, ContradictionBrief, ContradictionsSummary,
    DocumentCoverage, EvidenceCoverage, GapAnalysis,
};

#[derive(Clone)]
pub struct AnalysisRepository {
    graph: Graph,
}

#[derive(Debug)]
pub enum AnalysisRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
}

impl From<neo4rs::Error> for AnalysisRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        AnalysisRepositoryError::Neo4j(value)
    }
}

impl From<neo4rs::DeError> for AnalysisRepositoryError {
    fn from(value: neo4rs::DeError) -> Self {
        AnalysisRepositoryError::Value(value)
    }
}

impl AnalysisRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch complete analysis data from Neo4j
    pub async fn get_analysis(&self) -> Result<AnalysisResponse, AnalysisRepositoryError> {
        let gap_analysis = self.fetch_gap_analysis().await?;
        let contradictions_summary = self.fetch_contradictions_summary().await?;
        let evidence_coverage = self.fetch_evidence_coverage().await?;

        Ok(AnalysisResponse {
            gap_analysis,
            contradictions_summary,
            evidence_coverage,
        })
    }

    /// Calculate allegation strength based on evidence count
    ///
    /// Strength calculation:
    /// - 3+ evidence items = 90-98% (strong)
    /// - 2 evidence items = 75-89% (moderate)
    /// - 1 evidence item = 50-74% (weak)
    /// - 0 evidence items = 0-49% (gap)
    fn calculate_strength(evidence_count: i64) -> (i32, String) {
        match evidence_count {
            0 => (25, "gap".to_string()),
            1 => (60, "weak".to_string()),
            2 => (80, "moderate".to_string()),
            3 => (90, "strong".to_string()),
            _ => (95, "strong".to_string()),
        }
    }

    /// Fetch gap analysis data
    async fn fetch_gap_analysis(&self) -> Result<GapAnalysis, AnalysisRepositoryError> {
        let mut allegations: Vec<AllegationStrength> = Vec::new();

        // Query allegations with evidence counts via MotionClaim relationships
        // Path: Evidence <-[:RELIES_ON]- MotionClaim -[:PROVES]-> ComplaintAllegation
        let mut result = self
            .graph
            .execute(query(
                "MATCH (a:ComplaintAllegation)
                 OPTIONAL MATCH (a)<-[:PROVES]-(mc:MotionClaim)-[:RELIES_ON]->(e:Evidence)
                 WITH a, collect(DISTINCT e) AS evidence_list
                 RETURN a.id AS id,
                        a.allegation AS allegation,
                        a.paragraph AS paragraph,
                        a.evidence_status AS evidence_status,
                        size(evidence_list) AS evidence_count,
                        [e IN evidence_list | e.title][0..5] AS evidence_titles
                 ORDER BY size(evidence_list) DESC, a.id",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            let id: String = row.get("id").unwrap_or_default();
            let allegation: Option<String> = row.get("allegation").ok();
            let paragraph: Option<String> = row.get("paragraph").ok();
            let evidence_count: i64 = row.get("evidence_count").unwrap_or(0);

            // Get evidence titles (up to 5)
            let evidence_titles: Vec<String> = row
                .get::<Vec<String>>("evidence_titles")
                .unwrap_or_default();

            let (strength_percent, strength_category) = Self::calculate_strength(evidence_count);

            // Generate gap notes for weak/gap allegations
            let gap_notes = if strength_category == "gap" {
                Some("No evidence linked to this allegation".to_string())
            } else if strength_category == "weak" {
                Some("Limited evidence - consider additional documentation".to_string())
            } else {
                None
            };

            allegations.push(AllegationStrength {
                id,
                allegation,
                paragraph,
                strength_percent,
                strength_category,
                supporting_evidence_count: evidence_count as i32,
                supporting_evidence: evidence_titles,
                gap_notes,
            });
        }

        // Calculate summary counts
        let total_allegations = allegations.len() as i32;
        let strong_evidence = allegations
            .iter()
            .filter(|a| a.strength_category == "strong")
            .count() as i32;
        let moderate_evidence = allegations
            .iter()
            .filter(|a| a.strength_category == "moderate")
            .count() as i32;
        let weak_evidence = allegations
            .iter()
            .filter(|a| a.strength_category == "weak")
            .count() as i32;
        let gaps = allegations
            .iter()
            .filter(|a| a.strength_category == "gap")
            .count() as i32;

        Ok(GapAnalysis {
            total_allegations,
            strong_evidence,
            moderate_evidence,
            weak_evidence,
            gaps,
            allegations,
        })
    }

    /// Fetch contradictions summary
    async fn fetch_contradictions_summary(
        &self,
    ) -> Result<ContradictionsSummary, AnalysisRepositoryError> {
        let mut contradictions: Vec<ContradictionBrief> = Vec::new();

        let mut result = self
            .graph
            .execute(query(
                "MATCH (a:Evidence)-[r:CONTRADICTS]->(b:Evidence)
                 RETURN a.id AS evidence_a_id,
                        a.title AS evidence_a_title,
                        a.answer AS evidence_a_answer,
                        b.id AS evidence_b_id,
                        b.title AS evidence_b_title,
                        b.answer AS evidence_b_answer,
                        r.description AS description
                 ORDER BY a.id",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            contradictions.push(ContradictionBrief {
                evidence_a_id: row.get("evidence_a_id").unwrap_or_default(),
                evidence_a_title: row.get("evidence_a_title").ok(),
                evidence_a_answer: row.get("evidence_a_answer").ok(),
                evidence_b_id: row.get("evidence_b_id").unwrap_or_default(),
                evidence_b_title: row.get("evidence_b_title").ok(),
                evidence_b_answer: row.get("evidence_b_answer").ok(),
                description: row.get("description").ok(),
            });
        }

        let total = contradictions.len() as i32;

        Ok(ContradictionsSummary {
            total,
            contradictions,
        })
    }

    /// Fetch evidence coverage by document
    async fn fetch_evidence_coverage(&self) -> Result<EvidenceCoverage, AnalysisRepositoryError> {
        let mut by_document: Vec<DocumentCoverage> = Vec::new();

        // Get coverage per document
        let mut result = self
            .graph
            .execute(query(
                "MATCH (d:Document)
                 OPTIONAL MATCH (e:Evidence)-[:CONTAINED_IN]->(d)
                 OPTIONAL MATCH (e)<-[:RELIES_ON]-(mc:MotionClaim)-[:PROVES]->(a:ComplaintAllegation)
                 WITH d,
                      count(DISTINCT e) AS evidence_count,
                      count(DISTINCT CASE WHEN a IS NOT NULL THEN e END) AS linked_count
                 RETURN d.id AS document_id,
                        d.title AS document_title,
                        evidence_count,
                        linked_count
                 ORDER BY evidence_count DESC, d.title",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            by_document.push(DocumentCoverage {
                document_id: row.get("document_id").unwrap_or_default(),
                document_title: row.get("document_title").ok(),
                evidence_count: row.get::<i64>("evidence_count").unwrap_or(0) as i32,
                linked_count: row.get::<i64>("linked_count").unwrap_or(0) as i32,
            });
        }

        // Calculate totals
        let total_evidence_nodes: i32 = by_document.iter().map(|d| d.evidence_count).sum();
        let linked_to_allegations: i32 = by_document.iter().map(|d| d.linked_count).sum();
        let unlinked = total_evidence_nodes - linked_to_allegations;

        Ok(EvidenceCoverage {
            total_evidence_nodes,
            linked_to_allegations,
            unlinked,
            by_document,
        })
    }
}
