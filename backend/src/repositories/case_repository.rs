use neo4rs::{query, Graph};

use crate::dto::{CaseInfo, CaseResponse, CaseStats, LegalCountSummary, PartiesGroup, PartyDto};

#[derive(Clone)]
pub struct CaseRepository {
    graph: Graph,
}

#[derive(Debug)]
pub enum CaseRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
}

impl From<neo4rs::Error> for CaseRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        CaseRepositoryError::Neo4j(value)
    }
}

impl From<neo4rs::DeError> for CaseRepositoryError {
    fn from(value: neo4rs::DeError) -> Self {
        CaseRepositoryError::Value(value)
    }
}

impl CaseRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch the case with parties and stats.
    /// Returns None if no Case node exists.
    pub async fn get_case(&self) -> Result<Option<CaseResponse>, CaseRepositoryError> {
        // Query 1: Get Case node properties
        let case_info = self.get_case_info().await?;
        let Some(case_info) = case_info else {
            return Ok(None);
        };

        // Query 2: Get parties grouped by role
        let parties = self.get_parties().await?;

        // Query 3: Get aggregated stats
        let mut stats = self.get_stats().await?;

        // Query 4: Get legal count details
        stats.legal_count_details = self.get_legal_count_details().await?;

        Ok(Some(CaseResponse {
            case: Some(case_info),
            parties,
            stats,
        }))
    }

    /// Fetch case identity from the complaint Document node (v2 pipeline).
    /// Fields like court, case_number, filing_date don't exist on Document
    /// nodes — they return None until richer metadata is available.
    async fn get_case_info(&self) -> Result<Option<CaseInfo>, CaseRepositoryError> {
        let mut result = self
            .graph
            .execute(query(
                "MATCH (d:Document)
                 WHERE d.doc_type CONTAINS 'complaint'
                 RETURN d.id AS id, d.title AS title, d.doc_type AS doc_type,
                        d.ingested_at AS ingested_at, d.status AS status
                 LIMIT 1",
            ))
            .await?;

        if let Some(row) = result.next().await? {
            let id: String = row.get("id").unwrap_or_default();
            let title: String = row.get("title").unwrap_or_default();
            let status: Option<String> = row.get("status").ok();

            Ok(Some(CaseInfo {
                id,
                title,
                case_number: None,
                court: None,
                court_type: None,
                filing_date: None,
                status,
                summary: None,
            }))
        } else {
            Ok(None)
        }
    }

    /// Fetch parties by role property on Person/Organization nodes (v2 pipeline).
    /// V2 stores role directly on the node, not on an INVOLVES relationship.
    async fn get_parties(&self) -> Result<PartiesGroup, CaseRepositoryError> {
        let mut plaintiffs: Vec<PartyDto> = Vec::new();
        let mut defendants: Vec<PartyDto> = Vec::new();
        let mut other: Vec<PartyDto> = Vec::new();

        let mut result = self
            .graph
            .execute(query(
                "MATCH (n)
                 WHERE n.role IS NOT NULL
                 RETURN n.id AS id, n.name AS name, n.role AS role,
                        labels(n)[0] AS entity_type
                 ORDER BY n.role, n.name",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            let id: String = row.get("id").unwrap_or_default();
            let name: String = row.get("name").unwrap_or_default();
            let role: String = row.get("role").unwrap_or_default();
            let entity_type: String = row.get("entity_type").unwrap_or_default();

            let party_type = if entity_type == "Organization" {
                "organization".to_string()
            } else {
                "person".to_string()
            };

            let party = PartyDto {
                id,
                name,
                party_type,
                description: None,
            };

            match role.as_str() {
                "plaintiff" => plaintiffs.push(party),
                "defendant" => defendants.push(party),
                _ => other.push(party),
            }
        }

        Ok(PartiesGroup {
            plaintiffs,
            defendants,
            other,
        })
    }

    /// Fetch aggregated statistics across node types (v2 pipeline).
    /// Evidence nodes don't exist in v2 — evidence_count returns 0.
    /// Harm.amount is a string like "$25,000.00" — parsed in Cypher.
    /// ComplaintAllegation uses grounding_status instead of evidence_status.
    async fn get_stats(&self) -> Result<CaseStats, CaseRepositoryError> {
        let mut result = self
            .graph
            .execute(query(
                "OPTIONAL MATCH (a:ComplaintAllegation)
                 WITH count(a) AS allegations_total,
                      count(CASE WHEN a.grounding_status IN ['exact', 'normalized'] THEN 1 END) AS allegations_proven
                 OPTIONAL MATCH (d:Document)
                 WITH allegations_total, allegations_proven, count(d) AS document_count
                 OPTIONAL MATCH (h:Harm)
                 WITH allegations_total, allegations_proven, document_count,
                      SUM(CASE WHEN h.amount IS NOT NULL
                          THEN toFloat(replace(replace(h.amount, '$', ''), ',', ''))
                          ELSE 0 END) AS damages_total
                 OPTIONAL MATCH (l:LegalCount)
                 RETURN allegations_total, allegations_proven,
                        document_count, damages_total, count(l) AS legal_counts",
            ))
            .await?;

        if let Some(row) = result.next().await? {
            Ok(CaseStats {
                allegations_total: row.get("allegations_total").unwrap_or(0),
                allegations_proven: row.get("allegations_proven").unwrap_or(0),
                evidence_count: 0, // Evidence nodes don't exist in v2
                document_count: row.get("document_count").unwrap_or(0),
                damages_total: row.get("damages_total").unwrap_or(0.0),
                legal_counts: row.get("legal_counts").unwrap_or(0),
                legal_count_details: Vec::new(), // Populated separately
            })
        } else {
            Ok(CaseStats::default())
        }
    }

    /// Fetch legal count details (id and name)
    async fn get_legal_count_details(&self) -> Result<Vec<LegalCountSummary>, CaseRepositoryError> {
        let mut details: Vec<LegalCountSummary> = Vec::new();

        let mut result = self
            .graph
            .execute(query(
                "MATCH (lc:LegalCount)
                 RETURN lc.id AS id, lc.title AS name,
                        lc.count_number AS count_number
                 ORDER BY lc.count_number",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            let id: String = row.get("id").unwrap_or_default();
            let name: String = row.get("name").unwrap_or_default();
            let count_number: i64 = row.get("count_number").unwrap_or(0);
            details.push(LegalCountSummary { id, name, count_number });
        }

        Ok(details)
    }
}
