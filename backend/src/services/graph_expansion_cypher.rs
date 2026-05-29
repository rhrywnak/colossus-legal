//! Cypher builders for the three complex graph-expansion seed types.
//!
//! Split out of `graph_expansion_queries.rs` to keep that module under the
//! 300 code-line cap (CLAUDE.md §4-17) once the relationship types became
//! `format!`-interpolated `neo4j::schema` constants. Same precedent as
//! `graph_expansion_minor.rs` (extracted from the same parent).
//!
//! Each function returns a ready-to-run Cypher string. The relationship types
//! are interpolated from [`crate::neo4j::schema`] so the expansion queries and
//! the repository reads reference one set of constants. Node-property maps
//! (`{id: $id}`) are written with doubled braces (`{{id: $id}}`) because
//! `format!` treats single braces as placeholder syntax; only the lowercase
//! `{name}` placeholders are filled from `schema`.

use crate::neo4j::schema;

/// Evidence seed expansion: speaker, subject, document, allegation, rebuttals,
/// contradictions.
pub(super) fn evidence_expansion_cypher() -> String {
    format!(
        "MATCH (e:Evidence {{id: $id}})
        OPTIONAL MATCH (e)-[:{stated_by}]->(speaker)
        OPTIONAL MATCH (e)-[:{about}]->(subject)
        OPTIONAL MATCH (e)-[:{contained_in}]->(doc:Document)
        OPTIONAL MATCH (e)-[:{characterizes}]->(allegation:Allegation)
        OPTIONAL MATCH (e)<-[:{rebuts}]-(rebuttal:Evidence)
        OPTIONAL MATCH (e)-[:{contradicts}]-(contradiction:Evidence)
        RETURN e.id AS eid, e.title AS etitle, e.verbatim_quote AS equote,
               e.significance AS esig, e.page_number AS epage,
               speaker.id AS sid, speaker.name AS sname,
               subject.id AS subid, subject.name AS subname,
               doc.id AS did, doc.title AS dtitle, doc.document_type AS dtype,
               allegation.id AS aid, allegation.title AS atitle,
               NULL AS astatus,
               rebuttal.id AS rid, rebuttal.title AS rtitle,
               contradiction.id AS cid, contradiction.title AS ctitle",
        stated_by = schema::STATED_BY,
        about = schema::ABOUT,
        contained_in = schema::CONTAINED_IN,
        characterizes = schema::CHARACTERIZES,
        rebuts = schema::REBUTS,
        contradicts = schema::CONTRADICTS,
    )
}

/// Allegation seed expansion: motion claims, evidence, documents, speakers,
/// the two-hop path to LegalCount via Element, and harms.
///
/// v5.1: the direct `:SUPPORTS` edge became a two-hop through Element via
/// `:BEARS_ON` and `:HAS_ELEMENT`. `RETURN DISTINCT` dedupes the cartesian
/// fan-out (one Allegation bearing on multiple Elements of the same Count).
pub(super) fn allegation_expansion_cypher() -> String {
    format!(
        "MATCH (a:Allegation {{id: $id}})
        OPTIONAL MATCH (claim:MotionClaim)-[:{proves}]->(a)
        OPTIONAL MATCH (claim)-[:{relies_on}]->(evidence:Evidence)
        OPTIONAL MATCH (evidence)-[:{contained_in}]->(doc:Document)
        OPTIONAL MATCH (evidence)-[:{stated_by}]->(speaker)
        OPTIONAL MATCH (a)-[:{bears_on}]->(el)
                        <-[:{has_element}]-(count:LegalCount)
        OPTIONAL MATCH (harm:Harm)-[:{caused_by}]->(a)
        RETURN DISTINCT a.id AS aid, a.title AS atitle,
               NULL AS astatus,
               a.summary AS aalleg,
               claim.id AS cid, claim.title AS ctitle,
               evidence.id AS eid, evidence.title AS etitle,
               evidence.verbatim_quote AS equote,
               doc.id AS did, doc.title AS dtitle,
               speaker.id AS sid, speaker.name AS sname,
               count.id AS lcid, count.title AS lctitle,
               harm.id AS hid, harm.title AS htitle, harm.amount AS hamount",
        proves = schema::PROVES,
        relies_on = schema::RELIES_ON,
        contained_in = schema::CONTAINED_IN,
        stated_by = schema::STATED_BY,
        bears_on = schema::BEARS_ON,
        has_element = schema::HAS_ELEMENT,
        caused_by = schema::CAUSED_BY,
    )
}

/// MotionClaim seed expansion: evidence, documents, speakers, allegation, and
/// the motion document the claim appears in.
pub(super) fn motion_claim_expansion_cypher() -> String {
    format!(
        "MATCH (m:MotionClaim {{id: $id}})
        OPTIONAL MATCH (m)-[:{relies_on}]->(evidence:Evidence)
        OPTIONAL MATCH (evidence)-[:{contained_in}]->(doc:Document)
        OPTIONAL MATCH (evidence)-[:{stated_by}]->(speaker)
        OPTIONAL MATCH (m)-[:{proves}]->(allegation:Allegation)
        OPTIONAL MATCH (m)-[:{appears_in}]->(motion_doc:Document)
        RETURN m.id AS mid, m.title AS mtitle, m.claim_text AS mtext,
               m.significance AS msig,
               evidence.id AS eid, evidence.title AS etitle,
               evidence.verbatim_quote AS equote,
               doc.id AS did, doc.title AS dtitle,
               speaker.id AS sid, speaker.name AS sname,
               allegation.id AS aid, allegation.title AS atitle,
               motion_doc.id AS mdid, motion_doc.title AS mdtitle",
        relies_on = schema::RELIES_ON,
        contained_in = schema::CONTAINED_IN,
        stated_by = schema::STATED_BY,
        proves = schema::PROVES,
        appears_in = schema::APPEARS_IN,
    )
}
