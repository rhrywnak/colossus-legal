//! Cross-document context loader for the pass-2 prompt.
//!
//! Reads pass-1 entities from OTHER PUBLISHED documents and renders
//! them as prompt-shaped values (`{{entities_json}}` companion set in
//! the pass-2 prompt assembly). The per-type property allowlist trims
//! large fields (e.g. `verbatim_quote` on `ComplaintAllegation`) so
//! prompt size stays bounded as the case grows.
//!
//! The entity-type whitelist [`CROSS_DOC_ENTITY_TYPES`] is the data-
//! model fence that makes a v5.1-style silent drop impossible: every
//! type the cross-doc query is willing to surface is named here, the
//! regression-guard test in this file's `#[cfg(test)] mod tests`
//! locks in membership, and the `## CONST:` doc explains why an
//! operator YAML toggle would be a false escape hatch.

use sqlx::PgPool;

use crate::models::document_status::{
    ENTITY_COMPLAINT_ALLEGATION, ENTITY_LEGAL_COUNT, PARTY_SUBTYPES, REVIEW_STATUS_APPROVED,
    RUN_STATUS_COMPLETED, STATUS_PUBLISHED,
};

use super::authored_entities::AuthoredEntityRecord;
use super::PipelineRepoError;

// ── Constants ────────────────────────────────────────────────────

/// Prefix applied to cross-document entity ids in the pass-2 prompt.
///
/// The LLM receives prefixed ids like `"ctx:allegation-014"` so ids
/// from other documents can't collide with the current document's
/// local pass-1 ids (e.g., two docs both authoring `party-001`). When
/// the LLM emits a cross-document relationship, the endpoint string
/// retains the prefix and `store_pass2_relationships` resolves it via
/// the extended id_map the step builds from these entities.
pub const CROSS_DOC_ID_PREFIX: &str = "ctx:";

/// Entity types surfaced in the pass-2 cross-document context.
///
/// `Party` entities get rewritten to `Person`/`Organization` by Ingest
/// (R4), so we match against the effective type via the same
/// `COALESCE(resolved_entity_type, entity_type)` projection used by
/// every other item SELECT — otherwise post-Ingest Party rows would
/// fail the filter and drop out of the context.
///
/// ## CONST: why this is a compile-time list, not env/YAML config
///
/// Each entry is a Neo4j node label / `extraction_items.entity_type`
/// discriminator — i.e., a data-model identifier rather than an
/// operator-tunable threshold. Adding a label requires three coupled
/// changes that must land together: (1) the extraction schema YAML
/// must define the type, (2) `filter_properties_for_prompt` below
/// must decide which properties of that type are useful in the
/// prompt (or fall through to the wildcard arm), and (3) downstream
/// graph ingest must know how to write the label. None of those
/// downstream changes can be driven from a YAML toggle alone, so
/// operator-tunability via env/YAML would offer a false escape hatch:
/// flipping a flag without the matching schema + ingest support would
/// surface entities the LLM cannot reason about and the graph cannot
/// store. Keeping the list `const` forces the three changes to land
/// in the same commit and the `cross_doc_entity_types_includes_v5_1_labels`
/// test guards against silent regressions.
///
/// ## Why each type is included
/// - `Party` / `Person` / `Organization` — actors named across
///   documents; required so cross-doc relationships can resolve both
///   endpoints when one is a re-mention of an already-extracted actor.
/// - `LegalCount` — counts (causes of action) named in the complaint
///   that downstream evidence-anchoring documents cite via CORROBORATES.
/// - `ComplaintAllegation` / `Allegation` — both labels coexist:
///   `ComplaintAllegation` is the v4-era / pre-v5.1 label, `Allegation`
///   is the v5.1 complaint-schema label. Filtering on only one would
///   silently drop the other version's data from the cross-doc context.
/// - `Evidence` — evidence-anchoring profiles (affidavit,
///   discovery_response) emit `Evidence` entities; peer documents need
///   them as endpoints for CONTRADICTS / REBUTS.
/// - `Element` / `Harm` — proof-chain entities in v5.1. The pass-2 LLM
///   may anchor cross-document relationships against them (e.g., a
///   discovery response that admits the factual basis for an `Element`
///   on the opposing party's complaint).
///
/// `pub` so the Pass-2 step can pass this same whitelist to
/// [`load_authored_entities_for_context`] — keeping a single source of
/// truth for "which types participate in cross-document context" rather
/// than duplicating the list at the call site (Standing Rule 2).
pub const CROSS_DOC_ENTITY_TYPES: &[&str] = &[
    crate::models::document_status::ENTITY_PARTY,
    crate::models::document_status::ENTITY_PERSON,
    crate::models::document_status::ENTITY_ORGANIZATION,
    crate::models::document_status::ENTITY_LEGAL_COUNT,
    crate::models::document_status::ENTITY_COMPLAINT_ALLEGATION,
    crate::models::document_status::ENTITY_ALLEGATION,
    crate::models::document_status::ENTITY_EVIDENCE,
    crate::models::document_status::ENTITY_ELEMENT,
    crate::models::document_status::ENTITY_HARM,
];

// ── Types ────────────────────────────────────────────────────────

/// An entity loaded from another PUBLISHED document's pass-1 run for
/// injection into the current document's pass-2 prompt.
///
/// Carries both the original LLM id (as authored in the source doc)
/// and the prefixed id (used in the prompt and id_map). Serializing
/// via [`Self::to_prompt_value`] emits the prefixed id plus a
/// `source_document` / `source_document_type` pair so the LLM can see
/// which document contributed each entity.
#[derive(Debug, Clone)]
pub struct CrossDocEntity {
    /// DB primary key in `extraction_items` — target for cross-doc
    /// relationship endpoints.
    pub item_id: i32,
    /// LLM id as authored in the source document (e.g., `"party-001"`).
    pub original_id: String,
    /// Id used in the current doc's prompt and id_map
    /// (`CROSS_DOC_ID_PREFIX + original_id`).
    pub prefixed_id: String,
    /// Source document id — the `documents.id` this entity belongs to.
    pub source_document_id: String,
    /// Source document type (`complaint`, `discovery_response`, etc.)
    /// — propagates `documents.document_type` so the LLM can reason
    /// about provenance.
    pub source_document_type: String,
    /// Effective entity type (COALESCE of `resolved_entity_type` and
    /// `entity_type`) — what Ingest resolved the entity to.
    pub entity_type: String,
    /// Short human-readable label, if the source pass-1 output set one.
    pub label: Option<String>,
    /// Full property object from the source `item_data.properties`.
    /// [`Self::to_prompt_value`] applies a per-type allowlist to keep
    /// prompt size reasonable.
    pub properties: serde_json::Value,
}

/// Row shape returned by [`load_cross_document_context`]'s join query.
#[derive(sqlx::FromRow)]
struct CrossDocRow {
    item_id: i32,
    item_data: serde_json::Value,
    source_document_id: String,
    source_document_type: String,
    effective_entity_type: String,
}

impl CrossDocEntity {
    fn from_row(row: CrossDocRow) -> Self {
        let original_id = row
            .item_data
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let label = row
            .item_data
            .get("label")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let properties = row
            .item_data
            .get("properties")
            .cloned()
            .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
        let prefixed_id = format!("{CROSS_DOC_ID_PREFIX}{original_id}");
        Self {
            item_id: row.item_id,
            original_id,
            prefixed_id,
            source_document_id: row.source_document_id,
            source_document_type: row.source_document_type,
            entity_type: row.effective_entity_type,
            label,
            properties,
        }
    }

    /// Render the prompt-facing subset for injection into `{{entities_json}}`.
    ///
    /// Applies a per-type property allowlist to trim the payload —
    /// `verbatim_quote` in particular is dropped from
    /// `ComplaintAllegation` because it's large and not needed for the
    /// LLM's link-or-not decision. Types outside the allowlist fall
    /// through with their full properties intact (cheap insurance
    /// against schema drift).
    pub fn to_prompt_value(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        obj.insert(
            "id".into(),
            serde_json::Value::String(self.prefixed_id.clone()),
        );
        obj.insert(
            "entity_type".into(),
            serde_json::Value::String(self.entity_type.clone()),
        );
        if let Some(label) = &self.label {
            obj.insert("label".into(), serde_json::Value::String(label.clone()));
        }
        obj.insert(
            "source_document".into(),
            serde_json::Value::String(self.source_document_id.clone()),
        );
        obj.insert(
            "source_document_type".into(),
            serde_json::Value::String(self.source_document_type.clone()),
        );
        obj.insert(
            "properties".into(),
            filter_properties_for_prompt(&self.entity_type, &self.properties),
        );
        serde_json::Value::Object(obj)
    }
}

// ── Helpers ──────────────────────────────────────────────────────

/// Drop properties that aren't useful for cross-doc link decisions.
///
/// Keeps prompt size bounded as the number of PUBLISHED documents
/// grows. The allowlist is per effective entity type; unknown types
/// pass through untouched (schema-drift resilience).
fn filter_properties_for_prompt(
    entity_type: &str,
    properties: &serde_json::Value,
) -> serde_json::Value {
    let keep: &[&str] = match entity_type {
        ENTITY_COMPLAINT_ALLEGATION => &["paragraph_number", "summary"],
        ENTITY_LEGAL_COUNT => &["count_number", "legal_basis", "description"],
        t if PARTY_SUBTYPES.contains(&t) => &["full_name", "role", "entity_kind"],
        _ => return properties.clone(),
    };
    let src = match properties.as_object() {
        Some(o) => o,
        None => return properties.clone(),
    };
    let mut out = serde_json::Map::new();
    for k in keep {
        if let Some(v) = src.get(*k) {
            out.insert((*k).to_string(), v.clone());
        }
    }
    serde_json::Value::Object(out)
}

// ── Query ────────────────────────────────────────────────────────

/// Load entities from OTHER PUBLISHED documents for cross-doc pass-2
/// context.
///
/// Returns [`CrossDocEntity`] values drawn from every COMPLETED pass-1
/// run on any document whose `documents.status = 'PUBLISHED'` except
/// the current one, restricted to the approved-item set and to the
/// entity types useful for cross-document link creation (parties,
/// counts, complaint allegations). Empty `Vec` is a valid result —
/// the current doc may be the first published, or no cross-doc-worthy
/// types exist yet.
pub async fn load_cross_document_context(
    pool: &PgPool,
    current_document_id: &str,
) -> Result<Vec<CrossDocEntity>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, CrossDocRow>(
        "SELECT i.id AS item_id, \
                i.item_data, \
                i.document_id AS source_document_id, \
                docs.document_type AS source_document_type, \
                COALESCE(i.resolved_entity_type, i.entity_type) AS effective_entity_type \
         FROM extraction_items i \
         JOIN extraction_runs runs ON runs.id = i.run_id \
         JOIN documents docs ON docs.id = i.document_id \
         WHERE i.document_id <> $1 \
           AND docs.status = $3 \
           AND runs.pass_number = 1 \
           AND runs.status = $4 \
           AND i.review_status = $5 \
           AND COALESCE(i.resolved_entity_type, i.entity_type) = ANY($2) \
         ORDER BY i.document_id, i.id",
    )
    .bind(current_document_id)
    .bind(CROSS_DOC_ENTITY_TYPES)
    .bind(STATUS_PUBLISHED)
    .bind(RUN_STATUS_COMPLETED)
    .bind(REVIEW_STATUS_APPROVED)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(CrossDocEntity::from_row).collect())
}

// ── Authored (Tier-1) entity context ─────────────────────────────

/// Sentinel `source_document_id` for authored (Tier-1) entities. They
/// have no originating document, so this fixed marker tells the Pass-2
/// LLM the entity is canonical case knowledge rather than something
/// extracted from a peer document.
///
/// ## CONST: prompt protocol token, not deployment config
///
/// This is a value the Pass-2 LLM reads as the provenance marker for
/// canonical knowledge — part of the prompt contract, not an
/// operator-tunable setting. Changing it would require simultaneously
/// updating any prompt guidance that refers to canonical-library
/// provenance, so an env/YAML toggle would be a false escape hatch (same
/// reasoning as [`CROSS_DOC_ENTITY_TYPES`]). Frozen in code so a change
/// travels with the prompt edits it implies.
const AUTHORED_SOURCE_DOCUMENT_ID: &str = "canonical";

/// Sentinel `source_document_type` for authored entities. Pairs with
/// [`AUTHORED_SOURCE_DOCUMENT_ID`] in the prompt's `source_document_type`.
///
/// ## CONST: prompt protocol token, not deployment config
///
/// Same rationale as [`AUTHORED_SOURCE_DOCUMENT_ID`] — a prompt-contract
/// value, not deployment configuration.
const AUTHORED_SOURCE_DOCUMENT_TYPE: &str = "canonical_element_library";

/// Convert an [`AuthoredEntityRecord`] (Tier 1) into a [`CrossDocEntity`]
/// so authored Elements / LegalCounts render into the same
/// `{{entities_json}}` prompt slot as extracted cross-doc entities.
///
/// ## Domain note: prompt id and the missing source document
///
/// Authored entities carry no source document or extraction run, so
/// `source_document_id` / `source_document_type` are the sentinels above.
/// The prompt id is taken from `item_data["id"]` when present; otherwise
/// it falls back to the row's stable `entity_id` (the "inject entity_id as
/// the id" rule). Either way [`CrossDocEntity::to_prompt_value`] emits
/// `"ctx:<id>"` so Pass 2 can reference the entity.
///
/// `item_id` is the **negated** authored-row id. The caller does NOT add
/// authored entities to the Pass-2 id_map (Option B — see
/// [`load_authored_entities_for_context`]), so this value never resolves a
/// relationship endpoint; negating it is defence-in-depth so it could
/// never alias a real positive `extraction_items.id`.
fn authored_record_to_cross_doc(record: &AuthoredEntityRecord) -> CrossDocEntity {
    let original_id = record
        .item_data
        .get("id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| record.entity_id.clone());
    let label = record
        .item_data
        .get("label")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let properties = record
        .item_data
        .get("properties")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
    let prefixed_id = format!("{CROSS_DOC_ID_PREFIX}{original_id}");
    CrossDocEntity {
        item_id: -record.id,
        original_id,
        prefixed_id,
        source_document_id: AUTHORED_SOURCE_DOCUMENT_ID.to_string(),
        source_document_type: AUTHORED_SOURCE_DOCUMENT_TYPE.to_string(),
        entity_type: record.entity_type.clone(),
        label,
        properties,
    }
}

/// Load authored entities (Tier 1) for the Pass-2 cross-document context.
///
/// Reads `authored_entities` for `case_slug`, restricted to the same
/// `entity_type_filter` whitelist the extracted-entity loader applies (the
/// caller passes [`CROSS_DOC_ENTITY_TYPES`]), and converts each row via
/// [`authored_record_to_cross_doc`]. An empty `Vec` is a valid result —
/// the case may have no authored entities loaded yet.
///
/// ## Why this is separate from [`load_cross_document_context`] (Option B)
///
/// Extracted cross-doc entities carry a real `extraction_items.id`, so the
/// caller can safely add them to the Pass-2 id_map and persist edges to
/// them. Authored entities have no such row; the caller adds them to the
/// PROMPT only (not the id_map), so a Pass-2 relationship targeting an
/// authored entity is skipped-and-logged at storage rather than violating
/// the `extraction_relationships → extraction_items` foreign key.
/// Persisting authored-targeting edges belongs to the ingest/mapping step
/// (which writes the `authored_relationships` table), not here.
pub async fn load_authored_entities_for_context(
    pool: &PgPool,
    case_slug: &str,
    entity_type_filter: &[&str],
) -> Result<Vec<CrossDocEntity>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, AuthoredEntityRecord>(
        "SELECT id, case_slug, entity_type, entity_id, item_data, \
                provenance, created_by, created_at, updated_at \
         FROM authored_entities \
         WHERE case_slug = $1 AND entity_type = ANY($2) \
         ORDER BY id",
    )
    .bind(case_slug)
    .bind(entity_type_filter)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(authored_record_to_cross_doc).collect())
}

#[cfg(test)]
mod tests {
    //! Pure-function tests for cross-doc projection + whitelist
    //! membership. The DB-touching `load_cross_document_context` is
    //! exercised by integration tests; everything else here is
    //! pure-data and can be asserted directly.
    use super::*;

    fn cross_doc(
        item_id: i32,
        original_id: &str,
        entity_type: &str,
        properties: serde_json::Value,
    ) -> CrossDocEntity {
        CrossDocEntity {
            item_id,
            original_id: original_id.to_string(),
            prefixed_id: format!("{CROSS_DOC_ID_PREFIX}{original_id}"),
            source_document_id: "doc-complaint-1".into(),
            source_document_type: "complaint".into(),
            entity_type: entity_type.to_string(),
            label: Some(format!("{entity_type} label")),
            properties,
        }
    }

    #[test]
    fn cross_doc_prompt_value_emits_prefixed_id_and_source() {
        let e = cross_doc(
            42,
            "allegation-014",
            "ComplaintAllegation",
            serde_json::json!({
                "paragraph_number": "14",
                "summary": "Defendant failed to account for funds",
                "verbatim_quote": "very long verbatim complaint text that should not be sent to pass 2",
            }),
        );
        let v = e.to_prompt_value();
        let obj = v.as_object().expect("prompt value is an object");
        assert_eq!(obj["id"], "ctx:allegation-014");
        assert_eq!(obj["entity_type"], "ComplaintAllegation");
        assert_eq!(obj["source_document"], "doc-complaint-1");
        assert_eq!(obj["source_document_type"], "complaint");
        // Property allowlist: paragraph_number + summary survive,
        // verbatim_quote is dropped to keep prompt size bounded.
        let props = obj["properties"]
            .as_object()
            .expect("properties is an object");
        assert_eq!(props["paragraph_number"], "14");
        assert!(props.contains_key("summary"));
        assert!(
            !props.contains_key("verbatim_quote"),
            "verbatim_quote must be filtered out of the prompt: {props:?}"
        );
    }

    #[test]
    fn cross_doc_prompt_value_legal_count_keeps_count_number() {
        let e = cross_doc(
            7,
            "count-001",
            "LegalCount",
            serde_json::json!({
                "count_number": 1,
                "legal_basis": "Breach of Fiduciary Duty",
                "description": "Defendant CFS breached its fiduciary duties",
                "paragraph_range": "86-100",
            }),
        );
        let v = e.to_prompt_value();
        let props = v["properties"].as_object().unwrap();
        assert_eq!(props["count_number"], 1);
        assert_eq!(props["legal_basis"], "Breach of Fiduciary Duty");
        // `paragraph_range` isn't in the allowlist; filtering drops it.
        assert!(!props.contains_key("paragraph_range"));
    }

    #[test]
    fn cross_doc_prompt_value_party_types_share_allowlist() {
        let party_props = serde_json::json!({
            "full_name": "Marie Awad",
            "role": "plaintiff",
            "entity_kind": "person",
            "address": "unused extra property",
        });
        for effective_type in &["Party", "Person", "Organization"] {
            let e = cross_doc(1, "party-001", effective_type, party_props.clone());
            let v = e.to_prompt_value();
            let props = v["properties"].as_object().unwrap();
            assert_eq!(
                props["full_name"], "Marie Awad",
                "type {effective_type} must surface full_name"
            );
            assert!(
                !props.contains_key("address"),
                "type {effective_type} must filter unknown props: {props:?}"
            );
        }
    }

    #[test]
    fn cross_doc_prompt_value_unknown_type_keeps_all_properties() {
        // Schema-drift insurance: if a future document type surfaces an
        // entity the allowlist doesn't know about, pass the properties
        // through untouched instead of silently dropping everything.
        let props = serde_json::json!({
            "arbitrary_field": "value",
            "another_one": 42,
        });
        let e = cross_doc(99, "unknown-001", "UnknownType", props.clone());
        let v = e.to_prompt_value();
        assert_eq!(v["properties"], props);
    }

    /// Locks in the cross-document entity-type whitelist.
    ///
    /// This test is the regression guard against the v5.1 silent-drop
    /// bug: the v5.1 complaint schema emits `"Allegation"` entities,
    /// but the original whitelist only carried `"ComplaintAllegation"`,
    /// so all v5.1 allegations were filtered out of the pass-2
    /// cross-document context and the LLM saw no allegations to
    /// CORROBORATES against. The set is now asserted member-by-member
    /// so a future "cleanup" that re-drops `"Allegation"` (or removes
    /// `Evidence` / `Element` / `Harm`) fails this test immediately
    /// rather than producing an empty-context regression that is
    /// invisible until a downstream pass-2 run gets manually inspected.
    #[test]
    fn cross_doc_entity_types_includes_v5_1_labels() {
        // Use `contains` rather than full-array equality so cosmetic
        // reordering does not break the test; the asserted properties
        // are membership and length, not the literal slice layout.
        use crate::models::document_status::{
            ENTITY_ALLEGATION, ENTITY_COMPLAINT_ALLEGATION, ENTITY_ELEMENT, ENTITY_EVIDENCE,
            ENTITY_HARM, ENTITY_LEGAL_COUNT, ENTITY_ORGANIZATION, ENTITY_PARTY, ENTITY_PERSON,
        };
        let expected: &[&str] = &[
            ENTITY_PARTY,
            ENTITY_PERSON,
            ENTITY_ORGANIZATION,
            ENTITY_LEGAL_COUNT,
            ENTITY_COMPLAINT_ALLEGATION,
            ENTITY_ALLEGATION,
            ENTITY_EVIDENCE,
            ENTITY_ELEMENT,
            ENTITY_HARM,
        ];
        for et in expected {
            assert!(
                CROSS_DOC_ENTITY_TYPES.contains(et),
                "CROSS_DOC_ENTITY_TYPES missing required v5.1 type {et:?} — \
                 v5.1 cross-doc context would silently drop these entities"
            );
        }
        assert_eq!(
            CROSS_DOC_ENTITY_TYPES.len(),
            expected.len(),
            "CROSS_DOC_ENTITY_TYPES length drift: adding a type without \
             updating this test, or pulling a type without justification, \
             both break cross-doc context. Update the test and the doc \
             comment together."
        );
    }

    // ── Authored (Tier-1) → CrossDocEntity conversion ──────────────

    fn authored_record(
        id: i32,
        entity_type: &str,
        entity_id: &str,
        item_data: serde_json::Value,
    ) -> AuthoredEntityRecord {
        AuthoredEntityRecord {
            id,
            case_slug: "awad_v_catholic_family_service".into(),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            item_data,
            provenance: "canonical".into(),
            created_by: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Instruction test #4: when `item_data` has no `"id"`, the stable
    /// `entity_id` becomes the prompt id so Pass 2 can still reference it.
    #[test]
    fn authored_to_cross_doc_falls_back_to_entity_id_when_item_data_has_no_id() {
        let rec = authored_record(
            7,
            "Element",
            "element-1-1",
            serde_json::json!({ "label": "Duty", "properties": { "order_in_count": 1 } }),
        );
        let e = authored_record_to_cross_doc(&rec);
        assert_eq!(e.original_id, "element-1-1");
        assert_eq!(e.prefixed_id, "ctx:element-1-1");
        assert_eq!(e.item_id, -7, "authored item_id is the negated row id");
        assert_eq!(e.to_prompt_value()["id"], "ctx:element-1-1");
    }

    /// When `item_data` already carries an `"id"` it wins; the canonical
    /// source sentinels are set, and `to_prompt_value` still applies the
    /// per-type property allowlist (here: LegalCount drops non-allowlisted
    /// keys).
    #[test]
    fn authored_to_cross_doc_uses_item_data_id_and_sets_canonical_sentinels() {
        let rec = authored_record(
            3,
            "LegalCount",
            "count-1",
            serde_json::json!({
                "id": "count-1",
                "label": "Count I",
                "properties": { "count_number": 1, "legal_basis": "MCL", "extra": "drop me" }
            }),
        );
        let e = authored_record_to_cross_doc(&rec);
        assert_eq!(e.prefixed_id, "ctx:count-1");
        assert_eq!(e.source_document_id, "canonical");
        assert_eq!(e.source_document_type, "canonical_element_library");
        assert_eq!(e.entity_type, "LegalCount");
        let v = e.to_prompt_value();
        let props = v["properties"].as_object().expect("properties object");
        assert_eq!(props["count_number"], 1);
        assert!(
            !props.contains_key("extra"),
            "non-allowlisted property must be filtered out"
        );
        assert_eq!(v["source_document_type"], "canonical_element_library");
    }
}
