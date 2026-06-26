//! Scenario responses store (task 1.6).
//!
//! Owns CRUD for the three FK-chained tables created by migration
//! `20260626135022_create_scenario_responses_tables.sql` (in `colossus_legal_v2`,
//! the pipeline database):
//!
//! - `scenario_responses` — a prepared answer to a scenario (label, text,
//!   status, origin), keyed to a `scenarios` row.
//! - `response_items` — ordered itemization under a response (`item_index`).
//! - `response_item_fact_refs` — m:n link from an item to the GRAPH node ids of
//!   the evidence it rests on.
//!
//! Same tag-not-copy discipline as `scenario_fact_refs`: facts are referenced by
//! graph node id and read live from the graph at compose time — no case content
//! (quotes/citations/fact text) is ever stored here. Sibling of `scenario_store`
//! (split out to keep each module under the 300-line limit); the two together are
//! the scenario authored-state surface. Mutators take `impl PgExecutor<'_>`
//! (transaction-composable); readers take `&PgPool`. Errors flow through the
//! shared [`PipelineRepoError`].
//!
//! ## Rust Learning: a mixed identity model in one module
//!
//! Two different identity patterns live side by side here, and a reader can tell
//! them apart by **return type**:
//!
//! - `scenario_responses` and `response_items` are server-minted *entities*:
//!   their `id` is a DB-generated `gen_random_uuid()`, so [`insert_scenario_response`]
//!   and [`insert_response_item`] each return the freshly minted `Uuid` (the
//!   caller had no id until the insert produced one).
//! - `response_item_fact_refs` is a pure *link* with composite-key identity
//!   `(response_item_id, graph_node_id)` — both halves are ids the caller already
//!   holds — so [`upsert_response_item_fact_ref`] returns `()`. There is nothing
//!   for the database to mint.
//!
//! A function that hands back a `Uuid` created an entity; one returning `()`
//! merely linked two ids. (Same distinction as `insert_scenario` vs
//! `upsert_fact_ref` in [`super::scenario_store`].)

use sqlx::PgPool;

use super::PipelineRepoError;

// ── Record types ─────────────────────────────────────────────────

/// A row from `scenario_responses`. Server-minted `id`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ScenarioResponseRecord {
    pub id: uuid::Uuid,
    pub scenario_id: uuid::Uuid,
    pub label: Option<String>,
    pub text: String,
    pub status: String,
    pub origin: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A row from `response_items`. Server-minted `id`; `item_index` is the 0-based
/// order within the response.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ResponseItemRecord {
    pub id: uuid::Uuid,
    pub response_id: uuid::Uuid,
    pub item_index: i32,
    pub text: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A row from `response_item_fact_refs` — one evidence reference for one item.
/// No surrogate id: identity IS the composite key `(response_item_id,
/// graph_node_id)` (same modeling as [`super::scenario_store::ScenarioFactRefRecord`]).
/// `graph_node_id` is a pointer into Neo4j; no case content is stored here.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ResponseItemFactRefRecord {
    pub response_item_id: uuid::Uuid,
    pub graph_node_id: String,
    pub note: Option<String>,
    pub tagged_at: chrono::DateTime<chrono::Utc>,
}

// CONST: column projection locked to the `scenario_responses` schema — a
// structural schema-coupling invariant, NOT a deployment-time config value.
// Changing it requires a migration plus a matching `ScenarioResponseRecord`
// field update (Standing Rule 2 does not apply).
const SCENARIO_RESPONSE_COLUMNS: &str =
    "id, scenario_id, label, text, status, origin, created_at, updated_at";

// CONST: column projection locked to the `response_items` schema (see above).
const RESPONSE_ITEM_COLUMNS: &str = "id, response_id, item_index, text, created_at";

// CONST: column projection locked to the `response_item_fact_refs` schema (see above).
const RESPONSE_ITEM_FACT_REF_COLUMNS: &str = "response_item_id, graph_node_id, note, tagged_at";

// ── scenario_responses ───────────────────────────────────────────

/// Insert a response under a scenario; the DB mints `id`. Returns the new id.
///
/// `status` and `origin` are passed explicitly for a single clear code path; the
/// columns' `'draft'` / `'human'` defaults remain backstops for non-Rust inserts.
/// Both are validated by the table's CHECK constraints.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the insert fails — including a foreign-key
/// violation if `scenario_id` names no scenario, or a CHECK violation on
/// `status` / `origin`.
pub async fn insert_scenario_response(
    executor: impl sqlx::PgExecutor<'_>,
    scenario_id: uuid::Uuid,
    label: Option<&str>,
    text: &str,
    status: &str,
    origin: &str,
) -> Result<uuid::Uuid, PipelineRepoError> {
    let id = sqlx::query_scalar::<_, uuid::Uuid>(
        r#"INSERT INTO scenario_responses (scenario_id, label, text, status, origin)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id"#,
    )
    .bind(scenario_id)
    .bind(label)
    .bind(text)
    .bind(status)
    .bind(origin)
    .fetch_one(executor)
    .await?;
    Ok(id)
}

/// List a scenario's responses, oldest first.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn list_responses_for_scenario(
    pool: &PgPool,
    scenario_id: uuid::Uuid,
) -> Result<Vec<ScenarioResponseRecord>, PipelineRepoError> {
    let sql = format!(
        "SELECT {SCENARIO_RESPONSE_COLUMNS} FROM scenario_responses \
         WHERE scenario_id = $1 ORDER BY created_at, id"
    );
    let rows = sqlx::query_as::<_, ScenarioResponseRecord>(&sql)
        .bind(scenario_id)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

// ── response_items ───────────────────────────────────────────────

/// Insert an ordered item under a response; the DB mints `id`. Returns the new id.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the insert fails — including a foreign-key
/// violation if `response_id` names no response.
pub async fn insert_response_item(
    executor: impl sqlx::PgExecutor<'_>,
    response_id: uuid::Uuid,
    item_index: i32,
    text: &str,
) -> Result<uuid::Uuid, PipelineRepoError> {
    let id = sqlx::query_scalar::<_, uuid::Uuid>(
        r#"INSERT INTO response_items (response_id, item_index, text)
           VALUES ($1, $2, $3)
           RETURNING id"#,
    )
    .bind(response_id)
    .bind(item_index)
    .bind(text)
    .fetch_one(executor)
    .await?;
    Ok(id)
}

/// List a response's items in `item_index` order.
///
/// The `(response_id, item_index)` index serves both the `WHERE` and the
/// `ORDER BY`, so no separate index is needed.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn list_items_for_response(
    pool: &PgPool,
    response_id: uuid::Uuid,
) -> Result<Vec<ResponseItemRecord>, PipelineRepoError> {
    let sql = format!(
        "SELECT {RESPONSE_ITEM_COLUMNS} FROM response_items \
         WHERE response_id = $1 ORDER BY item_index"
    );
    let rows = sqlx::query_as::<_, ResponseItemRecord>(&sql)
        .bind(response_id)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

// ── response_item_fact_refs ──────────────────────────────────────

/// Reference a graph fact on a response item, or re-tag it in place
/// (composite-key upsert on `(response_item_id, graph_node_id)`).
///
/// Returns `()`: the caller already holds the identity (both halves of the
/// composite key), so nothing is server-minted to hand back. Re-tagging the same
/// pair is a normal edit — `ON CONFLICT … DO UPDATE` overwrites `note` and
/// advances `tagged_at`. The same `graph_node_id` under a different
/// `response_item_id` is a different row (one fact backs many items).
///
/// `graph_node_id` is stored as-is: a pointer into Neo4j, never validated against
/// the graph here, and never accompanied by the fact's content (tag-not-copy).
///
/// # Errors
/// Returns [`PipelineRepoError`] if the write fails — notably a foreign-key
/// violation if `response_item_id` names no item.
pub async fn upsert_response_item_fact_ref(
    executor: impl sqlx::PgExecutor<'_>,
    response_item_id: uuid::Uuid,
    graph_node_id: &str,
    note: Option<&str>,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        r#"INSERT INTO response_item_fact_refs (response_item_id, graph_node_id, note)
           VALUES ($1, $2, $3)
           ON CONFLICT (response_item_id, graph_node_id) DO UPDATE SET
               note      = EXCLUDED.note,
               tagged_at = NOW()"#,
    )
    .bind(response_item_id)
    .bind(graph_node_id)
    .bind(note)
    .execute(executor)
    .await?;
    Ok(())
}

/// List the evidence references for one response item, oldest tag first.
///
/// The composite PK's leading column (`response_item_id`) serves this `WHERE`,
/// so no separate index is needed.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn list_fact_refs_for_item(
    pool: &PgPool,
    response_item_id: uuid::Uuid,
) -> Result<Vec<ResponseItemFactRefRecord>, PipelineRepoError> {
    let sql = format!(
        "SELECT {RESPONSE_ITEM_FACT_REF_COLUMNS} FROM response_item_fact_refs \
         WHERE response_item_id = $1 ORDER BY tagged_at, graph_node_id"
    );
    let rows = sqlx::query_as::<_, ResponseItemFactRefRecord>(&sql)
        .bind(response_item_id)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}
