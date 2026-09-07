//! The Element-detail Cypher, in its own file.
//!
//! Split out of `element_detail_repository.rs` when PROOF_MATRIX_v2 added nine
//! projected columns per evidence leg: the query is now the largest single thing
//! in that read, and keeping it there pushed the module past the 300-line limit
//! (Rule 17). The repository keeps the DTOs, the orchestration and the Postgres
//! half; the query and its column names live here.
//!
//! ## Why a `fn -> String` and not a `const`
//!
//! Relationship types come from `neo4j::schema` so the read stays in lockstep
//! with one constant; a Rust `const` cannot call `format!`, so the query is built
//! by a function (the `fetch_hashes` pattern in `canonical_elements::cypher`). No
//! literal `{ }` braces appear here — node bindings use `labels(x)[0]`, not
//! property maps — so no `{{`/`}}` escaping is needed.
//!
//! ## Why label filters on every node binding
//!
//! `(a)-[:BEARS_ON]->(e)` with no label restriction would match any node-type
//! bearing on an Element. House style — established in
//! `causes_of_action_repository.rs` — gates every node binding with
//! `labels(x)[0] = $label` and reads the label name from an `ENTITY_*` constant,
//! so no domain string is ever hardcoded in a Cypher clause.
//!
//! ## Why the RELATIONSHIPS are now bound
//!
//! Task 396 matched `(a)<-[:CORROBORATES]-(ev)` anonymously, because everything
//! it needed lived on the nodes. The linking pass of 2026-09-06 wrote the rank,
//! the role, the confidence, the reason and the conflict flag onto the EDGE, so
//! the edge needs a name (`cr` / `dr`) to project from. Nothing else about the
//! match changed — same direction, same label gates, same `OPTIONAL`.

use crate::neo4j::schema;

/// Build the detail Cypher: Element properties, parent LegalCount (OPTIONAL),
/// every Allegation that bears on this Element (OPTIONAL), and both evidence
/// legs with everything the linking pass wrote on their edges.
///
/// `e.id` for the Element matches the `id` *property* (not Neo4j's internal id)
/// — that is the canonical, content-stable identifier the loader writes and the
/// one Postgres stores in `authored_entities.entity_id`.
pub(crate) fn element_detail_cypher() -> String {
    format!(
        "MATCH (e) \
       WHERE e.id = $element_id AND labels(e)[0] = $element_label \
     OPTIONAL MATCH (lc)-[:{has_element}]->(e) WHERE labels(lc)[0] = $count_label \
     OPTIONAL MATCH (a)-[:{bears_on}]->(e) WHERE labels(a)[0] = $allegation_label \
     OPTIONAL MATCH (a)<-[cr:{corroborates}]-(ev) WHERE labels(ev)[0] = $evidence_label \
     OPTIONAL MATCH (ev)-[:{contained_in}]->(d) WHERE labels(d)[0] = $document_label \
     OPTIONAL MATCH (ev)-[:{stated_by}]->(sp) \
     OPTIONAL MATCH (a)<-[dr:{rebuts}]-(dv) WHERE labels(dv)[0] = $evidence_label \
     OPTIONAL MATCH (dv)-[:{contained_in}]->(dd) WHERE labels(dd)[0] = $document_label \
     OPTIONAL MATCH (dv)-[:{stated_by}]->(dsp) \
     RETURN {header}, {supporting}, {disputing}",
        has_element = schema::HAS_ELEMENT,
        bears_on = schema::BEARS_ON,
        corroborates = schema::CORROBORATES,
        rebuts = schema::REBUTS,
        contained_in = schema::CONTAINED_IN,
        stated_by = schema::STATED_BY,
        header = HEADER_PROJECTION,
        supporting = leg_projection(SUPPORTING),
        disputing = leg_projection(DISPUTING),
    )
}

// CONST: the Element, Count and Allegation columns — a projection locked to the
// canonical node properties, not a deployment value.
const HEADER_PROJECTION: &str = "\
       e.id                         AS element_id, \
       e.element_name               AS element_name, \
       e.what_plaintiff_must_prove  AS what_plaintiff_must_prove, \
       e.order_in_count             AS order_in_count, \
       lc.count_number              AS count_number, \
       lc.title                     AS count_name, \
       a.id                         AS allegation_id, \
       a.paragraph_number           AS paragraph_number, \
       a.summary                    AS summary, \
       a.title                      AS title, \
       a.verbatim_quote             AS verbatim_quote";

/// The bindings one evidence leg projects from: its Evidence node, its
/// relationship, its source Document, its speaker, and the alias prefix its
/// columns carry.
///
/// ## Rust Learning: a named tuple struct instead of five `&str` arguments
///
/// Five `&str`s in a row is five chances to swap two at a call site with no
/// compile error — and swapping the node for the document here would silently
/// project a Document's `verbatim_quote`, which is `null` on every row. Naming
/// them costs one struct and removes the whole class.
struct Leg {
    /// The Evidence node binding: `ev` or `dv`.
    node: &'static str,
    /// The relationship binding: `cr` or `dr`.
    edge: &'static str,
    /// The source Document binding: `d` or `dd`.
    document: &'static str,
    /// The speaker binding: `sp` or `dsp`.
    speaker: &'static str,
    /// The alias prefix the fold reads: `evidence` or `disputing`.
    alias: &'static str,
}

// CONST: the two legs' bindings, matching the MATCH clauses above.
const SUPPORTING: Leg = Leg {
    node: "ev",
    edge: "cr",
    document: "d",
    speaker: "sp",
    alias: "evidence",
};

const DISPUTING: Leg = Leg {
    node: "dv",
    edge: "dr",
    document: "dd",
    speaker: "dsp",
    alias: "disputing",
};

/// One evidence leg's twenty columns.
///
/// ## Why both legs are generated from ONE function
///
/// They were spelled out twice, and the pair had to stay parallel by hand: the
/// fold destructures a fixed-width array per leg, and a column present on one
/// side and absent on the other would order that leg by a key it does not have —
/// silently, and differently from its twin. Generating both from one shape makes
/// the parallelism structural instead of a thing a test has to notice.
///
/// The supporting leg's document columns keep their historical `source_*` names
/// rather than `evidence_*`, because the frontend and the fold have read them by
/// those names since the panel shipped; renaming them here would be a wire change
/// dressed as a refactor.
fn leg_projection(leg: Leg) -> String {
    let Leg {
        node,
        edge,
        document,
        speaker,
        alias,
    } = leg;
    // The one asymmetry, named rather than branched on further down.
    let doc_alias = if alias == "evidence" { "source" } else { alias };
    format!(
        "\
       {node}.id                AS {alias}_id, \
       {node}.verbatim_quote    AS {alias}_quote, \
       {node}.page_number       AS {alias}_page_number, \
       {node}.paragraph         AS {alias}_paragraph, \
       {node}.page_note         AS {alias}_page_note, \
       {document}.id            AS {doc_alias}_document_id, \
       {document}.title         AS {doc_alias}_document_title, \
       {document}.document_date AS {alias}_document_date, \
       {node}.statement_type    AS {alias}_statement_type, \
       {node}.evidence_strength AS {alias}_strength, \
       {speaker}.name           AS {alias}_speaker, \
       {node}.question          AS {alias}_question, \
       {node}.answer            AS {alias}_answer, \
       {edge}.rank              AS {alias}_rank, \
       {edge}.role              AS {alias}_role, \
       {edge}.confidence        AS {alias}_confidence, \
       {edge}.rank_reason       AS {alias}_rank_reason, \
       {edge}.why               AS {alias}_why, \
       {edge}.conflict          AS {alias}_conflict, \
       {edge}.duplicate_of_card_id AS {alias}_duplicate_of"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Relationship types come from `neo4j::schema`, never inline literals
    /// (Rule 12 at the query layer).
    #[test]
    fn the_query_names_its_relationships_from_the_schema_constants() {
        let q = element_detail_cypher();
        for (binding, rel) in [("cr", schema::CORROBORATES), ("dr", schema::REBUTS)] {
            assert!(
                q.contains(&format!("<-[{binding}:{rel}]-")),
                "the {binding} leg must bind {rel}: {q}"
            );
        }
        assert!(q.contains(&format!("-[:{}]->", schema::HAS_ELEMENT)));
        assert!(q.contains(&format!("-[:{}]->", schema::BEARS_ON)));
    }

    /// Every node binding is label-gated by a PARAMETER, so no domain label is
    /// compiled into a Cypher clause.
    #[test]
    fn every_node_binding_is_gated_by_a_parameter() {
        let q = element_detail_cypher();
        for gate in [
            "labels(e)[0] = $element_label",
            "labels(lc)[0] = $count_label",
            "labels(a)[0] = $allegation_label",
            "labels(ev)[0] = $evidence_label",
            "labels(dv)[0] = $evidence_label",
            "labels(d)[0] = $document_label",
            "labels(dd)[0] = $document_label",
        ] {
            assert!(q.contains(gate), "missing gate `{gate}`: {q}");
        }
    }

    /// THE PARALLEL-LEGS TEST. Both legs project the same facts.
    ///
    /// The two legs are rendered by one component and ordered by one function. A
    /// leg missing a column would order by a key it does not have — silently, and
    /// differently from its twin, on the one page whose order is a claim.
    #[test]
    fn both_evidence_legs_project_the_same_seven_edge_properties() {
        let q = element_detail_cypher();
        for property in [
            "rank",
            "role",
            "confidence",
            "rank_reason",
            "why",
            "conflict",
            "duplicate_of_card_id",
        ] {
            assert!(
                q.contains(&format!("cr.{property}")),
                "the supporting leg does not project {property}"
            );
            assert!(
                q.contains(&format!("dr.{property}")),
                "the disputing leg does not project {property}"
            );
        }
        for node_property in ["answer", "question", "paragraph"] {
            assert!(q.contains(&format!("ev.{node_property}")));
            assert!(q.contains(&format!("dv.{node_property}")));
        }
        // The date §3 sorts on, from the SOURCE DOCUMENT, on both legs.
        assert!(q.contains("d.document_date"));
        assert!(q.contains("dd.document_date"));
    }

    /// Both legs stay OPTIONAL. A mandatory match on either would drop every
    /// Allegation with no evidence on that side — the visible gap the panel
    /// exists to show.
    #[test]
    fn both_evidence_legs_stay_optional() {
        let q = element_detail_cypher();
        assert_eq!(
            q.matches("OPTIONAL MATCH").count(),
            8,
            "one of the optional hops became mandatory: {q}"
        );
        assert_eq!(
            q.matches("MATCH (e)").count(),
            1,
            "one mandatory anchor only"
        );
    }
}
