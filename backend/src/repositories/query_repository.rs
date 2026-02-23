// Pre-approved query registry + generic Cypher executor.
// Only registered queries run — never arbitrary user-supplied Cypher.

use neo4rs::{query, Graph};
use std::collections::HashMap;

use crate::dto::query::{
    QueryCategory, QueryInfo, QueryListResponse, QueryResultResponse,
};

#[derive(Debug)]
pub enum QueryRepositoryError {
    Neo4j(neo4rs::Error),
    NotFound(String),
}

impl From<neo4rs::Error> for QueryRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        QueryRepositoryError::Neo4j(value)
    }
}

struct QueryDefinition {
    id: &'static str,
    title: &'static str,
    description: &'static str,
    category: &'static str,
    cypher: &'static str,
    columns: &'static [&'static str],
}

fn get_all_definitions() -> Vec<QueryDefinition> {
    vec![
        QueryDefinition {
            id: "phillips-admissions",
            title: "Phillips Admissions Against Interest",
            description: "All statements from George Phillips' discovery responses",
            category: "Defendant Analysis",
            cypher: "MATCH (e:Evidence)
                     WHERE e.id STARTS WITH 'evidence-phillips'
                     RETURN e.exhibit_number AS exhibit, e.title AS title,
                            e.question AS question, e.answer AS admission,
                            e.significance AS significance
                     ORDER BY e.exhibit_number",
            columns: &["exhibit", "title", "question", "admission", "significance"],
        },
        QueryDefinition {
            id: "cfs-admissions",
            title: "CFS Admissions Against Interest",
            description: "All statements from Catholic Family Service discovery",
            category: "Defendant Analysis",
            cypher: "MATCH (e:Evidence)
                     WHERE e.id STARTS WITH 'evidence-cfs'
                     RETURN e.exhibit_number AS exhibit, e.title AS title,
                            e.question AS question, e.answer AS admission,
                            e.significance AS significance
                     ORDER BY e.exhibit_number",
            columns: &["exhibit", "title", "question", "admission", "significance"],
        },
        QueryDefinition {
            id: "phillips-position-changes",
            title: "Phillips Position Changes Over Time",
            description: "Phillips statements about the $50,000 conversion and gifts",
            category: "Defendant Analysis",
            cypher: "MATCH (e:Evidence)
                     WHERE e.id STARTS WITH 'evidence-phillips'
                       AND (e.answer CONTAINS '50,000'
                            OR e.answer CONTAINS '$50'
                            OR e.answer CONTAINS 'gift'
                            OR e.answer CONTAINS 'video'
                            OR e.significance CONTAINS '50')
                     RETURN e.exhibit_number AS exhibit, e.question AS question,
                            e.answer AS answer, e.significance AS significance",
            columns: &["exhibit", "question", "answer", "significance"],
        },
        QueryDefinition {
            id: "selective-enforcement",
            title: "Selective Enforcement Pattern",
            description: "Evidence that Marie was treated differently than her sisters",
            category: "Court Presentation",
            cypher: "MATCH (e:Evidence)
                     WHERE e.significance CONTAINS 'selective'
                        OR e.significance CONTAINS 'only Marie'
                        OR e.significance CONTAINS 'sisters'
                        OR e.significance CONTAINS 'Nadia'
                        OR e.significance CONTAINS 'Camille'
                        OR e.answer CONTAINS 'Not that I recall'
                     RETURN e.exhibit_number AS exhibit, e.title AS title,
                            e.answer AS answer, e.significance AS significance",
            columns: &["exhibit", "title", "answer", "significance"],
        },
        QueryDefinition {
            id: "disparagement-pattern",
            title: "Pattern of Disparagement",
            description: "Systematic character attacks on Marie",
            category: "Court Presentation",
            cypher: "MATCH (e:Evidence)
                     WHERE e.answer CONTAINS 'unintelligible'
                        OR e.answer CONTAINS 'conspiracy'
                        OR e.answer CONTAINS 'North Korea'
                        OR e.answer CONTAINS 'roadblock'
                        OR e.answer CONTAINS 'assault'
                        OR e.significance CONTAINS 'disparag'
                     RETURN e.exhibit_number AS exhibit, e.title AS title,
                            e.answer AS answer, e.significance AS significance",
            columns: &["exhibit", "title", "answer", "significance"],
        },
        QueryDefinition {
            id: "conflict-of-interest",
            title: "Conflict of Interest Evidence",
            description: "CFS-Court financial relationship proof",
            category: "Court Presentation",
            cypher: "MATCH (e:Evidence)
                     WHERE e.significance CONTAINS 'conflict'
                        OR e.significance CONTAINS 'contract'
                        OR e.significance CONTAINS 'revenue'
                        OR e.answer CONTAINS 'contract'
                        OR e.answer CONTAINS 'excess'
                     RETURN e.exhibit_number AS exhibit, e.title AS title,
                            e.answer AS answer, e.significance AS significance",
            columns: &["exhibit", "title", "answer", "significance"],
        },
        QueryDefinition {
            id: "strongest-evidence",
            title: "Strongest Evidence (by Weight)",
            description: "Most impactful evidence for trial preparation",
            category: "Evidence Analysis",
            cypher: "MATCH (e:Evidence)
                     WHERE e.weight IS NOT NULL
                     RETURN e.exhibit_number AS exhibit, e.title AS title,
                            e.weight AS weight,
                            substring(e.answer, 0, 100) AS answer_preview
                     ORDER BY e.weight DESC
                     LIMIT 15",
            columns: &["exhibit", "title", "weight", "answer_preview"],
        },
        QueryDefinition {
            id: "multi-allegation-evidence",
            title: "Evidence Supporting Multiple Allegations",
            description: "High-value evidence that proves multiple claims",
            category: "Evidence Analysis",
            cypher: "MATCH (e:Evidence)<-[:RELIES_ON]-(m:MotionClaim)
                           -[:PROVES]->(a:ComplaintAllegation)
                     WITH e, count(DISTINCT a) AS allegation_count,
                          collect(DISTINCT a.id) AS allegations
                     WHERE allegation_count > 1
                     RETURN e.id AS id, e.title AS title,
                            allegation_count, allegations
                     ORDER BY allegation_count DESC",
            columns: &["id", "title", "allegation_count", "allegations"],
        },
        QueryDefinition {
            id: "damages-by-category",
            title: "Damages Breakdown by Category",
            description: "Financial vs. reputational damages with totals",
            category: "Damages",
            cypher: "MATCH (h:Harm)
                     RETURN h.category AS category, count(*) AS count,
                            sum(h.amount) AS total, collect(h.title) AS harms
                     ORDER BY total DESC",
            columns: &["category", "count", "total", "harms"],
        },
        QueryDefinition {
            id: "damages-by-count",
            title: "Damages by Legal Count",
            description: "Which harms support which legal counts",
            category: "Damages",
            cypher: "MATCH (h:Harm)-[:DAMAGES_FOR]->(c:LegalCount)
                     RETURN c.title AS legal_count,
                            collect(h.title) AS supporting_harms,
                            sum(h.amount) AS total_damages
                     ORDER BY total_damages DESC",
            columns: &["legal_count", "supporting_harms", "total_damages"],
        },
    ]
}

fn category_description(name: &str) -> &'static str {
    match name {
        "Defendant Analysis" => "What defendants said vs. what was proven",
        "Court Presentation" => "Evidence patterns for trial and motions",
        "Evidence Analysis" => "Strength and coverage of the evidence base",
        "Damages" => "Financial and reputational harm breakdowns",
        _ => "",
    }
}

#[derive(Clone)]
pub struct QueryRepository {
    graph: Graph,
}

impl QueryRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Return the catalogue of available queries, grouped by category.
    pub fn list_queries(&self) -> QueryListResponse {
        let defs = get_all_definitions();
        let mut cat_map: Vec<(String, Vec<QueryInfo>)> = Vec::new();

        for def in &defs {
            let info = QueryInfo {
                id: def.id.to_string(),
                title: def.title.to_string(),
                description: def.description.to_string(),
                category: def.category.to_string(),
            };
            if let Some(entry) = cat_map.iter_mut().find(|(n, _)| n == def.category) {
                entry.1.push(info);
            } else {
                cat_map.push((def.category.to_string(), vec![info]));
            }
        }

        let categories = cat_map
            .into_iter()
            .map(|(name, queries)| QueryCategory {
                description: category_description(&name).to_string(),
                name,
                queries,
            })
            .collect();

        QueryListResponse { categories }
    }

    /// Execute a pre-registered query by id and return generic tabular results.
    pub async fn run_query(
        &self,
        query_id: &str,
    ) -> Result<QueryResultResponse, QueryRepositoryError> {
        let defs = get_all_definitions();
        let def = defs
            .iter()
            .find(|d| d.id == query_id)
            .ok_or_else(|| QueryRepositoryError::NotFound(query_id.to_string()))?;

        let mut result = self.graph.execute(query(def.cypher)).await?;
        let columns: Vec<String> = def.columns.iter().map(|c| c.to_string()).collect();
        let mut rows: Vec<HashMap<String, serde_json::Value>> = Vec::new();

        while let Some(row) = result.next().await? {
            let mut map = HashMap::new();
            for col in &columns {
                map.insert(col.clone(), extract_value(&row, col));
            }
            rows.push(map);
        }

        let row_count = rows.len();
        Ok(QueryResultResponse {
            query_id: def.id.to_string(),
            title: def.title.to_string(),
            description: def.description.to_string(),
            columns,
            rows,
            row_count,
        })
    }
}

// Type cascade: String → i64 → f64 → Vec<Option<String>> → Null.
fn extract_value(row: &neo4rs::Row, col: &str) -> serde_json::Value {
    // 1. String (most common)
    if let Ok(s) = row.get::<String>(col) {
        return serde_json::Value::String(s);
    }
    // 2. Integer (count, allegation_count)
    if let Ok(n) = row.get::<i64>(col) {
        return serde_json::json!(n);
    }
    // 3. Float (sum of amounts)
    if let Ok(f) = row.get::<f64>(col) {
        return serde_json::json!(f);
    }
    // 4. List of strings (collect() results)
    if let Ok(list) = row.get::<Vec<Option<String>>>(col) {
        let strings: Vec<serde_json::Value> = list
            .into_iter()
            .flatten()
            .map(serde_json::Value::String)
            .collect();
        return serde_json::Value::Array(strings);
    }
    // 5. Null / unsupported
    serde_json::Value::Null
}
