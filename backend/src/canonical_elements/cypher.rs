//! Cypher query builders — one function per Neo4j operation.
//!
//! Each function returns a ready-to-run [`neo4rs::Query`] with its parameters
//! bound. **No query is executed here**; execution (and row parsing) lives in
//! [`super::plan`] (reads) and [`super::loader`] (writes). Keeping the Cypher
//! in one place makes the graph contract auditable at a glance.
//!
//! ## Rust Learning: parameterized queries vs. string interpolation
//!
//! Values are always bound with `.param(...)`, never formatted into the query
//! string — that is both injection-safe and lets Neo4j cache the query plan.
//! The *only* things interpolated are node **labels** (e.g. `BreachTheory`),
//! which Cypher cannot parameterize. Those come exclusively from fixed string
//! literals in this module, never from user input, so there is no injection
//! surface.
//!
//! ## Rust Learning: `Option<T>` as a parameter
//!
//! neo4rs implements the parameter conversion for `Option<T>`: `Some(v)` binds
//! the value, `None` binds Cypher `null`. Setting a property to `null` removes
//! it, so an absent optional field and a removed property stay consistent.

use super::schema::{CountMetadata, DeclarationDef, ElementDef, TheoryDef};
use super::PROVENANCE_CANONICAL;
// Aliased to `graph_schema` to keep it distinct from `super::schema` (the YAML
// definition types). This is the shared relationship-name vocabulary so the
// upsert/read Cypher here references the same constants as the repository reads.
use crate::neo4j::schema as graph_schema;
use neo4rs::{query, Query};

// CONST: These are Neo4j node labels and relationship discriminators — they
// are graph-schema identifiers fixed at data-model time, not environment- or
// case-specific configuration. They do not vary across deployments; changing
// one requires a graph migration, not a config edit, so they are deliberately
// compiled constants rather than config values (Standing Rule 2 does not
// apply to schema identifiers). Centralized here so the wipe/upsert/read
// queries cannot drift apart, and so the (trusted, literal) label
// interpolation in this module is auditable.
pub const ELEMENT_LABEL: &str = "Element";
pub const BREACH_THEORY_LABEL: &str = "BreachTheory";
pub const IMPROPER_ACT_THEORY_LABEL: &str = "ImproperActTheory";
pub const DECLARATION_LABEL: &str = "DeclarationSought";

// CONST: HAS_THEORY relationship discriminator (see above) — a graph-schema
// value letting one relationship type carry both theory kinds.
pub const THEORY_KIND_BREACH: &str = "breach";
pub const THEORY_KIND_IMPROPER_ACT: &str = "improper_act";

// ---------------------------------------------------------------------------
// Existence + state reads (used while building the change plan)
// ---------------------------------------------------------------------------

/// Read every existing node of `label`, returning its merge key and stored
/// `content_hash` (`hash` is null for nodes the loader has never touched).
///
/// `label` and `key_prop` are trusted literals from this module (see the
/// module docs on label interpolation).
fn fetch_hashes(label: &str, key_prop: &str) -> Query {
    query(&format!(
        "MATCH (n:{label}) RETURN n.{key_prop} AS key, n.content_hash AS hash"
    ))
}

/// Existing `Element` ids → content hashes.
pub fn fetch_element_hashes() -> Query {
    fetch_hashes(ELEMENT_LABEL, "id")
}

/// Existing `BreachTheory` keys → content hashes.
pub fn fetch_breach_theory_hashes() -> Query {
    fetch_hashes(BREACH_THEORY_LABEL, "key")
}

/// Existing `ImproperActTheory` keys → content hashes.
pub fn fetch_improper_act_theory_hashes() -> Query {
    fetch_hashes(IMPROPER_ACT_THEORY_LABEL, "key")
}

/// Existing `DeclarationSought` ids → content hashes.
pub fn fetch_declaration_hashes() -> Query {
    fetch_hashes(DECLARATION_LABEL, "id")
}

/// Read the loader-managed properties of one `LegalCount`, so the plan can
/// report exactly which properties change.
pub fn fetch_legal_count_state(count_number: u32) -> Query {
    query(
        "MATCH (c:LegalCount {count_number: $count_number}) \
         RETURN c.burden_of_proof          AS burden_of_proof, \
                c.template_name             AS template_name, \
                c.m_civ_ji_reference        AS m_civ_ji_reference, \
                c.controlling_authorities_json AS controlling_authorities_json, \
                c.doctrinal_requirements_json  AS doctrinal_requirements_json, \
                c.chuck_review_required     AS chuck_review_required, \
                c.chuck_review_note         AS chuck_review_note, \
                c.special_note              AS special_note",
    )
    .param("count_number", count_number as i64)
}

/// Attribute soon-to-be-deleted orphan Elements (and their incoming
/// `BEARS_ON` edges) to the Count they currently hang off, for the
/// per-Count report. `count_number` is null for orphans with no parent Count
/// (the "unattributed" bucket).
///
/// Domain note: the 17 structurally-wrong Elements from the May extraction are
/// the orphans this sweeps; their `BEARS_ON` edges (Allegation→Element)
/// are what the `DETACH DELETE` later removes.
pub fn orphan_element_attribution(yaml_element_ids: Vec<String>) -> Query {
    query(&format!(
        "MATCH (e:Element) WHERE NOT e.id IN $yaml_ids \
         OPTIONAL MATCH (c:LegalCount)-[:{has_element}]->(e) \
         OPTIONAL MATCH (e)<-[r:{bears_on}]-() \
         RETURN c.count_number AS count_number, \
                count(DISTINCT e) AS orphan_elements, \
                count(r) AS proves_edges",
        has_element = graph_schema::HAS_ELEMENT,
        bears_on = graph_schema::BEARS_ON,
    ))
    .param("yaml_ids", yaml_element_ids)
}

// ---------------------------------------------------------------------------
// Upserts (MERGE on the stable key, then SET all managed properties)
// ---------------------------------------------------------------------------

/// Upsert one `Element` and attach it to its `LegalCount` via `HAS_ELEMENT`.
pub fn upsert_element(count_number: u32, e: &ElementDef, content_hash: &str) -> Query {
    query(&format!(
        "MERGE (e:Element {{id: $id}}) \
         SET e.element_name = $element_name, \
             e.title = $title, \
             e.order_in_count = $order_in_count, \
             e.what_plaintiff_must_prove = $what_plaintiff_must_prove, \
             e.controlling_authority = $controlling_authority, \
             e.statutory_anchor = $statutory_anchor, \
             e.case_specific_notes = $case_specific_notes, \
             e.theory_variant = $theory_variant, \
             e.parent_count_id = $parent_count_id, \
             e.content_hash = $content_hash, \
             e.updated_at = datetime() \
         WITH e \
         MATCH (c:LegalCount {{count_number: $count_number}}) \
         MERGE (c)-[:{has_element}]->(e)",
        has_element = graph_schema::HAS_ELEMENT,
    ))
    .param("id", e.id.as_str())
    .param("element_name", e.element_name.as_str())
    .param("title", e.title.as_str())
    .param("order_in_count", e.order_in_count as i64)
    .param(
        "what_plaintiff_must_prove",
        e.what_plaintiff_must_prove.as_str(),
    )
    .param("controlling_authority", e.controlling_authority.as_str())
    .param("statutory_anchor", e.statutory_anchor.clone())
    .param("case_specific_notes", e.case_specific_notes.clone())
    .param("theory_variant", e.theory_variant.clone())
    // parent_count_id denormalizes the HAS_ELEMENT backref for cheap reads.
    .param("parent_count_id", count_number as i64)
    .param("content_hash", content_hash)
    .param("count_number", count_number as i64)
}

/// Upsert one theory node and attach it via `HAS_THEORY` with a `theory_kind`
/// discriminator. Shared by breach theories and improper-act theories;
/// `label`/`theory_kind` are trusted literals from this module.
///
/// `statutory_anchor` is `null` for improper-act theories, so that property
/// simply never materializes on `ImproperActTheory` nodes.
fn upsert_theory(
    count_number: u32,
    t: &TheoryDef,
    content_hash: &str,
    label: &str,
    theory_kind: &str,
) -> Query {
    query(&format!(
        "MERGE (t:{label} {{key: $key}}) \
         SET t.definition = $definition, \
             t.statutory_anchor = $statutory_anchor, \
             t.examples = $examples, \
             t.parent_count_id = $parent_count_id, \
             t.content_hash = $content_hash, \
             t.updated_at = datetime() \
         WITH t \
         MATCH (c:LegalCount {{count_number: $count_number}}) \
         MERGE (c)-[r:{has_theory}]->(t) \
         SET r.theory_kind = $theory_kind",
        has_theory = graph_schema::HAS_THEORY,
    ))
    .param("key", t.key.as_str())
    .param("definition", t.definition.as_str())
    .param("statutory_anchor", t.statutory_anchor.clone())
    .param("examples", t.examples.as_str())
    .param("parent_count_id", count_number as i64)
    .param("content_hash", content_hash)
    .param("count_number", count_number as i64)
    .param("theory_kind", theory_kind)
}

/// Upsert a Count I breach theory.
pub fn upsert_breach_theory(count_number: u32, t: &TheoryDef, content_hash: &str) -> Query {
    upsert_theory(
        count_number,
        t,
        content_hash,
        BREACH_THEORY_LABEL,
        THEORY_KIND_BREACH,
    )
}

/// Upsert a Count IV improper-act theory.
pub fn upsert_improper_act_theory(count_number: u32, t: &TheoryDef, content_hash: &str) -> Query {
    upsert_theory(
        count_number,
        t,
        content_hash,
        IMPROPER_ACT_THEORY_LABEL,
        THEORY_KIND_IMPROPER_ACT,
    )
}

/// Upsert one `DeclarationSought` and attach it via `SEEKS_DECLARATION`.
pub fn upsert_declaration(count_number: u32, d: &DeclarationDef, content_hash: &str) -> Query {
    query(&format!(
        "MERGE (d:DeclarationSought {{id: $id}}) \
         SET d.declaration = $declaration, \
             d.legal_basis = $legal_basis, \
             d.operative = $operative, \
             d.inoperative_reason = $inoperative_reason, \
             d.parent_count_id = $parent_count_id, \
             d.content_hash = $content_hash, \
             d.updated_at = datetime() \
         WITH d \
         MATCH (c:LegalCount {{count_number: $count_number}}) \
         MERGE (c)-[:{seeks_declaration}]->(d)",
        seeks_declaration = graph_schema::SEEKS_DECLARATION,
    ))
    .param("id", d.id.as_str())
    .param("declaration", d.declaration.as_str())
    .param("legal_basis", d.legal_basis.as_str())
    .param("operative", d.operative)
    .param("inoperative_reason", d.inoperative_reason.clone())
    .param("parent_count_id", count_number as i64)
    .param("content_hash", content_hash)
    .param("count_number", count_number as i64)
}

/// Upsert the loader-managed properties of a `LegalCount`, creating the node
/// if it doesn't already exist.
///
/// ## Why MERGE on `count_number`
///
/// `count_number` is the count's structural identity (1–4) and never changes,
/// so it is the safe MERGE key. Converting the former MATCH to MERGE removes
/// the create/update asymmetry and keeps this query self-sufficient. (Today
/// the node is still expected to pre-exist: `plan::build_plan` reads its
/// current state first and errors if it is missing — so in practice this
/// MERGE always MATCHes and `ON CREATE` is dormant safety. Letting the loader
/// own creation outright is deferred work in the plan/state layer.)
///
/// `provenance = 'canonical'` marks the node loader-owned on both branches.
/// `created_at` is stamped only `ON CREATE`; `canonical_updated_at` advances on
/// every managed write, matching the prior MATCH behaviour.
///
/// The cross-tier `id` (`count-{N}`) is intentionally *not* set here.
/// [`set_legal_count_id`] stamps it unconditionally for every Count — including
/// runs where no managed property changed and this upsert is skipped by the
/// caller's property-diff guard — so stamping it here too would be redundant.
///
/// `controlling_authorities_json` is always set (possibly `"[]"`).
/// `doctrinal_requirements_json` is `None` for Counts without doctrinal
/// requirements, so that property is removed/never set there. The four
/// review/note/special fields are likewise `None` outside the Counts that use
/// them.
pub fn upsert_legal_count(
    meta: &CountMetadata,
    controlling_authorities_json: String,
    doctrinal_requirements_json: Option<String>,
) -> Query {
    query(
        "MERGE (c:LegalCount {count_number: $count_number}) \
         ON CREATE SET c.burden_of_proof = $burden_of_proof, \
                       c.template_name = $template_name, \
                       c.m_civ_ji_reference = $m_civ_ji_reference, \
                       c.controlling_authorities_json = $controlling_authorities_json, \
                       c.doctrinal_requirements_json = $doctrinal_requirements_json, \
                       c.chuck_review_required = $chuck_review_required, \
                       c.chuck_review_note = $chuck_review_note, \
                       c.special_note = $special_note, \
                       c.provenance = $provenance, \
                       c.created_at = datetime(), \
                       c.canonical_updated_at = datetime() \
         ON MATCH SET  c.burden_of_proof = $burden_of_proof, \
                       c.template_name = $template_name, \
                       c.m_civ_ji_reference = $m_civ_ji_reference, \
                       c.controlling_authorities_json = $controlling_authorities_json, \
                       c.doctrinal_requirements_json = $doctrinal_requirements_json, \
                       c.chuck_review_required = $chuck_review_required, \
                       c.chuck_review_note = $chuck_review_note, \
                       c.special_note = $special_note, \
                       c.provenance = $provenance, \
                       c.canonical_updated_at = datetime()",
    )
    .param("count_number", meta.count_number as i64)
    .param("burden_of_proof", meta.burden_of_proof.as_str())
    .param("template_name", meta.template_name.as_str())
    .param("m_civ_ji_reference", meta.m_civ_ji_reference.clone())
    .param("controlling_authorities_json", controlling_authorities_json)
    .param("doctrinal_requirements_json", doctrinal_requirements_json)
    .param("chuck_review_required", meta.chuck_review_required)
    .param("chuck_review_note", meta.chuck_review_note.clone())
    .param("special_note", meta.special_note.clone())
    .param("provenance", PROVENANCE_CANONICAL)
}

/// Stamp the cross-tier `id` property on a `LegalCount` node.
///
/// Run unconditionally for every Count (not gated on the property diff) so
/// the Neo4j node's `id` always equals `authored_entities.entity_id`
/// (`count-{N}`). That shared string id is how the ingest step's MATCH
/// connects Tier-1 (authored) ↔ Tier-2 (extracted) edges. The `LegalCount`
/// node is created earlier by the case-structuring pipeline and keyed by
/// `count_number`; this only adds the matching `id`, leaving the
/// loader-managed property set (and `canonical_updated_at`) untouched so it
/// doesn't disturb the content-hash idempotency.
pub fn set_legal_count_id(count_number: u32, count_id: &str) -> Query {
    query("MATCH (c:LegalCount {count_number: $count_number}) SET c.id = $count_id")
        .param("count_number", count_number as i64)
        .param("count_id", count_id)
}

// ---------------------------------------------------------------------------
// Orphan wipes (DETACH DELETE anything not present in the YAML)
// ---------------------------------------------------------------------------

/// Build a `DETACH DELETE` for orphans of `label` whose `key_prop` is not in
/// the kept-keys list. `RETURN count(...)` reports how many were deleted.
/// `DETACH` also removes incoming relationships (e.g. `BEARS_ON`).
fn wipe_orphans(label: &str, key_prop: &str, kept_keys: Vec<String>) -> Query {
    query(&format!(
        "MATCH (n:{label}) WHERE NOT n.{key_prop} IN $kept \
         WITH n DETACH DELETE n RETURN count(n) AS deleted"
    ))
    .param("kept", kept_keys)
}

/// Delete every `Element` whose id is not in the YAML (the wrong Elements),
/// detaching their `BEARS_ON` edges in the process.
pub fn wipe_orphan_elements(yaml_element_ids: Vec<String>) -> Query {
    wipe_orphans(ELEMENT_LABEL, "id", yaml_element_ids)
}

/// Delete every `BreachTheory` whose key is not in the YAML.
pub fn wipe_orphan_breach_theories(yaml_keys: Vec<String>) -> Query {
    wipe_orphans(BREACH_THEORY_LABEL, "key", yaml_keys)
}

/// Delete every `ImproperActTheory` whose key is not in the YAML.
pub fn wipe_orphan_improper_act_theories(yaml_keys: Vec<String>) -> Query {
    wipe_orphans(IMPROPER_ACT_THEORY_LABEL, "key", yaml_keys)
}

/// Delete every `DeclarationSought` whose id is not in the YAML.
pub fn wipe_orphan_declarations(yaml_ids: Vec<String>) -> Query {
    wipe_orphans(DECLARATION_LABEL, "id", yaml_ids)
}
