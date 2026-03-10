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
            case: case_info,
            parties,
            stats,
        }))
    }

    /// Fetch Case node properties
    async fn get_case_info(&self) -> Result<Option<CaseInfo>, CaseRepositoryError> {
        let mut result = self
            .graph
            .execute(query(
                "MATCH (c:Case)
                 RETURN c.id AS id, c.title AS title, c.case_number AS case_number,
                        c.court AS court, c.court_type AS court_type,
                        c.filing_date AS filing_date, c.status AS status,
                        c.summary AS summary
                 LIMIT 1",
            ))
            .await?;

        if let Some(row) = result.next().await? {
            let id: String = row.get("id").unwrap_or_default();
            let title: String = row.get("title").unwrap_or_default();
            let case_number: Option<String> = row.get("case_number").ok();
            let court: Option<String> = row.get("court").ok();
            let court_type: Option<String> = row.get("court_type").ok();
            let filing_date: Option<String> = row.get("filing_date").ok();
            let status: Option<String> = row.get("status").ok();
            let summary: Option<String> = row.get("summary").ok();

            Ok(Some(CaseInfo {
                id,
                title,
                case_number,
                court,
                court_type,
                filing_date,
                status,
                summary,
            }))
        } else {
            Ok(None)
        }
    }

    /// Fetch parties via INVOLVES relationships, grouped by role
    async fn get_parties(&self) -> Result<PartiesGroup, CaseRepositoryError> {
        let mut plaintiffs: Vec<PartyDto> = Vec::new();
        let mut defendants: Vec<PartyDto> = Vec::new();
        let mut other: Vec<PartyDto> = Vec::new();

        // Query parties - uses labels() to detect Person vs Organization
        let mut result = self
            .graph
            .execute(query(
                "MATCH (c:Case)-[r:INVOLVES]->(party)
                 WHERE party:Person OR party:Organization
                 RETURN party.id AS id,
                        party.name AS name,
                        party.description AS description,
                        r.role AS role,
                        labels(party) AS labels
                 ORDER BY r.role, party.name",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            let id: String = row.get("id").unwrap_or_default();
            let name: String = row.get("name").unwrap_or_default();
            let description: Option<String> = row.get("description").ok();
            let role: String = row.get("role").unwrap_or_default();
            let labels: Vec<String> = row.get("labels").unwrap_or_default();

            // Determine party type from node labels
            let party_type = if labels.contains(&"Organization".to_string()) {
                "organization".to_string()
            } else {
                "person".to_string()
            };

            let party = PartyDto {
                id,
                name,
                party_type,
                description,
            };

            // Group by role
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

    /// Fetch aggregated statistics across node types
    async fn get_stats(&self) -> Result<CaseStats, CaseRepositoryError> {
        // Use separate OPTIONAL MATCH clauses for each node type to handle
        // cases where some node types may not exist
        let mut result = self
            .graph
            .execute(query(
                "OPTIONAL MATCH (a:ComplaintAllegation)
                 WITH count(a) AS allegations_total,
                      sum(CASE WHEN a.evidence_status = 'PROVEN' THEN 1 ELSE 0 END) AS allegations_proven
                 OPTIONAL MATCH (e:Evidence)
                 WITH allegations_total, allegations_proven, count(e) AS evidence_count
                 OPTIONAL MATCH (d:Document)
                 WITH allegations_total, allegations_proven, evidence_count, count(d) AS document_count
                 OPTIONAL MATCH (h:Harm)
                 WITH allegations_total, allegations_proven, evidence_count, document_count,
                      sum(COALESCE(h.amount, 0)) AS damages_total
                 OPTIONAL MATCH (l:LegalCount)
                 RETURN allegations_total, allegations_proven, evidence_count,
                        document_count, damages_total, count(l) AS legal_counts",
            ))
            .await?;

        if let Some(row) = result.next().await? {
            Ok(CaseStats {
                allegations_total: row.get("allegations_total").unwrap_or(0),
                allegations_proven: row.get("allegations_proven").unwrap_or(0),
                evidence_count: row.get("evidence_count").unwrap_or(0),
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
