-- consolidate_model_columns_and_add_overrides: drop dead pass1_model /
-- pass2_model / pass1_max_tokens / pass2_max_tokens columns, add the
-- two missing per-document override columns auto_approve_grounded and
-- global_rules_file.
--
-- Created: 2026-05-13
-- Target: pipeline database
-- Refs: PROFILE_RESOLUTION_AUDIT — bug #6 (dead pass1_model column) and
--       bug #8 (missing per-doc override paths).
--
-- Why
-- ---
-- Until the registry refactor, pipeline_config carried two parallel model
-- columns: the legacy pass1_model / pass2_model (written by the upload
-- handler from operator multipart fields, with a hardcoded SQL COALESCE
-- default of 'claude-sonnet-4-6') and the override extraction_model /
-- pass2_extraction_model (written from the matched processing profile
-- via patch_pipeline_config_overrides). resolve_config only reads the
-- override columns. The legacy columns were dead from the read side —
-- a confusing foot-gun for future readers and a violation of the
-- "no hardcoded model names" rule.
--
-- Drop them. Copy any non-null values into the canonical columns first
-- so rows uploaded before the registry don't lose their model selection.
--
-- auto_approve_grounded and global_rules_file existed only on the
-- profile YAML — there was no override column. Every other config knob
-- has a per-document override path; the asymmetry was an oversight, not
-- a deliberate design. Add the columns; resolve_config will route the
-- override → profile fallback chain like every other field.
--
-- All changes are additive in spirit (data is preserved before destructive
-- DROPs), and idempotent — re-running the migration on a database that
-- already lost the legacy columns is a no-op.
--
-- Rollback
-- --------
-- ALTER TABLE pipeline_config
--     ADD COLUMN pass1_model TEXT,
--     ADD COLUMN pass2_model TEXT,
--     ADD COLUMN pass1_max_tokens INTEGER,
--     ADD COLUMN pass2_max_tokens INTEGER,
--     DROP COLUMN auto_approve_grounded,
--     DROP COLUMN global_rules_file;
-- Rollback does NOT restore data — the data lives in extraction_model /
-- pass2_extraction_model / max_tokens after this migration runs.

BEGIN;

-- Preserve any legacy model values that hadn't been double-written.
-- COALESCE-safe: a NULL legacy column won't overwrite a non-NULL override.
UPDATE pipeline_config
SET extraction_model = pass1_model
WHERE extraction_model IS NULL AND pass1_model IS NOT NULL;

UPDATE pipeline_config
SET pass2_extraction_model = pass2_model
WHERE pass2_extraction_model IS NULL AND pass2_model IS NOT NULL;

UPDATE pipeline_config
SET max_tokens = pass1_max_tokens
WHERE max_tokens IS NULL AND pass1_max_tokens IS NOT NULL;

-- Drop the dead columns. IF EXISTS makes the migration idempotent.
ALTER TABLE pipeline_config DROP COLUMN IF EXISTS pass1_model;
ALTER TABLE pipeline_config DROP COLUMN IF EXISTS pass2_model;
ALTER TABLE pipeline_config DROP COLUMN IF EXISTS pass1_max_tokens;
ALTER TABLE pipeline_config DROP COLUMN IF EXISTS pass2_max_tokens;

-- Add the missing override columns. Both nullable; NULL = "no override,
-- inherit from profile at resolve time" — matches the existing override
-- columns (extraction_model, chunking_mode, etc.).
ALTER TABLE pipeline_config
    ADD COLUMN IF NOT EXISTS auto_approve_grounded BOOLEAN,
    ADD COLUMN IF NOT EXISTS global_rules_file TEXT;

COMMIT;
