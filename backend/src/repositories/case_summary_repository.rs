// Neo4j queries for GET /case-summary — analytical dashboard data.
// Uses separate query methods (same pattern as case_repository.rs).

use neo4rs::{query, Graph};

use crate::dto::case_summary::{CaseSummaryResponse, LegalCountInfo, PersonCharacterizationCount};

#[derive(Debug)]
pub enum CaseSummaryRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
}

impl From<neo4rs::Error> for CaseSummaryRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        CaseSummaryRepositoryError::Neo4j(value)
    }
}

impl From<neo4rs::DeError> for CaseSummaryRepositoryError {
    fn from(value: neo4rs::DeError) -> Self {
        CaseSummaryRepositoryError::Value(value)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Repository
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct CaseSummaryRepository {
    graph: Graph,
}

impl CaseSummaryRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Build the complete case summary by running 5 focused queries.
    ///
    /// RUST PATTERN: Orchestrator Method
    /// Each sub-query is its own method returning a partial result.
    /// This method assembles them into the final response struct.
    pub async fn get_case_summary(
        &self,
    ) -> Result<CaseSummaryResponse, CaseSummaryRepositoryError> {
        let (case_title, court, case_number) = self.get_case_identity().await?;
        let stats = self.get_core_stats().await?;
        let legal_count_details = self.get_legal_count_details().await?;
        let decomp = self.get_decomposition_stats().await?;
        let (plaintiffs, defendants) = self.get_parties().await?;

        Ok(CaseSummaryResponse {
            case_title,
            court,
            case_number,
            allegations_total: stats.allegations_total,
            allegations_proven: stats.allegations_proven,
            legal_counts: stats.legal_counts,
            legal_count_details,
            damages_total: stats.damages_total,
            damages_financial: stats.damages_financial,
            damages_reputational_count: stats.damages_reputational_count,
            harms_total: stats.harms_total,
            characterizations_total: decomp.characterizations_total,
            characterizations_by_person: decomp.by_person,
            rebuttals_total: decomp.rebuttals_total,
            unique_characterization_labels: decomp.labels,
            evidence_total: stats.evidence_total,
            evidence_grounded: stats.evidence_grounded,
            documents_total: stats.documents_total,
            plaintiffs,
            defendants,
        })
    }

    // ── Query 1: Case node identity ──────────────────────────────────────

    /// Case identity from the complaint Document node (v2 pipeline).
    /// Court and case_number don't exist on Document nodes — return None.
    async fn get_case_identity(
        &self,
    ) -> Result<(String, Option<String>, Option<String>), CaseSummaryRepositoryError> {
        let mut result = self
            .graph
            .execute(query(
                "MATCH (d:Document)
                 WHERE d.doc_type CONTAINS 'complaint'
                 RETURN d.title AS title
                 LIMIT 1",
            ))
            .await?;

        if let Some(row) = result.next().await? {
            let title: String = row.get("title").unwrap_or_default();
            Ok((title, None, None))
        } else {
            Ok(("Unknown Case".to_string(), None, None))
        }
    }

    // ── Query 2: Core stats (allegations, evidence, documents, harms) ────

    /// Core stats (v2 pipeline).
    /// Evidence nodes don't exist — evidence_total/evidence_grounded return 0.
    /// Harm.amount is a string like "$25,000.00" — parsed in Cypher.
    /// Harm.category doesn't exist in v2 — damages_financial and
    /// damages_reputational_count return 0.
    /// ComplaintAllegation uses grounding_status instead of evidence_status.
    async fn get_core_stats(&self) -> Result<CoreStats, CaseSummaryRepositoryError> {
        let mut result = self
            .graph
            .execute(query(
                "OPTIONAL MATCH (a:ComplaintAllegation)
                 WITH count(a) AS at,
                      count(CASE WHEN a.grounding_status IN ['exact', 'normalized'] THEN 1 END) AS ap
                 OPTIONAL MATCH (d:Document)
                 WITH at, ap, count(d) AS dt
                 OPTIONAL MATCH (h:Harm)
                 WITH at, ap, dt,
                      count(h) AS ht,
                      SUM(CASE WHEN h.amount IS NOT NULL
                          THEN toFloat(replace(replace(h.amount, '$', ''), ',', ''))
                          ELSE 0 END) AS dam_total
                 OPTIONAL MATCH (l:LegalCount)
                 RETURN at, ap, dt, ht, dam_total, count(l) AS lc",
            ))
            .await?;

        if let Some(row) = result.next().await? {
            Ok(CoreStats {
                allegations_total: row.get("at").unwrap_or(0),
                allegations_proven: row.get("ap").unwrap_or(0),
                evidence_total: 0,     // Evidence nodes don't exist in v2
                evidence_grounded: 0,  // Evidence nodes don't exist in v2
                documents_total: row.get("dt").unwrap_or(0),
                harms_total: row.get("ht").unwrap_or(0),
                damages_total: row.get("dam_total").unwrap_or(0.0),
                damages_financial: 0.0,  // h.category doesn't exist in v2
                damages_reputational_count: 0, // h.category doesn't exist in v2
                legal_counts: row.get("lc").unwrap_or(0),
            })
        } else {
            Ok(CoreStats::default())
        }
    }

    // ── Query 3: Legal count details (id, name, allegation count) ───────

    async fn get_legal_count_details(
        &self,
    ) -> Result<Vec<LegalCountInfo>, CaseSummaryRepositoryError> {
        let mut details: Vec<LegalCountInfo> = Vec::new();
        let mut result = self
            .graph
            .execute(query(
                "MATCH (lc:LegalCount)
                 OPTIONAL MATCH (a:ComplaintAllegation)-[:SUPPORTS]->(lc)
                 RETURN lc.id AS id, lc.title AS name,
                        lc.count_number AS count_number,
                        count(a) AS allegation_count
                 ORDER BY lc.count_number",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            details.push(LegalCountInfo {
                id: row.get("id").unwrap_or_default(),
                name: row.get("name").unwrap_or_default(),
                count_number: row.get("count_number").unwrap_or(0),
                allegation_count: row.get("allegation_count").unwrap_or(0),
            });
        }

        Ok(details)
    }

    // ── Query 4: Decomposition stats (characterizations + rebuttals) ─────

    /// Decomposition stats (v2 pipeline).
    /// CHARACTERIZES and REBUTS relationships don't exist in v2 — return zeros.
    /// These will populate when cross-document analysis is implemented.
    async fn get_decomposition_stats(
        &self,
    ) -> Result<DecompStats, CaseSummaryRepositoryError> {
        // V2 has no CHARACTERIZES or REBUTS relationships yet.
        // Return empty decomposition stats.
        Ok(DecompStats {
            characterizations_total: 0,
            by_person: Vec::new(),
            rebuttals_total: 0,
            labels: Vec::new(),
        })
    }

    // ── Query 5: Parties (plaintiff/defendant names) ─────────────────────

    /// Parties by role property on Person/Organization nodes (v2 pipeline).
    /// V2 stores role directly on the node, not on an INVOLVES relationship.
    async fn get_parties(
        &self,
    ) -> Result<(Vec<String>, Vec<String>), CaseSummaryRepositoryError> {
        let mut plaintiffs: Vec<String> = Vec::new();
        let mut defendants: Vec<String> = Vec::new();

        let mut result = self
            .graph
            .execute(query(
                "MATCH (n)
                 WHERE n.role IS NOT NULL
                 RETURN n.name AS name, n.role AS role
                 ORDER BY n.role, n.name",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            let name: String = row.get("name").unwrap_or_default();
            let role: String = row.get("role").unwrap_or_default();
            match role.as_str() {
                "plaintiff" => plaintiffs.push(name),
                "defendant" => defendants.push(name),
                _ => {}
            }
        }

        Ok((plaintiffs, defendants))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helper structs — not part of the public API, just used to pass
// data between private query methods and the orchestrator.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct CoreStats {
    allegations_total: i64,
    allegations_proven: i64,
    evidence_total: i64,
    evidence_grounded: i64,
    documents_total: i64,
    harms_total: i64,
    damages_total: f64,
    damages_financial: f64,
    damages_reputational_count: i64,
    legal_counts: i64,
}

struct DecompStats {
    characterizations_total: i64,
    by_person: Vec<PersonCharacterizationCount>,
    rebuttals_total: i64,
    labels: Vec<String>,
}
