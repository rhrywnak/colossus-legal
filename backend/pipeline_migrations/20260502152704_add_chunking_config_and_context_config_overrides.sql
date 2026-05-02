-- add_chunking_config_and_context_config_overrides: Add chunking_config and context_config overrides
--
-- Created: 2026-05-02 15:27:04
-- Target: pipeline database
-- Refs: AUDIT_PIPELINE_CONFIG_GAPS.md gap 1
--
-- Why
-- ---
-- Profiles carry `chunking_config` and `context_config` as flexible
-- HashMap<String, Value> bags (e.g., `{strategy: section_heading,
-- units_per_chunk: 5, ...}`). Yesterday's regression showed why a
-- per-document override path is required: changing the profile YAML
-- affects every future document of that type, with no escape hatch
-- and no audit log of operator deviations.
--
-- Both columns are JSONB and nullable. NULL is the explicit
-- "no override; inherit from the profile at resolve time" sentinel —
-- matching every other override column on this table (extraction_model,
-- chunking_mode, etc., all of which use NULL the same way). The
-- application's resolve_config merges override KEYS onto profile keys
-- so an operator can override a single sub-key (e.g., units_per_chunk)
-- without restating the rest of the map.
--
-- Empty-map override (`'{}'::jsonb`) is operationally distinct from
-- NULL — it means "this document has chunking_config but every key is
-- unset," and the operator must opt in explicitly to reach that state.
-- Hence no DEFAULT '{}' on the column; NULL preserves the three-state
-- contract documented in PipelineConfigOverrides.
--
-- Rollback
-- --------
-- ALTER TABLE pipeline_config
--     DROP COLUMN chunking_config,
--     DROP COLUMN context_config;
--
-- IF NOT EXISTS keeps this migration idempotent across re-runs.

ALTER TABLE pipeline_config
    ADD COLUMN IF NOT EXISTS chunking_config JSONB,
    ADD COLUMN IF NOT EXISTS context_config JSONB;
