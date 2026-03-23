//! Helper functions for the evidence import endpoint.
//!
//! Extracted from `admin_evidence.rs` to keep it under 300 lines.
//! These helpers create Neo4j relationships within an explicit transaction.

use neo4rs::query;

/// Create a relationship between two nodes matched by label + id.
///
/// Uses MATCH on both endpoints so the query returns 0 rows if the target
/// doesn't exist. We detect this via the RETURN clause and bubble up an
/// error string.
///
/// `source_label` can be empty to match any label (used when the source was
/// just created and we know it exists).
///
/// ## Rust Learning: Why return `Result<(), String>` instead of `AppError`?
///
/// This helper is called from many contexts with different error messages.
/// Returning a plain `String` lets the caller wrap it in the appropriate
/// `AppError` variant (BadRequest for missing targets, Internal for
/// unexpected failures). This keeps the helper generic and reusable.
pub async fn create_relationship(
    txn: &mut neo4rs::Txn,
    source_id: &str,
    target_id: &str,
    source_label: &str,
    target_label: &str,
    rel_type: &str,
    props: Option<&serde_json::Value>,
) -> Result<(), String> {
    // Build Cypher dynamically — relationship type and labels can't be parameterized.
    let prop_clause = match props {
        Some(val) => {
            let topic = val.get("topic").and_then(|v| v.as_str()).unwrap_or("");
            let value = val.get("value").and_then(|v| v.as_str()).unwrap_or("");
            format!(" {{topic: '{topic}', value: '{value}'}}")
        }
        None => String::new(),
    };

    let src = if source_label.is_empty() {
        "(src {id: $src_id})".to_string()
    } else {
        format!("(src:{source_label} {{id: $src_id}})")
    };

    let tgt = format!("(tgt:{target_label} {{id: $tgt_id}})");

    let cypher = format!(
        "MATCH {src}, {tgt} CREATE (src)-[:{rel_type}{prop_clause}]->(tgt) RETURN tgt.id"
    );

    let mut result = txn
        .execute(query(&cypher).param("src_id", source_id).param("tgt_id", target_id))
        .await
        .map_err(|e| format!("Cypher failed for {rel_type}: {e}"))?;

    if result
        .next(&mut *txn)
        .await
        .map_err(|e| format!("Row fetch failed: {e}"))?
        .is_none()
    {
        return Err(format!("{target_label} '{target_id}' not found"));
    }

    Ok(())
}

/// Create a relationship from an Evidence node to any node matched by id only.
///
/// Used for STATED_BY and ABOUT where the target can be a Person or Organization.
/// Omits the label on the MATCH so it finds either node type.
pub async fn create_relationship_labelless_target(
    txn: &mut neo4rs::Txn,
    source_id: &str,
    target_id: &str,
    rel_type: &str,
) -> Result<(), String> {
    let cypher = format!(
        "MATCH (src:Evidence {{id: $src_id}}), (tgt {{id: $tgt_id}}) \
         CREATE (src)-[:{rel_type}]->(tgt) RETURN tgt.id"
    );

    let mut result = txn
        .execute(query(&cypher).param("src_id", source_id).param("tgt_id", target_id))
        .await
        .map_err(|e| format!("Cypher failed for {rel_type}: {e}"))?;

    if result
        .next(&mut *txn)
        .await
        .map_err(|e| format!("Row fetch failed: {e}"))?
        .is_none()
    {
        return Err(format!("Node '{target_id}' not found for {rel_type}"));
    }

    Ok(())
}
