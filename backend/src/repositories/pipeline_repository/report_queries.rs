//! Quality-report aggregation queries.
//!
//! Sibling helpers to [`extraction`] focused on the per-document
//! quality report (Audit Gap 14, Instruction E). Kept in a separate
//! module because:
//!
//! 1. `extraction.rs` is already 1700+ lines and growing.
//! 2. These helpers are read-only aggregations — a different concern
//!    from the CRUD-heavy contents of `extraction.rs`.
//! 3. The new `PerPassRunMetadata` shape is larger than the existing
//!    `ExtractionRunRecord` and tying them in the same file would
//!    blur which fields are "wire shape from disk" vs "report-derived
//!    + JSONB-extracted."
//!
//! The functions in this module are designed to **degrade gracefully**:
//! a per-row JSONB parse failure surfaces as a populated
//! `parse_error` field on that row's metadata, not a fatal error
//! that crashes the whole report. The HTML / JSON layer above this
//! module records the parse error in the report's `limitations` list.

use sqlx::PgPool;
use sqlx::Row;

use crate::pipeline::config::ResolvedConfig;
use crate::repositories::pipeline_repository::PipelineRepoError;

// ── Public types ───────────────────────────────────────────────────

/// One row's worth of per-pass extraction metadata, augmented with
/// fingerprints extracted from the row's `processing_config` JSONB.
///
/// One instance per `extraction_runs` row. `pass_number` is `1` or `2`
/// in current schemas (the column is just `INTEGER` so it could grow).
///
/// ## `parse_error` and graceful degradation
///
/// `processing_config` is a JSONB blob written by
/// `write_processing_config_snapshot` (see Instructions A/B/C). It
/// SHOULD deserialize cleanly into [`ResolvedConfig`]. If a particular
/// row was written by an older binary (pre-Instruction-A), or by some
/// future change that broke shape compatibility, parsing fails — and
/// we record the error here rather than crashing the whole report.
/// All fingerprint fields below stay `None` for that row; the report
/// builder above this layer turns the `parse_error` into a
/// human-readable `limitations` entry naming the pass.
///
/// ## Tutorial: `cost_usd` is `Option<String>`, not `Option<Decimal>`
///
/// `extraction_runs.cost_usd` is `NUMERIC(10,4)` in PostgreSQL. The
/// existing `ExtractionRunRecord` casts it to `text` in the SQL and
/// stores the string-encoded decimal — avoids needing `rust_decimal`
/// as a dep just for one column. This struct mirrors that pattern:
/// the field is a string-encoded decimal (`"0.0327"`), not a typed
/// numeric. JSON-serialised reports will show it as a string for the
/// same reason.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PerPassRunMetadata {
    pub pass_number: i32,
    pub model_name: String,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    /// String-encoded decimal (`"0.0327"`). NULL → `None`. See
    /// `cost_usd is Option<String>, not Option<Decimal>` in the
    /// struct doc-comment for rationale.
    pub cost_usd: Option<String>,
    pub status: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,

    // Fingerprints extracted from processing_config JSONB. All
    // `Option` so a missing-or-malformed snapshot degrades to None
    // rather than failing the report build.
    pub effective_pass: Option<u8>,
    pub profile_name: Option<String>,
    pub profile_hash: Option<String>,
    pub template_file: Option<String>,
    pub template_hash: Option<String>,
    pub system_prompt_file: Option<String>,
    pub system_prompt_hash: Option<String>,
    pub global_rules_file: Option<String>,
    pub global_rules_hash: Option<String>,
    pub schema_file: Option<String>,
    pub pass2_cross_doc_entity_count: i64,
    pub pass2_source_document_count: i64,
    pub pass2_source_document_ids: Vec<String>,

    /// `None` when `processing_config` parsed cleanly.
    /// `Some(err_msg)` when parsing failed — fingerprint fields above
    /// are all `None` in that case. The report builder turns this into
    /// a `limitations` entry.
    pub parse_error: Option<String>,
}

/// One `(relationship_type, count)` row of the relationship breakdown.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RelationshipTypeCount {
    pub relationship_type: String,
    pub count: i64,
}

// ── Queries ────────────────────────────────────────────────────────

/// Read every `extraction_runs` row for a document, parse each row's
/// `processing_config` JSONB into [`ResolvedConfig`], and return the
/// per-pass metadata.
///
/// Order: ascending `pass_number`. A document with both Pass-1 and
/// Pass-2 yields a length-2 `Vec`.
///
/// ## Tutorial: `serde_json::from_value::<ResolvedConfig>(...)` per row
///
/// We deserialise the entire JSONB blob into the typed
/// [`ResolvedConfig`] struct in one call rather than picking each
/// fingerprint field via `processing_config->>'foo'`. Reasons:
///
/// 1. **Single error path.** A malformed blob raises one
///    `serde_json::Error` with file:line context, not N silent
///    `Option::None`s. Matches Instruction C's
///    `decode_jsonb_map`-style discipline.
/// 2. **No string literals in the report code.** The JSONB key set
///    lives on `ResolvedConfig`'s field names — adding a new audit
///    field updates this code automatically (the field appears on
///    `ResolvedConfig`, the report includes it).
/// 3. **`#[serde(default)]` on every new field** of `ResolvedConfig`
///    (verified across Instructions A/B/C) means historical snapshots
///    that pre-date a field still parse cleanly — backward
///    compatibility is by construction.
///
/// **Backward compatibility constraint:** `ResolvedConfig` must remain
/// **additive-only** — a non-additive change (renaming a field,
/// changing a type) would break this parse for every historical
/// snapshot in the database. A breaking change to the JSONB shape
/// requires a JSONB-rewriting migration, not a naked struct edit.
pub async fn get_extraction_runs_with_processing_config(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<PerPassRunMetadata>, PipelineRepoError> {
    // SELECT the full row including the JSONB column. The JSONB column
    // is fetched as `Option<serde_json::Value>` so a NULL snapshot
    // (no Instruction-A snapshot was ever written) decays to a
    // populated metadata with `None` fingerprint fields and no
    // `parse_error` (NULL is operationally distinct from "parse
    // failure" — the row simply has no audit body to surface).
    let rows = sqlx::query(
        "SELECT id, pass_number, model_name, input_tokens, output_tokens,
                cost_usd::text AS cost_usd_text, status, started_at, completed_at,
                processing_config
         FROM extraction_runs
         WHERE document_id = $1
         ORDER BY pass_number ASC, id ASC",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await?;

    let mut out: Vec<PerPassRunMetadata> = Vec::with_capacity(rows.len());
    for row in rows {
        let pass_number: i32 = row.try_get("pass_number")?;
        let model_name: String = row.try_get("model_name")?;
        let input_tokens: Option<i32> = row.try_get("input_tokens")?;
        let output_tokens: Option<i32> = row.try_get("output_tokens")?;
        let cost_usd: Option<String> = row.try_get("cost_usd_text")?;
        let status: String = row.try_get("status")?;
        let started_at: chrono::DateTime<chrono::Utc> = row.try_get("started_at")?;
        let completed_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("completed_at")?;
        let processing_config: Option<serde_json::Value> = row.try_get("processing_config")?;

        // Decode the JSONB. Three states:
        //   - NULL → no snapshot ever written; populate with row data
        //     only, all fingerprint fields stay None, no parse_error.
        //   - Some(value) parses → populate fingerprints from the
        //     resolved config.
        //   - Some(value) fails → record parse_error, leave
        //     fingerprints None.
        let mut metadata = PerPassRunMetadata {
            pass_number,
            model_name,
            input_tokens,
            output_tokens,
            cost_usd,
            status,
            started_at,
            completed_at,
            effective_pass: None,
            profile_name: None,
            profile_hash: None,
            template_file: None,
            template_hash: None,
            system_prompt_file: None,
            system_prompt_hash: None,
            global_rules_file: None,
            global_rules_hash: None,
            schema_file: None,
            pass2_cross_doc_entity_count: 0,
            pass2_source_document_count: 0,
            pass2_source_document_ids: Vec::new(),
            parse_error: None,
        };

        if let Some(value) = processing_config {
            match serde_json::from_value::<ResolvedConfig>(value) {
                Ok(rc) => {
                    metadata.effective_pass = Some(rc.effective_pass);
                    metadata.profile_name = Some(rc.profile_name);
                    if !rc.profile_hash.is_empty() {
                        metadata.profile_hash = Some(rc.profile_hash);
                    }
                    metadata.template_file = Some(rc.template_file);
                    metadata.template_hash = rc.template_hash;
                    metadata.system_prompt_file = rc.system_prompt_file;
                    metadata.system_prompt_hash = rc.system_prompt_hash;
                    metadata.global_rules_file = rc.global_rules_file;
                    metadata.global_rules_hash = rc.global_rules_hash;
                    metadata.schema_file = Some(rc.schema_file);
                    metadata.pass2_cross_doc_entity_count =
                        rc.pass2_cross_doc_entities.len() as i64;
                    metadata.pass2_source_document_count =
                        rc.pass2_source_document_ids.len() as i64;
                    metadata.pass2_source_document_ids = rc.pass2_source_document_ids;
                }
                Err(e) => {
                    // Graceful degradation: this one row's audit
                    // surface is missing, but the rest of the report
                    // builds. The `parse_error` field is the
                    // explicit-not-silent record of the failure.
                    metadata.parse_error = Some(format!("{e}"));
                }
            }
        }
        out.push(metadata);
    }
    Ok(out)
}

/// Group `extraction_relationships` for a document by
/// `relationship_type` and return counts.
///
/// Sort order: count descending, then `relationship_type` ascending —
/// so the highest-volume types surface first, with a stable
/// alphabetical tie-break for human readability. SQL does the
/// grouping; this function just unwraps each row.
pub async fn get_relationship_breakdown_by_type(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<RelationshipTypeCount>, PipelineRepoError> {
    let rows = sqlx::query(
        "SELECT relationship_type, COUNT(*) AS n
         FROM extraction_relationships
         WHERE document_id = $1
         GROUP BY relationship_type
         ORDER BY n DESC, relationship_type ASC",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await?;

    let mut out: Vec<RelationshipTypeCount> = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(RelationshipTypeCount {
            relationship_type: row.try_get("relationship_type")?,
            count: row.try_get("n")?,
        });
    }
    Ok(out)
}

/// Group `extraction_relationships` for a document by `(pass_number,
/// relationship_type)` via JOIN to `extraction_runs`.
///
/// `extraction_relationships` does not carry `pass_number` directly;
/// it only carries `run_id` (FK to `extraction_runs.id`). Per-pass
/// attribution is via the JOIN. This is what the audit's "per-pass
/// relationship breakdown" question (§7) needs.
///
/// Ordered `pass_number ASC, count DESC, relationship_type ASC` so
/// the report renders Pass-1 first, then Pass-2, with each pass
/// internally sorted highest-count first.
pub async fn get_per_pass_relationship_breakdown(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<(i32, RelationshipTypeCount)>, PipelineRepoError> {
    let rows = sqlx::query(
        "SELECT runs.pass_number AS pass_number,
                rels.relationship_type AS relationship_type,
                COUNT(*) AS n
         FROM extraction_relationships rels
         JOIN extraction_runs runs ON runs.id = rels.run_id
         WHERE rels.document_id = $1
         GROUP BY runs.pass_number, rels.relationship_type
         ORDER BY runs.pass_number ASC, n DESC, rels.relationship_type ASC",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await?;

    let mut out: Vec<(i32, RelationshipTypeCount)> = Vec::with_capacity(rows.len());
    for row in rows {
        out.push((
            row.try_get("pass_number")?,
            RelationshipTypeCount {
                relationship_type: row.try_get("relationship_type")?,
                count: row.try_get("n")?,
            },
        ));
    }
    Ok(out)
}

/// Group `extraction_items` for a document by `(pass_number, entity_type)`
/// via JOIN to `extraction_runs`.
///
/// Pass-1 typically owns every entity (Pass-2 only adds
/// relationships, no new entities). The breakdown therefore tends to
/// concentrate in Pass-1 — but the query handles the (uncommon)
/// future case where a Pass-2 also produces entities.
pub async fn get_per_pass_entity_breakdown(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<(i32, String, i64)>, PipelineRepoError> {
    let rows = sqlx::query(
        "SELECT runs.pass_number AS pass_number,
                items.entity_type AS entity_type,
                COUNT(*) AS n
         FROM extraction_items items
         JOIN extraction_runs runs ON runs.id = items.run_id
         WHERE items.document_id = $1
         GROUP BY runs.pass_number, items.entity_type
         ORDER BY runs.pass_number ASC, n DESC, items.entity_type ASC",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await?;

    let mut out: Vec<(i32, String, i64)> = Vec::with_capacity(rows.len());
    for row in rows {
        out.push((
            row.try_get("pass_number")?,
            row.try_get("entity_type")?,
            row.try_get("n")?,
        ));
    }
    Ok(out)
}
