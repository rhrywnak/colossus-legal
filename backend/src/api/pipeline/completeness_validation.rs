//! Completeness validation — checks extraction output against schema rules.
//!
//! Runs after LLM extraction, before status transitions to EXTRACTED.
//! If required entity types are missing, extraction fails.
//! Relationship percentage checks produce warnings but don't block.

use colossus_extract::{CompletenessRule, ExtractionSchema};

/// Result of completeness validation.
pub struct CompletenessResult {
    /// True if all required checks passed.
    pub passed: bool,
    /// Fatal errors — required entities missing. Block extraction.
    pub errors: Vec<String>,
    /// Non-fatal warnings — relationship coverage low. Log but proceed.
    pub warnings: Vec<String>,
    /// Per-entity-type counts found in the extraction.
    pub entity_counts: Vec<(String, usize)>,
}

/// Validate extraction output against the schema's completeness rules.
///
/// Checks:
/// 1. For each entity type with `required: true` in the schema,
///    verify the extracted JSON has at least `min_count` entities of that type.
/// 2. For each `CompletenessRule::EntityCount`, verify the count meets the minimum.
/// 3. For each `CompletenessRule::RelationshipExists`, compute the percentage
///    of `from` entities that have at least one relationship of the specified
///    type to a `to` entity. Warn if below `min_percentage`.
///
/// Returns `CompletenessResult` with `passed=false` if any errors exist.
pub fn validate_completeness(
    schema: &ExtractionSchema,
    parsed: &serde_json::Value,
) -> CompletenessResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut entity_counts = Vec::new();

    let empty_entities = vec![];
    let entities = parsed["entities"].as_array().unwrap_or(&empty_entities);

    let empty_rels = vec![];
    let relationships = parsed["relationships"].as_array().unwrap_or(&empty_rels);

    // Count entities by type
    let mut type_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for entity in entities {
        if let Some(entity_type) = entity["entity_type"].as_str() {
            *type_counts.entry(entity_type.to_string()).or_insert(0) += 1;
        }
    }

    // Collect counts for reporting
    for et in &schema.entity_types {
        let count = type_counts.get(&et.name).copied().unwrap_or(0);
        entity_counts.push((et.name.clone(), count));
    }

    // Check 1: Schema-level required entity types
    for et in &schema.entity_types {
        if et.required {
            let count = type_counts.get(&et.name).copied().unwrap_or(0);
            let min = if et.min_count > 0 {
                et.min_count as usize
            } else {
                1
            };
            if count < min {
                errors.push(format!(
                    "Required entity type '{}': found {}, need at least {}",
                    et.name, count, min
                ));
            }
        }
    }

    // Check 2: Explicit completeness rules
    for rule in &schema.completeness_rules {
        match rule {
            CompletenessRule::EntityCount {
                entity,
                min,
                message,
            } => {
                let count = type_counts.get(entity.as_str()).copied().unwrap_or(0);
                if count < (*min as usize) {
                    errors.push(format!(
                        "{} (found {} {}, need at least {})",
                        message, count, entity, min
                    ));
                }
            }
            CompletenessRule::RelationshipExists {
                from,
                relationship,
                to,
                min_percentage,
                message,
            } => {
                // Count how many `from` entities have at least one
                // relationship of the specified type to a `to` entity.
                let from_entities: Vec<&str> = entities
                    .iter()
                    .filter_map(|e| {
                        let et = e["entity_type"].as_str()?;
                        if et == from.as_str() {
                            e["id"].as_str()
                        } else {
                            None
                        }
                    })
                    .collect();

                let to_entities: Vec<&str> = entities
                    .iter()
                    .filter_map(|e| {
                        let et = e["entity_type"].as_str()?;
                        if et == to.as_str() {
                            e["id"].as_str()
                        } else {
                            None
                        }
                    })
                    .collect();

                if from_entities.is_empty() {
                    // If no from entities exist, the entity count check will catch it
                    continue;
                }

                let linked_count = from_entities
                    .iter()
                    .filter(|&from_id| {
                        relationships.iter().any(|rel| {
                            rel["relationship_type"].as_str()
                                == Some(relationship.as_str())
                                && rel["from_entity"].as_str() == Some(from_id)
                                && rel["to_entity"]
                                    .as_str()
                                    .map(|to_id| to_entities.contains(&to_id))
                                    .unwrap_or(false)
                        })
                    })
                    .count();

                let percentage =
                    (linked_count as f64 / from_entities.len() as f64 * 100.0) as u32;
                if percentage < *min_percentage {
                    warnings.push(format!(
                        "{} ({}% of {} linked, need {}%)",
                        message, percentage, from, min_percentage
                    ));
                }
            }
        }
    }

    CompletenessResult {
        passed: errors.is_empty(),
        errors,
        warnings,
        entity_counts,
    }
}
