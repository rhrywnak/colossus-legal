use neo4rs::{query, Graph};
use std::collections::{HashMap, HashSet};

use crate::dto::{GraphEdge, GraphNode, GraphNodeType, GraphResponse};

#[derive(Clone)]
pub struct GraphRepository {
    graph: Graph,
}

#[derive(Debug)]
pub enum GraphRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
}

impl From<neo4rs::Error> for GraphRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        GraphRepositoryError::Neo4j(value)
    }
}

impl From<neo4rs::DeError> for GraphRepositoryError {
    fn from(value: neo4rs::DeError) -> Self {
        GraphRepositoryError::Value(value)
    }
}

impl GraphRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch the legal proof chain graph from Neo4j
    ///
    /// Returns nodes and edges for: LegalCount <- Allegation <- MotionClaim <- Evidence <- Document
    pub async fn get_legal_proof_graph(
        &self,
        count_id: Option<&str>,
    ) -> Result<GraphResponse, GraphRepositoryError> {
        let mut nodes_map: HashMap<String, GraphNode> = HashMap::new();
        let mut edges_set: HashSet<GraphEdge> = HashSet::new();

        // Use different queries based on whether we're filtering by count_id
        let cypher = if count_id.is_some() {
            "MATCH (c:LegalCount {id: $count_id})
             OPTIONAL MATCH (a:ComplaintAllegation)-[:SUPPORTS]->(c)
             OPTIONAL MATCH (m:MotionClaim)-[:PROVES]->(a)
             OPTIONAL MATCH (m)-[:RELIES_ON]->(e:Evidence)
             OPTIONAL MATCH (e)-[:CONTAINED_IN]->(d:Document)
             RETURN c.id AS c_id, c.title AS c_title,
                    a.id AS a_id, a.title AS a_title, a.evidence_status AS a_status,
                    m.id AS m_id, m.title AS m_title,
                    e.id AS e_id, e.title AS e_title,
                    d.id AS d_id, d.title AS d_title"
        } else {
            "MATCH (c:LegalCount)
             OPTIONAL MATCH (a:ComplaintAllegation)-[:SUPPORTS]->(c)
             OPTIONAL MATCH (m:MotionClaim)-[:PROVES]->(a)
             OPTIONAL MATCH (m)-[:RELIES_ON]->(e:Evidence)
             OPTIONAL MATCH (e)-[:CONTAINED_IN]->(d:Document)
             RETURN c.id AS c_id, c.title AS c_title,
                    a.id AS a_id, a.title AS a_title, a.evidence_status AS a_status,
                    m.id AS m_id, m.title AS m_title,
                    e.id AS e_id, e.title AS e_title,
                    d.id AS d_id, d.title AS d_title"
        };

        let q = if let Some(cid) = count_id {
            query(cypher).param("count_id", cid)
        } else {
            query(cypher)
        };

        let mut result = self.graph.execute(q).await?;

        while let Some(row) = result.next().await? {
            // Extract LegalCount
            if let Ok(c_id) = row.get::<String>("c_id") {
                let c_title: String = row.get("c_title").unwrap_or_default();
                nodes_map.entry(c_id.clone()).or_insert(GraphNode {
                    id: c_id,
                    label: c_title,
                    node_type: GraphNodeType::LegalCount,
                    subtitle: None,
                    details: None,
                });
            }

            // Extract Allegation and edge to LegalCount
            if let Ok(a_id) = row.get::<String>("a_id") {
                let a_title: String = row.get("a_title").unwrap_or_default();
                let a_status: Option<String> = row.get("a_status").ok();
                let c_id: String = row.get("c_id").unwrap_or_default();

                nodes_map.entry(a_id.clone()).or_insert(GraphNode {
                    id: a_id.clone(),
                    label: a_title,
                    node_type: GraphNodeType::Allegation,
                    subtitle: a_status,
                    details: None,
                });

                if !c_id.is_empty() {
                    edges_set.insert(GraphEdge {
                        source: a_id,
                        target: c_id,
                        relationship: "SUPPORTS".to_string(),
                    });
                }
            }

            // Extract MotionClaim and edge to Allegation
            if let Ok(m_id) = row.get::<String>("m_id") {
                let m_title: String = row.get("m_title").unwrap_or_default();
                let a_id: String = row.get("a_id").unwrap_or_default();

                nodes_map.entry(m_id.clone()).or_insert(GraphNode {
                    id: m_id.clone(),
                    label: m_title,
                    node_type: GraphNodeType::MotionClaim,
                    subtitle: None,
                    details: None,
                });

                if !a_id.is_empty() {
                    edges_set.insert(GraphEdge {
                        source: m_id,
                        target: a_id,
                        relationship: "PROVES".to_string(),
                    });
                }
            }

            // Extract Evidence and edge to MotionClaim
            if let Ok(e_id) = row.get::<String>("e_id") {
                let e_title: String = row.get("e_title").unwrap_or_default();
                let m_id: String = row.get("m_id").unwrap_or_default();

                nodes_map.entry(e_id.clone()).or_insert(GraphNode {
                    id: e_id.clone(),
                    label: e_title,
                    node_type: GraphNodeType::Evidence,
                    subtitle: None,
                    details: None,
                });

                if !m_id.is_empty() {
                    edges_set.insert(GraphEdge {
                        source: m_id.clone(),
                        target: e_id,
                        relationship: "RELIES_ON".to_string(),
                    });
                }
            }

            // Extract Document and edge to Evidence
            if let Ok(d_id) = row.get::<String>("d_id") {
                let d_title: String = row.get("d_title").unwrap_or_default();
                let e_id: String = row.get("e_id").unwrap_or_default();

                nodes_map.entry(d_id.clone()).or_insert(GraphNode {
                    id: d_id.clone(),
                    label: d_title,
                    node_type: GraphNodeType::Document,
                    subtitle: None,
                    details: None,
                });

                if !e_id.is_empty() {
                    edges_set.insert(GraphEdge {
                        source: e_id,
                        target: d_id,
                        relationship: "CONTAINED_IN".to_string(),
                    });
                }
            }
        }

        Ok(GraphResponse {
            nodes: nodes_map.into_values().collect(),
            edges: edges_set.into_iter().collect(),
            title: "Legal Proof Chain".to_string(),
            hierarchy_type: "legal_proof".to_string(),
        })
    }
}
