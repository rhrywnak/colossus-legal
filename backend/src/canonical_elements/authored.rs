//! Tier-1 authored-entity writes to Postgres (Option A).
//!
//! The canonical loader is the system of record's writer: it replaces this
//! case's authored `LegalCount` / `Element` rows (`authored_entities`) and
//! their `HAS_ELEMENT` edges (`authored_relationships`) on every run, so the
//! YAML files are the source of truth. The matching Neo4j nodes/edges are the
//! operational copy (written by [`super::loader`] / [`super::cypher`]); the
//! shared string `entity_id` (`count-{N}`, `element-1-1`) is what lets the
//! two tiers connect.
//!
//! All DB operations go through the `authored_entities` repository functions,
//! enrolled in one transaction (`&mut *txn`) so a partial failure leaves the
//! tables untouched rather than half-written (Standing Rule 1).

use super::schema::{CountFile, CountMetadata, DeclarationDef, ElementDef, TheoryDef};
use super::{CanonicalLoaderError, PROVENANCE_CANONICAL};
use crate::repositories::pipeline_repository::{
    delete_authored_entities_for_case, delete_authored_relationships_by_type,
    upsert_authored_entity, upsert_authored_relationship,
};
use sqlx::PgPool;
use tracing::instrument;

// CONST: data-model identifiers for the Tier-1 authored writes — the
// `authored_entities.entity_type` / `authored_relationships.relationship_type`
// discriminators and the loader's author sentinel. Schema values fixed at
// data-model time, deliberately equal to the Neo4j labels in [`super::cypher`]
// so a row's `entity_type` and its graph node's label agree. Not environment-
// or case-specific, so they are compiled constants rather than configuration
// (Standing Rule 2 does not apply to schema identifiers). The shared
// provenance marker lives in the parent module ([`super::PROVENANCE_CANONICAL`])
// so both tiers stamp the same value.
const ENTITY_TYPE_LEGAL_COUNT: &str = "LegalCount";
const ENTITY_TYPE_ELEMENT: &str = "Element";
const ENTITY_TYPE_BREACH_THEORY: &str = "BreachTheory";
const ENTITY_TYPE_IMPROPER_ACT_THEORY: &str = "ImproperActTheory";
const ENTITY_TYPE_DECLARATION_SOUGHT: &str = "DeclarationSought";
const REL_HAS_ELEMENT: &str = "HAS_ELEMENT";
// HAS_THEORY carries both breach and improper-act theories (the target row's
// `entity_type` distinguishes them, mirroring the single Neo4j HAS_THEORY edge
// in [`super::cypher::upsert_theory`]). SEEKS_DECLARATION links a Count to a
// declaration it asks the court to issue.
const REL_HAS_THEORY: &str = "HAS_THEORY";
const REL_SEEKS_DECLARATION: &str = "SEEKS_DECLARATION";
const CREATED_BY_LOADER: &str = "loader";

/// How many authored rows the YAML produced, for the change report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AuthoredCounts {
    /// `authored_entities` rows: one per LegalCount, Element, BreachTheory,
    /// ImproperActTheory, and DeclarationSought.
    pub entities: u64,
    /// `authored_relationships` rows: one `HAS_ELEMENT` per Element, one
    /// `HAS_THEORY` per theory, and one `SEEKS_DECLARATION` per declaration.
    pub relationships: u64,
}

/// Stable `authored_entities.entity_id` for a LegalCount: `count-{N}`.
///
/// This same string is stamped onto the Neo4j `LegalCount.id` by
/// [`super::cypher::set_legal_count_id`], so the two tiers share one id.
pub fn legal_count_entity_id(count_number: u32) -> String {
    format!("count-{count_number}")
}

/// Build the `authored_entities.item_data` JSONB for a LegalCount.
///
/// `paragraph_range` is intentionally omitted — the canonical YAML schema
/// ([`CountMetadata`]) has no such field.
fn legal_count_item_data(meta: &CountMetadata) -> serde_json::Value {
    serde_json::json!({
        "count_number": meta.count_number,
        "count_name": meta.count_name,
        "burden_of_proof": meta.burden_of_proof,
        "controlling_authorities": meta.controlling_authorities,
        "m_civ_ji_reference": meta.m_civ_ji_reference,
        "template_name": meta.template_name,
    })
}

/// Build the `authored_entities.item_data` JSONB for an Element.
///
/// `parent_count_number` ties the Element back to its Count — the same link
/// the `HAS_ELEMENT` relationship encodes, denormalised for cheap reads.
fn element_item_data(e: &ElementDef, parent_count_number: u32) -> serde_json::Value {
    serde_json::json!({
        "element_name": e.element_name,
        "title": e.title,
        "order_in_count": e.order_in_count,
        "what_plaintiff_must_prove": e.what_plaintiff_must_prove,
        "controlling_authority": e.controlling_authority,
        "statutory_anchor": e.statutory_anchor,
        "case_specific_notes": e.case_specific_notes,
        "theory_variant": e.theory_variant,
        "parent_count_number": parent_count_number,
    })
}

/// Build the `authored_entities.item_data` JSONB for a breach or improper-act
/// theory.
///
/// Mirrors the property set [`super::cypher::upsert_theory`] writes to the
/// matching Neo4j node (`definition`, `statutory_anchor`, `examples`), so the
/// two tiers carry the same fields. `parent_count_number` ties the theory back
/// to its Count — the same link the `HAS_THEORY` edge encodes, denormalised for
/// cheap reads (parallel to [`element_item_data`]).
fn theory_item_data(t: &TheoryDef, parent_count_number: u32) -> serde_json::Value {
    serde_json::json!({
        "key": t.key,
        "definition": t.definition,
        "statutory_anchor": t.statutory_anchor,
        "examples": t.examples,
        "parent_count_number": parent_count_number,
    })
}

/// Build the `authored_entities.item_data` JSONB for a declaration sought.
///
/// Mirrors the property set [`super::cypher::upsert_declaration`] writes to the
/// matching Neo4j node. `parent_count_number` denormalises the
/// `SEEKS_DECLARATION` backref, as in [`theory_item_data`].
fn declaration_item_data(d: &DeclarationDef, parent_count_number: u32) -> serde_json::Value {
    serde_json::json!({
        "id": d.id,
        "declaration": d.declaration,
        "legal_basis": d.legal_basis,
        "operative": d.operative,
        "inoperative_reason": d.inoperative_reason,
        "parent_count_number": parent_count_number,
    })
}

/// Count the authored rows the YAML would produce. Pure — drives the report
/// on both the real and dry-run paths, so the reported numbers match whether
/// or not writes actually happened.
pub fn count_authored(files: &[CountFile]) -> AuthoredCounts {
    let element_total: u64 = files.iter().map(|f| f.elements.len() as u64).sum();
    let breach_total: u64 = files.iter().map(|f| f.breach_theories.len() as u64).sum();
    let improper_total: u64 = files
        .iter()
        .map(|f| f.improper_act_theories.len() as u64)
        .sum();
    let declaration_total: u64 = files
        .iter()
        .map(|f| f.declarations_sought.len() as u64)
        .sum();

    // Each theory contributes one HAS_THEORY edge, each declaration one
    // SEEKS_DECLARATION edge — so the child-node total equals the non-element
    // relationship total. Computed once and reused for both fields.
    let theory_and_declaration_total = breach_total + improper_total + declaration_total;

    AuthoredCounts {
        // entities: one LegalCount per file + every child node.
        entities: files.len() as u64 + element_total + theory_and_declaration_total,
        // relationships: HAS_ELEMENT per Element + HAS_THEORY/SEEKS_DECLARATION.
        relationships: element_total + theory_and_declaration_total,
    }
}

/// Map any Postgres-side error to [`CanonicalLoaderError::Postgres`], tagging
/// the operation so the failure is locatable in the logs (Standing Rule 1).
///
/// Generic over the error's `Display` because the sources differ
/// (`sqlx::Error` from begin/commit, `PipelineRepoError` from the repo fns);
/// both carry their own context in the message.
fn pg_err<E: std::fmt::Display>(operation: &'static str) -> impl Fn(E) -> CanonicalLoaderError {
    move |e| CanonicalLoaderError::Postgres {
        operation: operation.to_string(),
        message: e.to_string(),
    }
}

/// Replace this case's authored Elements/Counts and their `HAS_ELEMENT` edges
/// in Postgres, in a single transaction.
///
/// Delete-then-upsert makes the YAML the source of truth: each run replaces
/// the full authored set, so an Element removed from the YAML is removed here
/// too. The whole sequence runs in one `pool.begin()` … `txn.commit()`
/// transaction — a partial failure rolls back (the `Transaction` drops
/// without commit), never leaving the tables half-written. The repository
/// functions receive `&mut *txn`, so they enrol in this transaction.
#[instrument(skip(pool, files), fields(step = "write_authored_entities", case_slug = %case_slug, file_count = files.len()))]
pub async fn write_authored_entities(
    pool: &PgPool,
    case_slug: &str,
    files: &[CountFile],
) -> Result<(), CanonicalLoaderError> {
    let mut txn = pool.begin().await.map_err(pg_err("begin transaction"))?;

    // Reconcile: drop the prior authored set for this case first.
    // `delete_authored_entities_for_case` clears every entity row (so a type
    // dropped from the YAML disappears too); relationships are deleted per
    // type so each edge kind the loader manages is rebuilt from scratch. The
    // per-type loop keeps the failing edge kind named in the error (Rule 1).
    for rel_type in [REL_HAS_ELEMENT, REL_HAS_THEORY, REL_SEEKS_DECLARATION] {
        delete_authored_relationships_by_type(&mut *txn, case_slug, rel_type)
            .await
            .map_err(|err| CanonicalLoaderError::Postgres {
                operation: format!("delete {rel_type} relationships"),
                message: err.to_string(),
            })?;
    }
    delete_authored_entities_for_case(&mut *txn, case_slug)
        .await
        .map_err(pg_err("delete authored entities"))?;

    for file in files {
        let count_id = legal_count_entity_id(file.count.count_number);
        upsert_authored_entity(
            &mut *txn,
            case_slug,
            ENTITY_TYPE_LEGAL_COUNT,
            &count_id,
            &legal_count_item_data(&file.count),
            PROVENANCE_CANONICAL,
            Some(CREATED_BY_LOADER),
        )
        .await
        .map_err(|err| CanonicalLoaderError::Postgres {
            operation: format!("upsert LegalCount {count_id}"),
            message: err.to_string(),
        })?;

        // Children (Elements, theories, declarations) + their edges. Extracted
        // to keep this function within the per-function line budget (Rule 18).
        write_count_children(&mut txn, case_slug, &count_id, file).await?;
    }

    txn.commit().await.map_err(pg_err("commit transaction"))?;
    Ok(())
}

/// Upsert one Count's child entities and their edges into the open
/// transaction: Elements (`HAS_ELEMENT`), breach + improper-act theories
/// (`HAS_THEORY`), and declarations sought (`SEEKS_DECLARATION`).
///
/// ## Rust Learning: `&mut sqlx::PgConnection` to share one transaction
///
/// The caller holds an owned `Transaction`; `&mut *txn` derefs it to the
/// underlying `PgConnection`. Passing that `&mut PgConnection` down lets every
/// write here enrol in the *same* transaction, so a failure mid-Count rolls
/// the whole load back. Each `upsert_*` call reborrows with `&mut *txn`.
async fn write_count_children(
    txn: &mut sqlx::PgConnection,
    case_slug: &str,
    count_id: &str,
    file: &CountFile,
) -> Result<(), CanonicalLoaderError> {
    let parent = file.count.count_number;
    for e in &file.elements {
        write_element(&mut *txn, case_slug, count_id, e, parent).await?;
    }
    // Count I breach theories and Count IV improper-act theories both become
    // `HAS_THEORY` edges; the entity row's `entity_type` discriminates them.
    for t in &file.breach_theories {
        write_theory(
            &mut *txn,
            case_slug,
            count_id,
            ENTITY_TYPE_BREACH_THEORY,
            t,
            parent,
        )
        .await?;
    }
    for t in &file.improper_act_theories {
        write_theory(
            &mut *txn,
            case_slug,
            count_id,
            ENTITY_TYPE_IMPROPER_ACT_THEORY,
            t,
            parent,
        )
        .await?;
    }
    for d in &file.declarations_sought {
        write_declaration(&mut *txn, case_slug, count_id, d, parent).await?;
    }
    Ok(())
}

/// Upsert one Element and its `HAS_ELEMENT` edge from the parent Count.
async fn write_element(
    txn: &mut sqlx::PgConnection,
    case_slug: &str,
    count_id: &str,
    e: &ElementDef,
    parent_count_number: u32,
) -> Result<(), CanonicalLoaderError> {
    upsert_authored_entity(
        &mut *txn,
        case_slug,
        ENTITY_TYPE_ELEMENT,
        &e.id,
        &element_item_data(e, parent_count_number),
        PROVENANCE_CANONICAL,
        Some(CREATED_BY_LOADER),
    )
    .await
    .map_err(|err| CanonicalLoaderError::Postgres {
        operation: format!("upsert Element {}", e.id),
        message: err.to_string(),
    })?;

    // `order_in_count` rides on the edge so the mapping layer can render
    // Elements in pleading order without re-reading the node.
    let props = serde_json::json!({ "order_in_count": e.order_in_count });
    upsert_authored_relationship(
        &mut *txn,
        case_slug,
        count_id,
        &e.id,
        REL_HAS_ELEMENT,
        Some(&props),
        PROVENANCE_CANONICAL,
        Some(CREATED_BY_LOADER),
    )
    .await
    .map_err(|err| CanonicalLoaderError::Postgres {
        operation: format!("upsert HAS_ELEMENT {count_id}->{}", e.id),
        message: err.to_string(),
    })?;
    Ok(())
}

/// Upsert one theory (`entity_type` is `BreachTheory` or `ImproperActTheory`)
/// and its `HAS_THEORY` edge from the parent Count.
///
/// The edge carries no properties: the two theory kinds are distinguished by
/// the target row's `entity_type`, so a `theory_kind` edge property would only
/// duplicate that (unlike the Neo4j edge, which has no node-label to read).
async fn write_theory(
    txn: &mut sqlx::PgConnection,
    case_slug: &str,
    count_id: &str,
    entity_type: &str,
    t: &TheoryDef,
    parent_count_number: u32,
) -> Result<(), CanonicalLoaderError> {
    upsert_authored_entity(
        &mut *txn,
        case_slug,
        entity_type,
        &t.key,
        &theory_item_data(t, parent_count_number),
        PROVENANCE_CANONICAL,
        Some(CREATED_BY_LOADER),
    )
    .await
    .map_err(|err| CanonicalLoaderError::Postgres {
        operation: format!("upsert {entity_type} {}", t.key),
        message: err.to_string(),
    })?;

    upsert_authored_relationship(
        &mut *txn,
        case_slug,
        count_id,
        &t.key,
        REL_HAS_THEORY,
        None,
        PROVENANCE_CANONICAL,
        Some(CREATED_BY_LOADER),
    )
    .await
    .map_err(|err| CanonicalLoaderError::Postgres {
        operation: format!("upsert HAS_THEORY {count_id}->{}", t.key),
        message: err.to_string(),
    })?;
    Ok(())
}

/// Upsert one declaration sought and its `SEEKS_DECLARATION` edge from the
/// parent Count.
async fn write_declaration(
    txn: &mut sqlx::PgConnection,
    case_slug: &str,
    count_id: &str,
    d: &DeclarationDef,
    parent_count_number: u32,
) -> Result<(), CanonicalLoaderError> {
    upsert_authored_entity(
        &mut *txn,
        case_slug,
        ENTITY_TYPE_DECLARATION_SOUGHT,
        &d.id,
        &declaration_item_data(d, parent_count_number),
        PROVENANCE_CANONICAL,
        Some(CREATED_BY_LOADER),
    )
    .await
    .map_err(|err| CanonicalLoaderError::Postgres {
        operation: format!("upsert DeclarationSought {}", d.id),
        message: err.to_string(),
    })?;

    upsert_authored_relationship(
        &mut *txn,
        case_slug,
        count_id,
        &d.id,
        REL_SEEKS_DECLARATION,
        None,
        PROVENANCE_CANONICAL,
        Some(CREATED_BY_LOADER),
    )
    .await
    .map_err(|err| CanonicalLoaderError::Postgres {
        operation: format!("upsert SEEKS_DECLARATION {count_id}->{}", d.id),
        message: err.to_string(),
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Pure-data tests for the entity_id / item_data / count helpers. The
    //! DB-touching `write_authored_entities` is exercised by the live-DB
    //! integration tests (no `#[sqlx::test]` infra in this repo).
    use super::*;

    fn element(id: &str, order: u32) -> ElementDef {
        ElementDef {
            id: id.to_string(),
            order_in_count: order,
            element_name: "Duty".into(),
            title: "Duty of Care".into(),
            theory_variant: None,
            what_plaintiff_must_prove: "prove duty".into(),
            controlling_authority: "Some v Case".into(),
            statutory_anchor: None,
            case_specific_notes: Some("note".into()),
        }
    }

    fn count_file(n: u32, element_ids: &[&str]) -> CountFile {
        CountFile {
            count: CountMetadata {
                count_number: n,
                count_name: format!("Count {n}"),
                template_name: "tmpl".into(),
                burden_of_proof: "preponderance".into(),
                m_civ_ji_reference: None,
                chuck_review_required: None,
                chuck_review_note: None,
                special_note: None,
                controlling_authorities: vec![],
                doctrinal_requirements: vec![],
            },
            elements: element_ids
                .iter()
                .enumerate()
                .map(|(i, id)| element(id, i as u32 + 1))
                .collect(),
            breach_theories: vec![],
            improper_act_theories: vec![],
            declarations_sought: vec![],
        }
    }

    fn theory(key: &str) -> TheoryDef {
        TheoryDef {
            key: key.to_string(),
            definition: "def".into(),
            statutory_anchor: None,
            examples: "ex".into(),
        }
    }

    fn declaration(id: &str, operative: bool) -> DeclarationDef {
        DeclarationDef {
            id: id.to_string(),
            declaration: "decl text".into(),
            legal_basis: "basis".into(),
            operative,
            inoperative_reason: None,
        }
    }

    #[test]
    fn legal_count_entity_id_is_count_dash_n() {
        assert_eq!(legal_count_entity_id(1), "count-1");
        assert_eq!(legal_count_entity_id(4), "count-4");
    }

    #[test]
    fn element_item_data_carries_parent_count_and_fields() {
        let e = element("element-2-3", 3);
        let v = element_item_data(&e, 2);
        assert_eq!(v["element_name"], "Duty");
        assert_eq!(v["order_in_count"], 3);
        assert_eq!(v["parent_count_number"], 2);
        assert_eq!(v["case_specific_notes"], "note");
        // statutory_anchor is None → JSON null (distinct from absent).
        assert!(v["statutory_anchor"].is_null());
    }

    #[test]
    fn legal_count_item_data_omits_paragraph_range() {
        let f = count_file(1, &["element-1-1"]);
        let v = legal_count_item_data(&f.count);
        assert_eq!(v["count_number"], 1);
        assert_eq!(v["template_name"], "tmpl");
        assert!(
            v.get("paragraph_range").is_none(),
            "the canonical schema has no paragraph_range field"
        );
    }

    #[test]
    fn count_authored_totals_entities_and_relationships() {
        // Count 1 has two Elements, Count 2 has one.
        let files = vec![
            count_file(1, &["element-1-1", "element-1-2"]),
            count_file(2, &["element-2-1"]),
        ];
        let c = count_authored(&files);
        // entities = 2 LegalCounts + 3 Elements = 5; relationships = 3 HAS_ELEMENT.
        assert_eq!(c.entities, 5);
        assert_eq!(c.relationships, 3);
    }

    #[test]
    fn theory_item_data_carries_fields_and_parent() {
        let v = theory_item_data(&theory("loyalty"), 1);
        assert_eq!(v["key"], "loyalty");
        assert_eq!(v["definition"], "def");
        assert_eq!(v["examples"], "ex");
        assert_eq!(v["parent_count_number"], 1);
        // statutory_anchor is None → JSON null (distinct from absent).
        assert!(v["statutory_anchor"].is_null());
    }

    #[test]
    fn declaration_item_data_carries_fields_and_parent() {
        let v = declaration_item_data(&declaration("declaration-3-a", false), 3);
        assert_eq!(v["id"], "declaration-3-a");
        assert_eq!(v["declaration"], "decl text");
        assert_eq!(v["legal_basis"], "basis");
        assert_eq!(v["operative"], false);
        assert_eq!(v["parent_count_number"], 3);
        assert!(v["inoperative_reason"].is_null());
    }

    #[test]
    fn count_authored_includes_theories_and_declarations() {
        // Count 1: 2 Elements + 2 breach theories.
        let mut c1 = count_file(1, &["element-1-1", "element-1-2"]);
        c1.breach_theories = vec![theory("loyalty"), theory("care")];
        // Count 3: 1 Element + 1 declaration.
        let mut c3 = count_file(3, &["element-3-1"]);
        c3.declarations_sought = vec![declaration("decl-3-1", true)];
        // Count 4: 0 Elements + 1 improper-act theory.
        let mut c4 = count_file(4, &[]);
        c4.improper_act_theories = vec![theory("motive")];

        let c = count_authored(&[c1, c3, c4]);
        // entities = 3 LegalCounts + 3 Elements + 2 breach + 1 improper + 1 decl = 10.
        assert_eq!(c.entities, 10);
        // relationships = 3 HAS_ELEMENT + 3 HAS_THEORY + 1 SEEKS_DECLARATION = 7.
        assert_eq!(c.relationships, 7);
    }
}
