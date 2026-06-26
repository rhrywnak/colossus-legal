//! Human-authored case-fact writer for Neo4j.
//!
//! This module is the **parallel, purpose-built** write path for facts a human
//! authors directly against the graph — distinct from the pipeline's
//! extraction ingest (`api::pipeline::ingest_helpers`). It deliberately shares
//! none of that code: a human-authored fact is structurally identical to an
//! extracted one (same MERGE-on-id edge convention so reads stay uniform) but
//! is marked `provenance = 'human-authored'`, carries **no** citation trail
//! (no `source_document`, `verbatim_quote`, `grounding_status`, `page_number`
//! on the node; no `source_document_id` / `extraction_run_id` on the edge),
//! and therefore never routes through `validate_relationship_provenance` (which
//! would reject the missing citation).
//!
//! ## Idempotency — read this before "fixing" anything
//!
//! - **Nodes are non-idempotent by design.** Each write mints a fresh UUID and
//!   `CREATE`s a node. Writing the same logical fact twice produces *two*
//!   nodes. Dedup is a future 5.2 UI concern, not this path's job. See the
//!   `// Why:` comment on [`build_node_create_cypher`].
//! - **Edges are idempotent.** Each edge `MERGE`s on `(from)-[type]->(to)`, so
//!   writing the same edge twice yields *one* edge.
//!
//! ## Rust Learning: the borrowed `&mut Txn` + caller-owned-commit model
//!
//! Every public write function here takes `txn: &mut neo4rs::Txn` rather than a
//! `Graph`. It performs its `CREATE` / `MERGE` statements against that *one*
//! borrowed transaction and returns — it never calls `commit()` or `rollback()`.
//! The caller owns the transaction lifecycle. This is what makes "a node plus
//! its edges" a single atomic unit: [`write_human_fact`] creates the node and
//! then writes each edge against the same `txn`, propagating any failure with
//! `?`. If an edge fails, the function returns `Err` *before* the caller
//! commits, so the freshly-created node never lands — dropping a `Txn` without
//! committing discards every uncommitted statement. We mirror the borrow shape
//! of `create_ingest_relationship` exactly so both write paths compose the same
//! way inside a larger transaction.
//!
//! ## Rust Learning: why `ScalarValue` is a typed enum, not a JSON blob
//!
//! Node properties are passed as [`ScalarValue`] — a closed enum of the four
//! scalar shapes Neo4j stores (`Text`/`Int`/`Float`/`Bool`) — instead of a
//! `serde_json::Value`. A raw JSON blob would let a caller smuggle in nested
//! objects, arrays, or nulls that the graph can't represent as a flat property,
//! and the failure would surface deep in Cypher at runtime. A typed enum makes
//! the contract unrepresentable-if-wrong at the call site and lets the compiler
//! check it. The eventual 5.2 HTTP handler will deserialize its wire DTO and
//! map into these types — the wire-format concern lives there, not here.
//!
//! ## Rust Learning: `thiserror`, `#[from]`, and why we use `#[source]` instead
//!
//! `thiserror` can derive `From` for an error variant via `#[from]`, which lets
//! `?` auto-convert an inner error (e.g. `neo4rs::Error`) into our enum with no
//! `map_err`. We deliberately do *not* use `#[from]` on [`HumanFactError::Neo4j`]:
//! a bare conversion would discard *which* operation failed. Instead we keep the
//! inner error as `#[source]` (preserving the error chain for the logs) and use
//! `.map_err(|source| HumanFactError::Neo4j { operation, source })` at each call
//! site to attach a human-readable `operation` string. Context over convenience
//! — the same trade-off `IngestError` makes.

use neo4rs::{query, Query};
use uuid::Uuid;

use crate::neo4j::schema;

/// The Neo4j property value marking a node or edge as human-authored.
///
/// A single named constant (used by both the node and the edge write) so the
/// literal never drifts between the two. Colocated with its sole writer here,
/// mirroring how `PROVENANCE_CANONICAL` lives with the canonical loader rather
/// than in a shared module.
// CONST: graph-schema identity value, fixed at data-model time. Changing it
// requires a graph migration (every existing human-authored node/edge would
// need re-stamping), not a config edit — same class as `schema::STATED_BY` and
// `PROVENANCE_CANONICAL`, so Standing Rule 2 does not apply.
pub const PROVENANCE_HUMAN_AUTHORED: &str = "human-authored";

/// Property names a caller may **not** set on a human-authored node.
///
/// Two groups, both rejected for the same reason — "no citation trail" must be
/// structurally enforced, not merely conventional:
/// - Writer-owned: `id`, `provenance`, `created_at` (this module sets these).
/// - Citation trail: `source_document`, `verbatim_quote`, `grounding_status`,
///   `page_number`, `source_document_id`, `extraction_run_id` — the exact
///   properties whose *absence* distinguishes a human-authored fact from an
///   extracted one. Letting a caller set them would defeat the whole point.
// CONST: structural enforcement list (writer-owned + citation-trail property
// names), not a tunable. These are graph-schema property identifiers fixed at
// data-model time; adding one is a code change by definition, so Standing
// Rule 2 does not apply.
const RESERVED_PROPERTY_NAMES: [&str; 9] = [
    "id",
    "provenance",
    "created_at",
    "source_document",
    "verbatim_quote",
    "grounding_status",
    "page_number",
    "source_document_id",
    "extraction_run_id",
];

/// The relationship types a human may author. Anything else is rejected rather
/// than written as an arbitrary edge. These reference the shared graph-schema
/// vocabulary so a rename there flows here in one edit.
// CONST: schema-bounded allowlist. Extending it requires the new type to exist
// in `neo4j::schema` first (and a graph migration), so compile-time is the only
// valid home — same rationale as the schema relationship constants it cites.
const ALLOWED_HUMAN_REL_TYPES: [&str; 5] = [
    schema::STATED_BY,
    schema::ABOUT,
    schema::CONTRADICTS,
    schema::CHARACTERIZES,
    schema::REBUTS,
];

// ===========================================================================
// Request types
// ===========================================================================

/// One scalar property value, constrained to the shapes Neo4j stores as a flat
/// node property. See the module `## Rust Learning` note on why this is a typed
/// enum rather than a `serde_json::Value`.
#[derive(Debug, Clone)]
pub enum ScalarValue {
    /// A string property.
    Text(String),
    /// A 64-bit signed integer property.
    Int(i64),
    /// A 64-bit floating-point property.
    Float(f64),
    /// A boolean property.
    Bool(bool),
}

/// A single name/value property to set on a human-authored node.
#[derive(Debug, Clone)]
pub struct HumanFactProperty {
    /// Property name. Validated to `^[A-Za-z_][A-Za-z0-9_]*$` and rejected if it
    /// is a reserved name (see [`RESERVED_PROPERTY_NAMES`]).
    pub name: String,
    /// The scalar value to bind for this property.
    pub value: ScalarValue,
}

/// A human-authored node to create: a label plus its scalar properties.
#[derive(Debug, Clone)]
pub struct HumanFactNode {
    /// Neo4j node label. Validated to `^[A-Za-z_][A-Za-z0-9_]*$` before it is
    /// interpolated into Cypher (Cypher cannot parameterize a label).
    pub label: String,
    /// Scalar properties to set on the node. May be empty.
    pub properties: Vec<HumanFactProperty>,
}

/// An edge from the just-created node to an **existing** node.
#[derive(Debug, Clone)]
pub struct OutgoingEdge {
    /// Relationship type — must be one of [`ALLOWED_HUMAN_REL_TYPES`].
    pub rel_type: String,
    /// The `id` of the existing target node. Must already exist in the graph.
    pub to_id: String,
}

/// A full human-authored fact: one node plus its outgoing edges, written as a
/// single atomic unit against the caller's transaction.
#[derive(Debug, Clone)]
pub struct HumanFactRequest {
    /// The node to create.
    pub node: HumanFactNode,
    /// Edges from the new node to existing nodes. May be empty (node only).
    pub edges: Vec<OutgoingEdge>,
}

// ===========================================================================
// Error type
// ===========================================================================

/// Failure modes for the human-authored write path.
///
/// Each variant is a distinct, observable operational state (Standing Rule 1):
/// a caller can match on exactly what went wrong. Mirrors the typed-enum
/// precedent of `ScenarioRepositoryError` / `IngestError`.
#[derive(Debug, thiserror::Error)]
pub enum HumanFactError {
    /// The node label failed the `^[A-Za-z_][A-Za-z0-9_]*$` charset check.
    #[error("invalid node label '{label}': must match ^[A-Za-z_][A-Za-z0-9_]*$")]
    InvalidLabel { label: String },

    /// A property name failed the `^[A-Za-z_][A-Za-z0-9_]*$` charset check.
    #[error(
        "invalid property name '{name}' on label '{label}': \
         must match ^[A-Za-z_][A-Za-z0-9_]*$"
    )]
    InvalidPropertyName { label: String, name: String },

    /// A property name is reserved (writer-owned or part of the citation trail
    /// a human-authored fact must omit). See [`RESERVED_PROPERTY_NAMES`].
    #[error(
        "reserved property name '{name}' may not be set by callers — it is \
         writer-owned or part of the citation trail a human-authored fact omits"
    )]
    ReservedPropertyName { name: String },

    /// The relationship type is not one a human may author.
    #[error(
        "relationship type '{rel_type}' is not an allowed human-authored type \
         (expected one of STATED_BY, ABOUT, CONTRADICTS, CHARACTERIZES, REBUTS)"
    )]
    DisallowedRelType { rel_type: String },

    /// The edge `MATCH` found no nodes, so the `MERGE` wrote nothing. One or
    /// both endpoints are absent — the query cannot tell which (a single
    /// `MATCH (a),(b)` returns zero rows if *either* is missing), and we do
    /// not spend a second round-trip to disambiguate.
    #[error(
        "edge endpoint not found: MATCH matched no nodes for from_id '{from_id}' \
         and/or to_id '{to_id}' — both must already exist"
    )]
    EndpointNotFound { from_id: String, to_id: String },

    /// A Neo4j statement failed. `operation` names what we were doing; the
    /// underlying driver error is preserved as the source. The recovery hint
    /// tells an operator reading the log what class of action to take, since the
    /// driver error alone (a connection reset, an auth failure, a constraint
    /// violation) does not.
    #[error("Neo4j {operation} failed — check graph connectivity and the source error below")]
    Neo4j {
        operation: String,
        #[source]
        source: neo4rs::Error,
    },
}

// ===========================================================================
// Validators (pure, sync — unit-testable without a live Neo4j)
// ===========================================================================

/// True if `s` matches `^[A-Za-z_][A-Za-z0-9_]*$` (ASCII identifier).
///
/// Used for both labels and property names, which are interpolated into Cypher
/// (neither can be parameterized) — so this is the injection guard, the same
/// discipline `create_entity_node` applies to its interpolated label.
fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        // Empty string (None) or a non-alpha/underscore first char fails.
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// True if `name` is a reserved property name (see [`RESERVED_PROPERTY_NAMES`]).
fn is_reserved_property(name: &str) -> bool {
    RESERVED_PROPERTY_NAMES.contains(&name)
}

/// Validate a node label.
///
/// # Errors
/// Returns [`HumanFactError::InvalidLabel`] if `label` is empty or contains any
/// character outside `[A-Za-z0-9_]` / starts with a digit.
fn validate_label(label: &str) -> Result<(), HumanFactError> {
    if is_valid_identifier(label) {
        Ok(())
    } else {
        Err(HumanFactError::InvalidLabel {
            label: label.to_string(),
        })
    }
}

/// Validate a single property name.
///
/// The reserved check runs first because every reserved name is *also* a valid
/// identifier — checking reserved first yields the more specific error.
///
/// # Errors
/// - [`HumanFactError::ReservedPropertyName`] if `name` is writer-owned or a
///   citation-trail property.
/// - [`HumanFactError::InvalidPropertyName`] if `name` fails the charset check.
fn validate_property_name(label: &str, name: &str) -> Result<(), HumanFactError> {
    if is_reserved_property(name) {
        return Err(HumanFactError::ReservedPropertyName {
            name: name.to_string(),
        });
    }
    if is_valid_identifier(name) {
        Ok(())
    } else {
        Err(HumanFactError::InvalidPropertyName {
            label: label.to_string(),
            name: name.to_string(),
        })
    }
}

/// Validate a relationship type against the human-authorable set.
///
/// # Errors
/// Returns [`HumanFactError::DisallowedRelType`] if `rel_type` is not one of
/// [`ALLOWED_HUMAN_REL_TYPES`].
fn validate_human_rel_type(rel_type: &str) -> Result<(), HumanFactError> {
    if ALLOWED_HUMAN_REL_TYPES.contains(&rel_type) {
        Ok(())
    } else {
        Err(HumanFactError::DisallowedRelType {
            rel_type: rel_type.to_string(),
        })
    }
}

// ===========================================================================
// Cypher builders (pure, sync — unit-testable without a live Neo4j)
// ===========================================================================

/// Build the `CREATE` Cypher for a human-authored node.
///
/// `label` and every entry of `prop_names` MUST already be validated (this fn
/// interpolates them verbatim). Property *values* are bound as `$p0`, `$p1`, …
/// positionally matching `prop_names`; `id` / `provenance` are bound by name
/// and `created_at` is `datetime()`.
///
/// ## Why: a fresh-UUID `CREATE`, never a `MERGE`
///
/// This is non-idempotent on purpose. The node id is a freshly minted UUID, so
/// re-writing the same logical fact creates a second node. Deduplicating
/// human-authored facts is a future 5.2 UI decision; doing it here (by MERGEing
/// on content) would silently collapse two genuinely separate authoring acts.
/// Do not "fix" this into a MERGE.
fn build_node_create_cypher(label: &str, prop_names: &[&str]) -> String {
    // Each caller property becomes ", <key>: $p<i>" inside the node map.
    let mut extra = String::new();
    for (i, name) in prop_names.iter().enumerate() {
        extra.push_str(&format!(", {name}: $p{i}"));
    }
    format!(
        "CREATE (n:{label} {{id: $id, provenance: $provenance, \
         created_at: datetime(){extra}}})"
    )
}

/// Build the `MATCH`-`MATCH`-`MERGE` Cypher for a human-authored edge.
///
/// `rel_type` MUST already be validated (it is interpolated verbatim; Cypher
/// cannot parameterize a relationship type). `RETURN b.id` lets the caller
/// detect the zero-rows case that means an endpoint was missing.
///
/// `ON MATCH SET ... coalesce(...)` is first-wins: a re-MERGE of an edge that
/// already exists keeps whatever `provenance` it had rather than overwriting —
/// matching the coalesce discipline the extraction edge writer uses.
fn build_human_edge_cypher(rel_type: &str) -> String {
    format!(
        "MATCH (a {{id: $from_id}}), (b {{id: $to_id}}) \
         MERGE (a)-[r:{rel_type}]->(b) \
         ON CREATE SET r.provenance = $provenance, r.created_at = datetime() \
         ON MATCH SET  r.provenance = coalesce(r.provenance, $provenance) \
         RETURN b.id"
    )
}

/// Bind one [`ScalarValue`] onto `query` under `key`.
///
/// ## Rust Learning: `Query` is a move-builder
///
/// `neo4rs::Query::param` consumes `self` and returns the updated `Query`, so
/// binding in a loop is `q = q.param(...)` — each call moves the builder
/// forward. The match arms call `.param` with the concrete Rust type, relying
/// on neo4rs' `Into<BoltType>` impls for `&str` / `i64` / `f64` / `bool`.
fn bind_scalar(q: Query, key: &str, value: &ScalarValue) -> Query {
    match value {
        ScalarValue::Text(s) => q.param(key, s.as_str()),
        ScalarValue::Int(i) => q.param(key, *i),
        ScalarValue::Float(f) => q.param(key, *f),
        ScalarValue::Bool(b) => q.param(key, *b),
    }
}

// ===========================================================================
// Write functions (async — borrow the txn, caller owns the commit)
// ===========================================================================

/// Create one human-authored node and return its freshly minted id.
///
/// Mints a UUID, validates the label and every property name, then issues a
/// single `CREATE` against `txn`. Sets `provenance = 'human-authored'` and
/// `created_at = datetime()` and nothing else beyond the caller's validated
/// properties — no citation trail. Non-idempotent by design (see
/// [`build_node_create_cypher`]). Does not commit; the caller owns `txn`.
///
/// # Errors
/// - [`HumanFactError::InvalidLabel`] if the label fails the charset check.
/// - [`HumanFactError::ReservedPropertyName`] / [`HumanFactError::InvalidPropertyName`]
///   if any property name is reserved or malformed.
/// - [`HumanFactError::Neo4j`] if the `CREATE` statement fails.
#[tracing::instrument(skip(txn, node), fields(label = %node.label, props = node.properties.len()))]
pub async fn write_human_node(
    txn: &mut neo4rs::Txn,
    node: &HumanFactNode,
) -> Result<String, HumanFactError> {
    validate_label(&node.label)?;
    for prop in &node.properties {
        validate_property_name(&node.label, &prop.name)?;
    }

    // Fresh identity for this authoring act — see the non-idempotency Why:.
    let id = Uuid::new_v4().to_string();

    let prop_names: Vec<&str> = node.properties.iter().map(|p| p.name.as_str()).collect();
    let cypher = build_node_create_cypher(&node.label, &prop_names);

    let mut q = query(&cypher)
        .param("id", id.as_str())
        .param("provenance", PROVENANCE_HUMAN_AUTHORED);
    for (i, prop) in node.properties.iter().enumerate() {
        q = bind_scalar(q, &format!("p{i}"), &prop.value);
    }

    // `txn.run` executes and discards the result summary — the id we return is
    // the UUID we minted, so there is nothing to read back. Mirrors the
    // `create_entity_node` node-write idiom (CREATE/MERGE without RETURN).
    txn.run(q).await.map_err(|source| HumanFactError::Neo4j {
        operation: format!("CREATE :{} node", node.label),
        source,
    })?;

    tracing::debug!(node_id = %id, label = %node.label, "wrote human-authored node");
    Ok(id)
}

/// Create (or no-op re-create) one human-authored edge between two existing
/// nodes.
///
/// Validates `rel_type`, then runs the `MATCH`-`MATCH`-`MERGE`. Idempotent: a
/// repeat write of the same edge yields one edge. Does not commit; the caller
/// owns `txn`.
///
/// # Errors
/// - [`HumanFactError::DisallowedRelType`] if `rel_type` is not human-authorable.
/// - [`HumanFactError::EndpointNotFound`] if the `MATCH` matched no nodes. This
///   means **one or both** endpoints are absent — it does *not* specifically
///   identify `to_id`; a single `MATCH (a),(b)` returns zero rows if either is
///   missing, and we do not spend a second query to disambiguate. This is the
///   fail-loud contract: an edge to a non-existent id is a caller bug we surface
///   rather than silently no-op.
/// - [`HumanFactError::Neo4j`] if the statement or its row read fails.
#[tracing::instrument(skip(txn), fields(rel_type = %rel_type))]
pub async fn write_human_edge(
    txn: &mut neo4rs::Txn,
    from_id: &str,
    to_id: &str,
    rel_type: &str,
) -> Result<(), HumanFactError> {
    validate_human_rel_type(rel_type)?;

    let cypher = build_human_edge_cypher(rel_type);
    let mut result = txn
        .execute(
            query(&cypher)
                .param("from_id", from_id)
                .param("to_id", to_id)
                .param("provenance", PROVENANCE_HUMAN_AUTHORED),
        )
        .await
        .map_err(|source| HumanFactError::Neo4j {
            operation: format!("MERGE :{rel_type} edge {from_id}->{to_id}"),
            source,
        })?;

    // Zero rows ⇒ the MATCH found no nodes ⇒ an endpoint is missing. Hard
    // error, never a silent no-op (mirrors create_ingest_relationship, not the
    // tolerant write_cross_tier_relationship).
    if result
        .next(&mut *txn)
        .await
        .map_err(|source| HumanFactError::Neo4j {
            operation: format!("read MERGE :{rel_type} result {from_id}->{to_id}"),
            source,
        })?
        .is_none()
    {
        return Err(HumanFactError::EndpointNotFound {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
        });
    }

    tracing::debug!(%from_id, %to_id, %rel_type, "wrote human-authored edge");
    Ok(())
}

/// Write one human-authored fact — a node plus its outgoing edges — as a single
/// atomic unit, returning the new node's id.
///
/// Creates the node, then writes each edge from the new node to its (existing)
/// target, all against the same borrowed `txn`. Because every step uses `?`,
/// the first failure returns `Err` before the caller commits — so a failed edge
/// means the freshly-created node never lands (the caller drops the uncommitted
/// `txn`). Does not commit; the caller owns the transaction lifecycle.
///
/// # Errors
/// Any error from [`write_human_node`] or [`write_human_edge`] — notably
/// [`HumanFactError::EndpointNotFound`] when an edge target does not exist, in
/// which case the caller must not commit (the node write is rolled back with the
/// transaction).
#[tracing::instrument(skip(txn, request), fields(label = %request.node.label, edges = request.edges.len()))]
pub async fn write_human_fact(
    txn: &mut neo4rs::Txn,
    request: &HumanFactRequest,
) -> Result<String, HumanFactError> {
    let node_id = write_human_node(txn, &request.node).await?;
    for edge in &request.edges {
        write_human_edge(txn, &node_id, &edge.to_id, &edge.rel_type).await?;
    }
    tracing::info!(node_id = %node_id, edges = request.edges.len(), "wrote human-authored fact");
    Ok(node_id)
}

// ===========================================================================
// Tier-1 unit tests — pure validators + Cypher builders (always run)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── Cypher builder: node ────────────────────────────────────────

    #[test]
    fn node_cypher_sets_provenance_and_created_at() {
        let cypher = build_node_create_cypher("Statement", &[]);
        assert!(
            cypher.contains("provenance: $provenance"),
            "node CREATE must bind provenance; got: {cypher}"
        );
        assert!(
            cypher.contains("created_at: datetime()"),
            "node CREATE must stamp created_at; got: {cypher}"
        );
        assert!(
            cypher.starts_with("CREATE (n:Statement"),
            "must CREATE the validated label; got: {cypher}"
        );
        // Non-idempotency guard: must be CREATE, never MERGE.
        assert!(
            !cypher.contains("MERGE"),
            "node write must be CREATE, not MERGE (intentional non-idempotency); got: {cypher}"
        );
    }

    #[test]
    fn node_cypher_omits_all_citation_properties() {
        // The whole point: a human-authored node carries no citation trail.
        let cypher = build_node_create_cypher("Statement", &["text", "speaker"]);
        for forbidden in [
            "source_document",
            "verbatim_quote",
            "grounding_status",
            "page_number",
            "source_document_id",
            "extraction_run_id",
        ] {
            assert!(
                !cypher.contains(forbidden),
                "node CREATE must omit citation property '{forbidden}'; got: {cypher}"
            );
        }
        // Caller properties are interpolated and positionally bound.
        assert!(cypher.contains("text: $p0"), "got: {cypher}");
        assert!(cypher.contains("speaker: $p1"), "got: {cypher}");
    }

    // ── Cypher builder: edge ────────────────────────────────────────

    #[test]
    fn edge_cypher_is_match_match_merge_with_coalesce() {
        let cypher = build_human_edge_cypher("STATED_BY");
        assert!(
            cypher.contains("MATCH (a {id: $from_id}), (b {id: $to_id})"),
            "both endpoints must be MATCHed by id; got: {cypher}"
        );
        assert!(
            cypher.contains("MERGE (a)-[r:STATED_BY]->(b)"),
            "rel type must be interpolated into the MERGE; got: {cypher}"
        );
        assert!(
            cypher.contains("ON CREATE SET r.provenance = $provenance"),
            "ON CREATE must set provenance; got: {cypher}"
        );
        assert!(
            cypher.contains("ON MATCH SET  r.provenance = coalesce(r.provenance, $provenance)"),
            "ON MATCH must coalesce provenance (first-wins); got: {cypher}"
        );
        assert!(
            cypher.trim_end().ends_with("RETURN b.id"),
            "must RETURN b.id so zero-rows signals a missing endpoint; got: {cypher}"
        );
    }

    #[test]
    fn edge_cypher_omits_citation_provenance_trio() {
        // A human edge carries provenance only — never the v5.1 citation trio.
        let cypher = build_human_edge_cypher("ABOUT");
        assert!(!cypher.contains("source_document_id"), "got: {cypher}");
        assert!(!cypher.contains("extraction_run_id"), "got: {cypher}");
    }

    // ── Validators: label ───────────────────────────────────────────

    #[test]
    fn validate_label_accepts_valid_rejects_invalid() {
        assert!(validate_label("Statement").is_ok());
        assert!(validate_label("_Internal").is_ok());
        assert!(validate_label("Node_42").is_ok());

        assert!(matches!(
            validate_label(""),
            Err(HumanFactError::InvalidLabel { .. })
        ));
        assert!(
            matches!(
                validate_label("4Node"),
                Err(HumanFactError::InvalidLabel { .. })
            ),
            "must reject a leading digit"
        );
        assert!(
            matches!(
                validate_label("Bad Label"),
                Err(HumanFactError::InvalidLabel { .. })
            ),
            "must reject whitespace"
        );
        assert!(
            matches!(
                validate_label("X) DETACH DELETE n //"),
                Err(HumanFactError::InvalidLabel { .. })
            ),
            "must reject an injection payload"
        );
    }

    // ── Validators: property name ───────────────────────────────────

    #[test]
    fn validate_property_name_accepts_plain_name() {
        assert!(validate_property_name("Statement", "speaker").is_ok());
        assert!(validate_property_name("Statement", "page_count").is_ok());
    }

    #[test]
    fn validate_property_name_rejects_reserved() {
        // Both writer-owned and citation-trail names must be rejected, and with
        // the *specific* reserved error (not the charset error).
        for reserved in [
            "id",
            "provenance",
            "created_at",
            "source_document",
            "verbatim_quote",
            "grounding_status",
            "page_number",
            "source_document_id",
            "extraction_run_id",
        ] {
            assert!(
                matches!(
                    validate_property_name("Statement", reserved),
                    Err(HumanFactError::ReservedPropertyName { .. })
                ),
                "reserved name '{reserved}' must be rejected as reserved"
            );
        }
    }

    #[test]
    fn validate_property_name_rejects_bad_charset() {
        assert!(matches!(
            validate_property_name("Statement", "has space"),
            Err(HumanFactError::InvalidPropertyName { .. })
        ));
        assert!(matches!(
            validate_property_name("Statement", ""),
            Err(HumanFactError::InvalidPropertyName { .. })
        ));
        assert!(matches!(
            validate_property_name("Statement", "n: 1} ) //"),
            Err(HumanFactError::InvalidPropertyName { .. })
        ));
    }

    // ── Validators: relationship type ───────────────────────────────

    #[test]
    fn validate_human_rel_type_accepts_the_allowed_five() {
        for ok in [
            schema::STATED_BY,
            schema::ABOUT,
            schema::CONTRADICTS,
            schema::CHARACTERIZES,
            schema::REBUTS,
        ] {
            assert!(validate_human_rel_type(ok).is_ok(), "{ok} must be allowed");
        }
    }

    #[test]
    fn validate_human_rel_type_rejects_others() {
        // A real schema rel type that is NOT human-authorable.
        assert!(matches!(
            validate_human_rel_type(schema::HAS_ELEMENT),
            Err(HumanFactError::DisallowedRelType { .. })
        ));
        assert!(matches!(
            validate_human_rel_type(""),
            Err(HumanFactError::DisallowedRelType { .. })
        ));
        assert!(
            matches!(
                validate_human_rel_type("STATED_BY]->(x) DELETE r //"),
                Err(HumanFactError::DisallowedRelType { .. })
            ),
            "an injection payload is not in the allow-list, so it is rejected"
        );
    }

    // ── Provenance constant ─────────────────────────────────────────

    #[test]
    fn provenance_constant_value_is_exact() {
        // Disk/code invariant (Standing Rule 21): the constant value is the
        // graph string. A typo would silently mismatch read queries.
        assert_eq!(PROVENANCE_HUMAN_AUTHORED, "human-authored");
    }

    // ── Error Display ────────────────────────────────────────────────
    //
    // Each variant's `thiserror` Display interpolates its fields; a silent
    // regression there (e.g. dropping `{name}` from the message) would make a
    // log line useless. Construct each variant and assert the interpolated
    // values survive into the rendered string.

    #[test]
    fn error_display_invalid_label_includes_label() {
        let msg = HumanFactError::InvalidLabel {
            label: "4Bad".to_string(),
        }
        .to_string();
        assert!(msg.contains("4Bad"), "got: {msg}");
    }

    #[test]
    fn error_display_invalid_property_name_includes_label_and_name() {
        let msg = HumanFactError::InvalidPropertyName {
            label: "Statement".to_string(),
            name: "bad name".to_string(),
        }
        .to_string();
        assert!(msg.contains("Statement"), "got: {msg}");
        assert!(msg.contains("bad name"), "got: {msg}");
    }

    #[test]
    fn error_display_reserved_property_name_includes_name_and_reason() {
        let msg = HumanFactError::ReservedPropertyName {
            name: "source_document".to_string(),
        }
        .to_string();
        assert!(msg.contains("source_document"), "got: {msg}");
        assert!(msg.contains("citation trail"), "got: {msg}");
    }

    #[test]
    fn error_display_disallowed_rel_type_includes_type_and_allowed_set() {
        let msg = HumanFactError::DisallowedRelType {
            rel_type: "FIRED".to_string(),
        }
        .to_string();
        assert!(msg.contains("FIRED"), "got: {msg}");
        // The allowed set is part of the recovery instruction.
        assert!(msg.contains("STATED_BY"), "got: {msg}");
    }

    #[test]
    fn error_display_endpoint_not_found_includes_both_ids() {
        let msg = HumanFactError::EndpointNotFound {
            from_id: "from-xyz".to_string(),
            to_id: "to-abc".to_string(),
        }
        .to_string();
        assert!(msg.contains("from-xyz"), "got: {msg}");
        assert!(msg.contains("to-abc"), "got: {msg}");
    }

    #[test]
    fn error_display_neo4j_includes_operation_and_recovery_hint() {
        // `neo4rs::Error::AuthenticationError` is a publicly constructible
        // variant (the same one the ingest-helper tests use to synthesize a
        // driver error without a live connection).
        let msg = HumanFactError::Neo4j {
            operation: "CREATE :Statement node".to_string(),
            source: neo4rs::Error::AuthenticationError("simulated".to_string()),
        }
        .to_string();
        assert!(msg.contains("CREATE :Statement node"), "got: {msg}");
        assert!(msg.contains("check graph connectivity"), "got: {msg}");
    }
}
