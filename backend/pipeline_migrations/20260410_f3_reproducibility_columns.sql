-- F3: Extraction reproducibility columns.
-- Stores everything needed to reproduce an extraction run:
-- the assembled prompt, template/rules hashes, schema snapshot,
-- and model parameters.
ALTER TABLE extraction_runs ADD COLUMN IF NOT EXISTS assembled_prompt TEXT;
ALTER TABLE extraction_runs ADD COLUMN IF NOT EXISTS template_name TEXT;
ALTER TABLE extraction_runs ADD COLUMN IF NOT EXISTS template_hash TEXT;
ALTER TABLE extraction_runs ADD COLUMN IF NOT EXISTS rules_name TEXT;
ALTER TABLE extraction_runs ADD COLUMN IF NOT EXISTS rules_hash TEXT;
ALTER TABLE extraction_runs ADD COLUMN IF NOT EXISTS schema_hash TEXT;
ALTER TABLE extraction_runs ADD COLUMN IF NOT EXISTS schema_content JSONB;
ALTER TABLE extraction_runs ADD COLUMN IF NOT EXISTS temperature DOUBLE PRECISION;
ALTER TABLE extraction_runs ADD COLUMN IF NOT EXISTS max_tokens_requested INTEGER;
ALTER TABLE extraction_runs ADD COLUMN IF NOT EXISTS admin_instructions TEXT;
ALTER TABLE extraction_runs ADD COLUMN IF NOT EXISTS prior_context TEXT;
