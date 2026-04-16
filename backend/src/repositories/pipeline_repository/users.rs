//! Known-user tracking and reviewer assignment.
//!
//! `known_users` is populated automatically when users hit `/api/me`.
//! `assign_reviewer` links a reviewer username to a pipeline document.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

use super::PipelineRepoError;

// ── Types ────────────────────────────────────────────────────────

/// A user who has logged into the application at least once.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct KnownUser {
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub last_seen_at: DateTime<Utc>,
}

// ── User tracking ────────────────────────────────────────────────

/// Upsert a user record. Called on every authenticated `/api/me` request.
///
/// If the user already exists, updates `display_name`, `email`, and
/// `last_seen_at`. If new, inserts with `first_seen_at = NOW()`.
///
/// ## Rust Learning: ON CONFLICT … DO UPDATE
///
/// PostgreSQL's `ON CONFLICT` clause lets us handle "insert or update"
/// atomically in a single statement. The `EXCLUDED` pseudo-table refers
/// to the values that *would* have been inserted, so we can use them
/// in the UPDATE set-list.
pub async fn upsert_known_user(
    pool: &PgPool,
    username: &str,
    display_name: &str,
    email: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        r#"INSERT INTO known_users (username, display_name, email)
           VALUES ($1, $2, $3)
           ON CONFLICT (username) DO UPDATE
             SET display_name = EXCLUDED.display_name,
                 email        = EXCLUDED.email,
                 last_seen_at = NOW()"#,
    )
    .bind(username)
    .bind(display_name)
    .bind(email)
    .execute(pool)
    .await?;
    Ok(())
}

/// List all known users, ordered by display_name.
///
/// Used to populate reviewer dropdowns and other user-selection UIs.
pub async fn list_known_users(pool: &PgPool) -> Result<Vec<KnownUser>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, KnownUser>(
        "SELECT username, display_name, email, last_seen_at
         FROM known_users ORDER BY display_name",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Assign a reviewer to a pipeline document. Pass `None` to unassign.
///
/// Sets `assigned_at` to NOW() when assigning, or NULL when unassigning.
pub async fn assign_reviewer(
    pool: &PgPool,
    document_id: &str,
    reviewer: Option<&str>,
) -> Result<(), PipelineRepoError> {
    let result = sqlx::query(
        r#"UPDATE documents
           SET assigned_reviewer = $1,
               assigned_at = CASE WHEN $1 IS NOT NULL THEN NOW() ELSE NULL END,
               updated_at = NOW()
           WHERE id = $2"#,
    )
    .bind(reviewer)
    .bind(document_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(PipelineRepoError::NotFound(document_id.to_string()));
    }
    Ok(())
}
