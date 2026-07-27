//! Wire DTOs for `GET /api/cases/:slug/case-health/inventory` — Pane 1 of the
//! Case Health dashboard ("Graph Inventory").
//!
//! Everything the pane displays arrives here already computed. The frontend
//! renders these values verbatim: it performs no division, no summing, no
//! sorting, and knows no edge-class vocabulary of its own (the column list ships
//! as [`CaseHealthInventoryResponse::edge_classes`]). That is the "zero business
//! logic in the frontend" non-negotiable expressed as a payload shape.
//!
//! ## Why the payload nests a `current` / `previous` pair
//!
//! Snapshot history is explicitly out of scope for this chunk, but the payload
//! must be shaped so it can attach later without a breaking change. So the
//! measurable content lives in ONE struct, [`GraphInventory`], and the response
//! carries it twice: `current` (always present) and `previous` (always `None`
//! today). When the history store lands, a delta is `current` vs. `previous`
//! with identical shapes and identical field names — no DTO surgery, no
//! frontend re-type, and no risk of the two halves of a comparison drifting
//! apart because they were modelled separately.
//!
//! ## Property-name discipline
//!
//! Every graph property this payload surfaces is named as the code's WRITE PATH
//! spells it, not as a reader assumed it. `doc_type` (not `document_type`) is
//! what `api::pipeline::ingest_helpers::create_document_node` sets, and `title`
//! (not `file_name`) is the Document node's display name. Two of the 2026-07-26
//! assessment's alarm findings were queries against property names that have
//! never existed; that class of error is designed out here by construction.

use serde::{Deserialize, Serialize};

/// The full response body for the Graph Inventory pane.
///
/// ## Rust Learning: `Deserialize` on a response DTO
///
/// A response type only strictly needs `Serialize`. Deriving `Deserialize` too
/// costs nothing at runtime and lets the in-module tests round-trip a payload
/// (serialize → parse → assert), which is how the wire-shape tests below pin the
/// contract the TypeScript client mirrors. Same posture as `dto::trial_prep`.
///
/// ## Why no `deny_unknown_fields` on any struct in this module
///
/// `deny_unknown_fields` earns its keep on REQUEST bodies, where a client's
/// camelCase typo must fail loudly rather than be silently ignored (see
/// `ScenarioCreateRequest`). Nothing here is a request: these are response
/// shapes, and the only deserializer is a test. Making them strict would buy no
/// safety and would actively cost forward compatibility — a later chunk stores a
/// snapshot of [`GraphInventory`], and a row written by a newer build (one that
/// added a field) must still deserialize on older code rather than hard-failing.
/// `connection_tier_lookup_v` is the real shape/definition gate, exactly as
/// `schema_v` is for `ScenarioDefinition`.
// serde: allows unknown fields because these are response shapes whose only
// deserializer is a test, and because a future stored snapshot written by a newer
// build must still parse on older code; `connection_tier_lookup_v` is the gate,
// not `deny_unknown_fields`. Applies to every struct in this module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseHealthInventoryResponse {
    /// The case slug from the request path, echoed so a cached payload can be
    /// correlated with the case it describes.
    pub case_slug: String,

    /// The version of the probative/topical partition the rates were computed
    /// under ([`crate::domain::connection_tier::CONNECTION_TIER_LOOKUP_V`]).
    ///
    /// Domain note: this is what makes a stored snapshot honest later. Two rates
    /// computed under different partitions are not comparable, and a delta that
    /// silently compared them would report a "improvement" that was really a
    /// definition change. Carrying the version means the comparison can refuse.
    pub connection_tier_lookup_v: u32,

    /// The Evidence→Allegation edge classes, in display order, that the
    /// per-document breakdown reports. The frontend renders one column per entry
    /// and holds no edge vocabulary of its own; a class added to the tier
    /// definition appears as a new column with no frontend change.
    ///
    /// Always populated even when `current.documents` is empty, so the table
    /// still renders its headers on an empty corpus.
    pub edge_classes: Vec<String>,

    /// The live measurement.
    pub current: GraphInventory,

    /// A prior measurement, for the delta view. **Always `None` in this chunk**
    /// — no history store exists yet. Present in the shape so history attaches
    /// without a breaking change (see the module note).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous: Option<GraphInventory>,
}

/// One complete measurement of the graph's structural health. The unit a
/// snapshot would store and a delta would compare.
// serde: allows unknown fields because this is a response shape, not a request — see the module-level note on CaseHealthInventoryResponse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphInventory {
    /// When this measurement was taken, RFC 3339 / UTC. Rendered by the pane so
    /// the operator never mistakes a stale tab for a live reading.
    pub computed_at: String,

    /// The corpus-wide rollup — the headline figures.
    pub corpus: CorpusConnection,

    /// Node counts by POPULATED label, most populous first.
    ///
    /// Domain note: derived from the nodes themselves at query time, never from
    /// `db.labels()`. Neo4j retains label names from earlier schema generations
    /// forever, so `db.labels()` reports labels with zero nodes as though they
    /// were part of the model. Deriving from `MATCH (n)` means a label appears
    /// here if and only if something actually carries it.
    pub node_labels: Vec<LabelCount>,

    /// Nodes carrying NO label at all.
    ///
    /// Standing Rule 1: `labels(n)[0]` is null for an unlabeled node, and such a
    /// node would otherwise vanish from `node_labels` without a trace, making
    /// the label counts silently fail to sum to the node total. Counting them
    /// into their own field keeps "there are no unlabeled nodes" (0) and "we
    /// didn't look" (impossible — the field is always present) distinguishable.
    /// A non-zero value here is a real data-integrity signal.
    pub unlabeled_node_count: i64,

    /// Edge counts by (from-label, rel-type, to-label) triple, most numerous
    /// first. The full edge taxonomy with endpoints — a plain count by
    /// relationship type cannot distinguish `Evidence ABOUT Person` from
    /// `Evidence ABOUT Allegation`, and those mean entirely different things.
    pub edge_triples: Vec<EdgeTripleCount>,

    /// One row per Document node, most Evidence first.
    pub documents: Vec<DocumentConnection>,
}

/// The corpus-wide connection rollup.
///
/// Domain note on the two rates, because the distinction is the whole point of
/// this pane: `probative` counts Evidence carrying at least one CORROBORATES /
/// REBUTS / CHARACTERIZES edge to an Allegation — edges that bear on whether the
/// allegation is TRUE. `topical` additionally counts ABOUT, which asserts only
/// that the item is on the subject. The probative rate is the headline; the
/// topical rate is displayed beside it, separately labeled. They are never
/// blended into one number, because a corpus wholly "about" the case would then
/// report as fully connected while proving nothing.
// serde: allows unknown fields because this is a response shape, not a request — see the module-level note on CaseHealthInventoryResponse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusConnection {
    /// Every Evidence node in the graph, whether or not it reaches a Document.
    pub evidence_total: i64,
    /// Evidence with ≥1 probative edge to an Allegation.
    pub probative_connected: i64,
    /// Evidence with ≥1 edge of any of the four classes to an Allegation.
    pub topical_connected: i64,
    /// Evidence with NO edge to any Allegation. `evidence_total - topical_connected`.
    pub inert: i64,

    /// Probative connection as a percentage, one decimal place. **The headline.**
    ///
    /// `None` — not `0.0` — when there is no Evidence at all. Standing Rule 1:
    /// "0% of nothing" and "0% of 525" are operationally different states and a
    /// bare `0.0` collapses them. The frontend renders `None` as "—".
    pub probative_rate_pct: Option<f64>,
    /// Topical connection as a percentage, one decimal place. `None` on an empty
    /// corpus, same rationale as above.
    pub topical_rate_pct: Option<f64>,
    /// Inert share as a percentage, one decimal place. `None` on an empty corpus.
    pub inert_rate_pct: Option<f64>,

    /// Evidence nodes with no `CONTAINED_IN` edge to a Document.
    ///
    /// Why this is on the payload rather than left implicit: the corpus totals
    /// are measured over ALL Evidence, while the per-document table can only see
    /// Evidence that reaches a Document. If the two ever disagree, this field
    /// says by how much and why, instead of leaving an unexplained gap between
    /// the headline and the sum of the rows. A non-zero value is itself a
    /// finding (an orphaned extraction).
    pub evidence_without_document: i64,
}

/// A populated node label and how many nodes carry it.
// serde: allows unknown fields because this is a response shape, not a request — see the module-level note on CaseHealthInventoryResponse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelCount {
    pub label: String,
    pub node_count: i64,
}

/// One (from-label, rel-type, to-label) triple and its edge count.
// serde: allows unknown fields because this is a response shape, not a request — see the module-level note on CaseHealthInventoryResponse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeTripleCount {
    /// First label of the source node, or `None` if the node is unlabeled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_label: Option<String>,
    pub rel_type: String,
    /// First label of the target node, or `None` if the node is unlabeled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_label: Option<String>,
    pub edge_count: i64,
}

/// One Document's Evidence yield and how much of it is wired into the case.
// serde: allows unknown fields because this is a response shape, not a request — see the module-level note on CaseHealthInventoryResponse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentConnection {
    /// The Neo4j `Document.id` — equal to the pipeline document id for anything
    /// the pipeline ingested, so it is also the drill-down key a later chunk
    /// joins Postgres cost against.
    pub document_id: String,
    /// The Document node's display name: the `title` property. Named as the
    /// write path spells it (see the module note on property-name discipline).
    /// `None` if the node carries no title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The `doc_type` property — the pipeline's document type, written at ingest
    /// from the Postgres `documents.document_type` column. `None` if absent on
    /// the node, which is itself worth seeing rather than papering over.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_type: Option<String>,

    /// Evidence nodes `CONTAINED_IN` this Document.
    pub evidence_total: i64,
    /// Of those, how many carry ≥1 probative edge to an Allegation.
    pub probative_connected: i64,
    /// Of those, how many carry ≥1 edge of any class to an Allegation.
    pub topical_connected: i64,
    /// Of those, how many reach no Allegation at all.
    pub inert: i64,

    /// Probative rate for this document, one decimal. `None` when the document
    /// produced no Evidence (a real and different state from 0%).
    pub probative_rate_pct: Option<f64>,
    /// Topical rate for this document, one decimal. `None` when no Evidence.
    pub topical_rate_pct: Option<f64>,
    /// Inert share for this document, one decimal. `None` when no Evidence.
    pub inert_rate_pct: Option<f64>,

    /// Per-edge-class breakdown, in [`CaseHealthInventoryResponse::edge_classes`]
    /// order.
    ///
    /// Domain note — these are ITEM counts, not edge counts, and they do NOT sum
    /// to `topical_connected`. One Evidence item may both corroborate one
    /// allegation and rebut another, so it is counted under both classes while
    /// contributing 1 to `topical_connected`. That is deliberate: the question
    /// each class answers is "how many items of this document do this kind of
    /// work?", which double-counting an item that does two kinds of work answers
    /// correctly. A reader must not try to reconcile the class counts against the
    /// connected totals by addition.
    pub by_edge_class: Vec<EdgeClassCount>,
}

/// One Evidence→Allegation edge class and how many of a document's Evidence
/// items carry at least one edge of it.
// serde: allows unknown fields because this is a response shape, not a request — see the module-level note on CaseHealthInventoryResponse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeClassCount {
    pub rel_type: String,
    pub item_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_corpus() -> CorpusConnection {
        CorpusConnection {
            evidence_total: 10,
            probative_connected: 3,
            topical_connected: 5,
            inert: 5,
            probative_rate_pct: Some(30.0),
            topical_rate_pct: Some(50.0),
            inert_rate_pct: Some(50.0),
            evidence_without_document: 0,
        }
    }

    /// The corpus block must keep the two rates as SEPARATE, distinctly named
    /// fields. A future "simplification" that merged them into one
    /// `connection_rate_pct` would destroy the ruling this pane exists to
    /// enforce, so the field names are pinned on the wire.
    #[test]
    fn corpus_keeps_probative_and_topical_as_distinct_fields() {
        let value = serde_json::to_value(sample_corpus()).expect("serializes");
        let object = value.as_object().expect("object body");
        assert!(object.contains_key("probative_rate_pct"));
        assert!(object.contains_key("topical_rate_pct"));
        assert!(
            !object.contains_key("connection_rate_pct"),
            "the two tiers must never collapse into one blended rate"
        );
        assert_eq!(object["probative_connected"], json!(3));
        assert_eq!(object["topical_connected"], json!(5));
    }

    /// A `None` rate must serialize as JSON `null`, NOT be omitted and NOT
    /// become `0.0`. The frontend keys on `null` to render "—"; an omitted key
    /// would be indistinguishable from a contract mismatch, and `0.0` would
    /// assert a measurement that was never made (Standing Rule 1).
    #[test]
    fn absent_rate_serializes_as_null_not_zero_and_not_omitted() {
        let empty = CorpusConnection {
            evidence_total: 0,
            probative_connected: 0,
            topical_connected: 0,
            inert: 0,
            probative_rate_pct: None,
            topical_rate_pct: None,
            inert_rate_pct: None,
            evidence_without_document: 0,
        };
        let value = serde_json::to_value(empty).expect("serializes");
        let object = value.as_object().expect("object body");
        assert!(object.contains_key("probative_rate_pct"));
        assert_eq!(object["probative_rate_pct"], json!(null));
        assert_eq!(object["topical_rate_pct"], json!(null));
        assert_eq!(object["inert_rate_pct"], json!(null));
    }

    /// `previous` is absent from the wire in this chunk, and `current` is always
    /// present. Pins the snapshot-attach shape so a later chunk adds history by
    /// populating a field rather than reshaping the payload.
    #[test]
    fn response_omits_previous_when_absent_and_always_carries_current() {
        let response = CaseHealthInventoryResponse {
            case_slug: "some_case".to_string(),
            connection_tier_lookup_v: 1,
            edge_classes: vec!["CORROBORATES".to_string()],
            current: GraphInventory {
                computed_at: "2026-07-27T00:00:00+00:00".to_string(),
                corpus: sample_corpus(),
                node_labels: vec![LabelCount {
                    label: "Evidence".to_string(),
                    node_count: 10,
                }],
                unlabeled_node_count: 0,
                edge_triples: vec![],
                documents: vec![],
            },
            previous: None,
        };
        let value = serde_json::to_value(&response).expect("serializes");
        let object = value.as_object().expect("object body");
        assert!(object.contains_key("current"));
        assert!(!object.contains_key("previous"));
        assert!(object.contains_key("edge_classes"));
        assert_eq!(object["connection_tier_lookup_v"], json!(1));

        // Round-trips: the TypeScript client mirrors this shape, so it must
        // survive a serialize/parse cycle unchanged.
        let parsed: CaseHealthInventoryResponse =
            serde_json::from_value(value).expect("round-trips");
        assert_eq!(parsed.case_slug, "some_case");
        assert!(parsed.previous.is_none());
    }

    /// An unlabeled endpoint serializes its label key away rather than emitting
    /// an invented placeholder string. The frontend renders a missing label as
    /// "(unlabeled)" presentationally; the payload never fabricates a label that
    /// does not exist in the graph.
    #[test]
    fn edge_triple_omits_absent_endpoint_labels() {
        let triple = EdgeTripleCount {
            from_label: None,
            rel_type: "ABOUT".to_string(),
            to_label: Some("Person".to_string()),
            edge_count: 3,
        };
        let value = serde_json::to_value(triple).expect("serializes");
        let object = value.as_object().expect("object body");
        assert!(!object.contains_key("from_label"));
        assert_eq!(object["to_label"], json!("Person"));
        assert_eq!(object["rel_type"], json!("ABOUT"));
    }
}
