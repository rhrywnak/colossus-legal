use neo4rs::{query, Graph};
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

impl HarmRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch all harms from Neo4j with linked allegations and legal counts
    pub async fn list_harms(&self) -> Result<HarmsResponse, HarmRepositoryError> {
        let mut harms: Vec<HarmDto> = Vec::new();
        let mut total_damages: f64 = 0.0;
        let mut by_category: HashMap<String, f64> = HashMap::new();

        let mut result = self
            .graph
            .execute(query(
                "MATCH (h:Harm)
                 OPTIONAL MATCH (h)-[:CAUSED_BY]->(a:ComplaintAllegation)
                 OPTIONAL MATCH (h)-[:DAMAGES_FOR]->(c:LegalCount)
                 WITH h,
                      collect(DISTINCT a.id) AS allegation_ids,
                      collect(DISTINCT c.title) AS legal_counts
                 RETURN h.id AS id,
                        h.title AS title,
                        h.category AS category,
                        h.subcategory AS subcategory,
                        h.amount AS amount,
                        h.description AS description,
                        h.date AS date,
                        h.source_reference AS source_reference,
                        allegation_ids,
                        legal_counts
                 ORDER BY h.id",
            ))
            .await?;

        while let Some(row) = result.next().await? {
            let id: String = row.get("id").unwrap_or_default();
            let title: String = row.get("title").unwrap_or_default();
            let category: Option<String> = row.get("category").ok();
            let subcategory: Option<String> = row.get("subcategory").ok();
            let amount: Option<f64> = row.get("amount").ok();
            let description: Option<String> = row.get("description").ok();
            let date: Option<String> = row.get("date").ok();
            let source_reference: Option<String> = row.get("source_reference").ok();

            // Get arrays, filtering out nulls
            let allegation_ids_raw: Vec<Option<String>> =
                row.get("allegation_ids").unwrap_or_default();
            let caused_by_allegations: Vec<String> =
                allegation_ids_raw.into_iter().flatten().collect();

            let legal_counts_raw: Vec<Option<String>> =
                row.get("legal_counts").unwrap_or_default();
            let damages_for_counts: Vec<String> =
                legal_counts_raw.into_iter().flatten().collect();

            // Sum damages
            if let Some(amt) = amount {
                total_damages += amt;
                if let Some(ref cat) = category {
                    *by_category.entry(cat.clone()).or_insert(0.0) += amt;
                }
            }

            harms.push(HarmDto {
                id,
                title,
                category,
                subcategory,
                amount,
                description,
                date,
                source_reference,
                caused_by_allegations,
                damages_for_counts,
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
