// =============================================================================
// backend/src/repositories/decomposition_repository.rs
// =============================================================================
//
// Neo4j queries + row mapping for the Decomposition API (Phase F).
//
// RUST PATTERN: HashMap Accumulator for Row-to-Nested-Struct Mapping
// ──────────────────────────────────────────────────────────────────
// Neo4j returns flat rows (one per OPTIONAL MATCH combination). To build
// nested JSON, we accumulate into a HashMap keyed by a grouping field
// (e.g., evidence_id), then convert to a Vec of DTOs.
//
// This is the Rust equivalent of Java's Map<String, List<T>> or
// Python's defaultdict(list). Key Rust idiom:
//   map.entry(key).or_insert_with(|| default_value)
// This "get-or-create" pattern avoids double lookups and is idiomatic.
// =============================================================================

use neo4rs::{query, Graph};
use std::collections::HashMap;

use crate::dto::decomposition::{
    AllegationDetailResponse, AllegationInfo, AllegationOverview, CharacterizationDetail,
    DecompositionResponse, DecompositionSummary, GeorgeClaimWithRebuttals, ProofClaimSummary,
    RebuttalDetail, RebuttalsResponse, RebuttalsSummary, UnrebuttedReason,
};

// ─────────────────────────────────────────────────────────────────────────────
// Error type — follows the same pattern as AllegationRepositoryError, etc.
//
// RUST LESSON: Why a custom error enum?
// Each repository defines its own error type wrapping neo4rs errors. The
// `From` impls let us use `?` operator throughout — when neo4rs returns
// an error, Rust auto-converts it via From. This is called the
// "newtype error pattern" and is standard in Rust projects.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum DecompositionRepositoryError {
    Neo4j(neo4rs::Error),
    Value(neo4rs::DeError),
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

    // =========================================================================
    // GET /decomposition — Overview of all 18 allegations
    // =========================================================================
    //
    // STRATEGY: Two separate queries to avoid cartesian products.
    //   Query 1: Allegations + characterizations + rebuttal counts
    //   Query 2: Proof claim counts per allegation
    // Merge in Rust using allegation ID as key.
    // =========================================================================

    pub async fn get_decomposition(
        &self,
    ) -> Result<DecompositionResponse, DecompositionRepositoryError> {
        // ── Query 1: Allegations with characterizations ──────────────────
        let mut char_result = self
            .graph
            .execute(query(
                "MATCH (a:ComplaintAllegation)
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
                 ORDER BY a.id",
            ))
            .await?;

        // ── Query 2: Proof counts per allegation ─────────────────────────
        let mut proof_result = self
            .graph
            .execute(query(
                "MATCH (a:ComplaintAllegation)
                 OPTIONAL MATCH (mc:MotionClaim)-[:PROVES]->(a)
                 RETURN a.id AS id,
                        count(DISTINCT mc) AS proof_count
                 ORDER BY a.id",
            ))
            .await?;

        // ── Build proof count lookup (HashMap for O(1) merge) ────────────
        let mut proof_counts: HashMap<String, i64> = HashMap::new();
        while let Some(row) = proof_result.next().await? {
            let id: String = row.get("id").unwrap_or_default();
            let count: i64 = row.get("proof_count").unwrap_or(0);
            proof_counts.insert(id, count);
        }

        // ── Map characterization rows to DTOs ────────────────────────────
        let mut allegations: Vec<AllegationOverview> = Vec::new();
        let mut total_chars: i64 = 0;
        let mut total_rebuttals: i64 = 0;
        let mut proven_count: i64 = 0;

        while let Some(row) = char_result.next().await? {
            let id: String = row.get("id").unwrap_or_default();
            let status: String = row.get("status").unwrap_or_default();

            // collect() returns Vec<String> — filter out nulls from OPTIONAL MATCH
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

            total_chars += characterizations.len() as i64;
            total_rebuttals += rebuttal_count;
            if status == "PROVEN" {
                proven_count += 1;
            }

            allegations.push(AllegationOverview {
                id,
                title: row.get("title").unwrap_or_default(),
                description: row.get("description").ok(),
                status,
                characterized_by: speakers.into_iter().next(),
                characterizations,
                proof_count,
                rebuttal_count,
            });
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

    // =========================================================================
    // GET /allegations/:id/detail — Deep dive into one allegation
    // =========================================================================
    //
    // Three separate queries:
    //   1. Allegation + legal counts
    //   2. Characterizations + rebuttals (the "he said / they proved" chain)
    //   3. Proof claims with evidence counts
    //
    // RUST PATTERN: HashMap<evidence_id, CharacterizationDetail> accumulator.
    // Multiple rebuttals for the same characterization = multiple rows with
    // the same char_evidence_id. We group them using .entry().or_insert_with().
    // =========================================================================

    pub async fn get_allegation_detail(
        &self,
        allegation_id: &str,
    ) -> Result<Option<AllegationDetailResponse>, DecompositionRepositoryError> {
        // ── Query 1: Allegation + legal counts ───────────────────────────
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (a:ComplaintAllegation {id: $id})
                     OPTIONAL MATCH (a)-[:SUPPORTS]->(lc:LegalCount)
                     RETURN a.id AS id,
                            a.title AS title,
                            a.allegation AS description,
                            a.evidence_status AS status,
                            collect(DISTINCT lc.title) AS legal_counts",
                )
                .param("id", allegation_id),
            )
            .await?;

        // No rows = allegation doesn't exist → return None (handler sends 404)
        let allegation_row = match result.next().await? {
            Some(row) => row,
            None => return Ok(None),
        };

        let legal_counts: Vec<String> = allegation_row
            .get::<Vec<Option<String>>>("legal_counts")
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .collect();

        let allegation_info = AllegationInfo {
            id: allegation_row.get("id").unwrap_or_default(),
            title: allegation_row.get("title").unwrap_or_default(),
            description: allegation_row.get("description").ok(),
            status: allegation_row.get("status").unwrap_or_default(),
            legal_counts,
        };

        // ── Query 2: Characterizations with rebuttals ────────────────────
        //
        // Multiple rebuttals per characterization → multiple rows with same
        // char_evidence_id. We accumulate with a HashMap.
        let mut char_result = self
            .graph
            .execute(
                query(
                    "MATCH (a:ComplaintAllegation {id: $id})
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
                     ORDER BY charE.id, rebE.id",
                )
                .param("id", allegation_id),
            )
            .await?;

        // ── HashMap accumulator ──────────────────────────────────────────
        //
        // RUST LESSON: .entry(key).or_insert_with(|| ...)
        //
        // This is the "get-or-create" pattern. It does a single hash lookup:
        //   - If the key exists → returns a &mut to the existing value
        //   - If the key is absent → calls the closure to create a default,
        //     inserts it, and returns a &mut to the new value
        //
        // Compare to the naive approach:
        //   if !map.contains_key(&key) { map.insert(key.clone(), default); }
        //   let val = map.get_mut(&key).unwrap();
        //
        // .entry() is better because: one lookup instead of two, no unwrap(),
        // and the borrow checker is happy because we hold a single &mut.
        let mut char_map: HashMap<String, CharacterizationDetail> = HashMap::new();
        let mut char_order: Vec<String> = Vec::new(); // preserve insertion order

        while let Some(row) = char_result.next().await? {
            let char_evidence_id: Option<String> = row.get("char_evidence_id").ok();
            let char_evidence_id = match char_evidence_id {
                Some(id) => id,
                None => continue, // No characterization for this allegation
            };

            // Get-or-create the CharacterizationDetail
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

            // If this row has a rebuttal, push it
            let reb_id: Option<String> = row.get("reb_evidence_id").ok();
            if let Some(reb_id) = reb_id {
                if let Some(detail) = char_map.get_mut(&char_evidence_id) {
                    detail.rebuttals.push(RebuttalDetail {
                        evidence_id: reb_id,
                        topic: row.get("reb_topic").ok(),
                        verbatim_quote: row.get("reb_quote").ok(),
                        page_number: row.get("reb_page").ok(),
                        document: row.get("reb_doc_title").ok(),
                        stated_by: row.get("reb_speaker").ok(),
                    });
                }
            }
        }

        // Convert HashMap → Vec in insertion order
        let characterizations: Vec<CharacterizationDetail> = char_order
            .into_iter()
            .filter_map(|id| char_map.remove(&id))
            .collect();

        // ── Query 3: Proof claims ────────────────────────────────────────
        let mut proof_result = self
            .graph
            .execute(
                query(
                    "MATCH (a:ComplaintAllegation {id: $id})
                     OPTIONAL MATCH (mc:MotionClaim)-[:PROVES]->(a)
                     OPTIONAL MATCH (mc)-[:RELIES_ON]->(e:Evidence)
                     RETURN mc.id AS mc_id,
                            mc.title AS mc_title,
                            mc.category AS mc_category,
                            count(DISTINCT e) AS evidence_count
                     ORDER BY mc.id",
                )
                .param("id", allegation_id),
            )
            .await?;

        let mut proof_claims: Vec<ProofClaimSummary> = Vec::new();
        while let Some(row) = proof_result.next().await? {
            let mc_id: Option<String> = row.get("mc_id").ok();
            if let Some(mc_id) = mc_id {
                proof_claims.push(ProofClaimSummary {
                    id: mc_id,
                    title: row.get("mc_title").unwrap_or_default(),
                    category: row.get("mc_category").ok(),
                    evidence_count: row.get("evidence_count").unwrap_or(0),
                });
            }
        }

        Ok(Some(AllegationDetailResponse {
            allegation: allegation_info,
            characterizations,
            proof_claims,
        }))
    }

    // =========================================================================
    // GET /rebuttals — All REBUTS grouped by George's claims
    // =========================================================================

    pub async fn get_rebuttals(
        &self,
    ) -> Result<RebuttalsResponse, DecompositionRepositoryError> {
        // ── Query: All REBUTS with both sides ────────────────────────────
        let mut result = self
            .graph
            .execute(query(
                "MATCH (rebE:Evidence)-[r:REBUTS]->(georgeE:Evidence)
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
                 ORDER BY georgeE.id, r.topic",
            ))
            .await?;

        // ── Accumulate into HashMap ──────────────────────────────────────
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

        let george_claims: Vec<GeorgeClaimWithRebuttals> = claims_order
            .into_iter()
            .filter_map(|id| claims_map.remove(&id))
            .collect();

        // ── Count totals (including unrebutted claims) ───────────────────
        let mut total_result = self
            .graph
            .execute(query(
                "MATCH (e:Evidence)-[:STATED_BY]->(p:Person {name: 'George Phillips'})
                 WHERE e.id STARTS WITH 'evidence-phillips-coa-'
                 OPTIONAL MATCH (rebE:Evidence)-[:REBUTS]->(e)
                 RETURN e.id AS claim_id, count(rebE) AS reb_count",
            ))
            .await?;

        let mut total_rebutted: i64 = 0;
        let mut total_unrebutted: i64 = 0;
        while let Some(row) = total_result.next().await? {
            let reb_count: i64 = row.get("reb_count").unwrap_or(0);
            if reb_count > 0 {
                total_rebutted += 1;
            } else {
                total_unrebutted += 1;
            }
        }

        let total_rebuttals: i64 = george_claims.iter().map(|c| c.rebuttal_count).sum();

        // Unrebutted reasons — from Phase D analysis
        let unrebutted_reasons = vec![
            UnrebuttedReason {
                claim: "frivolous-claims".to_string(),
                reason: "Blanket characterization — rebutted indirectly through individual allegation proof chains".to_string(),
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
                reason: "George's personal conclusion — contradicted by his own admission (CONTRADICTS), not third-party rebuttal".to_string(),
            },
        ];

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
}
