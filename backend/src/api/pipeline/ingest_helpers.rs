//! Helper functions for the graph ingest endpoint.
//!
//! Extracted from `ingest.rs` to keep it under 300 lines.
//!
//! ## Rust Learning
//!
//! - **HashMap<i32, String>**: Maps PG extraction_item IDs → Neo4j string IDs.
//!   Built during node creation; used when creating relationships. All nodes
//!   must be created before relationships so the map is fully populated.
//! - **txn.run()** discards the result — good for CREATE. **txn.execute()**
//!   returns a RowStream so we can verify MATCH found nodes (used for rels).

use std::collections::{HashMap, HashSet};

use neo4rs::query;

use crate::error::AppError;
use crate::repositories::pipeline_repository::ExtractionItemRecord;

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
            "CREATE (d:Document {
                id: $id, title: $title, source_document_id: $source_id,
                doc_type: $doc_type, status: 'INGESTED',
                ingested_at: datetime()
            })",
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

/// Create Person and Organization nodes. Deduplicates by slug.
/// Returns (person_count, org_count).
pub async fn create_party_nodes(
    txn: &mut neo4rs::Txn,
    items: &[ExtractionItemRecord],
    doc_id: &str,
    pg_to_neo4j: &mut HashMap<i32, String>,
) -> Result<(usize, usize), AppError> {
    let mut seen: HashSet<String> = HashSet::new();
    let (mut persons, mut orgs) = (0usize, 0usize);

    for item in items.iter().filter(|i| i.entity_type == "Party") {
        let props = &item.item_data["properties"];
        let name = props["party_name"].as_str().unwrap_or("unknown");
        let role = props["role"].as_str().unwrap_or("");
        let party_type = props["party_type"].as_str().unwrap_or("individual");

        let (label, prefix) = if party_type == "organization" {
            ("Organization", "org")
        } else {
            ("Person", "person")
        };

        let neo4j_id = format!("{prefix}-{}", slug(name));
        pg_to_neo4j.insert(item.id, neo4j_id.clone());

        // Skip if we already created this node (dedup by slug)
        if !seen.insert(neo4j_id.clone()) {
            continue;
        }

        let cypher = format!(
            "CREATE (n:{label} {{id: $id, name: $name, role: $role, source_document: $doc}})"
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
            message: format!("Failed to create {label} '{name}': {e}"),
        })?;

        match party_type {
            "organization" => orgs += 1,
            _ => persons += 1,
        }
    }
    Ok((persons, orgs))
}

/// Create ComplaintAllegation nodes from FactualAllegation items.
pub async fn create_allegation_nodes(
    txn: &mut neo4rs::Txn,
    items: &[ExtractionItemRecord],
    doc_id: &str,
    pg_to_neo4j: &mut HashMap<i32, String>,
) -> Result<usize, AppError> {
    let mut seq = 0usize;

    for item in items.iter().filter(|i| i.entity_type == "FactualAllegation") {
        seq += 1;
        let neo4j_id = format!("complaint-allegation-{seq:03}");
        pg_to_neo4j.insert(item.id, neo4j_id.clone());

        let props = &item.item_data["properties"];
        let label_text = item.item_data["label"].as_str().unwrap_or("");
        let allegation = props["allegation_text"].as_str().unwrap_or("");
        let paragraph_ref = props["paragraph_ref"].as_str().unwrap_or("");

        txn.run(
            query(
                "CREATE (n:ComplaintAllegation {
                    id: $id, title: $title, allegation: $allegation,
                    verbatim_quote: $quote, page_number: $page,
                    paragraph_ref: $para_ref, grounding_status: $grounding,
                    source_document: $doc, extraction_item_id: $ext_id
                })",
            )
            .param("id", neo4j_id.as_str())
            .param("title", label_text)
            .param("allegation", allegation)
            .param("quote", item.verbatim_quote.clone())
            .param("page", item.grounded_page)
            .param("para_ref", paragraph_ref)
            .param("grounding", item.grounding_status.clone())
            .param("doc", doc_id)
            .param("ext_id", item.id as i64),
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to create ComplaintAllegation {neo4j_id}: {e}"),
        })?;
    }
    Ok(seq)
}

/// Create LegalCount nodes.
pub async fn create_count_nodes(
    txn: &mut neo4rs::Txn,
    items: &[ExtractionItemRecord],
    doc_id: &str,
    pg_to_neo4j: &mut HashMap<i32, String>,
) -> Result<usize, AppError> {
    let mut count = 0usize;

    for item in items.iter().filter(|i| i.entity_type == "LegalCount") {
        count += 1;
        let props = &item.item_data["properties"];
        let count_number = props["count_number"].as_i64().unwrap_or(count as i64);
        let neo4j_id = format!("legal-count-{count_number}");
        pg_to_neo4j.insert(item.id, neo4j_id.clone());

        let label_text = item.item_data["label"].as_str().unwrap_or("");
        let legal_basis = props["legal_basis"].as_str().unwrap_or("");

        txn.run(
            query(
                "CREATE (n:LegalCount {
                    id: $id, title: $title, count_number: $num,
                    legal_basis: $basis, source_document: $doc
                })",
            )
            .param("id", neo4j_id.as_str())
            .param("title", label_text)
            .param("num", count_number)
            .param("basis", legal_basis)
            .param("doc", doc_id),
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to create LegalCount {neo4j_id}: {e}"),
        })?;
    }
    Ok(count)
}

/// Create Harm nodes from DamagesClaim items.
pub async fn create_harm_nodes(
    txn: &mut neo4rs::Txn,
    items: &[ExtractionItemRecord],
    doc_id: &str,
    pg_to_neo4j: &mut HashMap<i32, String>,
) -> Result<usize, AppError> {
    let mut seq = 0usize;

    for item in items.iter().filter(|i| i.entity_type == "DamagesClaim") {
        seq += 1;
        let neo4j_id = format!("harm-{seq:03}");
        pg_to_neo4j.insert(item.id, neo4j_id.clone());

        let props = &item.item_data["properties"];
        let label_text = item.item_data["label"].as_str().unwrap_or("");
        let description = props["claim_text"].as_str().unwrap_or("");
        let amount = props["amount"].as_str().unwrap_or("");
        let damages_type = props["damages_type"].as_str().unwrap_or("");

        txn.run(
            query(
                "CREATE (n:Harm {
                    id: $id, title: $title, description: $desc,
                    amount: $amount, damages_type: $dtype,
                    verbatim_quote: $quote, page_number: $page,
                    source_document: $doc, extraction_item_id: $ext_id
                })",
            )
            .param("id", neo4j_id.as_str())
            .param("title", label_text)
            .param("desc", description)
            .param("amount", amount)
            .param("dtype", damages_type)
            .param("quote", item.verbatim_quote.clone())
            .param("page", item.grounded_page)
            .param("doc", doc_id)
            .param("ext_id", item.id as i64),
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to create Harm {neo4j_id}: {e}"),
        })?;
    }
    Ok(seq)
}

/// Create a relationship between two nodes inside a transaction.
/// Uses `txn.execute()` + RETURN to verify the MATCH found both nodes.
/// Zero rows = broken ID mapping = hard error (transaction rolls back).
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

/// Create CONTAINED_IN relationships from all non-Document nodes to the Document.
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

