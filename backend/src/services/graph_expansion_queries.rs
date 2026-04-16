//! Per-type Neo4j expansion queries for the graph expander (part 1).
//!
//! Contains shared helpers and expansion functions for the three most
//! complex node types: Evidence, ComplaintAllegation, MotionClaim.
//!
//! ## Pattern: HashSet for deduplication
//! A shared `&mut HashSet<String>` tracks node IDs already collected.
//! Before adding any node, we check `seen.contains(&id)` and skip if
//! already present. This prevents duplicates when multiple seeds share
//! neighbors (e.g., two Evidence nodes from the same Document).

use neo4rs::{query, Graph};
use std::collections::{HashMap, HashSet};

use super::graph_expander::{ExpandedNode, ExpandedRelationship, GraphExpanderError};

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

    let cypher = "MATCH (e:Evidence {id: $id})
        OPTIONAL MATCH (e)-[:STATED_BY]->(speaker)
        OPTIONAL MATCH (e)-[:ABOUT]->(subject)
        OPTIONAL MATCH (e)-[:CONTAINED_IN]->(doc:Document)
        OPTIONAL MATCH (e)-[:CHARACTERIZES]->(allegation:ComplaintAllegation)
        OPTIONAL MATCH (e)<-[:REBUTS]-(rebuttal:Evidence)
        OPTIONAL MATCH (e)-[:CONTRADICTS]-(contradiction:Evidence)
        RETURN e.id AS eid, e.title AS etitle, e.verbatim_quote AS equote,
               e.significance AS esig, e.page_number AS epage,
               speaker.id AS sid, speaker.name AS sname,
               subject.id AS subid, subject.name AS subname,
               doc.id AS did, doc.title AS dtitle, doc.document_type AS dtype,
               allegation.id AS aid, allegation.title AS atitle,
               allegation.evidence_status AS astatus,
               rebuttal.id AS rid, rebuttal.title AS rtitle,
               contradiction.id AS cid, contradiction.title AS ctitle";

    let mut result = graph.execute(query(cypher).param("id", id)).await?;

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
            rels.push(ExpandedRelationship::new(id, &sid, "STATED_BY"));
            nodes.push(n);
        }

        let subid = get_str(&row, "subid");
        if let Some(n) = try_extract_node(&row, "subid", "Person", &[("subname", "name")], seen) {
            rels.push(ExpandedRelationship::new(id, &subid, "ABOUT"));
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
            rels.push(ExpandedRelationship::new(id, &did, "CONTAINED_IN"));
            nodes.push(n);
        }

        let aid = get_str(&row, "aid");
        if let Some(n) = try_extract_node(
            &row,
            "aid",
            "ComplaintAllegation",
            &[("atitle", "title"), ("astatus", "evidence_status")],
            seen,
        ) {
            rels.push(ExpandedRelationship::new(id, &aid, "CHARACTERIZES"));
            nodes.push(n);
        }

        let rid = get_str(&row, "rid");
        if let Some(n) = try_extract_node(&row, "rid", "Evidence", &[("rtitle", "title")], seen) {
            rels.push(ExpandedRelationship::new(&rid, id, "REBUTS"));
            nodes.push(n);
        }

        let cid = get_str(&row, "cid");
        if let Some(n) = try_extract_node(&row, "cid", "Evidence", &[("ctitle", "title")], seen) {
            rels.push(ExpandedRelationship::new(id, &cid, "CONTRADICTS"));
            nodes.push(n);
        }
    }

    Ok((nodes, rels))
}

// ---------------------------------------------------------------------------
// ComplaintAllegation expansion
// ---------------------------------------------------------------------------

/// Expand a ComplaintAllegation seed: claims, evidence, documents, counts, harms.
pub async fn expand_allegation(
    graph: &Graph,
    id: &str,
    seen: &mut HashSet<String>,
) -> Result<(Vec<ExpandedNode>, Vec<ExpandedRelationship>), GraphExpanderError> {
    let mut nodes = Vec::new();
    let mut rels = Vec::new();

    let cypher = "MATCH (a:ComplaintAllegation {id: $id})
        OPTIONAL MATCH (claim:MotionClaim)-[:PROVES]->(a)
        OPTIONAL MATCH (claim)-[:RELIES_ON]->(evidence:Evidence)
        OPTIONAL MATCH (evidence)-[:CONTAINED_IN]->(doc:Document)
        OPTIONAL MATCH (evidence)-[:STATED_BY]->(speaker)
        OPTIONAL MATCH (a)-[:SUPPORTS]->(count:LegalCount)
        OPTIONAL MATCH (harm:Harm)-[:CAUSED_BY]->(a)
        RETURN a.id AS aid, a.title AS atitle, a.evidence_status AS astatus,
               a.allegation AS aalleg,
               claim.id AS cid, claim.title AS ctitle,
               evidence.id AS eid, evidence.title AS etitle,
               evidence.verbatim_quote AS equote,
               doc.id AS did, doc.title AS dtitle,
               speaker.id AS sid, speaker.name AS sname,
               count.id AS lcid, count.title AS lctitle,
               harm.id AS hid, harm.title AS htitle, harm.amount AS hamount";

    let mut result = graph.execute(query(cypher).param("id", id)).await?;

    while let Some(row) = result.next().await? {
        if let Some(n) = try_extract_node(
            &row,
            "aid",
            "ComplaintAllegation",
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
            rels.push(ExpandedRelationship::new(&cid, id, "PROVES"));
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
                rels.push(ExpandedRelationship::new(&cid, &eid, "RELIES_ON"));
            }
            nodes.push(n);
        }

        let did = get_str(&row, "did");
        if let Some(n) = try_extract_node(&row, "did", "Document", &[("dtitle", "title")], seen) {
            if !eid.is_empty() {
                rels.push(ExpandedRelationship::new(&eid, &did, "CONTAINED_IN"));
            }
            nodes.push(n);
        }

        if let Some(n) = try_extract_node(&row, "sid", "Person", &[("sname", "name")], seen) {
            nodes.push(n);
        }

        let lcid = get_str(&row, "lcid");
        if let Some(n) = try_extract_node(&row, "lcid", "LegalCount", &[("lctitle", "title")], seen)
        {
            rels.push(ExpandedRelationship::new(id, &lcid, "SUPPORTS"));
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
            rels.push(ExpandedRelationship::new(&hid, id, "CAUSED_BY"));
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

    let cypher = "MATCH (m:MotionClaim {id: $id})
        OPTIONAL MATCH (m)-[:RELIES_ON]->(evidence:Evidence)
        OPTIONAL MATCH (evidence)-[:CONTAINED_IN]->(doc:Document)
        OPTIONAL MATCH (evidence)-[:STATED_BY]->(speaker)
        OPTIONAL MATCH (m)-[:PROVES]->(allegation:ComplaintAllegation)
        OPTIONAL MATCH (m)-[:APPEARS_IN]->(motion_doc:Document)
        RETURN m.id AS mid, m.title AS mtitle, m.claim_text AS mtext,
               m.significance AS msig,
               evidence.id AS eid, evidence.title AS etitle,
               evidence.verbatim_quote AS equote,
               doc.id AS did, doc.title AS dtitle,
               speaker.id AS sid, speaker.name AS sname,
               allegation.id AS aid, allegation.title AS atitle,
               motion_doc.id AS mdid, motion_doc.title AS mdtitle";

    let mut result = graph.execute(query(cypher).param("id", id)).await?;

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
            rels.push(ExpandedRelationship::new(id, &eid, "RELIES_ON"));
            nodes.push(n);
        }

        let did = get_str(&row, "did");
        if let Some(n) = try_extract_node(&row, "did", "Document", &[("dtitle", "title")], seen) {
            if !eid.is_empty() {
                rels.push(ExpandedRelationship::new(&eid, &did, "CONTAINED_IN"));
            }
            nodes.push(n);
        }

        if let Some(n) = try_extract_node(&row, "sid", "Person", &[("sname", "name")], seen) {
            nodes.push(n);
        }

        let aid = get_str(&row, "aid");
        if let Some(n) = try_extract_node(
            &row,
            "aid",
            "ComplaintAllegation",
            &[("atitle", "title")],
            seen,
        ) {
            rels.push(ExpandedRelationship::new(id, &aid, "PROVES"));
            nodes.push(n);
        }

        let mdid = get_str(&row, "mdid");
        if let Some(n) = try_extract_node(&row, "mdid", "Document", &[("mdtitle", "title")], seen) {
            rels.push(ExpandedRelationship::new(id, &mdid, "APPEARS_IN"));
            nodes.push(n);
        }
    }

    Ok((nodes, rels))
}
