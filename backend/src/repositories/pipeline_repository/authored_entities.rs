//! Authored-entity repository (three-tier architecture, Option A).
//!
//! Owns CRUD for the two tables created by migration
//! `20260526141630_create_authored_entity_tables.sql`:
//!
//! - `authored_entities` (Tier 1) — human-authored entities (Elements,
//!   LegalCounts, future authored types). NOT extracted from documents.
//! - `authored_relationships` (Tier 3) — the mapping layer connecting
//!   authored entities to each other and to extracted entities.
//!
//! ## Domain note: why no foreign keys
//!
//! Endpoints are referenced by `entity_id` *strings*, never integer FKs.
//! An `authored_relationships` endpoint may point at an
//! `authored_entities.entity_id` (Tier 1) OR at the `neo4j_node_id` of an
//! `extraction_items` row (Tier 2) — the two tiers live in different
//! tables, so a single integer FK cannot span them. The string id is the
//! same value used as the Neo4j node `id` property, so the graph MERGE
//! (which matches purely on `{id}`) connects the tiers regardless of which
//! table an endpoint originated in. This also lets the mapping layer be
//! rebuilt without reprocessing documents.
//!
//! Functions stay stateless. The four mutating functions (`upsert_*` /
//! `delete_*`) take `impl sqlx::PgExecutor<'_>` rather than `&PgPool` so a
//! caller can run them inside a single transaction (the canonical loader's
//! delete-then-insert) — a `&PgPool` still satisfies the bound, so simple
//! call sites pass `&pool` unchanged. The read helpers (`get_*` / `list_*`)
//! keep `&PgPool`.

use sqlx::PgPool;

use super::PipelineRepoError;
use crate::models::document_status::ENTITY_ELEMENT;

// ── Record types ─────────────────────────────────────────────────

/// A row from `authored_entities`.
///
/// `entity_id` is the stable, globally-unique string used as the Neo4j
/// node `id`. `item_data` is the full entity payload whose shape depends
/// on `entity_type` (see the migration's column comments for the Element
/// and LegalCount shapes).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuthoredEntityRecord {
    pub id: i32,
    pub case_slug: String,
    pub entity_type: String,
    pub entity_id: String,
    pub item_data: serde_json::Value,
    pub provenance: String,
    pub created_by: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A row from `authored_relationships`.
///
/// `from_entity_id` / `to_entity_id` are `entity_id` strings (Tier 1) or
/// `neo4j_node_id` strings (Tier 2) — see the module-level domain note.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuthoredRelationshipRecord {
    pub id: i32,
    pub case_slug: String,
    pub from_entity_id: String,
    pub to_entity_id: String,
    pub relationship_type: String,
    pub properties: Option<serde_json::Value>,
    pub provenance: String,
    pub created_by: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Shared SELECT projection for every `AuthoredEntityRecord` read, so the
/// `FromRow` column set never drifts between query sites.
const ENTITY_COLUMNS: &str = "id, case_slug, entity_type, entity_id, item_data, \
     provenance, created_by, created_at, updated_at";

/// Shared SELECT projection for every `AuthoredRelationshipRecord` read.
const RELATIONSHIP_COLUMNS: &str = "id, case_slug, from_entity_id, to_entity_id, \
     relationship_type, properties, provenance, created_by, created_at, updated_at";

// ── authored_entities CRUD ───────────────────────────────────────

/// Insert or update an authored entity, keyed on its unique `entity_id`.
/// Returns the row's id (newly-generated on insert, existing on update).
///
/// ## Rust Learning: idempotent upsert via `ON CONFLICT … DO UPDATE`
///
/// The canonical loader re-runs over the same Elements/Counts on every
/// reload, so a plain INSERT would violate `authored_entities_entity_id_unique`
/// the second time. `ON CONFLICT ON CONSTRAINT … DO UPDATE` makes the
/// write idempotent: a fresh `entity_id` inserts, a repeat updates the
/// mutable columns in place. `EXCLUDED` refers to the row that *would*
/// have been inserted, so `item_data = EXCLUDED.item_data` adopts the new
/// payload. `created_by` / `created_at` are deliberately left out of the
/// SET clause — they record the original author, not the last writer —
/// while `updated_at = NOW()` advances on every touch.
pub async fn upsert_authored_entity(
    executor: impl sqlx::PgExecutor<'_>,
    case_slug: &str,
    entity_type: &str,
    entity_id: &str,
    item_data: &serde_json::Value,
    provenance: &str,
    created_by: Option<&str>,
) -> Result<i32, PipelineRepoError> {
    let id = sqlx::query_scalar::<_, i32>(
        r#"INSERT INTO authored_entities
               (case_slug, entity_type, entity_id, item_data, provenance, created_by)
           VALUES ($1, $2, $3, $4, $5, $6)
           ON CONFLICT ON CONSTRAINT authored_entities_entity_id_unique DO UPDATE SET
               case_slug   = EXCLUDED.case_slug,
               entity_type = EXCLUDED.entity_type,
               item_data   = EXCLUDED.item_data,
               provenance  = EXCLUDED.provenance,
               updated_at  = NOW()
           RETURNING id"#,
    )
    .bind(case_slug)
    .bind(entity_type)
    .bind(entity_id)
    .bind(item_data)
    .bind(provenance)
    .bind(created_by)
    .fetch_one(executor)
    .await?;
    Ok(id)
}

/// List authored entities for a case, optionally filtered by `entity_type`.
///
/// ## Rust Learning: a single bound param for an optional filter
///
/// `($2::text IS NULL OR entity_type = $2::text)` lets one query serve
/// both "all types" and "one type" without building SQL strings by hand.
/// Binding `Option<&str>` sends `None` as SQL `NULL`, which makes the
/// left disjunct true and disables the filter; `Some(t)` makes it false
/// and applies `entity_type = t`. The `::text` cast tells Postgres the
/// parameter's type when it appears only inside `IS NULL`.
pub async fn list_authored_entities(
    pool: &PgPool,
    case_slug: &str,
    entity_type: Option<&str>,
) -> Result<Vec<AuthoredEntityRecord>, PipelineRepoError> {
    let sql = format!(
        "SELECT {ENTITY_COLUMNS} FROM authored_entities \
         WHERE case_slug = $1 AND ($2::text IS NULL OR entity_type = $2::text) \
         ORDER BY id"
    );
    let rows = sqlx::query_as::<_, AuthoredEntityRecord>(&sql)
        .bind(case_slug)
        .bind(entity_type)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// Get a single authored entity by its `entity_id`. `None` if absent.
pub async fn get_authored_entity(
    pool: &PgPool,
    entity_id: &str,
) -> Result<Option<AuthoredEntityRecord>, PipelineRepoError> {
    let sql = format!("SELECT {ENTITY_COLUMNS} FROM authored_entities WHERE entity_id = $1");
    let row = sqlx::query_as::<_, AuthoredEntityRecord>(&sql)
        .bind(entity_id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

/// Update the `review_notes` column for the Element row whose `entity_id`
/// matches. Passing `None` clears the notes (sets the column to SQL NULL);
/// passing `Some("")` writes an empty string — these are intentionally
/// distinguishable states (Rule 1: distinct observables).
///
/// Returns [`PipelineRepoError::NotFound`] when zero rows were updated, so
/// the API handler can map that to HTTP 404. A successful update bumps
/// `updated_at = NOW()` on the row.
///
/// ## Rust Learning: `impl PgExecutor<'_>` instead of `&PgPool`
///
/// `PgExecutor` is the sqlx trait for "anything you can run a query on" —
/// a pool, a transaction, a single connection. Taking the trait at the
/// boundary means a caller inside a transaction can pass `&mut *tx`
/// without forcing every helper to know about transactions. A bare
/// `&PgPool` still satisfies the bound, so the simple call site (the API
/// handler) passes `&pool` unchanged. Same idiom as
/// [`upsert_authored_entity`].
pub async fn update_element_review_notes(
    executor: impl sqlx::PgExecutor<'_>,
    entity_id: &str,
    review_notes: Option<&str>,
) -> Result<(), PipelineRepoError> {
    let result = sqlx::query(
        "UPDATE authored_entities \
         SET review_notes = $1, updated_at = NOW() \
         WHERE entity_id = $2 AND entity_type = $3",
    )
    // Defensive `AND entity_type = $3` clause keeps a stray entity_id
    // collision with a different entity type (LegalCount, future authored
    // types) from silently mutating the wrong row. The canonical
    // `ENTITY_ELEMENT` constant is the single source of truth for the
    // label string — both the Cypher label parameter on the read side and
    // this SQL discriminator on the write side bind to it.
    .bind(review_notes)
    .bind(entity_id)
    .bind(ENTITY_ELEMENT)
    .execute(executor)
    .await?;

    if result.rows_affected() == 0 {
        // Distinct observable: the SQL succeeded, the row simply did not
        // exist. Caller maps to 404. Payload identifies which id was
        // missing so the operator log is actionable (Rule 1).
        return Err(PipelineRepoError::NotFound(format!(
            "authored_entities Element entity_id={entity_id}"
        )));
    }
    Ok(())
}

/// Delete every authored entity for a case. Returns the number of rows
/// removed. Used when reloading canonical data so a stale entity that was
/// dropped from the source no longer lingers.
///
/// ## Rust Learning: `rows_affected()` reports work done, not failure
///
/// A DELETE that matches zero rows is a success, not an error — the
/// caller gets `Ok(0)`. Returning the count lets the loader log exactly
/// how many rows it cleared, keeping "deleted 12" distinguishable from
/// "deleted 0" in the logs (Rule 1: distinct states, distinct observables).
pub async fn delete_authored_entities_for_case(
    executor: impl sqlx::PgExecutor<'_>,
    case_slug: &str,
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query("DELETE FROM authored_entities WHERE case_slug = $1")
        .bind(case_slug)
        .execute(executor)
        .await?;
    Ok(result.rows_affected())
}

// ── authored_relationships CRUD ──────────────────────────────────

/// Insert or update an authored relationship, keyed on the unique edge
/// `(from_entity_id, to_entity_id, relationship_type)`. Returns the row id.
///
/// On conflict the mutable columns (`properties`, `provenance`,
/// `case_slug`) are overwritten and `updated_at` advances; the edge
/// identity columns and `created_by` / `created_at` are preserved. See
/// [`upsert_authored_entity`] for the `ON CONFLICT` / `EXCLUDED` idiom.
///
/// 8 args is one over clippy's default threshold. Grouping them into a
/// dedicated struct would add a layer of indirection at every call site
/// (the canonical loader, the Element-mapping step, and a future
/// authoring UI handler) for no readability gain — the function is a flat
/// insert of eight columns. The lint is silenced locally (not project-
/// wide) so other functions still get the warning. Matches the precedent
/// in [`super::document_records::insert_document`].
#[allow(clippy::too_many_arguments)]
pub async fn upsert_authored_relationship(
    executor: impl sqlx::PgExecutor<'_>,
    case_slug: &str,
    from_entity_id: &str,
    to_entity_id: &str,
    relationship_type: &str,
    properties: Option<&serde_json::Value>,
    provenance: &str,
    created_by: Option<&str>,
) -> Result<i32, PipelineRepoError> {
    let id = sqlx::query_scalar::<_, i32>(
        r#"INSERT INTO authored_relationships
               (case_slug, from_entity_id, to_entity_id, relationship_type,
                properties, provenance, created_by)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           ON CONFLICT ON CONSTRAINT authored_relationships_unique_edge DO UPDATE SET
               case_slug  = EXCLUDED.case_slug,
               properties = EXCLUDED.properties,
               provenance = EXCLUDED.provenance,
               updated_at = NOW()
           RETURNING id"#,
    )
    .bind(case_slug)
    .bind(from_entity_id)
    .bind(to_entity_id)
    .bind(relationship_type)
    .bind(properties)
    .bind(provenance)
    .bind(created_by)
    .fetch_one(executor)
    .await?;
    Ok(id)
}

/// List authored relationships for a case, optionally filtered by
/// `relationship_type`. See [`list_authored_entities`] for the
/// optional-filter idiom.
pub async fn list_authored_relationships(
    pool: &PgPool,
    case_slug: &str,
    relationship_type: Option<&str>,
) -> Result<Vec<AuthoredRelationshipRecord>, PipelineRepoError> {
    let sql = format!(
        "SELECT {RELATIONSHIP_COLUMNS} FROM authored_relationships \
         WHERE case_slug = $1 AND ($2::text IS NULL OR relationship_type = $2::text) \
         ORDER BY id"
    );
    let rows = sqlx::query_as::<_, AuthoredRelationshipRecord>(&sql)
        .bind(case_slug)
        .bind(relationship_type)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// Delete authored relationships for a case that have a specific
/// `relationship_type`, leaving other types untouched. Returns the number
/// of rows removed. Used when rebuilding one mapping layer (e.g. re-running
/// Element mapping wipes only `PROVES_ELEMENT`, not `HAS_ELEMENT`).
pub async fn delete_authored_relationships_by_type(
    executor: impl sqlx::PgExecutor<'_>,
    case_slug: &str,
    relationship_type: &str,
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(
        "DELETE FROM authored_relationships WHERE case_slug = $1 AND relationship_type = $2",
    )
    .bind(case_slug)
    .bind(relationship_type)
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}

// ── Extracted cross-tier edges (Pass-2 PROVES_ELEMENT etc.) ───────
//
// These rows differ from the canonical loader's: they carry a `document_id`
// (the document whose Pass-2 extraction asserted the edge) and a distinct
// `provenance`, so a re-process can reconcile just its own edges without
// touching case-global canonical rows. The endpoints are still TEXT stable
// ids (the extraction node's Neo4j id on the from side, the canonical
// entity_id on the to side).

/// Provenance marker for Pass-2-extracted cross-tier edges, distinct from the
/// canonical loader's `'canonical'`. Keeps the loader's type-scoped
/// reconciliation and these document-scoped edges from clobbering each other.
const PROVENANCE_EXTRACTED: &str = "extracted";

/// `created_by` sentinel recording that Pass-2 wrote these rows.
const CREATED_BY_PASS2: &str = "pass2";

/// Endpoints + type + properties of one extracted cross-tier edge, as ingest
/// needs them to write the matching Neo4j relationship.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ExtractedAuthoredEdge {
    pub from_entity_id: String,
    pub to_entity_id: String,
    pub relationship_type: String,
    pub properties: Option<serde_json::Value>,
}

/// Delete the extracted (Pass-2) authored relationships a document previously
/// asserted, so a re-process can re-insert a fresh set without leaving stale
/// edges behind. Scoped by `document_id` AND `provenance = 'extracted'` so
/// canonical rows (provenance `'canonical'`, `document_id` NULL) are never
/// touched. Returns the number of rows removed (Rule 1: distinct observable).
pub async fn delete_extracted_authored_relationships_for_document(
    executor: impl sqlx::PgExecutor<'_>,
    document_id: &str,
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(
        "DELETE FROM authored_relationships WHERE document_id = $1 AND provenance = $2",
    )
    .bind(document_id)
    .bind(PROVENANCE_EXTRACTED)
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}

/// Insert (or update) an extracted cross-tier authored relationship, stamping
/// `provenance = 'extracted'`, `created_by = 'pass2'`, and the owning
/// `document_id`. Distinct from [`upsert_authored_relationship`] — that one is
/// the canonical loader's and leaves `document_id` NULL. Returns the row id.
///
/// On conflict (the `(from, to, type)` unique edge) the mutable columns
/// (`properties`, `document_id`, `updated_at`) are refreshed; the
/// `provenance` / `created_by` identity columns are preserved.
#[allow(clippy::too_many_arguments)]
pub async fn insert_extracted_authored_relationship(
    executor: impl sqlx::PgExecutor<'_>,
    case_slug: &str,
    from_entity_id: &str,
    to_entity_id: &str,
    relationship_type: &str,
    properties: Option<&serde_json::Value>,
    document_id: &str,
) -> Result<i32, PipelineRepoError> {
    let id = sqlx::query_scalar::<_, i32>(
        r#"INSERT INTO authored_relationships
               (case_slug, from_entity_id, to_entity_id, relationship_type,
                properties, provenance, created_by, document_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           ON CONFLICT ON CONSTRAINT authored_relationships_unique_edge DO UPDATE SET
               properties  = EXCLUDED.properties,
               document_id = EXCLUDED.document_id,
               updated_at  = NOW()
           RETURNING id"#,
    )
    .bind(case_slug)
    .bind(from_entity_id)
    .bind(to_entity_id)
    .bind(relationship_type)
    .bind(properties)
    .bind(PROVENANCE_EXTRACTED)
    .bind(CREATED_BY_PASS2)
    .bind(document_id)
    .fetch_one(executor)
    .await?;
    Ok(id)
}

/// List the extracted (Pass-2) authored relationships a document asserted, for
/// ingest's Neo4j write. Scoped to `provenance = 'extracted'`.
pub async fn list_extracted_authored_relationships_for_document(
    executor: impl sqlx::PgExecutor<'_>,
    document_id: &str,
) -> Result<Vec<ExtractedAuthoredEdge>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ExtractedAuthoredEdge>(
        "SELECT from_entity_id, to_entity_id, relationship_type, properties \
         FROM authored_relationships \
         WHERE document_id = $1 AND provenance = $2 \
         ORDER BY id",
    )
    .bind(document_id)
    .bind(PROVENANCE_EXTRACTED)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}
