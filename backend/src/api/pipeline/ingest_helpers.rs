//! Helper functions for the graph ingest endpoint.
//! Extracted from `ingest.rs` to keep it under 300 lines.
//!
//! ## Rust Learning: Generic entity node creation
//!
//! The `create_entity_node` function creates Neo4j nodes for ANY entity type.
//! The entity_type string from the extraction schema becomes the Neo4j label
//! directly (e.g., "ComplaintAllegation" → `:ComplaintAllegation`).
//!
//! Party entities are special: they use MERGE (upsert) and split into
//! `:Person` or `:Organization` based on the `entity_kind`/`party_type` property.

use std::collections::{HashMap, HashSet};

use neo4rs::query;

use crate::error::AppError;
use crate::repositories::pipeline_repository::ExtractionItemRecord;

use super::ingest_resolver::ResolutionMap;

/// Generate a stable, URL-friendly slug from a name.
/// Lowercases for natural dedup: `"MARIE AWAD"` and `"Marie Awad"` both → `"marie-awad"`.
pub fn slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Create the Document node in Neo4j. Returns the generated neo4j ID.
pub async fn create_document_node(
    txn: &mut neo4rs::Txn,
    doc_id: &str,
    title: &str,
    doc_type: &str,
) -> Result<String, AppError> {
    let neo4j_id = format!("doc-{}", slug(title));

    txn.run(
        query(
            "CREATE (d:Document { id: $id, title: $title, \
                source_document_id: $source_id, doc_type: $doc_type, \
                status: 'INGESTED', ingested_at: datetime() })",
        )
        .param("id", neo4j_id.as_str())
        .param("title", title)
        .param("source_id", doc_id)
        .param("doc_type", doc_type),
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to create Document node: {e}"),
    })?;
    Ok(neo4j_id)
}

/// Create or merge Party nodes (Person/Organization) using entity resolution.
///
/// ## Rust Learning: MERGE for cross-document entity resolution
///
/// Parties are the only entity type that uses MERGE instead of CREATE.
/// The same person (e.g., "Marie Awad") can appear in multiple documents.
/// MERGE matches on the node ID and either creates a new node or updates
/// the existing one by appending the new document to `source_documents`.
pub async fn create_party_nodes(
    txn: &mut neo4rs::Txn,
    items: &[ExtractionItemRecord],
    doc_id: &str,
    pg_to_neo4j: &mut HashMap<i32, String>,
    pg_to_label: &mut HashMap<i32, String>,
    resolution_map: &ResolutionMap,
) -> Result<(usize, usize), AppError> {
    let mut seen: HashSet<String> = HashSet::new();
    let (mut persons, mut orgs) = (0usize, 0usize);

    for item in items.iter().filter(|i| i.entity_type == "Party") {
        let props = &item.item_data["properties"];
        // Support both property naming conventions across schemas
        let name = props["party_name"].as_str()
            .or_else(|| props["full_name"].as_str())
            .unwrap_or("unknown");
        let role = props["role"].as_str().unwrap_or("");
        // Support both "party_type" (complaint.yaml) and "entity_kind" (general_legal.yaml)
        let party_type = props["party_type"].as_str()
            .or_else(|| props["entity_kind"].as_str())
            .unwrap_or("individual");

        let is_org = party_type == "organization"
            || party_type.to_lowercase().contains("org");
        let label = if is_org { "Organization" } else { "Person" };

        // Look up resolved ID from the resolution map
        let neo4j_id = resolution_map
            .get(name)
            .map(|(id, _)| id.clone())
            .unwrap_or_else(|| {
                let prefix = if is_org { "org" } else { "person" };
                format!("{prefix}-{}", slug(name))
            });

        pg_to_neo4j.insert(item.id, neo4j_id.clone());
        pg_to_label.insert(item.id, label.to_string());

        // Skip if we already MERGE'd this node in this batch
        if !seen.insert(neo4j_id.clone()) {
            continue;
        }

        let cypher = format!(
            "MERGE (n:{label} {{id: $id}}) \
             ON CREATE SET n.name = $name, n.role = $role, \
               n.source_document = $doc, n.source_documents = [$doc] \
             ON MATCH SET n.source_documents = CASE \
               WHEN NOT $doc IN coalesce(n.source_documents, []) \
               THEN coalesce(n.source_documents, []) + $doc \
               ELSE n.source_documents END"
        );
        txn.run(
            query(&cypher)
                .param("id", neo4j_id.as_str())
                .param("name", name)
                .param("role", role)
                .param("doc", doc_id),
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to merge {label} '{name}': {e}"),
        })?;

        if is_org { orgs += 1; } else { persons += 1; }
    }
    Ok((persons, orgs))
}

/// Create a Neo4j node for any non-Party entity type.
///
/// ## Rust Learning: Dynamic Neo4j labels from schema
///
/// The entity_type string from the extraction output becomes the Neo4j label
/// directly. Neo4j Cypher does not support parameterized labels, so we
/// interpolate the label into the query string. This is safe because the
/// label originates from our schema YAML — we validate it contains only
/// alphanumeric characters and underscores to prevent Cypher injection.
///
/// Properties are extracted generically from `item.item_data["properties"]`
/// as individual key-value pairs. Standard fields (verbatim_quote, page_number,
/// grounding_status) come from the ExtractionItemRecord itself.
pub async fn create_entity_node(
    txn: &mut neo4rs::Txn,
    item: &ExtractionItemRecord,
    doc_id: &str,
    seq: usize,
) -> Result<String, AppError> {
    let entity_type = &item.entity_type;

    // Validate label: alphanumeric + underscore only (prevent Cypher injection)
    if !entity_type.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(AppError::BadRequest {
            message: format!("Invalid entity type for Neo4j label: '{entity_type}'"),
            details: serde_json::json!({ "entity_type": entity_type }),
        });
    }

    // Generate a readable node ID from entity type + sequence number
    let neo4j_id = format!("{}-{seq:03}", slug(entity_type));

    // Extract title from the item label or first property
    let title = item.item_data["label"].as_str().unwrap_or("");

    // Build the CREATE query with all properties from item_data["properties"]
    // plus standard fields from the item record itself.
    //
    // ## Rust Learning: Building dynamic Cypher properties
    //
    // We set fixed fields (id, title, source_document, etc.) via named params,
    // then use SET n.key = $value for each schema-defined property.
    // This avoids a giant property map while keeping each field explicit.
    let mut cypher = format!(
        "CREATE (n:{entity_type} {{ id: $id, title: $title, \
         source_document: $doc, extraction_item_id: $ext_id"
    );

    // Add verbatim_quote and grounding fields if present
    if item.verbatim_quote.is_some() {
        cypher.push_str(", verbatim_quote: $quote");
    }
    if item.grounded_page.is_some() {
        cypher.push_str(", page_number: $page");
    }
    if item.grounding_status.is_some() {
        cypher.push_str(", grounding_status: $grounding");
    }

    cypher.push_str(" })");

    let mut q = query(&cypher)
        .param("id", neo4j_id.as_str())
        .param("title", title)
        .param("doc", doc_id)
        .param("ext_id", item.id as i64);

    if let Some(ref quote) = item.verbatim_quote {
        q = q.param("quote", quote.as_str());
    }
    if let Some(page) = item.grounded_page {
        q = q.param("page", page as i64);
    }
    if let Some(ref status) = item.grounding_status {
        q = q.param("grounding", status.as_str());
    }

    txn.run(q).await.map_err(|e| AppError::Internal {
        message: format!("Failed to create {entity_type} {neo4j_id}: {e}"),
    })?;

    // Set schema-defined properties from item_data["properties"]
    if let Some(props) = item.item_data.get("properties").and_then(|p| p.as_object()) {
        for (key, value) in props {
            // Skip properties that would conflict with standard fields
            if key == "verbatim_quote" || key == "page_number" {
                continue;
            }
            // Validate property name
            if !key.chars().all(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }

            let set_cypher = format!("MATCH (n {{ id: $id }}) SET n.{key} = $val");

            // Convert serde_json::Value to neo4rs param
            let set_q = match value {
                serde_json::Value::String(s) => {
                    Some(query(&set_cypher).param("id", neo4j_id.as_str()).param("val", s.as_str()))
                }
                serde_json::Value::Number(n) => {
                    n.as_i64()
                        .map(|i| query(&set_cypher).param("id", neo4j_id.as_str()).param("val", i))
                        .or_else(|| n.as_f64().map(|f| query(&set_cypher).param("id", neo4j_id.as_str()).param("val", f)))
                }
                serde_json::Value::Bool(b) => {
                    Some(query(&set_cypher).param("id", neo4j_id.as_str()).param("val", *b))
                }
                _ => None, // Skip arrays, objects, nulls
            };

            if let Some(set_q) = set_q {
                txn.run(set_q).await.map_err(|e| AppError::Internal {
                    message: format!("Failed to set property '{key}' on {neo4j_id}: {e}"),
                })?;
            }
        }
    }

    Ok(neo4j_id)
}

/// Create a relationship between two nodes inside a transaction.
/// Zero rows from MATCH = broken ID mapping = hard error (rolls back).
pub async fn create_ingest_relationship(
    txn: &mut neo4rs::Txn,
    from_id: &str,
    to_id: &str,
    rel_type: &str,
) -> Result<(), AppError> {
    let cypher = format!(
        "MATCH (a {{id: $from_id}}), (b {{id: $to_id}}) \
         CREATE (a)-[:{rel_type}]->(b) RETURN b.id"
    );

    let mut result = txn
        .execute(query(&cypher).param("from_id", from_id).param("to_id", to_id))
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Cypher failed for {rel_type} {from_id}->{to_id}: {e}"),
        })?;

    if result
        .next(&mut *txn)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Row fetch failed for {rel_type}: {e}"),
        })?
        .is_none()
    {
        return Err(AppError::Internal {
            message: format!(
                "Broken ID mapping: {rel_type} from '{from_id}' to '{to_id}' — \
                 MATCH found no nodes. Transaction will roll back."
            ),
        });
    }

    Ok(())
}

/// Create CONTAINED_IN from all non-Document nodes to the Document.
pub async fn create_contained_in_relationships(
    txn: &mut neo4rs::Txn,
    node_ids: &[String],
    doc_neo4j_id: &str,
) -> Result<usize, AppError> {
    for node_id in node_ids {
        create_ingest_relationship(txn, node_id, doc_neo4j_id, "CONTAINED_IN").await?;
    }
    Ok(node_ids.len())
}
