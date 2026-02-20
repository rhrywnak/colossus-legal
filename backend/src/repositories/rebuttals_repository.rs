// =============================================================================
// backend/src/repositories/rebuttals_repository.rs
// =============================================================================
//
// Neo4j queries for GET /rebuttals — all REBUTS grouped by George's claims.
//
// Extracted from decomposition_repository.rs to keep modules under 300 lines.
// Uses a HashMap accumulator to group flat rows by george_evidence_id.
// =============================================================================

use neo4rs::{query, Graph};
use std::collections::HashMap;

use crate::dto::decomposition::{
    GeorgeClaimWithRebuttals, RebuttalDetail, RebuttalsResponse, RebuttalsSummary,
    UnrebuttedReason,
};
use crate::repositories::decomposition_repository::DecompositionRepositoryError;

// ─────────────────────────────────────────────────────────────────────────────
// Query constants
// ─────────────────────────────────────────────────────────────────────────────

const REBUTS_QUERY: &str = "
    MATCH (rebE:Evidence)-[r:REBUTS]->(georgeE:Evidence)
    MATCH (georgeE)-[:STATED_BY]->(george:Person)
    MATCH (rebE)-[:CONTAINED_IN]->(rebDoc:Document)
    MATCH (georgeE)-[:CONTAINED_IN]->(georgeDoc:Document)
    OPTIONAL MATCH (rebE)-[:STATED_BY]->(rebSpeaker)
    RETURN georgeE.id AS george_id,
           georgeE.title AS george_title,
           georgeE.verbatim_quote AS george_quote,
           georgeDoc.title AS george_doc,
           r.topic AS topic,
           rebE.id AS reb_id,
           rebE.verbatim_quote AS reb_quote,
           rebE.page_number AS reb_page,
           rebDoc.title AS reb_doc,
           CASE WHEN rebSpeaker:Person THEN rebSpeaker.name
                WHEN rebSpeaker:Organization THEN rebSpeaker.name
                ELSE null END AS reb_speaker
    ORDER BY georgeE.id, r.topic";

const TOTAL_COUNTS_QUERY: &str = "
    MATCH (e:Evidence)-[:STATED_BY]->(p:Person {name: 'George Phillips'})
    WHERE e.id STARTS WITH 'evidence-phillips-coa-'
    OPTIONAL MATCH (rebE:Evidence)-[:REBUTS]->(e)
    RETURN e.id AS claim_id, count(rebE) AS reb_count";

// ─────────────────────────────────────────────────────────────────────────────
// Repository
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct RebuttalsRepository {
    graph: Graph,
}

impl RebuttalsRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Fetch all REBUTS relationships grouped by George Phillips' claims.
    pub async fn get_rebuttals(
        &self,
    ) -> Result<RebuttalsResponse, DecompositionRepositoryError> {
        let george_claims = self.fetch_and_group_claims().await?;
        let (total_rebutted, total_unrebutted) = self.fetch_rebuttal_totals().await?;

        let total_rebuttals: i64 = george_claims.iter().map(|c| c.rebuttal_count).sum();
        let unrebutted_reasons = Self::build_unrebutted_reasons();

        Ok(RebuttalsResponse {
            summary: RebuttalsSummary {
                total_george_claims_rebutted: total_rebutted,
                total_george_claims_unrebutted: total_unrebutted,
                total_rebuttals,
                unrebutted_reasons,
            },
            george_claims,
        })
    }

    // ── Private: query + HashMap accumulator for George's claims ──────────

    async fn fetch_and_group_claims(
        &self,
    ) -> Result<Vec<GeorgeClaimWithRebuttals>, DecompositionRepositoryError> {
        let mut result = self.graph.execute(query(REBUTS_QUERY)).await?;

        let mut claims_map: HashMap<String, GeorgeClaimWithRebuttals> = HashMap::new();
        let mut claims_order: Vec<String> = Vec::new();

        while let Some(row) = result.next().await? {
            let george_id: String = row.get("george_id").unwrap_or_default();

            if !claims_map.contains_key(&george_id) {
                claims_order.push(george_id.clone());
                claims_map.insert(
                    george_id.clone(),
                    GeorgeClaimWithRebuttals {
                        claim_id: george_id.clone(),
                        claim_title: row.get("george_title").unwrap_or_default(),
                        george_quote: row.get("george_quote").ok(),
                        document: row.get("george_doc").ok(),
                        rebuttals: Vec::new(),
                        rebuttal_count: 0,
                    },
                );
            }

            if let Some(claim) = claims_map.get_mut(&george_id) {
                claim.rebuttals.push(RebuttalDetail {
                    evidence_id: row.get("reb_id").unwrap_or_default(),
                    topic: row.get("topic").ok(),
                    verbatim_quote: row.get("reb_quote").ok(),
                    page_number: row.get("reb_page").ok(),
                    document: row.get("reb_doc").ok(),
                    stated_by: row.get("reb_speaker").ok(),
                });
                claim.rebuttal_count = claim.rebuttals.len() as i64;
            }
        }

        Ok(claims_order
            .into_iter()
            .filter_map(|id| claims_map.remove(&id))
            .collect())
    }

    // ── Private: count rebutted vs unrebutted George claims ──────────────

    async fn fetch_rebuttal_totals(
        &self,
    ) -> Result<(i64, i64), DecompositionRepositoryError> {
        let mut result = self.graph.execute(query(TOTAL_COUNTS_QUERY)).await?;

        let mut total_rebutted: i64 = 0;
        let mut total_unrebutted: i64 = 0;

        while let Some(row) = result.next().await? {
            let reb_count: i64 = row.get("reb_count").unwrap_or(0);
            if reb_count > 0 {
                total_rebutted += 1;
            } else {
                total_unrebutted += 1;
            }
        }

        Ok((total_rebutted, total_unrebutted))
    }

    // ── Private: static unrebutted reasons from Phase D analysis ─────────

    fn build_unrebutted_reasons() -> Vec<UnrebuttedReason> {
        vec![
            UnrebuttedReason {
                claim: "frivolous-claims".to_string(),
                reason: "Blanket characterization — rebutted indirectly \
                         through individual allegation proof chains"
                    .to_string(),
            },
            UnrebuttedReason {
                claim: "scattershot-accusations".to_string(),
                reason: "Blanket label, not a specific factual claim".to_string(),
            },
            UnrebuttedReason {
                claim: "laundry-list".to_string(),
                reason: "Blanket label, not a specific factual claim".to_string(),
            },
            UnrebuttedReason {
                claim: "nadia-refused".to_string(),
                reason: "An admission — actually helps Marie's case".to_string(),
            },
            UnrebuttedReason {
                claim: "burnt-bridges".to_string(),
                reason: "George's personal conclusion — contradicted by his own \
                         admission (CONTRADICTS), not third-party rebuttal"
                    .to_string(),
            },
        ]
    }
}
