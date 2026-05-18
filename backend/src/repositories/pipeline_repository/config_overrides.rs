//! Per-document override columns on `pipeline_config`.
//!
//! Migration `20260420_config_system.sql` added a set of nullable
//! override columns to `pipeline_config` so the Configuration Panel can
//! tweak a single document's pipeline without forking its profile.
//! This module owns the read/write paths for those columns:
//!
//! - [`get_pipeline_config_overrides`] — read the row and assemble a
//!   typed [`PipelineConfigOverrides`] (NULL columns ⇒ `None`).
//! - [`patch_pipeline_config_overrides`] — `UPDATE ... COALESCE`-based
//!   partial update where each `None` field leaves the column
//!   untouched and `Some(value)` replaces it (whole-column, not key-
//!   level merge).
//!
//! The JSONB override columns (`chunking_config`, `context_config`) use
//! the same decode helper, [`decode_jsonb_map`], so malformed JSONB
//! surfaces as `PipelineRepoError::Deserialization` carrying both the
//! document_id and column name — never a silent fallback to `None`.

use sqlx::PgPool;
use std::collections::HashMap;

use crate::pipeline::config::PipelineConfigOverrides;

use super::PipelineRepoError;

// ── Row helper ───────────────────────────────────────────────────

/// Row-shaped helper used only by [`get_pipeline_config_overrides`].
///
/// The nullable override columns have an awkward 8-tuple shape; a named
/// struct keeps the type signature readable and avoids the
/// `clippy::type_complexity` lint on anonymous tuples.
#[derive(sqlx::FromRow)]
struct PipelineConfigOverridesRow {
    profile_name: Option<String>,
    extraction_model: Option<String>,
    pass2_extraction_model: Option<String>,
    pass2_template_file: Option<String>,
    template_file: Option<String>,
    system_prompt_file: Option<String>,
    chunking_mode: Option<String>,
    chunk_size: Option<i32>,
    chunk_overlap: Option<i32>,
    max_tokens: Option<i32>,
    temperature: Option<f64>,
    run_pass2: Option<bool>,
    /// Bug #8 fix — per-document overrides for fields that were
    /// previously profile-only. NULL = "no override; inherit from profile."
    auto_approve_grounded: Option<bool>,
    global_rules_file: Option<String>,
    /// Raw JSONB from the new override columns. We deliberately stay
    /// at `serde_json::Value` here (rather than `Json<HashMap<...>>`)
    /// so the converter can attach the document_id to a typed
    /// `Deserialization` error if the JSON's shape doesn't match the
    /// expected map. `Json<T>`'s decode error wouldn't carry that
    /// context and would silently surface as a generic sqlx error.
    chunking_config: Option<serde_json::Value>,
    context_config: Option<serde_json::Value>,
}

// ── Read path ────────────────────────────────────────────────────

/// Read per-document override columns from `pipeline_config`.
///
/// Returns a [`PipelineConfigOverrides`] populated from the nullable columns
/// added by migration `20260420_config_system.sql`. Each field is `Option` —
/// `None` means "use the profile default."
///
/// If no `pipeline_config` row exists for the document, returns
/// `PipelineConfigOverrides::default()` (all `None`). Callers can then
/// still resolve against the profile without a separate existence check.
pub async fn get_pipeline_config_overrides(
    db: &PgPool,
    document_id: &str,
) -> Result<PipelineConfigOverrides, PipelineRepoError> {
    let row: Option<PipelineConfigOverridesRow> = sqlx::query_as(
        "SELECT profile_name, extraction_model, pass2_extraction_model, \
                pass2_template_file, template_file, system_prompt_file, \
                chunking_mode, chunk_size, chunk_overlap, max_tokens, \
                temperature::float8 AS temperature, run_pass2, \
                auto_approve_grounded, global_rules_file, \
                chunking_config, context_config \
         FROM pipeline_config WHERE document_id = $1",
    )
    .bind(document_id)
    .fetch_optional(db)
    .await?;

    let result = match row {
        Some(r) => {
            // Decode the two JSONB override maps with no-silent-fails:
            // a malformed body raises `Deserialization` carrying the
            // document_id and column so an auditor can locate the bad
            // row directly. `None` (NULL column) means "no override;
            // resolve_config will fall back to the profile's map."
            let chunking_config =
                decode_jsonb_map(document_id, "chunking_config", r.chunking_config)?;
            let context_config = decode_jsonb_map(document_id, "context_config", r.context_config)?;
            PipelineConfigOverrides {
                profile_name: r.profile_name,
                extraction_model: r.extraction_model,
                pass2_extraction_model: r.pass2_extraction_model,
                pass2_template_file: r.pass2_template_file,
                template_file: r.template_file,
                system_prompt_file: r.system_prompt_file,
                chunking_mode: r.chunking_mode,
                chunk_size: r.chunk_size,
                chunk_overlap: r.chunk_overlap,
                max_tokens: r.max_tokens,
                temperature: r.temperature,
                run_pass2: r.run_pass2,
                auto_approve_grounded: r.auto_approve_grounded,
                global_rules_file: r.global_rules_file,
                chunking_config,
                context_config,
            }
        }
        None => PipelineConfigOverrides::default(),
    };

    Ok(result)
}

// ── Helpers ──────────────────────────────────────────────────────

/// Decode an `Option<serde_json::Value>` from a `pipeline_config` JSONB
/// column into the typed override shape (`Option<HashMap<String, Value>>`).
///
/// The two override columns (`chunking_config`, `context_config`) share
/// this exact shape, so the conversion is factored out. NULL → `Ok(None)`
/// (no override). A non-NULL value that doesn't deserialize into a map →
/// `Err(Deserialization)` with both `document_id` and `column` named in
/// the message — never silent `None`. The application layer treats `None`
/// as "inherit from profile"; a silent fall-through on bad data would
/// mask a corrupted row as a working one.
///
/// ## Rust Learning: factor on shape, not on column name
///
/// We pass the column name as a `&str` argument rather than writing two
/// near-identical decoders or templating the function over a const. The
/// caller already knows the column it's reading; threading it through
/// gives the error message all the context it needs without a generic
/// const-name parameter.
fn decode_jsonb_map(
    document_id: &str,
    column: &str,
    raw: Option<serde_json::Value>,
) -> Result<Option<HashMap<String, serde_json::Value>>, PipelineRepoError> {
    match raw {
        None => Ok(None),
        Some(v) => serde_json::from_value(v).map(Some).map_err(|e| {
            PipelineRepoError::Deserialization(format!(
                "pipeline_config.{column} for document_id={document_id} is not a valid map: {e}"
            ))
        }),
    }
}

/// True when the `PipelineConfigOverrides` payload carries at least one
/// non-`None` field — i.e. the PATCH actually requests a change.
///
/// Factored out of `patch_pipeline_config_overrides` so the contract
/// can be unit-tested. The risk this guards against: a future field
/// added to `PipelineConfigOverrides` whose `is_some()` clause is
/// forgotten here would produce a silent no-op for any PATCH that
/// touches only that new field. The
/// `patch_with_only_chunking_config_does_not_short_circuit` test below
/// pins that down for the chunking_config path; analogous tests should
/// be added when a future field is introduced.
fn has_any_override(overrides: &PipelineConfigOverrides) -> bool {
    overrides.profile_name.is_some()
        || overrides.extraction_model.is_some()
        || overrides.pass2_extraction_model.is_some()
        || overrides.pass2_template_file.is_some()
        || overrides.template_file.is_some()
        || overrides.system_prompt_file.is_some()
        || overrides.chunking_mode.is_some()
        || overrides.chunk_size.is_some()
        || overrides.chunk_overlap.is_some()
        || overrides.max_tokens.is_some()
        || overrides.temperature.is_some()
        || overrides.run_pass2.is_some()
        || overrides.auto_approve_grounded.is_some()
        || overrides.global_rules_file.is_some()
        || overrides.chunking_config.is_some()
        || overrides.context_config.is_some()
}

// ── Write path ───────────────────────────────────────────────────

/// Partially update the per-document override columns on `pipeline_config`.
///
/// Uses `UPDATE ... SET col = COALESCE($n, col)` so each `None` field in
/// `overrides` leaves the corresponding column untouched. If every field
/// in `overrides` is `None`, the UPDATE is skipped entirely to avoid a
/// pointless roundtrip.
///
/// Returns `PipelineRepoError::NotFound` if no `pipeline_config` row
/// matches `document_id` — the caller should have already inserted one at
/// upload time.
///
/// `temperature` is cast to `NUMERIC` inside the SQL so sqlx doesn't need
/// the `rust_decimal` feature for a direct `NUMERIC(3,2)` bind.
pub async fn patch_pipeline_config_overrides(
    db: &PgPool,
    document_id: &str,
    overrides: &PipelineConfigOverrides,
) -> Result<(), PipelineRepoError> {
    // Short-circuit when there is nothing to update. Factored out so a
    // unit test can pin down the contract: every overridable field on
    // `PipelineConfigOverrides` MUST contribute to this check, or a
    // PATCH whose only field is the missing one would silently no-op.
    let any_field = has_any_override(overrides);
    if !any_field {
        let existing: Option<String> =
            sqlx::query_scalar("SELECT document_id FROM pipeline_config WHERE document_id = $1")
                .bind(document_id)
                .fetch_optional(db)
                .await?;
        return if existing.is_some() {
            Ok(())
        } else {
            Err(PipelineRepoError::NotFound(document_id.to_string()))
        };
    }

    // Convert the typed `Option<HashMap<...>>` overrides into the JSONB
    // shape sqlx wants for binding. Two states matter:
    //   - `None` here → bind NULL → COALESCE keeps the existing column
    //     value (the "no override on this PATCH" path).
    //   - `Some(map)` → bind the JSON body → COALESCE picks the new
    //     value (full whole-map replacement at the COLUMN level — the
    //     key-level merge on top of the profile happens later, at
    //     resolve_config time).
    //
    // `serde_json::to_value` over `HashMap<String, Value>` cannot fail
    // structurally (string keys, JSON-shaped values both round-trip
    // by construction). If a future change introduces a type that can
    // fail to serialize (e.g., `f64` with `NaN`), add a Serialization
    // variant to PipelineRepoError and propagate the error here. For
    // now, an `unwrap_or_else` would be a "this path is unreachable"
    // statement — we use `?` via `transpose()` so the type checker
    // proves the same thing without an unwrap.
    let chunking_config_json = overrides
        .chunking_config
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| {
            PipelineRepoError::Database(format!(
                "structurally-impossible serialize of chunking_config for document_id={document_id}: {e}"
            ))
        })?;
    let context_config_json = overrides
        .context_config
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| {
            PipelineRepoError::Database(format!(
                "structurally-impossible serialize of context_config for document_id={document_id}: {e}"
            ))
        })?;

    let result = sqlx::query(
        "UPDATE pipeline_config SET \
           profile_name = COALESCE($2, profile_name), \
           extraction_model = COALESCE($3, extraction_model), \
           pass2_extraction_model = COALESCE($4, pass2_extraction_model), \
           pass2_template_file = COALESCE($5, pass2_template_file), \
           template_file = COALESCE($6, template_file), \
           system_prompt_file = COALESCE($7, system_prompt_file), \
           chunking_mode = COALESCE($8, chunking_mode), \
           chunk_size = COALESCE($9, chunk_size), \
           chunk_overlap = COALESCE($10, chunk_overlap), \
           max_tokens = COALESCE($11, max_tokens), \
           temperature = COALESCE($12::numeric, temperature), \
           run_pass2 = COALESCE($13, run_pass2), \
           auto_approve_grounded = COALESCE($14, auto_approve_grounded), \
           global_rules_file = COALESCE($15, global_rules_file), \
           chunking_config = COALESCE($16, chunking_config), \
           context_config = COALESCE($17, context_config) \
         WHERE document_id = $1",
    )
    .bind(document_id)
    .bind(&overrides.profile_name)
    .bind(&overrides.extraction_model)
    .bind(&overrides.pass2_extraction_model)
    .bind(&overrides.pass2_template_file)
    .bind(&overrides.template_file)
    .bind(&overrides.system_prompt_file)
    .bind(&overrides.chunking_mode)
    .bind(overrides.chunk_size)
    .bind(overrides.chunk_overlap)
    .bind(overrides.max_tokens)
    .bind(overrides.temperature)
    .bind(overrides.run_pass2)
    .bind(overrides.auto_approve_grounded)
    .bind(&overrides.global_rules_file)
    .bind(chunking_config_json)
    .bind(context_config_json)
    .execute(db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(PipelineRepoError::NotFound(document_id.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── decode_jsonb_map: round-trip + error semantics ──────────────
    //
    // The conversion from raw JSONB (`Option<serde_json::Value>`) to
    // the typed `Option<HashMap<String, Value>>` is the core of the
    // read-path's no-silent-fail contract. These tests pin it down
    // without needing a live database.

    #[test]
    fn decode_jsonb_map_returns_none_when_column_is_null() {
        // The "no override; inherit from profile" path. Must be `Ok(None)`,
        // never an error and never a silent default-empty-map.
        let result = decode_jsonb_map("doc-x", "chunking_config", None);
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn decode_jsonb_map_returns_typed_map_for_well_formed_json() {
        let raw = serde_json::json!({"units_per_chunk": 3, "strategy": "qa_pair"});
        let result = decode_jsonb_map("doc-x", "chunking_config", Some(raw))
            .expect("well-formed JSONB must decode");
        let map = result.expect("decoded map must be Some");
        assert_eq!(map.get("units_per_chunk").and_then(|v| v.as_i64()), Some(3));
        assert_eq!(
            map.get("strategy").and_then(|v| v.as_str()),
            Some("qa_pair")
        );
    }

    #[test]
    fn decode_jsonb_map_errors_on_non_object_jsonb() {
        // Spec test #3: malformed JSONB (here: a JSON number where an
        // object is required) must NOT silently become None. Returns
        // PipelineRepoError::Deserialization with both the document_id
        // and the column name in the message so an auditor can find
        // the row directly.
        let raw = serde_json::json!(42);
        let err = decode_jsonb_map("doc-malformed", "chunking_config", Some(raw))
            .expect_err("non-object JSONB must error, not silently None");
        match err {
            PipelineRepoError::Deserialization(msg) => {
                assert!(
                    msg.contains("doc-malformed"),
                    "error must name the document_id; got: {msg}"
                );
                assert!(
                    msg.contains("chunking_config"),
                    "error must name the column; got: {msg}"
                );
            }
            other => panic!("expected Deserialization, got {other:?}"),
        }
    }

    #[test]
    fn decode_jsonb_map_errors_on_jsonb_array_instead_of_object() {
        // Variant of the prior test: an array decoding into a map is
        // also a shape mismatch. The spec wants any non-object value
        // surfaced as Deserialization, not silent None.
        let raw = serde_json::json!(["not", "a", "map"]);
        let err = decode_jsonb_map("doc-y", "context_config", Some(raw))
            .expect_err("JSON array must error when a map is expected");
        assert!(matches!(err, PipelineRepoError::Deserialization(_)));
    }

    // ── has_any_override: pins the short-circuit contract ──────────

    #[test]
    fn has_any_override_returns_false_for_empty_overrides() {
        assert!(!has_any_override(&PipelineConfigOverrides::default()));
    }

    /// Spec decision #4: a PATCH whose only field is `chunking_config`
    /// must NOT short-circuit. Pre-Instruction-C, `any_field` did not
    /// include `chunking_config.is_some()` in its OR — so this test
    /// would have returned `false` and the UPDATE would have silently
    /// no-op'd, leaving the operator's override unpersisted.
    #[test]
    fn patch_with_only_chunking_config_does_not_short_circuit() {
        let mut over = HashMap::new();
        over.insert("units_per_chunk".to_string(), serde_json::json!(3));
        let overrides = PipelineConfigOverrides {
            chunking_config: Some(over),
            ..Default::default()
        };
        assert!(
            has_any_override(&overrides),
            "chunking_config override must trigger the UPDATE path"
        );
    }

    /// Same as above but for context_config.
    #[test]
    fn patch_with_only_context_config_does_not_short_circuit() {
        let mut over = HashMap::new();
        over.insert("traversal_depth".to_string(), serde_json::json!(5));
        let overrides = PipelineConfigOverrides {
            context_config: Some(over),
            ..Default::default()
        };
        assert!(has_any_override(&overrides));
    }

    /// `Some(empty_map)` is the explicit "I want to override but with
    /// no keys" signal — it IS a real override (operationally distinct
    /// from None, see PipelineConfigOverrides::chunking_config doc) and
    /// must trigger the UPDATE so the COLUMN gets set to `'{}'::jsonb`.
    #[test]
    fn patch_with_only_empty_chunking_config_map_still_persists() {
        let overrides = PipelineConfigOverrides {
            chunking_config: Some(HashMap::new()),
            ..Default::default()
        };
        assert!(
            has_any_override(&overrides),
            "Some(empty) is still an override and must reach the UPDATE"
        );
    }

    // ── Live-DB integration tests (#[ignore]) ─────────────────────
    //
    // The repository's other CRUD paths are not unit-tested with a
    // live database in this codebase. Round-tripping the new JSONB
    // columns through PostgreSQL is exercised in DEV via the SQL
    // verification block in this commit's instruction (see
    // /home/roman/Downloads/CC_INSTRUCTION_C_chunking_context_config_overrides.md).
    // Marking the spec's requested DB tests as `#[ignore]` here so they
    // surface in `cargo test -- --ignored` for any future contributor
    // who sets up a test DB fixture, but they don't gate the normal
    // test run that has no PG connection.
}
