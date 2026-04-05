use neo4rs::Graph;
use std::collections::HashMap;

use crate::dto::{HarmDto, HarmsResponse};

#[derive(Clone)]
pub struct HarmRepository {
    graph: Graph,
}

#[derive(Debug)]
pub enum HarmRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
    GraphAccess(colossus_graph::GraphAccessError),
}

impl From<neo4rs::Error> for HarmRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        HarmRepositoryError::Neo4j(value)
    }
}

impl From<neo4rs::DeError> for HarmRepositoryError {
    fn from(value: neo4rs::DeError) -> Self {
        HarmRepositoryError::Value(value)
    }
}

impl From<colossus_graph::GraphAccessError> for HarmRepositoryError {
    fn from(value: colossus_graph::GraphAccessError) -> Self {
        HarmRepositoryError::GraphAccess(value)
    }
}

impl HarmRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch all harms from Neo4j.
    ///
    /// ## Rust Learning: Skipping empty relationship joins
    ///
    /// The v1 query joined Harm nodes to ComplaintAllegation (via CAUSED_BY)
    /// and LegalCount (via DAMAGES_FOR). Neither relationship type exists in v2,
    /// so those OPTIONAL MATCHes always returned nulls. By switching to
    /// `get_nodes_by_label("Harm")`, we skip the useless joins entirely.
    /// The DTO fields `caused_by_allegations` and `damages_for_counts` return
    /// empty Vecs — same result, but without wasted Cypher evaluation.
    pub async fn list_harms(&self) -> Result<HarmsResponse, HarmRepositoryError> {
        let nodes = colossus_graph::get_nodes_by_label(&self.graph, "Harm").await?;

        let mut harms: Vec<HarmDto> = Vec::new();
        let mut total_damages: f64 = 0.0;
        let mut by_category: HashMap<String, f64> = HashMap::new();

        for node in &nodes {
            let id = node.id.clone();
            let title = node.properties.get("title")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let category = node.properties.get("category")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let subcategory = node.properties.get("subcategory")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let description = node.properties.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let date = node.properties.get("date")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let source_reference = node.properties.get("source_reference")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Amount can be stored as a number or a currency string like "$25,000.00"
            let amount: Option<f64> = node.properties.get("amount").and_then(|v| {
                v.as_f64().or_else(|| {
                    v.as_str()
                        .and_then(|s| s.replace(['$', ','], "").parse::<f64>().ok())
                })
            });

            // Sum damages
            if let Some(amt) = amount {
                total_damages += amt;
                if let Some(ref cat) = category {
                    *by_category.entry(cat.clone()).or_insert(0.0) += amt;
                }
            }

            // CAUSED_BY and DAMAGES_FOR relationships don't exist in v2 —
            // return empty Vecs for now. Will populate when cross-document
            // relationship types are added.
            harms.push(HarmDto {
                id,
                title,
                category,
                subcategory,
                amount,
                description,
                date,
                source_reference,
                caused_by_allegations: Vec::new(),
                damages_for_counts: Vec::new(),
            });
        }

        let total = harms.len();

        Ok(HarmsResponse {
            harms,
            total,
            total_damages,
            by_category,
        })
    }
}
