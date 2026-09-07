//! The five sentences a witness's card carries (FACT_CARD_v2 §1).
//!
//! Two tables, one module: `scenario_fact_cards` holds the current card and
//! `scenario_fact_card_events` holds the append-only record of every field ever
//! written. They are written together, in one transaction, because the value of
//! the ledger is that a machine drafted a sentence and a human replaced it — and
//! an UPDATE destroys the draft it replaced.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Both tables live in the **pipeline** database (`colossus_legal_v2`), so every
//! call site passes `&state.pipeline_pool`, never `state.pg_pool`.
//!
//! ## Domain note: scenario-scoped, unlike a Matrix ruling
//!
//! Keyed `(scenario_id, graph_node_id)`. The same statement carries a different
//! sentence in two scenarios — under "Marie is obstructive" the certified letter
//! is proof she offered to cooperate; under "the $50,000" the same words are
//! background. Contrast `evidence_allegation_rulings`, which is case-wide because
//! whether an item belongs under an accusation is true everywhere or nowhere.
//!
//! ## Why the writer takes ONE field at a time
//!
//! §2's edit is "click a field → inline edit → PUT one field". A whole-row write
//! would re-stamp all five authorships from one edit, and the draft mark is
//! per-field precisely so a card with an edited Answer can still show a drafted
//! Watch out. The loader is the one exception and says so: it writes the row and
//! records a single `card` ledger entry.
//!
//! ## v2 §8: human-authored content, machine drafts included
//!
//! This module is the ONLY writer of both tables, and no scan, gather or merge
//! path calls it. Both tables are listed in `HUMAN_AUTHORED_TABLES`, and the two
//! scan-path invariant tests fail the build if a scan path so much as names one.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::fact_card::CardField;

use super::PipelineRepoError;

/// A row from `scenario_fact_cards`.
///
/// ## Rust learning: `sqlx::FromRow` and why the types are not negotiable
///
/// The derive maps columns to fields BY NAME, and each field's type must be one
/// sqlx can decode from that column's SQL type. `TEXT → Option<String>`,
/// `INTEGER → Option<i32>`, `JSONB → Option<serde_json::Value>` and
/// `TIMESTAMPTZ → DateTime<Utc>` are the four pairings here. Nothing is
/// `NUMERIC`: beta.364 died at boot decoding one into an `Option<f64>` with no
/// `rust_decimal` in the tree, and the way that stays fixed is by not
/// introducing the type.
///
/// `supports` is a raw `Value` rather than a typed list because this struct is
/// the row as the DATABASE holds it. It becomes typed at the read boundary in
/// [`CardRecord::supports_list`], where a malformed entry can be refused with the
/// ids that carry it — a `FromRow` that parsed it would fail with sqlx's decode
/// error instead, which names the column and not the card.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CardRecord {
    pub scenario_id: Uuid,
    pub graph_node_id: String,
    pub title: Option<String>,
    pub backs_position: Option<i32>,
    pub supports: Option<serde_json::Value>,
    pub watch_out: Option<String>,
    pub answer: Option<String>,
    pub title_authored_by: Option<String>,
    pub backs_position_authored_by: Option<String>,
    pub supports_authored_by: Option<String>,
    pub watch_out_authored_by: Option<String>,
    pub answer_authored_by: Option<String>,
    pub authored_by: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: DateTime<Utc>,
}

/// One accusation a card names, and which way it cuts.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardSupport {
    pub allegation_id: String,
    pub stance: crate::domain::fact_card::CardStance,
}

impl CardRecord {
    /// The `supports` column as a typed list, or a named error.
    ///
    /// ## Why this is a method and not a `TryFrom`
    ///
    /// The caller has the card's ids in hand and can say WHICH card carries the
    /// bad JSON. A conversion that consumed the record would have to rebuild that
    /// context, and a witness surface that says "one of these is unreadable"
    /// without saying which is not much better than saying nothing.
    ///
    /// An absent column is an empty list, not an error: a card that names no
    /// accusation is the ordinary state, and §2 renders it as an em dash.
    ///
    /// # Errors
    /// Returns [`PipelineRepoError::Database`] naming the card and the parse
    /// failure when the column holds something this build cannot read.
    pub fn supports_list(&self) -> Result<Vec<CardSupport>, PipelineRepoError> {
        let Some(raw) = &self.supports else {
            return Ok(Vec::new());
        };
        serde_json::from_value::<Vec<CardSupport>>(raw.clone()).map_err(|e| {
            PipelineRepoError::Database(format!(
                "scenario_fact_cards holds an unreadable supports list for \
                 (scenario {}, node {}): {e}",
                self.scenario_id, self.graph_node_id
            ))
        })
    }

    /// The author of one field, or `None` when nobody has written it.
    ///
    /// ## Why a match and not a column name built from the token
    ///
    /// `CardField::code()` IS the column name, and the writer does build SQL from
    /// it. Reading is different: the row is already decoded into named fields, so
    /// a lookup by string would be a second mapping that could drift from the
    /// struct. The exhaustive match cannot.
    pub fn field_author(&self, field: CardField) -> Option<&str> {
        match field {
            CardField::Title => self.title_authored_by.as_deref(),
            CardField::BacksPosition => self.backs_position_authored_by.as_deref(),
            CardField::Supports => self.supports_authored_by.as_deref(),
            CardField::WatchOut => self.watch_out_authored_by.as_deref(),
            CardField::Answer => self.answer_authored_by.as_deref(),
            // The ledger's word for "the loader wrote the whole row" is not a
            // column and has no author of its own — the row's `authored_by` is
            // that answer.
            CardField::Card => None,
        }
    }
}

/// Shared SELECT projection, so the `FromRow` column set cannot drift between
/// query sites (the house anti-drift discipline).
// CONST: column projection locked to the `scenario_fact_cards` schema — a
// structural schema-coupling invariant, not a deployment value.
const CARD_COLUMNS: &str = "scenario_id, graph_node_id, title, backs_position, supports, \
     watch_out, answer, title_authored_by, backs_position_authored_by, \
     supports_authored_by, watch_out_authored_by, answer_authored_by, \
     authored_by, created_at, edited_at";

// CONST: the ledger append. Every field ever written lands here.
const INSERT_CARD_EVENT_SQL: &str = r#"INSERT INTO scenario_fact_card_events
        (scenario_id, graph_node_id, field, value, actor, at)
    VALUES ($1, $2, $3, $4, $5, $6)"#;

/// Everything needed to write one field.
///
/// A params struct rather than six positional arguments: `graph_node_id`,
/// `author` and `value` are all string-shaped, so a call site could swap two with
/// no compile error and write a username into a witness's answer.
#[derive(Debug)]
pub struct FieldWrite<'a> {
    pub scenario_id: Uuid,
    pub graph_node_id: &'a str,
    pub field: CardField,
    /// The new value, already rendered as the text the column holds. `None`
    /// clears the field — a real act, and the ledger records it as one.
    pub value: Option<&'a str>,
    pub author: &'a str,
    /// Passed in rather than read from the clock here, so a write and its ledger
    /// row share a timestamp and the row matches its log line.
    pub written_at: DateTime<Utc>,
}

/// Write ONE field of one card, and its ledger entry, in ONE transaction.
///
/// Creates the card row if this is the first field written on it.
///
/// ## Why the SQL is built rather than held as a `const`
///
/// The column name IS `field.code()` (see [`CardField`]), and Postgres cannot
/// parameterize an identifier. The token comes from a CLOSED enum, never from a
/// request body — `CardField` is a serde enum, so an unknown token fails at the
/// HTTP parse boundary and cannot reach here. That is what makes the
/// interpolation safe, and it is the same argument the Cypher builders make for
/// interpolating `schema::*` constants.
///
/// ## Why every value is bound as TEXT and cast per field
///
/// The value arrives as TEXT like every other field's, so one code path writes
/// all five. The columns are not all TEXT — `backs_position` is INTEGER and
/// `supports` is JSONB — and that difference lives on
/// [`CardField::sql_cast`], which keeps the caller from having to know which
/// fields are which.
///
/// # Errors
/// Returns [`PipelineRepoError`] if either write or the commit fails. Nothing is
/// stored when it does.
pub async fn write_field(pool: &PgPool, write: &FieldWrite<'_>) -> Result<(), PipelineRepoError> {
    let column = write.field.code();
    // The cast belongs to the FIELD, not to this statement — see
    // `CardField::sql_cast` for the live failure that moved it there.
    let cast = write.field.sql_cast();
    let sql = format!(
        "INSERT INTO scenario_fact_cards \
             (scenario_id, graph_node_id, {column}, {column}_authored_by, \
              authored_by, created_at, edited_at) \
         VALUES ($1, $2, $3{cast}, $4, $4, $5, $5) \
         ON CONFLICT (scenario_id, graph_node_id) DO UPDATE SET \
             {column}             = EXCLUDED.{column}, \
             {column}_authored_by = EXCLUDED.{column}_authored_by, \
             authored_by          = EXCLUDED.authored_by, \
             edited_at            = EXCLUDED.edited_at"
    );

    let mut tx = pool.begin().await?;
    sqlx::query(&sql)
        .bind(write.scenario_id)
        .bind(write.graph_node_id)
        .bind(write.value)
        .bind(write.author)
        .bind(write.written_at)
        .execute(&mut *tx)
        .await?;
    append_event(&mut tx, write).await?;
    tx.commit().await?;
    Ok(())
}

/// Append one ledger row for the field that was just written.
///
/// Split out so [`write_field`] reads as the two writes it is, and so the ledger
/// shape is stated in exactly one place.
async fn append_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    write: &FieldWrite<'_>,
) -> Result<(), PipelineRepoError> {
    sqlx::query(INSERT_CARD_EVENT_SQL)
        .bind(write.scenario_id)
        .bind(write.graph_node_id)
        .bind(write.field.code())
        .bind(write.value)
        .bind(write.author)
        .bind(write.written_at)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Every card in one scenario, in ONE query.
///
/// ## Why keyed by scenario and not by node
///
/// Both surfaces that read these ask the same question — "what does this scenario
/// say about its facts" — and answer it for the whole deck at once. A per-card
/// query would be forty round trips to answer what one `WHERE scenario_id = $1`
/// answers, on a page a human is waiting on.
///
/// Ordered by `graph_node_id` so two reads of unchanged data return the same
/// order. The DISPLAY order is `scenario_fact_order`'s and is applied elsewhere;
/// this one exists only so the map below is built deterministically.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn list_cards_for_scenario(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<CardRecord>, PipelineRepoError> {
    let sql = format!(
        "SELECT {CARD_COLUMNS} FROM scenario_fact_cards \
         WHERE scenario_id = $1 ORDER BY graph_node_id"
    );
    let rows = sqlx::query_as::<_, CardRecord>(&sql)
        .bind(scenario_id)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

#[cfg(test)]
#[path = "scenario_fact_cards_tests.rs"]
mod tests;
