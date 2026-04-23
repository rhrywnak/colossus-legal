-- add_pass2_extraction_model_column: Add pass2 extraction model column
--
-- Created: 2026-04-22 21:48:42
-- Target: pipeline database
--
-- Why
-- ---
-- Pass 2 (relationship extraction) reasons over the pass-1 entity list and
-- often benefits from a different model than pass 1 (e.g., a larger thinking
-- model for relationship inference while pass 1 runs on a faster / cheaper
-- one for entity parsing). Prior to this column, pass 2 was hardwired to the
-- pass-1 model with no per-document operator control.
--
-- This column joins the existing `pipeline_config` nullable-override pattern
-- (see migration 20260420_config_system.sql). `NULL` means "use the profile
-- default, or fall back to `extraction_model`."

ALTER TABLE pipeline_config
    ADD COLUMN IF NOT EXISTS pass2_extraction_model TEXT NULL;
