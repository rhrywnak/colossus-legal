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
                entity_type: "Person".to_string(),
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
                entity_type: "Organization".to_string(),
                id,
                label: name.clone(),
                properties: serde_json::json!({"name": name, "role": role}),
            });
        }
    }

    Ok(known)
}

// ── Convert extraction items to resolver input ──────────────────

/// Convert a pipeline ExtractionItemRecord (Party) to an ExtractedEntity
/// for the resolver.
fn to_extracted_entity(item: &ExtractionItemRecord) -> ExtractedEntity {
    let label = item.item_data["label"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let properties = item.item_data["properties"].clone();

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

// ── Run resolver and build resolution map ───────────────────────

/// Resolution map entry: (neo4j_id, is_new).
pub type ResolutionMap = HashMap<String, (String, bool)>;

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
    let party_entities: Vec<ExtractedEntity> = items
        .iter()
        .filter(|i| i.entity_type == "Party")
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

        let (neo4j_id, is_new) = if let Some(ref matched_id) = r.matched_to {
            // Matched existing node — use its ID
            tracing::info!(
                party = %party_name, matched_to = %matched_id,
                method = ?r.resolution, "Resolved → existing node"
            );
            matched_existing += 1;
            (matched_id.clone(), false)
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
            (neo4j_id, true)
        };

        let resolution_str = match r.resolution {
            ResolutionMethod::ExactMatch => "exact_match",
            ResolutionMethod::NormalizedMatch => "normalized_match",
            ResolutionMethod::FuzzyMatch => "fuzzy_match",
            ResolutionMethod::SemanticMatch => "semantic_match",
            ResolutionMethod::NewEntity => "new_entity",
        };

        match_details.push(MatchDetail {
            party_name: party_name.to_string(),
            neo4j_id: neo4j_id.clone(),
            resolution: resolution_str.to_string(),
            is_new,
        });

        resolution_map.insert(party_name.to_string(), (neo4j_id, is_new));
    }

    let summary = ResolutionSummary {
        total_parties,
        matched_existing,
        created_new,
        match_details,
    };

    Ok((resolution_map, summary))
}
