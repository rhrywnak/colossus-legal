//! Persists merged chunk-extraction results (legacy-shaped JSON) into the
//! `extraction_items` / `extraction_relationships` tables. Split from
//! `chunk_orchestration` to keep each module under the 300-line rule.

use std::collections::HashMap;

use crate::error::AppError;
use crate::repositories::pipeline_repository;
use crate::state::AppState;

/// Parse merged-chunk legacy JSON and insert entities + relationships.
///
/// Expected shape: `{ entities: [{entity_type, id, properties, verbatim_quote?}, ...],
/// relationships: [{relationship_type, from_entity, to_entity, properties}, ...] }`.
pub(super) async fn store_entities_and_relationships(
    state: &AppState,
    run_id: i32,
    doc_id: &str,
    parsed: &serde_json::Value,
) -> Result<(usize, usize), AppError> {
    let entities = parsed["entities"].as_array();
    let mut id_map: HashMap<String, i32> = HashMap::new();
    let mut entity_count = 0usize;

    if let Some(entities) = entities {
        for entity in entities {
            let entity_type = entity["entity_type"].as_str().unwrap_or("unknown");
            let json_id = entity["id"].as_str().unwrap_or("");
            let verbatim = entity["verbatim_quote"]
                .as_str()
                .or_else(|| entity["properties"]["verbatim_quote"].as_str());

            let db_id = pipeline_repository::insert_extraction_item(
                &state.pipeline_pool,
                run_id,
                doc_id,
                entity_type,
                entity,
                verbatim,
            )
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to insert entity '{json_id}': {e}"),
            })?;

            if !json_id.is_empty() {
                id_map.insert(json_id.to_string(), db_id);
            }
            entity_count += 1;
        }
    }

    let relationships = parsed["relationships"].as_array();
    let mut rel_count = 0usize;

    if let Some(relationships) = relationships {
        for rel in relationships {
            let rel_type = rel["relationship_type"].as_str().unwrap_or("UNKNOWN");
            let from_id_str = rel["from_entity"].as_str().unwrap_or("");
            let to_id_str = rel["to_entity"].as_str().unwrap_or("");

            let from_db_id = match id_map.get(from_id_str) {
                Some(&id) => id,
                None => continue,
            };
            let to_db_id = match id_map.get(to_id_str) {
                Some(&id) => id,
                None => continue,
            };

            let props = rel.get("properties");

            pipeline_repository::insert_extraction_relationship(
                &state.pipeline_pool,
                run_id,
                doc_id,
                from_db_id,
                to_db_id,
                rel_type,
                props,
                1,
            )
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to insert relationship: {e}"),
            })?;
            rel_count += 1;
        }
    }

    Ok((entity_count, rel_count))
}
