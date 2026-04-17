// Neo4j queries for GET /case-summary — analytical dashboard data.
// Uses separate query methods (same pattern as case_repository.rs).

use neo4rs::{query, Graph};

use crate::dto::case_summary::{CaseSummaryResponse, LegalCountInfo, PersonCharacterizationCount};

#[derive(Debug)]
pub enum CaseSummaryRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
    GraphAccess(colossus_graph::GraphAccessError),
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

impl From<colossus_graph::GraphAccessError> for CaseSummaryRepositoryError {
    fn from(value: colossus_graph::GraphAccessError) -> Self {
        CaseSummaryRepositoryError::GraphAccess(value)
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
    ///
    /// Uses colossus_graph::get_nodes_by_label to fetch Document nodes,
    /// then filters in Rust for the complaint document.
    async fn get_case_identity(
        &self,
    ) -> Result<(String, Option<String>, Option<String>), CaseSummaryRepositoryError> {
        let docs = colossus_graph::get_nodes_by_label(&self.graph, "Document").await?;

        let complaint = docs.into_iter().find(|d| {
            d.properties
                .get("doc_type")
                .and_then(|v| v.as_str())
                .map(|s| s.contains("complaint"))
                .unwrap_or(false)
        });

        if let Some(doc) = complaint {
            let title = doc
                .properties
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            Ok((title, None, None))
        } else {
            Ok(("Unknown Case".to_string(), None, None))
        }
    }

    // ── Query 2: Core stats (allegations, evidence, documents, harms) ────

    /// Core stats (v2 pipeline).
    /// Evidence nodes don't exist — evidence_total/evidence_grounded return 0.
    /// Harm.category doesn't exist in v2 — damages_financial and
    /// damages_reputational_count return 0.
    ///
    /// ## Rust Learning: Aggregation in Rust vs Cypher
    ///
    /// Uses colossus_graph::get_label_counts for simple entity counts and
    /// colossus_graph::get_nodes_by_label to fetch nodes for Rust-side
    /// aggregation (damages parsing, grounding_status filtering).
    async fn get_core_stats(&self) -> Result<CoreStats, CaseSummaryRepositoryError> {
        // Get entity type counts from the graph.
        let label_counts = colossus_graph::get_label_counts(&self.graph).await?;
        let count_for = |label: &str| -> i64 {
            label_counts
                .iter()
                .find(|lc| lc.label == label)
                .map(|lc| lc.count)
                .unwrap_or(0)
        };

        let documents_total = count_for("Document");
        let legal_counts = count_for("LegalCount");

        // Fetch Harm nodes and compute damages total in Rust.
        // Harm.amount is a string like "$25,000.00" — strip currency formatting.
        let harms = colossus_graph::get_nodes_by_label(&self.graph, "Harm").await?;
        let harms_total = harms.len() as i64;
        let damages_total: f64 = harms
            .iter()
            .filter_map(|h| h.properties.get("amount"))
            .filter_map(|v| v.as_str())
            .filter_map(|s| s.replace(['$', ','], "").parse::<f64>().ok())
            .sum();

        // Fetch ComplaintAllegation nodes and count proven in Rust.
        let allegations =
            colossus_graph::get_nodes_by_label(&self.graph, "ComplaintAllegation").await?;
        let allegations_total = allegations.len() as i64;
        let allegations_proven = allegations
            .iter()
            .filter(|a| {
                a.properties
                    .get("grounding_status")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "exact" || s == "normalized")
                    .unwrap_or(false)
            })
            .count() as i64;

        Ok(CoreStats {
            allegations_total,
            allegations_proven,
            evidence_total: 0,    // Evidence nodes don't exist in v2
            evidence_grounded: 0, // Evidence nodes don't exist in v2
            documents_total,
            harms_total,
            damages_total,
            damages_financial: 0.0,        // h.category doesn't exist in v2
            damages_reputational_count: 0, // h.category doesn't exist in v2
            legal_counts,
        })
    }

    // ── Query 3: Legal count details (id, name, allegation count) ───────
    // TODO: DAL Phase 2 — use colossus_graph once batch neighbor counting is available.
    // Kept as raw Cypher because the SUPPORTS aggregation (count allegations per
    // legal count) would require N+1 get_node_neighbors calls.

    async fn get_legal_count_details(
        &self,
    ) -> Result<Vec<LegalCountInfo>, CaseSummaryRepositoryError> {
        let mut details: Vec<LegalCountInfo> = Vec::new();
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (lc) WHERE labels(lc)[0] = $count_label
                     OPTIONAL MATCH (a)-[:SUPPORTS]->(lc)
                       WHERE labels(a)[0] = $allegation_label
                     RETURN lc.id AS id, lc.title AS name,
                            lc.count_number AS count_number,
                            count(a) AS allegation_count
                     ORDER BY lc.count_number",
                )
                .param("count_label", "LegalCount")
                .param("allegation_label", "ComplaintAllegation"),
            )
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
    async fn get_decomposition_stats(&self) -> Result<DecompStats, CaseSummaryRepositoryError> {
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
    ///
    /// Uses colossus_graph::get_nodes_with_property to fetch all nodes that
    /// have a `role` property, then groups by role in Rust.
    async fn get_parties(&self) -> Result<(Vec<String>, Vec<String>), CaseSummaryRepositoryError> {
        let mut plaintiffs: Vec<String> = Vec::new();
        let mut defendants: Vec<String> = Vec::new();

        let nodes = colossus_graph::get_nodes_with_property(&self.graph, "role").await?;

        for node in &nodes {
            let name = node
                .properties
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let role = node
                .properties
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            match role {
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
