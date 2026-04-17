// =============================================================================
// backend/src/repositories/decomposition_repository.rs
// =============================================================================
//
// Neo4j queries for GET /decomposition — overview of all 18 allegations.
//
// Also houses the shared error type used by all three decomposition
// repositories (this file, allegation_detail_repository, rebuttals_repository).
//
// RUST PATTERN: HashMap Accumulator for Row-to-Nested-Struct Mapping
// ──────────────────────────────────────────────────────────────────
// Neo4j returns flat rows (one per OPTIONAL MATCH combination). To build
// nested JSON, we accumulate into a HashMap keyed by a grouping field,
// then convert to a Vec of DTOs.
// =============================================================================

use neo4rs::{query, Graph, Row};
use std::collections::HashMap;

use crate::dto::decomposition::{AllegationOverview, DecompositionResponse, DecompositionSummary};

// ─────────────────────────────────────────────────────────────────────────────
// Shared error type — used by all three decomposition repositories.
//
// RUST LESSON: Why a custom error enum?
// Each repository group defines its own error type wrapping neo4rs errors.
// The `From` impls let us use the `?` operator throughout — when neo4rs
// returns an error, Rust auto-converts it via From. This is called the
// "newtype error pattern" and is standard in Rust projects.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum DecompositionRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
    GraphAccess(colossus_graph::GraphAccessError),
}

impl From<neo4rs::Error> for DecompositionRepositoryError {
    fn from(value: neo4rs::Error) -> Self {
        DecompositionRepositoryError::Neo4j(value)
    }
}

impl From<neo4rs::DeError> for DecompositionRepositoryError {
    fn from(value: neo4rs::DeError) -> Self {
        DecompositionRepositoryError::Value(value)
    }
}

impl From<colossus_graph::GraphAccessError> for DecompositionRepositoryError {
    fn from(value: colossus_graph::GraphAccessError) -> Self {
        DecompositionRepositoryError::GraphAccess(value)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Query constants — separates "what to query" from "how to process results"
// ─────────────────────────────────────────────────────────────────────────────

const OVERVIEW_CHAR_QUERY: &str = "
    MATCH (a:ComplaintAllegation)
    OPTIONAL MATCH (charE:Evidence)-[c:CHARACTERIZES]->(a)
    OPTIONAL MATCH (charE)-[:STATED_BY]->(speaker:Person)
    OPTIONAL MATCH (rebE:Evidence)-[:REBUTS]->(charE)
    RETURN a.id AS id,
           a.title AS title,
           a.allegation AS description,
           a.evidence_status AS status,
           collect(DISTINCT c.characterization) AS characterizations,
           collect(DISTINCT speaker.name) AS speakers,
           count(DISTINCT rebE) AS rebuttal_count
    ORDER BY a.id";

const OVERVIEW_PROOF_QUERY: &str = "
    MATCH (a:ComplaintAllegation)
    OPTIONAL MATCH (mc:MotionClaim)-[:PROVES]->(a)
    RETURN a.id AS id,
           count(DISTINCT mc) AS proof_count
    ORDER BY a.id";

// ─────────────────────────────────────────────────────────────────────────────
// Repository
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct DecompositionRepository {
    graph: Graph,
}

impl DecompositionRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch the decomposition overview: all allegations with characterizations,
    /// proof counts, rebuttal counts, and a summary row.
    pub async fn get_decomposition(
        &self,
    ) -> Result<DecompositionResponse, DecompositionRepositoryError> {
        let mut char_result = self.graph.execute(query(OVERVIEW_CHAR_QUERY)).await?;
        let proof_counts = self.build_proof_count_map().await?;

        let mut allegations: Vec<AllegationOverview> = Vec::new();
        let mut total_chars: i64 = 0;
        let mut total_rebuttals: i64 = 0;
        let mut proven_count: i64 = 0;

        while let Some(row) = char_result.next().await? {
            let (overview, chars, rebuttals, is_proven) =
                Self::map_overview_row(&row, &proof_counts);
            allegations.push(overview);
            total_chars += chars;
            total_rebuttals += rebuttals;
            if is_proven {
                proven_count += 1;
            }
        }

        let total_allegations = allegations.len() as i64;

        Ok(DecompositionResponse {
            summary: DecompositionSummary {
                total_allegations,
                proven_count,
                all_proven: proven_count == total_allegations,
                total_characterizations: total_chars,
                total_rebuttals,
            },
            allegations,
        })
    }

    // ── Private: build proof count lookup from second query ───────────────

    async fn build_proof_count_map(
        &self,
    ) -> Result<HashMap<String, i64>, DecompositionRepositoryError> {
        let mut result = self.graph.execute(query(OVERVIEW_PROOF_QUERY)).await?;
        let mut proof_counts: HashMap<String, i64> = HashMap::new();

        while let Some(row) = result.next().await? {
            let id: String = row.get("id").unwrap_or_default();
            let count: i64 = row.get("proof_count").unwrap_or(0);
            proof_counts.insert(id, count);
        }

        Ok(proof_counts)
    }

    // ── Private: map a single row to AllegationOverview ──────────────────
    //
    // Returns (overview, char_count, rebuttal_count, is_proven) so the
    // caller can accumulate summary totals.

    fn map_overview_row(
        row: &Row,
        proof_counts: &HashMap<String, i64>,
    ) -> (AllegationOverview, i64, i64, bool) {
        let id: String = row.get("id").unwrap_or_default();
        let status: String = row.get("status").unwrap_or_default();

        let characterizations: Vec<String> = row
            .get::<Vec<Option<String>>>("characterizations")
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .collect();

        let speakers: Vec<String> = row
            .get::<Vec<Option<String>>>("speakers")
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .collect();

        let rebuttal_count: i64 = row.get("rebuttal_count").unwrap_or(0);
        let proof_count = proof_counts.get(&id).copied().unwrap_or(0);
        let char_count = characterizations.len() as i64;
        let is_proven = status == "PROVEN";

        let overview = AllegationOverview {
            id,
            title: row.get("title").unwrap_or_default(),
            description: row.get("description").ok(),
            status,
            characterized_by: speakers.into_iter().next(),
            characterizations,
            proof_count,
            rebuttal_count,
        };

        (overview, char_count, rebuttal_count, is_proven)
    }
}
