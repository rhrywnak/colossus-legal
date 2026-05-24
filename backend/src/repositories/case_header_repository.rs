//! Postgres read layer for the Home page case header (`GET /api/cases/:slug`).
//!
//! Three independent SELECTs against the **main** (`colossus_legal`) pool — the
//! `cases`, `parties`, and `counsel` tables from migration
//! `20260524095049_case_metadata_tables.sql`. Reads only, so no transaction is
//! needed. The rows are shaped into the response DTO by
//! [`super::case_header_builder::build_case_header`]; keeping the SQL here and
//! the shaping there lets the (DB-free) shaping logic be unit-tested.
//!
//! ## Rust Learning: `sqlx::FromRow` + runtime queries
//!
//! The project does not use the compile-time-checked `query!` macros (there is
//! no `.sqlx` offline cache and no build-time DB), so we use runtime
//! `sqlx::query_as::<_, Row>(SQL).bind(..)`. `#[derive(sqlx::FromRow)]` maps
//! result columns to struct fields by name — the `SELECT` column list must
//! match the field names below.

use chrono::NaiveDate;
use sqlx::PgPool;

/// Errors from the case-header reads. Each carries the inputs and the
/// underlying `sqlx::Error` so a failure is locatable in the logs — this
/// context is for operators, not the HTTP response body (Standing Rule 1).
#[derive(Debug, thiserror::Error)]
pub enum CaseHeaderRepoError {
    #[error("Failed to fetch case by slug '{slug}': {source}")]
    FetchCase {
        slug: String,
        #[source]
        source: sqlx::Error,
    },
    #[error("Failed to fetch parties for case '{case_id}': {source}")]
    FetchParties {
        case_id: String,
        #[source]
        source: sqlx::Error,
    },
    #[error("Failed to fetch counsel for case '{case_id}': {source}")]
    FetchCounsel {
        case_id: String,
        #[source]
        source: sqlx::Error,
    },
}

/// One row of `cases`. Field names mirror the column names for `FromRow`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct CaseRow {
    pub case_id: String,
    pub case_slug: String,
    pub display_title: String,
    pub display_title_full: Option<String>,
    pub court_name: Option<String>,
    pub jurisdiction: Option<String>,
    pub case_number: Option<String>,
    pub filed_date: Option<NaiveDate>,
    pub transferred_from: Option<String>,
    pub transfer_date: Option<NaiveDate>,
    pub status: String,
    pub complaint_document_id: Option<String>,
}

/// One row of `parties`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct PartyRow {
    pub party_id: String,
    pub name: String,
    pub role: String,
    pub entity_type: Option<String>,
    pub status: String,
    pub dismissal_date: Option<NaiveDate>,
    pub dismissal_basis: Option<String>,
    pub notes: Option<String>,
    pub sort_order: i32,
}

/// One row of `counsel`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct CounselRow {
    pub counsel_id: String,
    pub represents_role: String,
    pub firm_name: Option<String>,
    pub attorney_name: String,
    pub bar_number: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub sort_order: i32,
}

// Explicit column lists (not `SELECT *`) so a future column addition can't
// silently change the row shape, and the FromRow mapping stays pinned.
const CASE_BY_SLUG_SQL: &str = "SELECT case_id, case_slug, display_title, display_title_full, \
     court_name, jurisdiction, case_number, filed_date, transferred_from, transfer_date, \
     status, complaint_document_id \
     FROM cases WHERE case_slug = $1";

// ORDER BY in SQL is a convenience; the shaping layer re-sorts each bucket so
// the ordering is verifiable in a DB-free unit test.
const PARTIES_BY_CASE_SQL: &str = "SELECT party_id, name, role, entity_type, status, \
     dismissal_date, dismissal_basis, notes, sort_order \
     FROM parties WHERE case_id = $1 ORDER BY sort_order";

const COUNSEL_BY_CASE_SQL: &str = "SELECT counsel_id, represents_role, firm_name, attorney_name, \
     bar_number, address, phone, email, sort_order \
     FROM counsel WHERE case_id = $1 ORDER BY sort_order";

/// Fetch a case by its slug. `Ok(None)` ⇒ no such case (the handler turns that
/// into a 404); `Err` ⇒ a database failure (→ 500).
pub(crate) async fn fetch_case_by_slug(
    pool: &PgPool,
    slug: &str,
) -> Result<Option<CaseRow>, CaseHeaderRepoError> {
    sqlx::query_as::<_, CaseRow>(CASE_BY_SLUG_SQL)
        .bind(slug)
        .fetch_optional(pool)
        .await
        .map_err(|source| CaseHeaderRepoError::FetchCase {
            slug: slug.to_string(),
            source,
        })
}

/// Fetch all parties for a case (any role/status — bucketing happens in the
/// shaping layer).
pub(crate) async fn fetch_parties(
    pool: &PgPool,
    case_id: &str,
) -> Result<Vec<PartyRow>, CaseHeaderRepoError> {
    sqlx::query_as::<_, PartyRow>(PARTIES_BY_CASE_SQL)
        .bind(case_id)
        .fetch_all(pool)
        .await
        .map_err(|source| CaseHeaderRepoError::FetchParties {
            case_id: case_id.to_string(),
            source,
        })
}

/// Fetch all counsel rows for a case.
pub(crate) async fn fetch_counsel(
    pool: &PgPool,
    case_id: &str,
) -> Result<Vec<CounselRow>, CaseHeaderRepoError> {
    sqlx::query_as::<_, CounselRow>(COUNSEL_BY_CASE_SQL)
        .bind(case_id)
        .fetch_all(pool)
        .await
        .map_err(|source| CaseHeaderRepoError::FetchCounsel {
            case_id: case_id.to_string(),
            source,
        })
}
