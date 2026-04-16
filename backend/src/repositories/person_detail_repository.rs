// Neo4j queries for GET /persons/:id/detail — person profile with all statements.
//
// RUST PATTERN: HashMap Accumulator for Row-to-Nested-Struct Mapping
// The Cypher returns flat rows (one per evidence × characterization × rebuttal
// combination). We accumulate into nested HashMaps keyed by doc_id and
// evidence_id, then convert to sorted Vecs of DTOs.

use neo4rs::{query, Graph};
use std::collections::{HashMap, HashSet};

use crate::dto::person_detail::{
    CharacterizesInfo, DocumentGroup, PersonDetailResponse, PersonInfo, PersonSummary,
    RebuttalInfo, StatementDetail,
};

#[derive(Debug)]
pub enum PersonDetailRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
    GraphAccess(colossus_graph::GraphAccessError),
}

impl From<neo4rs::Error> for PersonDetailRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        PersonDetailRepositoryError::Neo4j(value)
    }
}

impl From<neo4rs::DeError> for PersonDetailRepositoryError {
    fn from(value: neo4rs::DeError) -> Self {
        PersonDetailRepositoryError::Value(value)
    }
}

impl From<colossus_graph::GraphAccessError> for PersonDetailRepositoryError {
    fn from(value: colossus_graph::GraphAccessError) -> Self {
        PersonDetailRepositoryError::GraphAccess(value)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Cypher constants
// ─────────────────────────────────────────────────────────────────────────────

/// Statements query — relationship-anchored, label-agnostic.
///
/// Queries ALL nodes connected to the person via STATED_BY, regardless of
/// their label. This works with v2 data (ComplaintAllegation nodes have
/// STATED_BY relationships) and will automatically pick up Evidence nodes
/// when they exist in the graph.
///
/// The `p.id = $person_id` filter uses the person's application-level ID.
/// CONTAINED_IN anchors to the Document node. CHARACTERIZES and REBUTS
/// are optional — they'll populate when cross-document analysis exists.
const STATEMENTS_QUERY: &str = "
    MATCH (e)-[:STATED_BY]->(p {id: $person_id})
    MATCH (e)-[:CONTAINED_IN]->(d)
      WHERE labels(d)[0] = 'Document'
    OPTIONAL MATCH (e)-[c:CHARACTERIZES]->(a)
    OPTIONAL MATCH (reb)-[:REBUTS]->(e)
    OPTIONAL MATCH (reb)-[:STATED_BY]->(rp)
    OPTIONAL MATCH (reb)-[:CONTAINED_IN]->(rd)
    RETURN e.id AS eid, e.title AS etitle, e.verbatim_quote AS quote,
           e.page_number AS page_number, e.kind AS kind,
           e.significance AS significance,
           d.id AS doc_id, d.title AS doc_title,
           c.characterization AS char_label,
           a.id AS allegation_id, a.allegation AS allegation_text,
           reb.id AS reb_id, reb.title AS reb_title,
           reb.verbatim_quote AS reb_quote,
           rp.name AS reb_by, rd.title AS reb_doc
    ORDER BY d.title, e.page_number";

// ─────────────────────────────────────────────────────────────────────────────
// Repository
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct PersonDetailRepository {
    graph: Graph,
}

impl PersonDetailRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch person detail. Returns None if the person doesn't exist.
    pub async fn get_person_detail(
        &self,
        person_id: &str,
    ) -> Result<Option<PersonDetailResponse>, PersonDetailRepositoryError> {
        let person = self.get_person_info(person_id).await?;
        let Some(person) = person else {
            return Ok(None);
        };

        let (documents, summary) = self.get_statements(person_id).await?;

        Ok(Some(PersonDetailResponse {
            person,
            summary,
            documents,
        }))
    }

    // ── Query 1: Person identity ─────────────────────────────────────────
    //
    // Uses colossus_graph::get_node_by_id — schema-agnostic node lookup
    // by application-level ID property.

    async fn get_person_info(
        &self,
        person_id: &str,
    ) -> Result<Option<PersonInfo>, PersonDetailRepositoryError> {
        let node = colossus_graph::get_node_by_id(&self.graph, person_id).await?;

        match node {
            Some(n) => {
                let name = n
                    .properties
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let role = n
                    .properties
                    .get("role")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                Ok(Some(PersonInfo {
                    id: n.id,
                    name,
                    role,
                }))
            }
            None => Ok(None),
        }
    }

    // ── Query 2: All statements grouped by document ──────────────────────

    async fn get_statements(
        &self,
        person_id: &str,
    ) -> Result<(Vec<DocumentGroup>, PersonSummary), PersonDetailRepositoryError> {
        let mut result = self
            .graph
            .execute(query(STATEMENTS_QUERY).param("person_id", person_id))
            .await?;

        let mut doc_map: HashMap<String, DocBuilder> = HashMap::new();
        let mut char_ids: HashSet<String> = HashSet::new();
        let mut reb_ids: HashSet<String> = HashSet::new();

        while let Some(row) = result.next().await? {
            let eid: String = row.get("eid").unwrap_or_default();
            let doc_id: String = row.get("doc_id").unwrap_or_default();
            let doc_title: String = row.get("doc_title").unwrap_or_default();

            let doc = doc_map.entry(doc_id.clone()).or_insert_with(|| DocBuilder {
                doc_id,
                doc_title,
                statements: HashMap::new(),
            });

            let stmt = doc
                .statements
                .entry(eid.clone())
                .or_insert_with(|| StmtBuilder {
                    evidence_id: eid,
                    title: row.get("etitle").unwrap_or_default(),
                    verbatim_quote: row.get("quote").ok(),
                    page_number: row.get("page_number").ok(),
                    kind: row.get("kind").ok(),
                    significance: row.get("significance").ok(),
                    characterizes: Vec::new(),
                    seen_chars: HashSet::new(),
                    rebutted_by: Vec::new(),
                    seen_rebs: HashSet::new(),
                });

            // Accumulate characterization (dedup by allegation_id + label)
            if let Ok(allegation_id) = row.get::<String>("allegation_id") {
                let char_label: Option<String> = row.get("char_label").ok();
                let dedup_key =
                    format!("{}:{}", allegation_id, char_label.as_deref().unwrap_or(""));
                if stmt.seen_chars.insert(dedup_key.clone()) {
                    char_ids.insert(dedup_key);
                    stmt.characterizes.push(CharacterizesInfo {
                        allegation_id,
                        allegation_text: row.get("allegation_text").ok(),
                        characterization_label: char_label,
                    });
                }
            }

            // Accumulate rebuttal (dedup by rebuttal evidence_id)
            if let Ok(reb_id) = row.get::<String>("reb_id") {
                if stmt.seen_rebs.insert(reb_id.clone()) {
                    reb_ids.insert(reb_id.clone());
                    stmt.rebutted_by.push(RebuttalInfo {
                        evidence_id: reb_id,
                        title: row.get("reb_title").ok(),
                        verbatim_quote: row.get("reb_quote").ok(),
                        stated_by: row.get("reb_by").ok(),
                        document_title: row.get("reb_doc").ok(),
                    });
                }
            }
        }

        let documents = Self::build_document_groups(doc_map);
        let total_statements: i64 = documents.iter().map(|d| d.statement_count as i64).sum();

        let summary = PersonSummary {
            total_statements,
            documents_count: documents.len() as i64,
            characterizations_count: char_ids.len() as i64,
            rebuttals_received_count: reb_ids.len() as i64,
        };

        Ok((documents, summary))
    }

    /// Convert HashMap accumulators into sorted Vec<DocumentGroup>.
    fn build_document_groups(doc_map: HashMap<String, DocBuilder>) -> Vec<DocumentGroup> {
        let mut documents: Vec<DocumentGroup> = doc_map
            .into_values()
            .map(|db| {
                let mut statements: Vec<StatementDetail> = db
                    .statements
                    .into_values()
                    .map(|sb| StatementDetail {
                        evidence_id: sb.evidence_id,
                        title: sb.title,
                        verbatim_quote: sb.verbatim_quote,
                        page_number: sb.page_number,
                        kind: sb.kind,
                        significance: sb.significance,
                        characterizes: sb.characterizes,
                        rebutted_by: sb.rebutted_by,
                    })
                    .collect();
                statements.sort_by_key(|s| s.page_number.unwrap_or(i64::MAX));

                DocumentGroup {
                    document_id: db.doc_id,
                    document_title: db.doc_title,
                    statement_count: statements.len(),
                    statements,
                }
            })
            .collect();

        documents.sort_by(|a, b| a.document_title.cmp(&b.document_title));
        documents
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal builder structs for HashMap accumulation
// ─────────────────────────────────────────────────────────────────────────────

struct DocBuilder {
    doc_id: String,
    doc_title: String,
    statements: HashMap<String, StmtBuilder>,
}

struct StmtBuilder {
    evidence_id: String,
    title: String,
    verbatim_quote: Option<String>,
    page_number: Option<i64>,
    kind: Option<String>,
    significance: Option<String>,
    characterizes: Vec<CharacterizesInfo>,
    seen_chars: HashSet<String>,
    rebutted_by: Vec<RebuttalInfo>,
    seen_rebs: HashSet<String>,
}
