//! Entity resolution helpers for the graph writer.
//!
//! Resolves extracted Party entities against existing Neo4j Person and
//! Organization nodes before writing. Matched parties reuse the existing
//! node (via MERGE); new parties create a fresh node.
//!
//! ## Rust Learning: "Resolve before write" pattern
//!
//! Instead of blindly creating nodes, we first query existing state (Neo4j),
//! compare against incoming data (pipeline DB), decide what's new vs existing,
//! then act. This pattern prevents duplicate nodes when the same person
//! appears in multiple documents.
//!
//! ## Rust Learning: resolution_map bridges two ID systems
//!
//! The pipeline DB identifies parties by name (from item_data). Neo4j
//! identifies them by string IDs (e.g. "person-marie-awad"). The
//! resolution_map (HashMap<String, (String, bool)>) connects these:
//! party_name → (neo4j_id, is_new). This map is consumed by
//! create_party_nodes to decide whether to MERGE-match or MERGE-create.

use std::collections::HashMap;

use colossus_extract::{
    EntityResolver, ExtractedEntity, KnownEntity, NormalizedEntityResolver, ResolutionMethod,
};
use neo4rs::{query, Graph};
use serde::Serialize;

use crate::error::AppError;
use crate::models::document_status::{ENTITY_ORGANIZATION, ENTITY_PERSON, PARTY_SUBTYPES};
use crate::repositories::pipeline_repository::ExtractionItemRecord;

use super::ingest_helpers::slug;

// ── Response DTOs ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ResolutionSummary {
    pub total_parties: usize,
    pub matched_existing: usize,
    pub created_new: usize,
    pub match_details: Vec<MatchDetail>,
}

#[derive(Debug, Serialize)]
pub struct MatchDetail {
    pub party_name: String,
    pub neo4j_id: String,
    pub resolution: String,
    pub is_new: bool,
}

// ── Fetch existing parties from Neo4j ───────────────────────────

/// Query all existing Person and Organization nodes from Neo4j
/// and convert them to `KnownEntity` structs for the resolver.
///
/// This runs OUTSIDE the transaction (read-only) — we need the existing
/// state before deciding what to create inside the transaction.
pub async fn fetch_existing_parties(graph: &Graph) -> Result<Vec<KnownEntity>, AppError> {
    let mut known = Vec::new();

    // Fetch Person nodes
    let mut result = graph
        .execute(query(
            "MATCH (p:Person) RETURN p.id AS id, p.name AS name, p.role AS role",
        ))
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j Person query failed: {e}"),
        })?;

    while let Some(row) = result.next().await.map_err(|e| AppError::Internal {
        message: format!("Row fetch: {e}"),
    })? {
        let id: String = row.get("id").unwrap_or_default();
        let name: String = row.get("name").unwrap_or_default();
        let role: String = row.get("role").unwrap_or_default();
        if !id.is_empty() {
            known.push(KnownEntity {
                entity_type: ENTITY_PERSON.to_string(),
                id,
                label: name.clone(),
                properties: serde_json::json!({"name": name, "role": role}),
            });
        }
    }

    // Fetch Organization nodes
    let mut result = graph
        .execute(query(
            "MATCH (o:Organization) RETURN o.id AS id, o.name AS name, o.role AS role",
        ))
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j Org query failed: {e}"),
        })?;

    while let Some(row) = result.next().await.map_err(|e| AppError::Internal {
        message: format!("Row fetch: {e}"),
    })? {
        let id: String = row.get("id").unwrap_or_default();
        let name: String = row.get("name").unwrap_or_default();
        let role: String = row.get("role").unwrap_or_default();
        if !id.is_empty() {
            known.push(KnownEntity {
                entity_type: ENTITY_ORGANIZATION.to_string(),
                id,
                label: name.clone(),
                properties: serde_json::json!({"name": name, "role": role}),
            });
        }
    }

    Ok(known)
}

// ── Convert extraction items to resolver input ──────────────────

// CONST: the two halves of a vocabulary mismatch between our templates and the
// upstream resolver. Every extraction template instructs `party_type: "person"`
// (and every schema YAML documents it that way), but colossus-extract's
// `compatible_type` matches Person nodes on the token "individual" and returns
// FALSE for anything else. Protocol constants, not deployment config — changing
// either would require changing the templates and the upstream crate in the same
// breath (Rule 2 N/A).
const TEMPLATE_PERSON_TYPE: &str = "person";
const RESOLVER_PERSON_TYPE: &str = "individual";

/// Convert a pipeline ExtractionItemRecord (Party) to an ExtractedEntity
/// for the resolver.
///
/// ## Rust Learning: a boundary adapter, and why the fix lives HERE
///
/// This function is the single point where our data crosses into
/// `colossus-extract`. That makes it the right place to reconcile a vocabulary
/// difference between the two sides — the translation happens once, in one
/// place, and reverting it is deleting one call.
///
/// ## Why: this normalization is load-bearing, not cosmetic
///
/// Upstream `compatible_type` reads:
///
/// ```ignore
/// match party_type {
///     "individual"   => known.entity_type == "Person",
///     "organization" => known.entity_type == "Organization",
///     _ => false,
/// }
/// ```
///
/// Our templates emit `"person"`, which falls to the `_` arm. The candidate list
/// for every human party therefore came back EMPTY, the matcher had nothing to
/// compare against, and every person resolved as a brand-new entity with an id
/// derived from whatever name variant that document happened to use. That is the
/// mechanism behind the duplicate Person nodes in the graph ("Judge Tighe",
/// "Karen A. Tighe", "Tighe" and "Karen A." became four separate people).
/// Organizations were unaffected — `"organization"` matches on both sides.
///
/// Translating `"person"` → `"individual"` here turns the resolver's exact and
/// normalized matching on for people for the first time. The proper fix is
/// upstream (teach `compatible_type` the real vocabulary); this adapter is the
/// reversible local version that does not require a crate release, and it is
/// deliberately a no-op for every other value.
fn to_extracted_entity(item: &ExtractionItemRecord) -> ExtractedEntity {
    let label = item.item_data["label"].as_str().unwrap_or("").to_string();
    let properties = normalize_party_type(item.item_data["properties"].clone());

    ExtractedEntity {
        entity_type: item.entity_type.clone(),
        id: None,
        label,
        properties,
        verbatim_quote: item.verbatim_quote.clone(),
        page_number: item.grounded_page.map(|p| p as u32),
        grounding_status: None,
    }
}

/// Rewrite `party_type: "person"` to the token the upstream resolver understands.
///
/// Everything else passes through untouched — `"organization"` already matches,
/// and an unrecognized value is left exactly as found so it keeps failing the
/// upstream type check loudly rather than being coerced into a wrong match.
/// A non-object `properties` (or a missing `party_type`) is returned unchanged:
/// the resolver's own `unwrap_or("individual")` default then applies, which is
/// the pre-existing behavior for those shapes.
fn normalize_party_type(mut properties: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = properties.as_object_mut() {
        if obj.get("party_type").and_then(|v| v.as_str()) == Some(TEMPLATE_PERSON_TYPE) {
            obj.insert(
                "party_type".to_string(),
                serde_json::Value::String(RESOLVER_PERSON_TYPE.to_string()),
            );
        }
    }
    properties
}

// ── Run resolver and build resolution map ───────────────────────

/// What resolution decided about one party.
///
/// ## Rust Learning: a named struct instead of a growing tuple
///
/// This started as `(String, bool)` and needed a third field. A three-element
/// tuple would have forced every reader to remember what `.2` meant; a struct
/// makes the writer's call site read `resolved.existing_name` instead of
/// `resolved.2`. Same cost at runtime, far less to get wrong.
// Note: this deliberately carries no `is_new` flag. The writer does not branch on
// it — MERGE handles create-vs-match itself — and the new/matched COUNTS the API
// reports are computed independently into `ResolutionSummary`. A field nothing
// reads is a field that can silently drift out of agreement with the truth.
#[derive(Debug, Clone)]
pub struct ResolvedParty {
    /// The Neo4j node id this party will be MERGE'd on.
    pub neo4j_id: String,
    /// The `name` already stored on the matched node, when this resolved to an
    /// existing party. `None` for a new entity.
    ///
    /// Carried so the writer can detect a canonical-form disagreement between
    /// two documents: the first writer's name stays authoritative, and the
    /// incoming variant is preserved as an alias rather than lost. Without this
    /// the writer has the id but no way to know it disagreed with anything.
    pub existing_name: Option<String>,
}

/// Resolution map: `party_name` → what resolution decided.
pub type ResolutionMap = HashMap<String, ResolvedParty>;

/// Whether a match method is allowed to bind two parties into one node.
///
/// ## Domain note: a false merge is worse than a duplicate
///
/// Ruling 2026-07-20: only EXACT and NORMALIZED matches auto-merge. In a legal
/// knowledge graph a duplicate fragments a person's evidence — visible, and
/// fixable by a human-approved dedup pass. A FALSE merge silently welds two real
/// people into one node and attributes one person's sworn statements to another,
/// which is both far more damaging and far harder to detect after the fact.
///
/// Jaro-Winkler similarity is a good metric and a bad adjudicator of identity, so
/// it does not get a vote. Fuzzy and semantic hits are DEMOTED rather than
/// discarded: the party resolves as a new entity and the near-match is logged for
/// the dedup pass to consider with a human in the loop.
fn is_auto_mergeable(method: &ResolutionMethod) -> bool {
    matches!(
        method,
        ResolutionMethod::ExactMatch | ResolutionMethod::NormalizedMatch
    )
}

/// The label reported in [`ResolutionSummary`] for a match method.
///
/// Reports what ACTUALLY happened, not what the matcher proposed: a demoted fuzzy
/// hit created a NEW entity, and a summary claiming "fuzzy_match" would tell an
/// operator two parties were merged when they were not (Standing Rule 1 — the
/// observable must match the behavior).
fn resolution_label(method: &ResolutionMethod) -> &'static str {
    match method {
        ResolutionMethod::ExactMatch => "exact_match",
        ResolutionMethod::NormalizedMatch => "normalized_match",
        ResolutionMethod::FuzzyMatch => "fuzzy_match_not_merged",
        ResolutionMethod::SemanticMatch => "semantic_match_not_merged",
        ResolutionMethod::NewEntity => "new_entity",
    }
}

/// Run entity resolution on Party items and build the resolution map.
///
/// Returns:
/// - `ResolutionMap`: party_name → (neo4j_id, is_new)
/// - `ResolutionSummary`: for the API response
pub async fn resolve_parties(
    items: &[ExtractionItemRecord],
    existing_parties: &[KnownEntity],
) -> Result<(ResolutionMap, ResolutionSummary), AppError> {
    // Build ExtractedEntity list from Party items only
    // R4: accept the resolved forms as well as the raw "Party". Fresh items from
    // the LLM carry "Party"; rows that have already been through ingest carry the
    // COALESCE'd resolved label ("Person"/"Organization") written by
    // `update_item_entity_type`. The WRITER (`create_party_nodes`) has always
    // accepted all three via PARTY_SUBTYPES — this filter accepted only the first,
    // so on a re-ingest of an already-ingested run the resolver silently skipped
    // every party, handed back an empty map, and the writer fell through to its
    // slug-derived id. Matching the writer's filter closes that asymmetry.
    let party_entities: Vec<ExtractedEntity> = items
        .iter()
        .filter(|i| PARTY_SUBTYPES.contains(&i.entity_type.as_str()))
        .map(to_extracted_entity)
        .collect();

    let total_parties = party_entities.len();

    // Run resolver
    let resolver = NormalizedEntityResolver::new();
    let resolved = resolver
        .resolve(party_entities, existing_parties)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Entity resolution failed: {e}"),
        })?;

    // Build resolution map and summary
    let mut resolution_map: ResolutionMap = HashMap::new();
    let mut match_details: Vec<MatchDetail> = Vec::new();
    let (mut matched_existing, mut created_new) = (0usize, 0usize);

    for r in &resolved {
        let party_name = r
            .extracted
            .properties
            .get("party_name")
            .and_then(|v| v.as_str())
            .unwrap_or(&r.extracted.label);

        // Skip if we already resolved this name (dedup within same document)
        if resolution_map.contains_key(party_name) {
            continue;
        }

        // RULING (2026-07-20): only EXACT and NORMALIZED matches may auto-merge.
        //
        // Domain note: in a legal knowledge graph a false merge is worse than a
        // duplicate. A duplicate fragments a person's evidence and is visible and
        // fixable; a false merge silently welds two real people into one node,
        // attributing one person's sworn statements to another — and it is very
        // hard to detect after the fact. Jaro-Winkler is a good similarity metric
        // and a bad adjudicator of identity, so it does not get to decide.
        //
        // Fuzzy hits are not discarded, they are DEMOTED: the party resolves as a
        // new entity (so nothing is silently welded) and the near-match is logged
        // at WARN as input for the human-approved dedup pass. Rejecting by METHOD
        // rather than by raising the upstream threshold keeps the near-match
        // visible — a threshold change would make these disappear entirely.
        let auto_mergeable = is_auto_mergeable(&r.resolution);

        if let (Some(matched_id), false) = (r.matched_to.as_ref(), auto_mergeable) {
            // Name the candidate, not just its id. This log IS the input to the
            // human-approved dedup pass, and "person-karen-a-tighe" alone forces
            // the reader back into the graph to learn what it is. The lookup is
            // the same one the auto-merge arm below performs.
            let near_match_name = existing_parties
                .iter()
                .find(|k| &k.id == matched_id)
                .map(|k| k.label.as_str())
                .unwrap_or("<name unavailable>");
            tracing::warn!(
                party = %party_name,
                near_match_id = %matched_id,
                near_match_name = %near_match_name,
                method = ?r.resolution,
                "Fuzzy/semantic near-match NOT auto-merged (policy: exact+normalized only); \
                 resolving as a new entity — review for the dedup pass"
            );
        }

        let (neo4j_id, is_new, existing_name) =
            if let (Some(ref matched_id), true) = (&r.matched_to, auto_mergeable) {
                // Matched existing node — use its ID, and carry its stored name so the
                // writer can spot a canonical-form disagreement (ruling 2026-07-20 #4).
                let existing_name = existing_parties
                    .iter()
                    .find(|k| &k.id == matched_id)
                    .map(|k| k.label.clone());
                tracing::info!(
                    party = %party_name, matched_to = %matched_id,
                    method = ?r.resolution, "Resolved → existing node"
                );
                matched_existing += 1;
                (matched_id.clone(), false, existing_name)
            } else {
                // New entity — generate slug ID
                let party_type = r
                    .extracted
                    .properties
                    .get("party_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("individual");
                let neo4j_id = match party_type {
                    "organization" => format!("org-{}", slug(party_name)),
                    _ => format!("person-{}", slug(party_name)),
                };
                tracing::info!(
                    party = %party_name, neo4j_id = %neo4j_id, "Resolved → new entity"
                );
                created_new += 1;
                (neo4j_id, true, None)
            };

        let resolution_str = resolution_label(&r.resolution);

        match_details.push(MatchDetail {
            party_name: party_name.to_string(),
            neo4j_id: neo4j_id.clone(),
            resolution: resolution_str.to_string(),
            is_new,
        });

        resolution_map.insert(
            party_name.to_string(),
            ResolvedParty {
                neo4j_id,
                existing_name,
            },
        );
    }

    let summary = ResolutionSummary {
        total_parties,
        matched_existing,
        created_new,
        match_details,
    };

    Ok((resolution_map, summary))
}

#[cfg(test)]
#[path = "ingest_resolver_tests.rs"]
mod tests;
