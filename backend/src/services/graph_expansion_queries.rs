//! Per-type Neo4j expansion queries for the graph expander (part 1).
//!
//! Contains shared helpers and expansion functions for the three most
//! complex node types: Evidence, Allegation, MotionClaim.
//!
//! ## Pattern: HashSet for deduplication
//! A shared `&mut HashSet<String>` tracks node IDs already collected.
//! Before adding any node, we check `seen.contains(&id)` and skip if
//! already present. This prevents duplicates when multiple seeds share
//! neighbors (e.g., two Evidence nodes from the same Document).

use neo4rs::{query, Graph};
use std::collections::{HashMap, HashSet};

use super::graph_expander::{ExpandedNode, ExpandedRelationship, GraphExpanderError};
use super::graph_expansion_cypher::{
    allegation_expansion_cypher, evidence_expansion_cypher, motion_claim_expansion_cypher,
};
use crate::neo4j::schema;

/// Safe extraction: get a String from a Neo4j row, returning "" if null/missing.
pub(crate) fn get_str(row: &neo4rs::Row, key: &str) -> String {
    row.get(key).unwrap_or_default()
}

/// Try to extract a node from the row at the given column prefix.
/// Returns None if the id column is empty (meaning OPTIONAL MATCH didn't find it).
pub(crate) fn try_extract_node(
    row: &neo4rs::Row,
    id_col: &str,
    node_type: &str,
    prop_cols: &[(&str, &str)], // (row_column, property_name) pairs
    seen: &mut HashSet<String>,
) -> Option<ExpandedNode> {
    let id: String = row.get(id_col).unwrap_or_default();
    if id.is_empty() || seen.contains(&id) {
        return None;
    }
    seen.insert(id.clone());

    let mut properties = HashMap::new();
    for (col, prop_name) in prop_cols {
        let val: String = row.get(col).unwrap_or_default();
        if !val.is_empty() {
            properties.insert(prop_name.to_string(), val);
        }
    }

    let title = properties
        .get("title")
        .or_else(|| properties.get("name"))
        .cloned()
        .unwrap_or_default();

    Some(ExpandedNode {
        id,
        node_type: node_type.to_string(),
        title,
        properties,
    })
}

// ---------------------------------------------------------------------------
// Evidence expansion
// ---------------------------------------------------------------------------

/// Expand an Evidence seed: speaker, subject, document, allegation, rebuttals.
pub async fn expand_evidence(
    graph: &Graph,
    id: &str,
    seen: &mut HashSet<String>,
) -> Result<(Vec<ExpandedNode>, Vec<ExpandedRelationship>), GraphExpanderError> {
    let mut nodes = Vec::new();
    let mut rels = Vec::new();

    // Cypher (with its relationship types interpolated from `neo4j::schema`)
    // lives in `graph_expansion_cypher` so this module stays under the
    // 300-line cap; the rendered edge labels below still come from `schema`.
    let cypher = evidence_expansion_cypher();
    let mut result = graph.execute(query(&cypher).param("id", id)).await?;

    while let Some(row) = result.next().await? {
        if let Some(n) = try_extract_node(
            &row,
            "eid",
            "Evidence",
            &[
                ("etitle", "title"),
                ("equote", "verbatim_quote"),
                ("esig", "significance"),
                ("epage", "page_number"),
            ],
            seen,
        ) {
            nodes.push(n);
        }

        let sid = get_str(&row, "sid");
        if let Some(n) = try_extract_node(&row, "sid", "Person", &[("sname", "name")], seen) {
            rels.push(ExpandedRelationship::new(id, &sid, schema::STATED_BY));
            nodes.push(n);
        }

        let subid = get_str(&row, "subid");
        if let Some(n) = try_extract_node(&row, "subid", "Person", &[("subname", "name")], seen) {
            rels.push(ExpandedRelationship::new(id, &subid, schema::ABOUT));
            nodes.push(n);
        }

        let did = get_str(&row, "did");
        if let Some(n) = try_extract_node(
            &row,
            "did",
            "Document",
            &[("dtitle", "title"), ("dtype", "document_type")],
            seen,
        ) {
            rels.push(ExpandedRelationship::new(id, &did, schema::CONTAINED_IN));
            nodes.push(n);
        }

        let aid = get_str(&row, "aid");
        if let Some(n) = try_extract_node(
            &row,
            "aid",
            "Allegation",
            &[("atitle", "title"), ("astatus", "evidence_status")],
            seen,
        ) {
            rels.push(ExpandedRelationship::new(id, &aid, schema::CHARACTERIZES));
            nodes.push(n);
        }

        let rid = get_str(&row, "rid");
        if let Some(n) = try_extract_node(&row, "rid", "Evidence", &[("rtitle", "title")], seen) {
            rels.push(ExpandedRelationship::new(&rid, id, schema::REBUTS));
            nodes.push(n);
        }

        let cid = get_str(&row, "cid");
        if let Some(n) = try_extract_node(&row, "cid", "Evidence", &[("ctitle", "title")], seen) {
            rels.push(ExpandedRelationship::new(id, &cid, schema::CONTRADICTS));
            nodes.push(n);
        }
    }

    Ok((nodes, rels))
}

// ---------------------------------------------------------------------------
// Allegation expansion
// ---------------------------------------------------------------------------

/// Expand an Allegation seed: claims, evidence, documents, counts, harms.
///
/// v5.1 migration (parallel to the repositories migration in 2b51d38):
///   - Label `:ComplaintAllegation` → `:Allegation`.
///   - Direct `:SUPPORTS` edge → two-hop through Element via
///     `:BEARS_ON` and `:HAS_ELEMENT`.
///   - Property `a.evidence_status` dropped (NULL).
///   - Property `a.allegation` (v4 prose) → `a.summary`.
///   - `a.title` kept (v5.1 has the short-label property).
///
/// `RETURN DISTINCT` dedupes the cartesian fan-out from the two-hop
/// (one Allegation bearing on multiple Elements of the same Count).
/// Rust-side `seen: HashSet<String>` would catch it too, but Cypher-side
/// dedup matches the migration discipline.
pub async fn expand_allegation(
    graph: &Graph,
    id: &str,
    seen: &mut HashSet<String>,
) -> Result<(Vec<ExpandedNode>, Vec<ExpandedRelationship>), GraphExpanderError> {
    let mut nodes = Vec::new();
    let mut rels = Vec::new();

    let cypher = allegation_expansion_cypher();
    let mut result = graph.execute(query(&cypher).param("id", id)).await?;

    while let Some(row) = result.next().await? {
        if let Some(n) = try_extract_node(
            &row,
            "aid",
            "Allegation",
            &[
                ("atitle", "title"),
                ("astatus", "evidence_status"),
                ("aalleg", "allegation"),
            ],
            seen,
        ) {
            nodes.push(n);
        }

        let cid = get_str(&row, "cid");
        if let Some(n) = try_extract_node(&row, "cid", "MotionClaim", &[("ctitle", "title")], seen)
        {
            rels.push(ExpandedRelationship::new(&cid, id, schema::PROVES));
            nodes.push(n);
        }

        let eid = get_str(&row, "eid");
        if let Some(n) = try_extract_node(
            &row,
            "eid",
            "Evidence",
            &[("etitle", "title"), ("equote", "verbatim_quote")],
            seen,
        ) {
            if !cid.is_empty() {
                rels.push(ExpandedRelationship::new(&cid, &eid, schema::RELIES_ON));
            }
            nodes.push(n);
        }

        let did = get_str(&row, "did");
        if let Some(n) = try_extract_node(&row, "did", "Document", &[("dtitle", "title")], seen) {
            if !eid.is_empty() {
                rels.push(ExpandedRelationship::new(&eid, &did, schema::CONTAINED_IN));
            }
            nodes.push(n);
        }

        if let Some(n) = try_extract_node(&row, "sid", "Person", &[("sname", "name")], seen) {
            nodes.push(n);
        }

        let lcid = get_str(&row, "lcid");
        if let Some(n) = try_extract_node(&row, "lcid", "LegalCount", &[("lctitle", "title")], seen)
        {
            rels.push(ExpandedRelationship::new(id, &lcid, schema::SUPPORTS));
            nodes.push(n);
        }

        let hid = get_str(&row, "hid");
        if let Some(n) = try_extract_node(
            &row,
            "hid",
            "Harm",
            &[("htitle", "title"), ("hamount", "amount")],
            seen,
        ) {
            rels.push(ExpandedRelationship::new(&hid, id, schema::CAUSED_BY));
            nodes.push(n);
        }
    }

    Ok((nodes, rels))
}

// ---------------------------------------------------------------------------
// MotionClaim expansion
// ---------------------------------------------------------------------------

/// Expand a MotionClaim seed: evidence, documents, speakers, allegation.
pub async fn expand_motion_claim(
    graph: &Graph,
    id: &str,
    seen: &mut HashSet<String>,
) -> Result<(Vec<ExpandedNode>, Vec<ExpandedRelationship>), GraphExpanderError> {
    let mut nodes = Vec::new();
    let mut rels = Vec::new();

    let cypher = motion_claim_expansion_cypher();
    let mut result = graph.execute(query(&cypher).param("id", id)).await?;

    while let Some(row) = result.next().await? {
        if let Some(n) = try_extract_node(
            &row,
            "mid",
            "MotionClaim",
            &[
                ("mtitle", "title"),
                ("mtext", "claim_text"),
                ("msig", "significance"),
            ],
            seen,
        ) {
            nodes.push(n);
        }

        let eid = get_str(&row, "eid");
        if let Some(n) = try_extract_node(
            &row,
            "eid",
            "Evidence",
            &[("etitle", "title"), ("equote", "verbatim_quote")],
            seen,
        ) {
            rels.push(ExpandedRelationship::new(id, &eid, schema::RELIES_ON));
            nodes.push(n);
        }

        let did = get_str(&row, "did");
        if let Some(n) = try_extract_node(&row, "did", "Document", &[("dtitle", "title")], seen) {
            if !eid.is_empty() {
                rels.push(ExpandedRelationship::new(&eid, &did, schema::CONTAINED_IN));
            }
            nodes.push(n);
        }

        if let Some(n) = try_extract_node(&row, "sid", "Person", &[("sname", "name")], seen) {
            nodes.push(n);
        }

        let aid = get_str(&row, "aid");
        if let Some(n) = try_extract_node(&row, "aid", "Allegation", &[("atitle", "title")], seen) {
            rels.push(ExpandedRelationship::new(id, &aid, schema::PROVES));
            nodes.push(n);
        }

        let mdid = get_str(&row, "mdid");
        if let Some(n) = try_extract_node(&row, "mdid", "Document", &[("mdtitle", "title")], seen) {
            rels.push(ExpandedRelationship::new(id, &mdid, schema::APPEARS_IN));
            nodes.push(n);
        }
    }

    Ok((nodes, rels))
}
