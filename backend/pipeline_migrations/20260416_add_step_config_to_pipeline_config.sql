-- Adds the step_config column to pipeline_config.
--
-- step_config is a per-document JSONB blob that lets pipeline steps read
-- their own configuration without needing a separate table per step.
--
-- Schema (informal — validated by the step implementations, not by PG):
-- {
--   "LlmExtract": {
--     "retry_limit": 3,
--     "retry_delay_secs": 60,
--     "timeout_secs": 900,
--     "llm_provider": "anthropic",        -- optional, overrides global LLM_PROVIDER
--     "llm_model": "claude-sonnet-4-6",   -- optional, overrides global LLM_MODEL
--     "max_tokens": 32000
--   },
--   "ExtractText": {
--     "ocr_char_threshold": 50,
--     "ocr_dpi": 300,
--     "ocr_lang": "eng",
--     "ocr_oem": 1
--   }
-- }
--
-- Per v5_2 Part 3.3.

ALTER TABLE pipeline_config
    ADD COLUMN IF NOT EXISTS step_config JSONB NOT NULL DEFAULT '{}'::jsonb;
