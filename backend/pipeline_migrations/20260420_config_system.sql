-- Configuration system tables and columns.
-- Phase 1 of DOC_PROCESSING_CONFIG_DESIGN_v2.md.
--
-- Creates llm_models table (model registry with provider, endpoint, costs).
-- Extends pipeline_config with per-document override columns.
-- Extends extraction_runs with processing_config JSONB snapshot.
--
-- All changes are additive — no existing data is modified or deleted.
-- All new columns on existing tables are nullable.

-- ── Model registry ──────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS llm_models (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    provider TEXT NOT NULL,
    api_endpoint TEXT,
    max_context_tokens INTEGER,
    max_output_tokens INTEGER,
    cost_per_input_token NUMERIC(12,8),
    cost_per_output_token NUMERIC(12,8),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    notes TEXT
);

-- Seed models
INSERT INTO llm_models (id, display_name, provider, api_endpoint,
    max_context_tokens, max_output_tokens,
    cost_per_input_token, cost_per_output_token,
    is_active, created_at, notes)
VALUES
    ('claude-sonnet-4-6', 'Claude Sonnet 4.6', 'anthropic', NULL,
     200000, 64000, 0.000003, 0.000015,
     true, NOW(), 'Anthropic cloud — primary extraction model'),
    ('claude-opus-4-6', 'Claude Opus 4.6', 'anthropic', NULL,
     200000, 64000, 0.000015, 0.000075,
     true, NOW(), 'Anthropic cloud — synthesis/curation model')
ON CONFLICT (id) DO NOTHING;

-- ── Pipeline config: per-document override columns ──────────────

ALTER TABLE pipeline_config
    ADD COLUMN IF NOT EXISTS profile_name TEXT,
    ADD COLUMN IF NOT EXISTS template_file TEXT,
    ADD COLUMN IF NOT EXISTS system_prompt_file TEXT,
    ADD COLUMN IF NOT EXISTS chunking_mode TEXT,
    ADD COLUMN IF NOT EXISTS chunk_size INTEGER,
    ADD COLUMN IF NOT EXISTS chunk_overlap INTEGER,
    ADD COLUMN IF NOT EXISTS temperature NUMERIC(3,2),
    ADD COLUMN IF NOT EXISTS run_pass2 BOOLEAN;

-- ── Extraction runs: processing config snapshot ─────────────────

ALTER TABLE extraction_runs
    ADD COLUMN IF NOT EXISTS processing_config JSONB;
