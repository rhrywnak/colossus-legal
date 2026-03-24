//! Repository for admin audit log writes.
//!
//! ## Rust Learning — Append-Only Patterns
//!
//! This repository only has INSERT operations — no UPDATE, no DELETE.
//! The audit log is an immutable record. This is a common pattern for
//! audit trails, event sourcing, and compliance logging. The Rust type
//! system doesn't enforce this (you could add a delete method), but
//! by convention we keep this module deliberately minimal.

use serde_json::Value as JsonValue;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AuditRepository {
    pool: PgPool,
}

impl AuditRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Record an admin action. Called by the audit logging middleware
    /// or directly by handlers that need custom detail recording.
    pub async fn log_action(
        &self,
        username: &str,
        action: &str,
        resource_type: Option<&str>,
        resource_id: Option<&str>,
        details: Option<JsonValue>,
        ip_address: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO admin_audit_log
               (username, action, resource_type, resource_id, details, ip_address)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(username)
        .bind(action)
        .bind(resource_type)
        .bind(resource_id)
        .bind(details)
        .bind(ip_address)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get recent audit log entries (for the admin UI, paginated).
    pub async fn get_recent(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditLogEntry>, sqlx::Error> {
        let rows = sqlx::query_as::<_, AuditLogEntry>(
            r#"SELECT id, username, action, resource_type, resource_id,
                      details, ip_address, performed_at
               FROM admin_audit_log
               ORDER BY performed_at DESC
               LIMIT $1 OFFSET $2"#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}

/// Log an admin action, swallowing errors with a warning.
///
/// ## Rust Learning — Fire-and-Forget Async
///
/// We call `.await` on the audit log write, but discard any error via
/// `tracing::warn!`. This is intentional — audit logging should never
/// prevent the primary action from completing. The pattern is: we wait
/// for completion but don't propagate failures.
pub async fn log_admin_action(
    audit_repo: &AuditRepository,
    username: &str,
    action: &str,
    resource_type: Option<&str>,
    resource_id: Option<&str>,
    details: Option<JsonValue>,
) {
    if let Err(e) = audit_repo
        .log_action(username, action, resource_type, resource_id, details, None)
        .await
    {
        tracing::warn!("Failed to write audit log: {e}");
    }
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct AuditLogEntry {
    pub id: i64,
    pub username: String,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub details: Option<JsonValue>,
    pub ip_address: Option<String>,
    pub performed_at: chrono::DateTime<chrono::Utc>,
}
