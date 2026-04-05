// =============================================================================
// backend/src/repositories/allegation_detail_repository.rs
// =============================================================================
//
// Neo4j queries for GET /allegations/:id/detail — deep dive into one allegation.
//
// Extracted from decomposition_repository.rs to keep modules under 300 lines.
// Uses three separate queries to avoid cartesian products:
//   1. Allegation + legal counts
//   2. Characterizations + rebuttals (HashMap accumulator pattern)
//   3. Proof claims with evidence counts
// =============================================================================

use neo4rs::{query, Graph, Row};
use std::collections::HashMap;

use crate::dto::decomposition::{
    AllegationDetailResponse, AllegationInfo, CharacterizationDetail, ProofClaimSummary,
    RebuttalDetail,
};
use crate::repositories::decomposition_repository::DecompositionRepositoryError;

// ─────────────────────────────────────────────────────────────────────────────
// Query constants — separates "what to query" from "how to process results"
// ─────────────────────────────────────────────────────────────────────────────

// TODO: DAL Phase 2 — migrate to colossus_graph once batch relationship queries
// are available. Kept as raw Cypher because the SUPPORTS join (collect legal count
// titles per allegation) is done efficiently in one query.
const ALLEGATION_INFO_QUERY: &str = "
    MATCH (a:ComplaintAllegation {id: $id})
    OPTIONAL MATCH (a)-[:SUPPORTS]->(lc:LegalCount)
    RETURN a.id AS id,
           a.title AS title,
           a.allegation AS description,
           a.evidence_status AS status,
           collect(DISTINCT lc.title) AS legal_counts";

// TODO: DAL Phase 2 — this query targets v1 :Evidence nodes and CHARACTERIZES/REBUTS
// relationships which don't exist in v2. Returns empty results. Needs v2 equivalent
// once cross-document analysis relationships are available.
const CHARACTERIZATION_QUERY: &str = "
    MATCH (a:ComplaintAllegation {id: $id})
    OPTIONAL MATCH (charE:Evidence)-[c:CHARACTERIZES]->(a)
    OPTIONAL MATCH (charE)-[:CONTAINED_IN]->(charDoc:Document)
    OPTIONAL MATCH (charE)-[:STATED_BY]->(charSpeaker:Person)
    OPTIONAL MATCH (rebE:Evidence)-[r:REBUTS]->(charE)
    OPTIONAL MATCH (rebE)-[:CONTAINED_IN]->(rebDoc:Document)
    OPTIONAL MATCH (rebE)-[:STATED_BY]->(rebSpeaker)
    RETURN c.characterization AS label,
           charE.id AS char_evidence_id,
           charE.verbatim_quote AS char_quote,
           charE.page_number AS char_page,
           charDoc.title AS char_doc_title,
           charSpeaker.name AS char_speaker,
           rebE.id AS reb_evidence_id,
           r.topic AS reb_topic,
           rebE.verbatim_quote AS reb_quote,
           rebE.page_number AS reb_page,
           rebDoc.title AS reb_doc_title,
           CASE WHEN rebSpeaker:Person THEN rebSpeaker.name
                WHEN rebSpeaker:Organization THEN rebSpeaker.name
                ELSE null END AS reb_speaker
    ORDER BY charE.id, rebE.id";

// TODO: DAL Phase 2 — this query targets v1 :MotionClaim and :PROVES/:RELIES_ON
// relationships which don't exist in v2. Returns empty results.
const PROOF_CLAIMS_QUERY: &str = "
    MATCH (a:ComplaintAllegation {id: $id})
    OPTIONAL MATCH (mc:MotionClaim)-[:PROVES]->(a)
    OPTIONAL MATCH (mc)-[:RELIES_ON]->(e:Evidence)
    RETURN mc.id AS mc_id,
           mc.title AS mc_title,
           mc.category AS mc_category,
           count(DISTINCT e) AS evidence_count
    ORDER BY mc.id";

// ─────────────────────────────────────────────────────────────────────────────
// Repository
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AllegationDetailRepository {
    graph: Graph,
}

impl AllegationDetailRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch full detail for a single allegation: info, characterizations, proofs.
    /// Returns None if the allegation ID doesn't exist (handler sends 404).
    pub async fn get_allegation_detail(
        &self,
        allegation_id: &str,
    ) -> Result<Option<AllegationDetailResponse>, DecompositionRepositoryError> {
        let allegation_info = match self.fetch_allegation_info(allegation_id).await? {
            Some(info) => info,
            None => return Ok(None),
        };

        let characterizations = self.fetch_characterizations(allegation_id).await?;
        let proof_claims = self.fetch_proof_claims(allegation_id).await?;

        Ok(Some(AllegationDetailResponse {
            allegation: allegation_info,
            characterizations,
            proof_claims,
        }))
    }

    // ── Private: Query 1 — allegation + legal counts ─────────────────────

    async fn fetch_allegation_info(
        &self,
        allegation_id: &str,
    ) -> Result<Option<AllegationInfo>, DecompositionRepositoryError> {
        let mut result = self
            .graph
            .execute(query(ALLEGATION_INFO_QUERY).param("id", allegation_id))
            .await?;

        let row = match result.next().await? {
            Some(row) => row,
            None => return Ok(None),
        };

        let legal_counts: Vec<String> = row
            .get::<Vec<Option<String>>>("legal_counts")
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .collect();

        Ok(Some(AllegationInfo {
            id: row.get("id").unwrap_or_default(),
            title: row.get("title").unwrap_or_default(),
            description: row.get("description").ok(),
            status: row.get("status").unwrap_or_default(),
            legal_counts,
        }))
    }

    // ── Private: Query 2 — characterizations with nested rebuttals ───────
    //
    // Multiple rebuttals per characterization → multiple rows with same
    // char_evidence_id. We accumulate with a HashMap, then convert to Vec
    // in insertion order.

    async fn fetch_characterizations(
        &self,
        allegation_id: &str,
    ) -> Result<Vec<CharacterizationDetail>, DecompositionRepositoryError> {
        let mut result = self
            .graph
            .execute(query(CHARACTERIZATION_QUERY).param("id", allegation_id))
            .await?;

        let mut char_map: HashMap<String, CharacterizationDetail> = HashMap::new();
        let mut char_order: Vec<String> = Vec::new();

        while let Some(row) = result.next().await? {
            let char_evidence_id: Option<String> = row.get("char_evidence_id").ok();
            let char_evidence_id = match char_evidence_id {
                Some(id) => id,
                None => continue,
            };

            // Get-or-create the CharacterizationDetail entry
            if !char_map.contains_key(&char_evidence_id) {
                char_order.push(char_evidence_id.clone());
                char_map.insert(
                    char_evidence_id.clone(),
                    CharacterizationDetail {
                        label: row.get("label").unwrap_or_default(),
                        evidence_id: char_evidence_id.clone(),
                        verbatim_quote: row.get("char_quote").ok(),
                        page_number: row.get("char_page").ok(),
                        document: row.get("char_doc_title").ok(),
                        stated_by: row.get("char_speaker").ok(),
                        rebuttals: Vec::new(),
                    },
                );
            }

            // If this row has a rebuttal, push it onto the characterization
            if let Some(rebuttal) = Self::parse_rebuttal_from_row(&row) {
                if let Some(detail) = char_map.get_mut(&char_evidence_id) {
                    detail.rebuttals.push(rebuttal);
                }
            }
        }

        // Convert HashMap → Vec preserving insertion order
        Ok(char_order
            .into_iter()
            .filter_map(|id| char_map.remove(&id))
            .collect())
    }

    // ── Private: Query 3 — proof claims with evidence counts ─────────────

    async fn fetch_proof_claims(
        &self,
        allegation_id: &str,
    ) -> Result<Vec<ProofClaimSummary>, DecompositionRepositoryError> {
        let mut result = self
            .graph
            .execute(query(PROOF_CLAIMS_QUERY).param("id", allegation_id))
            .await?;

        let mut proof_claims: Vec<ProofClaimSummary> = Vec::new();
        while let Some(row) = result.next().await? {
            if let Ok(mc_id) = row.get::<String>("mc_id") {
                proof_claims.push(ProofClaimSummary {
                    id: mc_id,
                    title: row.get("mc_title").unwrap_or_default(),
                    category: row.get("mc_category").ok(),
                    evidence_count: row.get("evidence_count").unwrap_or(0),
                });
            }
        }

        Ok(proof_claims)
    }

    // ── Private: parse a rebuttal from a characterization query row ───────

    fn parse_rebuttal_from_row(row: &Row) -> Option<RebuttalDetail> {
        let reb_id = row.get::<String>("reb_evidence_id").ok()?;
        Some(RebuttalDetail {
            evidence_id: reb_id,
            topic: row.get("reb_topic").ok(),
            verbatim_quote: row.get("reb_quote").ok(),
            page_number: row.get("reb_page").ok(),
            document: row.get("reb_doc_title").ok(),
            stated_by: row.get("reb_speaker").ok(),
        })
    }
}
