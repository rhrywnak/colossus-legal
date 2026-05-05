-- add_pass2_template_file_and_schema_override: Add per-document Pass 2 template override
--
-- Created: 2026-05-05 13:46:18
-- Target: pipeline database
--
-- Why
-- ---
-- Today, schema_file is NOT NULL on pipeline_config and is set at upload
-- time from the profile. There's no way to override it per document.
-- pass2_template_file doesn't exist as a column at all.
--
-- Both gaps prevent v5 schema/template selection from being expressed
-- on the document-config row, forcing the resolve_config() function
-- and the UI to fall back to profile-matched-by-document_type, which
-- breaks v4/v5 co-existence (both have document_type='complaint').
--
-- This migration adds:
--   - pass2_template_file: optional override for the Pass 2 template
--   - (Note: schema_file already exists as NOT NULL; we don't add a
--     parallel override column. Instead, WI-FIX-2 will relax
--     resolve_config() to honor the existing pipeline_config.schema_file
--     value as the override. The row is already populated correctly
--     today by the upload handler.)

ALTER TABLE pipeline_config
    ADD COLUMN IF NOT EXISTS pass2_template_file TEXT NULL;

COMMENT ON COLUMN pipeline_config.pass2_template_file IS
    'Per-document override for the Pass 2 (synthesis) template filename. NULL means use the profile default. Mirrors the pass2_extraction_model override pattern.';
