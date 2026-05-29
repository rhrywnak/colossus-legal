use neo4rs::{query, Graph};
use std::collections::{HashMap, HashSet};

use crate::dto::{GraphEdge, GraphNode, GraphNodeType, GraphResponse};
use crate::neo4j::schema;

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

        // v5.1 migration:
        //   - `:ComplaintAllegation` → `:Allegation`.
        //   - Direct `:SUPPORTS` edge → two-hop through Element via
        //     `:BEARS_ON` and `:HAS_ELEMENT`.
        //   - Property `a.evidence_status` dropped; returned as `NULL`
        //     (the GraphNode `subtitle` field is `Option<String>` —
        //     null degrades cleanly to "no subtitle").
        //   - `a.title` kept (v5.1 has the property and the frontend
        //     uses it as the node label).
        //
        // `RETURN DISTINCT` dedupes the cartesian fan-out from the
        // two-hop: one Allegation bearing on multiple Elements of the same
        // Count would otherwise produce multiple identical rows for the
        // (a, c, m, e, d) tuple. Rust-side `HashSet<GraphEdge>` already
        // dedupes the edges, but Cypher-side dedup keeps the wire
        // transfer small and matches the agreed migration discipline.
        //
        // The rendered `relationship: schema::SUPPORTS` label is the legal
        // meaning (Allegation supports LegalCount) — the graph still
        // displays this synthetic edge between Allegation and
        // LegalCount, NOT the two intermediate edges through Element.
        // Switching the rendered label to `BEARS_ON` would expose
        // the implementation, not the user's mental model.
        //
        // ## Rust Learning: `{head}` interpolation avoids brace-escaping
        //
        // The only Cypher containing literal `{ }` braces (the node-property
        // map `{id: $count_id}`) is built as a separate `&str` and spliced in
        // via the `{head}` placeholder. Because `format!` does not re-scan
        // interpolated *values* for placeholders, those braces never need the
        // `{{`/`}}` doubling that an inline property map would require.
        let head = if count_id.is_some() {
            "MATCH (c:LegalCount {id: $count_id})"
        } else {
            "MATCH (c:LegalCount)"
        };
        let cypher = format!(
            "{head}
             OPTIONAL MATCH (a:Allegation)-[:{bears_on}]->(el)
                             <-[:{has_element}]-(c)
             OPTIONAL MATCH (m:MotionClaim)-[:{proves}]->(a)
             OPTIONAL MATCH (m)-[:{relies_on}]->(e:Evidence)
             OPTIONAL MATCH (e)-[:{contained_in}]->(d:Document)
             RETURN DISTINCT c.id AS c_id, c.title AS c_title,
                    a.id AS a_id, a.title AS a_title, NULL AS a_status,
                    m.id AS m_id, m.title AS m_title,
                    e.id AS e_id, e.title AS e_title,
                    d.id AS d_id, d.title AS d_title",
            bears_on = schema::BEARS_ON,
            has_element = schema::HAS_ELEMENT,
            proves = schema::PROVES,
            relies_on = schema::RELIES_ON,
            contained_in = schema::CONTAINED_IN,
        );

        let q = if let Some(cid) = count_id {
            query(&cypher).param("count_id", cid)
        } else {
            query(&cypher)
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
                        relationship: schema::SUPPORTS.to_string(),
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
                        relationship: schema::PROVES.to_string(),
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
                        relationship: schema::RELIES_ON.to_string(),
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
                        relationship: schema::CONTAINED_IN.to_string(),
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
