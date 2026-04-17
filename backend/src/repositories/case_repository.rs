use neo4rs::Graph;

use crate::dto::{CaseInfo, CaseResponse, CaseStats, LegalCountSummary, PartiesGroup, PartyDto};

#[derive(Clone)]
pub struct CaseRepository {
    graph: Graph,
}

#[derive(Debug)]
pub enum CaseRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
    GraphAccess(colossus_graph::GraphAccessError),
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

impl From<colossus_graph::GraphAccessError> for CaseRepositoryError {
    fn from(value: colossus_graph::GraphAccessError) -> Self {
        CaseRepositoryError::GraphAccess(value)
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
    ///
    /// Uses colossus_graph::get_nodes_by_label to fetch all Document nodes,
    /// then filters in Rust for the complaint document.
    async fn get_case_info(&self) -> Result<Option<CaseInfo>, CaseRepositoryError> {
        let docs = colossus_graph::get_nodes_by_label(&self.graph, "Document").await?;

        let complaint = docs.into_iter().find(|d| {
            d.properties
                .get("doc_type")
                .and_then(|v| v.as_str())
                .map(|s| s.contains("complaint"))
                .unwrap_or(false)
        });

        if let Some(doc) = complaint {
            let id = doc.id;
            let title = doc
                .properties
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let status = doc
                .properties
                .get("status")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

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
    ///
    /// ## Rust Learning: colossus_graph::get_nodes_with_property
    ///
    /// Instead of writing raw Cypher with `WHERE n.role IS NOT NULL`, we use
    /// the schema-agnostic `get_nodes_with_property` function from colossus-graph.
    /// It returns `Vec<GraphNode>` with all properties as serde_json::Value,
    /// which we then map to our domain-specific PartyDto structs.
    async fn get_parties(&self) -> Result<PartiesGroup, CaseRepositoryError> {
        let mut plaintiffs: Vec<PartyDto> = Vec::new();
        let mut defendants: Vec<PartyDto> = Vec::new();
        let mut other: Vec<PartyDto> = Vec::new();

        let nodes = colossus_graph::get_nodes_with_property(&self.graph, "role").await?;

        for node in &nodes {
            let id = node.id.clone();
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
                .unwrap_or_default()
                .to_string();
            let entity_type = node.labels.first().map(|s| s.as_str()).unwrap_or_default();

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
    ///
    /// ## Rust Learning: Aggregation in Rust vs Cypher
    ///
    /// Instead of one complex Cypher query with multiple OPTIONAL MATCH clauses,
    /// we use colossus_graph::get_label_counts for simple counts and
    /// colossus_graph::get_nodes_by_label to fetch nodes for Rust-side aggregation.
    /// This trades one complex query for several simple ones, but decouples us
    /// from label names embedded in Cypher strings.
    async fn get_stats(&self) -> Result<CaseStats, CaseRepositoryError> {
        // Get entity type counts from the graph (label → count).
        let label_counts = colossus_graph::get_label_counts(&self.graph).await?;
        let count_for = |label: &str| -> i64 {
            label_counts
                .iter()
                .find(|lc| lc.label == label)
                .map(|lc| lc.count)
                .unwrap_or(0)
        };

        let document_count = count_for("Document");
        let legal_counts = count_for("LegalCount");

        // Fetch Harm nodes and compute damages total in Rust.
        // Harm.amount is a string like "$25,000.00" — strip currency formatting.
        let harms = colossus_graph::get_nodes_by_label(&self.graph, "Harm").await?;
        let damages_total: f64 = harms
            .iter()
            .filter_map(|h| h.properties.get("amount"))
            .filter_map(|v| v.as_str())
            .filter_map(|s| s.replace(['$', ','], "").parse::<f64>().ok())
            .sum();

        // Fetch ComplaintAllegation nodes and count proven in Rust.
        // grounding_status of "exact" or "normalized" means the allegation is proven.
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

        Ok(CaseStats {
            allegations_total,
            allegations_proven,
            evidence_count: 0, // Evidence nodes don't exist in v2
            document_count,
            damages_total,
            legal_counts,
            legal_count_details: Vec::new(), // Populated separately
        })
    }

    /// Fetch legal count details (id and name).
    /// Uses colossus_graph::get_nodes_by_label to fetch LegalCount nodes,
    /// then maps properties to LegalCountSummary in Rust.
    async fn get_legal_count_details(&self) -> Result<Vec<LegalCountSummary>, CaseRepositoryError> {
        let nodes = colossus_graph::get_nodes_by_label(&self.graph, "LegalCount").await?;

        let mut details: Vec<LegalCountSummary> = nodes
            .iter()
            .map(|node| {
                let id = node.id.clone();
                let name = node
                    .properties
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let count_number = node
                    .properties
                    .get("count_number")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                LegalCountSummary {
                    id,
                    name,
                    count_number,
                }
            })
            .collect();

        // Sort by count_number (colossus_graph doesn't guarantee order)
        details.sort_by_key(|d| d.count_number);

        Ok(details)
    }
}
