//! Pipeline-config base CRUD.
//!
//! Owns the canonical `pipeline_config` row shape ([`PipelineConfigRecord`]),
//! the upload-time input shape ([`PipelineConfigInput`]), and the
//! insert / read paths for the base columns. The override columns
//! (`profile_name`, `extraction_model`, `chunking_config`, etc.) plus
//! the PATCH-by-field-presence semantics live in
//! [`super::config_overrides`].

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use super::PipelineRepoError;

// ── Types ────────────────────────────────────────────────────────

/// Input for creating pipeline configuration (from the upload request).
///
/// Bug #6 fix: the legacy `pass1_model`/`pass2_model`/`pass1_max_tokens`/
/// `pass2_max_tokens` fields are gone. The resolver reads
/// `extraction_model` / `pass2_extraction_model` / `max_tokens` from the
/// override columns; the legacy columns were dead from the read side.
/// Migration 20260513_consolidate_model_columns_and_add_overrides.sql
/// drops them. All model/max-token selection now flows through the
/// profile → override path written by `patch_pipeline_config_overrides`.
///
/// `#[serde(deny_unknown_fields)]` is the no-silent-fail guard for any
/// caller that ever deserialises this struct from external JSON: a
/// stale field name (e.g. the dropped `pass1_model`) or a typo in a
/// future field rename will produce a deserialization error naming the
/// offending key, rather than silently dropping the field and writing
/// a row that drifts away from the operator's intent. Currently the
/// upload handler builds this struct programmatically — the attribute
/// is the regression guard for the day someone wires it up to a JSON
/// request body without remembering to opt in to strict parsing.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PipelineConfigInput {
    pub schema_file: String,
    pub admin_instructions: Option<String>,
    pub prior_context_doc_ids: Option<Vec<String>>,
}

/// A pipeline_config record from the database.
///
/// Bug #6 fix: the legacy `pass1_model`/`pass2_model`/`pass1_max_tokens`/
/// `pass2_max_tokens` fields are gone — the columns were dropped by
/// migration 20260513_consolidate_model_columns_and_add_overrides.sql.
/// Resolved model selection comes from the override columns
/// (`extraction_model`, `pass2_extraction_model`, `max_tokens`) read via
/// `get_pipeline_config_overrides`.
///
/// `#[serde(deny_unknown_fields)]` is defensive against the day this
/// struct is ever deserialized from JSON: a stale field name (e.g. the
/// dropped `pass1_model`) will fail loudly instead of silently dropping.
/// `sqlx::FromRow` ignores serde attributes, so the attribute has zero
/// effect on the existing DB read path.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
#[serde(deny_unknown_fields)]
pub struct PipelineConfigRecord {
    pub document_id: String,
    pub schema_file: String,
    pub admin_instructions: Option<String>,
    pub prior_context_doc_ids: Option<Vec<String>>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ── CRUD ─────────────────────────────────────────────────────────

/// Insert pipeline configuration for a document.
///
/// Bug #6/#10 fix: no model column is written here. The hardcoded SQL
/// COALESCE default `'claude-sonnet-4-6'` is gone — model resolution
/// runs entirely through the profile → override path written by
/// `patch_pipeline_config_overrides` immediately after upload. There is
/// no hardcoded model name anywhere in this codebase post-fix.
pub async fn insert_pipeline_config(
    pool: &PgPool,
    document_id: &str,
    config: &PipelineConfigInput,
    created_by: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        r#"INSERT INTO pipeline_config
           (document_id, schema_file, admin_instructions, prior_context_doc_ids, created_by)
           VALUES ($1, $2, $3, $4, $5)"#,
    )
    .bind(document_id)
    .bind(&config.schema_file)
    .bind(&config.admin_instructions)
    .bind(&config.prior_context_doc_ids)
    .bind(created_by)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get pipeline config for a document. Returns None if not configured.
pub async fn get_pipeline_config(
    pool: &PgPool,
    document_id: &str,
) -> Result<Option<PipelineConfigRecord>, PipelineRepoError> {
    let row = sqlx::query_as::<_, PipelineConfigRecord>(
        "SELECT document_id, schema_file, admin_instructions,
                prior_context_doc_ids, created_by, created_at
         FROM pipeline_config WHERE document_id = $1",
    )
    .bind(document_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

#[cfg(test)]
mod tests {
    //! Tests for `PipelineConfigInput`'s strict-parsing contract.
    //!
    //! The struct currently has no `from_json` call site in production
    //! (the upload handler builds it programmatically). These tests
    //! exist so a future contributor who wires it up to a JSON request
    //! body cannot silently lose the `deny_unknown_fields` invariant —
    //! removing the attribute would flip the
    //! `rejects_unknown_field` test from passing to failing.
    use super::*;

    #[test]
    fn deserialize_accepts_well_formed_input() {
        // Sanity: the strict-parsing contract does not break the
        // intended-shape path. The full set of declared fields round-
        // trips through serde successfully.
        let raw = serde_json::json!({
            "schema_file": "complaint_v5_1.yaml",
            "admin_instructions": "extract only facts pre-2024",
            "prior_context_doc_ids": ["doc-a", "doc-b"]
        });
        let parsed: PipelineConfigInput =
            serde_json::from_value(raw).expect("well-formed input must deserialize");
        assert_eq!(parsed.schema_file, "complaint_v5_1.yaml");
        assert_eq!(
            parsed.admin_instructions.as_deref(),
            Some("extract only facts pre-2024")
        );
        assert_eq!(
            parsed.prior_context_doc_ids.as_deref(),
            Some(["doc-a".to_string(), "doc-b".to_string()].as_slice())
        );
    }

    #[test]
    fn deserialize_accepts_minimal_input() {
        // Only `schema_file` is required; the two Option fields default
        // to None. Strict parsing must not reject the minimal shape.
        let raw = serde_json::json!({ "schema_file": "complaint_v5_1.yaml" });
        let parsed: PipelineConfigInput =
            serde_json::from_value(raw).expect("minimal input must deserialize");
        assert_eq!(parsed.schema_file, "complaint_v5_1.yaml");
        assert!(parsed.admin_instructions.is_none());
        assert!(parsed.prior_context_doc_ids.is_none());
    }

    #[test]
    fn deserialize_rejects_unknown_field() {
        // Regression guard for `#[serde(deny_unknown_fields)]`. A stale
        // or misspelled field name in a future request body must fail
        // loudly here, naming the offending key, instead of silently
        // dropping the field and writing a row that drifts away from
        // the operator's intent. If a future "cleanup" removes the
        // attribute, this test fails and forces a re-justification.
        let raw = serde_json::json!({
            "schema_file": "complaint_v5_1.yaml",
            "pass1_model": "claude-sonnet-4-6"
        });
        let err = serde_json::from_value::<PipelineConfigInput>(raw)
            .expect_err("unknown field must error, not be silently dropped");
        let msg = err.to_string();
        assert!(
            msg.contains("pass1_model"),
            "error must name the offending field; got: {msg}"
        );
        assert!(
            msg.contains("unknown field"),
            "error must identify the rejection as 'unknown field'; got: {msg}"
        );
    }
}
