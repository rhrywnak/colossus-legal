-- Schema cleanup: remove dead tables and columns identified by
-- POSTGRES_SCHEMA_AUDIT_2026-05-26.md
--
-- Tables dropped:
--   rag_config        — zero DML anywhere in backend/src (Category C: DEAD)
--   pipeline_events   — zero DML anywhere in backend/src (Category C: DEAD)
--   extraction_chunks — write-only, never SELECTed (Category C: DEAD)
--
-- Columns dropped from extraction_runs (11 columns, all Category C: DEAD):
--   Written at INSERT but never read by any SELECT. The quality report reads
--   its fingerprints from the processing_config JSONB blob instead
--   (report_queries.rs:147), so the dedicated columns were orphaned.
--   chunks_pruned_{nodes,relationships} were only ever reset to NULL — never
--   populated with a real value.
--
-- Forward-only (the project does not use down-migrations). This is
-- irreversible: extraction_chunks rows and the 11 columns' data are
-- permanently removed on apply. Accepted per the cleanup instruction.

-- 1. Drop tables.
--    extraction_chunks FKs to extraction_runs (ON DELETE CASCADE) — dropping
--    the child table is safe and leaves extraction_runs intact.
--    pipeline_events FKs to pipeline_jobs — pipeline_jobs is intentionally NOT
--    dropped here (it has a trigger dependency; separate instruction later).
DROP TABLE IF EXISTS rag_config;
DROP TABLE IF EXISTS pipeline_events;
DROP TABLE IF EXISTS extraction_chunks;

-- 2. Drop dead columns from extraction_runs.
--    template_name and rules_name are deliberately retained — they are NOT in
--    the dead-column set for this instruction.
ALTER TABLE extraction_runs DROP COLUMN IF EXISTS assembled_prompt;
ALTER TABLE extraction_runs DROP COLUMN IF EXISTS prior_context;
ALTER TABLE extraction_runs DROP COLUMN IF EXISTS temperature;
ALTER TABLE extraction_runs DROP COLUMN IF EXISTS max_tokens_requested;
ALTER TABLE extraction_runs DROP COLUMN IF EXISTS admin_instructions;
ALTER TABLE extraction_runs DROP COLUMN IF EXISTS template_hash;
ALTER TABLE extraction_runs DROP COLUMN IF EXISTS rules_hash;
ALTER TABLE extraction_runs DROP COLUMN IF EXISTS schema_hash;
ALTER TABLE extraction_runs DROP COLUMN IF EXISTS schema_content;
ALTER TABLE extraction_runs DROP COLUMN IF EXISTS chunks_pruned_nodes;
ALTER TABLE extraction_runs DROP COLUMN IF EXISTS chunks_pruned_relationships;
