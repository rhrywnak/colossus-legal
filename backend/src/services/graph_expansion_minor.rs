//! Per-type Neo4j expansion queries for the graph expander (part 2).
//!
//! Contains expansion functions for the simpler/smaller node types:
//! Harm, Document, Person, Organization.

use neo4rs::{query, Graph};
use std::collections::HashSet;

use super::graph_expander::{ExpandedNode, ExpandedRelationship, GraphExpanderError};
use super::graph_expansion_queries::{get_str, try_extract_node};

// ---------------------------------------------------------------------------
// Harm expansion
// ---------------------------------------------------------------------------

/// Expand a Harm seed: allegation, evidence, documents, legal count.
pub async fn expand_harm(
    graph: &Graph,
    id: &str,
    seen: &mut HashSet<String>,
) -> Result<(Vec<ExpandedNode>, Vec<ExpandedRelationship>), GraphExpanderError> {
    let mut nodes = Vec::new();
    let mut rels = Vec::new();

    let cypher = "MATCH (h:Harm {id: $id})
        OPTIONAL MATCH (h)-[:CAUSED_BY]->(allegation:ComplaintAllegation)
        OPTIONAL MATCH (h)-[:EVIDENCED_BY]->(evidence:Evidence)
        OPTIONAL MATCH (evidence)-[:CONTAINED_IN]->(doc:Document)
        OPTIONAL MATCH (h)-[:DAMAGES_FOR]->(count:LegalCount)
        RETURN h.id AS hid, h.title AS htitle, h.description AS hdesc,
               h.amount AS hamount,
               allegation.id AS aid, allegation.title AS atitle,
               evidence.id AS eid, evidence.title AS etitle,
               evidence.verbatim_quote AS equote,
               doc.id AS did, doc.title AS dtitle,
               count.id AS cid, count.title AS ctitle";

    let mut result = graph.execute(query(cypher).param("id", id)).await?;

    while let Some(row) = result.next().await? {
        if let Some(n) = try_extract_node(
            &row,
            "hid",
            "Harm",
            &[
                ("htitle", "title"),
                ("hdesc", "description"),
                ("hamount", "amount"),
            ],
            seen,
        ) {
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
            rels.push(ExpandedRelationship::new(id, &aid, "CAUSED_BY"));
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
            rels.push(ExpandedRelationship::new(id, &eid, "EVIDENCED_BY"));
            nodes.push(n);
        }

        let did = get_str(&row, "did");
        if let Some(n) = try_extract_node(&row, "did", "Document", &[("dtitle", "title")], seen) {
            if !eid.is_empty() {
                rels.push(ExpandedRelationship::new(&eid, &did, "CONTAINED_IN"));
            }
            nodes.push(n);
        }

        let cid = get_str(&row, "cid");
        if let Some(n) = try_extract_node(&row, "cid", "LegalCount", &[("ctitle", "title")], seen) {
            rels.push(ExpandedRelationship::new(id, &cid, "DAMAGES_FOR"));
            nodes.push(n);
        }
    }

    Ok((nodes, rels))
}

// ---------------------------------------------------------------------------
// Document expansion
// ---------------------------------------------------------------------------

/// Expand a Document seed: evidence contained in it, speakers. LIMIT 20.
pub async fn expand_document(
    graph: &Graph,
    id: &str,
    seen: &mut HashSet<String>,
) -> Result<(Vec<ExpandedNode>, Vec<ExpandedRelationship>), GraphExpanderError> {
    let mut nodes = Vec::new();
    let mut rels = Vec::new();

    let cypher = "MATCH (d:Document {id: $id})
        OPTIONAL MATCH (evidence:Evidence)-[:CONTAINED_IN]->(d)
        OPTIONAL MATCH (evidence)-[:STATED_BY]->(speaker)
        RETURN d.id AS did, d.title AS dtitle, d.document_type AS dtype,
               evidence.id AS eid, evidence.title AS etitle,
               speaker.id AS sid, speaker.name AS sname
        LIMIT 20";

    let mut result = graph.execute(query(cypher).param("id", id)).await?;

    while let Some(row) = result.next().await? {
        if let Some(n) = try_extract_node(
            &row,
            "did",
            "Document",
            &[("dtitle", "title"), ("dtype", "document_type")],
            seen,
        ) {
            nodes.push(n);
        }

        let eid = get_str(&row, "eid");
        if let Some(n) = try_extract_node(&row, "eid", "Evidence", &[("etitle", "title")], seen) {
            rels.push(ExpandedRelationship::new(&eid, id, "CONTAINED_IN"));
            nodes.push(n);
        }

        if let Some(n) = try_extract_node(&row, "sid", "Person", &[("sname", "name")], seen) {
            nodes.push(n);
        }
    }

    Ok((nodes, rels))
}

// ---------------------------------------------------------------------------
// Person expansion
// ---------------------------------------------------------------------------

/// Expand a Person seed: evidence stated by them, documents. LIMIT 15.
pub async fn expand_person(
    graph: &Graph,
    id: &str,
    seen: &mut HashSet<String>,
) -> Result<(Vec<ExpandedNode>, Vec<ExpandedRelationship>), GraphExpanderError> {
    let mut nodes = Vec::new();
    let mut rels = Vec::new();

    let cypher = "MATCH (p:Person {id: $id})
        OPTIONAL MATCH (evidence:Evidence)-[:STATED_BY]->(p)
        OPTIONAL MATCH (evidence)-[:CONTAINED_IN]->(doc:Document)
        RETURN p.id AS pid, p.name AS pname, p.role AS prole,
               p.description AS pdesc,
               evidence.id AS eid, evidence.title AS etitle,
               doc.id AS did, doc.title AS dtitle
        LIMIT 15";

    let mut result = graph.execute(query(cypher).param("id", id)).await?;

    while let Some(row) = result.next().await? {
        if let Some(n) = try_extract_node(
            &row,
            "pid",
            "Person",
            &[
                ("pname", "name"),
                ("prole", "role"),
                ("pdesc", "description"),
            ],
            seen,
        ) {
            nodes.push(n);
        }

        let eid = get_str(&row, "eid");
        if let Some(n) = try_extract_node(&row, "eid", "Evidence", &[("etitle", "title")], seen) {
            rels.push(ExpandedRelationship::new(&eid, id, "STATED_BY"));
            nodes.push(n);
        }

        let did = get_str(&row, "did");
        if let Some(n) = try_extract_node(&row, "did", "Document", &[("dtitle", "title")], seen) {
            if !eid.is_empty() {
                rels.push(ExpandedRelationship::new(&eid, &did, "CONTAINED_IN"));
            }
            nodes.push(n);
        }
    }

    Ok((nodes, rels))
}

// ---------------------------------------------------------------------------
// Organization expansion
// ---------------------------------------------------------------------------

/// Expand an Organization seed: same pattern as Person.
pub async fn expand_organization(
    graph: &Graph,
    id: &str,
    seen: &mut HashSet<String>,
) -> Result<(Vec<ExpandedNode>, Vec<ExpandedRelationship>), GraphExpanderError> {
    let mut nodes = Vec::new();
    let mut rels = Vec::new();

    let cypher = "MATCH (o:Organization {id: $id})
        OPTIONAL MATCH (evidence:Evidence)-[:STATED_BY]->(o)
        OPTIONAL MATCH (evidence)-[:CONTAINED_IN]->(doc:Document)
        RETURN o.id AS oid, o.name AS oname, o.role AS orole,
               evidence.id AS eid, evidence.title AS etitle,
               doc.id AS did, doc.title AS dtitle
        LIMIT 15";

    let mut result = graph.execute(query(cypher).param("id", id)).await?;

    while let Some(row) = result.next().await? {
        if let Some(n) = try_extract_node(
            &row,
            "oid",
            "Organization",
            &[("oname", "name"), ("orole", "role")],
            seen,
        ) {
            nodes.push(n);
        }

        let eid = get_str(&row, "eid");
        if let Some(n) = try_extract_node(&row, "eid", "Evidence", &[("etitle", "title")], seen) {
            rels.push(ExpandedRelationship::new(&eid, id, "STATED_BY"));
            nodes.push(n);
        }

        let did = get_str(&row, "did");
        if let Some(n) = try_extract_node(&row, "did", "Document", &[("dtitle", "title")], seen) {
            if !eid.is_empty() {
                rels.push(ExpandedRelationship::new(&eid, &did, "CONTAINED_IN"));
            }
            nodes.push(n);
        }
    }

    Ok((nodes, rels))
}
