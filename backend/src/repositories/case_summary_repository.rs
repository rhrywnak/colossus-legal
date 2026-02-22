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

    async fn get_case_identity(
        &self,
    ) -> Result<(String, Option<String>, Option<String>), CaseSummaryRepositoryError> {
        let mut result = self
            .graph
            .execute(query(
                "MATCH (c:Case)
                 RETURN c.title AS title, c.court AS court,
                        c.case_number AS case_number
                 LIMIT 1",
            ))
            .await?;

        if let Some(row) = result.next().await? {
            let title: String = row.get("title").unwrap_or_default();
            let court: Option<String> = row.get("court").ok();
            let case_number: Option<String> = row.get("case_number").ok();
            Ok((title, court, case_number))
        } else {
            Ok(("Unknown Case".to_string(), None, None))
        }
    }

    // ── Query 2: Core stats (allegations, evidence, documents, harms) ────

    async fn get_core_stats(&self) -> Result<CoreStats, CaseSummaryRepositoryError> {
        let mut result = self
            .graph
            .execute(query(
                "OPTIONAL MATCH (a:ComplaintAllegation)
                 WITH count(a) AS at,
                      sum(CASE WHEN a.evidence_status = 'PROVEN' THEN 1 ELSE 0 END) AS ap
                 OPTIONAL MATCH (e:Evidence)
                 WITH at, ap, count(e) AS et
                 OPTIONAL MATCH (eg:Evidence) WHERE eg.verbatim_quote IS NOT NULL
                 WITH at, ap, et, count(eg) AS egr
                 OPTIONAL MATCH (d:Document)
                 WITH at, ap, et, egr, count(d) AS dt
                 OPTIONAL MATCH (h:Harm)
                 WITH at, ap, et, egr, dt,
                      count(h) AS ht,
                      sum(COALESCE(h.amount, 0)) AS dam_total,
                      sum(CASE WHEN h.category STARTS WITH 'financial'
                          THEN COALESCE(h.amount, 0) ELSE 0 END) AS dam_fin,
                      sum(CASE WHEN h.category = 'reputational'
                          THEN 1 ELSE 0 END) AS dam_rep
                 OPTIONAL MATCH (l:LegalCount)
                 RETURN at, ap, et, egr, dt, ht, dam_total, dam_fin, dam_rep,
                        count(l) AS lc",
            ))
            .await?;

        if let Some(row) = result.next().await? {
            Ok(CoreStats {
                allegations_total: row.get("at").unwrap_or(0),
                allegations_proven: row.get("ap").unwrap_or(0),
                evidence_total: row.get("et").unwrap_or(0),
                evidence_grounded: row.get("egr").unwrap_or(0),
                documents_total: row.get("dt").unwrap_or(0),
                harms_total: row.get("ht").unwrap_or(0),
                damages_total: row.get("dam_total").unwrap_or(0.0),
                damages_financial: row.get("dam_fin").unwrap_or(0.0),
                damages_reputational_count: row.get("dam_rep").unwrap_or(0),
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
                 RETURN lc.id AS id, lc.title AS name, count(a) AS allegation_count
                 ORDER BY lc.title",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            details.push(LegalCountInfo {
                id: row.get("id").unwrap_or_default(),
                name: row.get("name").unwrap_or_default(),
                allegation_count: row.get("allegation_count").unwrap_or(0),
            });
        }

        Ok(details)
    }

    // ── Query 4: Decomposition stats (characterizations + rebuttals) ─────

    async fn get_decomposition_stats(
        &self,
    ) -> Result<DecompStats, CaseSummaryRepositoryError> {
        // 4a: Characterizations by person + unique labels
        let mut by_person: Vec<PersonCharacterizationCount> = Vec::new();
        let mut all_labels: Vec<String> = Vec::new();
        let mut characterizations_total: i64 = 0;

        let mut result = self
            .graph
            .execute(query(
                "MATCH (e:Evidence)-[c:CHARACTERIZES]->(a:ComplaintAllegation)
                 MATCH (e)-[:STATED_BY]->(p:Person)
                 WITH p.name AS person,
                      count(c) AS char_count,
                      collect(DISTINCT c.characterization) AS labels
                 RETURN person, char_count, labels
                 ORDER BY char_count DESC",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            let person: String = row.get("person").unwrap_or_default();
            let count: i64 = row.get("char_count").unwrap_or(0);
            let labels: Vec<String> = row
                .get::<Vec<Option<String>>>("labels")
                .unwrap_or_default()
                .into_iter()
                .flatten()
                .collect();

            characterizations_total += count;
            all_labels.extend(labels);
            by_person.push(PersonCharacterizationCount { person, count });
        }

        // Deduplicate labels across all persons
        all_labels.sort();
        all_labels.dedup();

        // 4b: Rebuttals count
        let mut reb_result = self
            .graph
            .execute(query("MATCH ()-[r:REBUTS]->() RETURN count(r) AS total"))
            .await?;

        let rebuttals_total = if let Some(row) = reb_result.next().await? {
            row.get("total").unwrap_or(0)
        } else {
            0
        };

        Ok(DecompStats {
            characterizations_total,
            by_person,
            rebuttals_total,
            labels: all_labels,
        })
    }

    // ── Query 5: Parties (plaintiff/defendant names) ─────────────────────

    async fn get_parties(
        &self,
    ) -> Result<(Vec<String>, Vec<String>), CaseSummaryRepositoryError> {
        let mut plaintiffs: Vec<String> = Vec::new();
        let mut defendants: Vec<String> = Vec::new();

        let mut result = self
            .graph
            .execute(query(
                "MATCH (c:Case)-[r:INVOLVES]->(party)
                 WHERE party:Person OR party:Organization
                 RETURN party.name AS name, r.role AS role
                 ORDER BY r.role, party.name",
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
