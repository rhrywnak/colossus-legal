-- scan_eligible_column_and_model_refresh: durable model set + scan-picker filter
--
-- Created: 2026-07-15 23:04:27
-- Target: pipeline database (colossus_legal_v2, applied by the runtime
--         sqlx::migrate::Migrator at backend boot — NOT the compile-time
--         migrate! macro. The file ships with the image; a backend restart
--         re-runs the Migrator and applies it. No cargo rebuild is needed for
--         the migration itself.)
--
-- WHY (registry refresh + scan-eligible filter, ruling A):
--   The scan/benchmark model picker should show only the CURRENT model set, but
--   the old Claude rows (opus-4-6 / sonnet-4-6) must stay `is_active = true`
--   because the extraction profiles resolve them via get_active_model_by_id
--   (is_active only) — deactivating them would break extraction. So we add a
--   SEPARATE `scan_eligible` flag: the scan picker filters on it (via the new
--   list_scan_eligible_models query), while extraction / the chat provider map /
--   model-id validation keep using list_active_models (is_active only), unchanged.
--
-- NO CHECK CONSTRAINT — the boolean is self-describing; semantics are owned by the
--   Rust query, matching this table's convention.
--
-- FORWARD-ONLY: no down migration; correct a mistake with a further forward
--   migration. Every statement is idempotent (guarded / ON CONFLICT).

-- 1. The scan-eligible flag. DEFAULT true so EVERY existing row (and any future
--    row that forgets to set it) is scan-visible by default; we then turn OFF the
--    retired models below. `IF NOT EXISTS` keeps a re-run safe.
ALTER TABLE llm_models
    ADD COLUMN IF NOT EXISTS scan_eligible BOOLEAN NOT NULL DEFAULT true;

COMMENT ON COLUMN llm_models.scan_eligible IS
    'Show this model in the scan/benchmark picker (list_scan_eligible_models). Distinct from is_active: a model can be extraction-active (is_active=true, used by profiles via get_active_model_by_id) yet scan-hidden (scan_eligible=false) — e.g. retired Claude rows kept for extraction. NOT NULL DEFAULT true.';

-- 2. New current-generation Claude rows.
--    temperature_mode='omit' (these deprecate the temperature parameter, like
--    opus-4-7); structured_output_mode='native' (Anthropic tool-use).
--    PLACEHOLDER cost/context: copied verbatim from the SAME-TIER existing row via
--    SELECT (no invented pricing — Roman corrects the real numbers when known).
--    ON CONFLICT DO NOTHING so a pre-existing (operator-corrected) row is never
--    clobbered. If the same-tier source row is absent the SELECT yields no row and
--    the model is simply not created (flagged — add it explicitly then).

-- claude-opus-4-8  <-  cost/context copied from claude-opus-4-6 (PLACEHOLDER)
INSERT INTO llm_models (
    id, display_name, provider,
    max_context_tokens, max_output_tokens, cost_per_input_token, cost_per_output_token,
    is_active, scan_eligible, temperature_mode, structured_output_mode)
SELECT
    'claude-opus-4-8', 'Claude Opus 4.8', 'anthropic',
    max_context_tokens, max_output_tokens, cost_per_input_token, cost_per_output_token,
    true, true, 'omit', 'native'
FROM llm_models WHERE id = 'claude-opus-4-6'
ON CONFLICT (id) DO NOTHING;

-- claude-sonnet-5  <-  cost/context copied from claude-sonnet-4-6 (PLACEHOLDER)
INSERT INTO llm_models (
    id, display_name, provider,
    max_context_tokens, max_output_tokens, cost_per_input_token, cost_per_output_token,
    is_active, scan_eligible, temperature_mode, structured_output_mode)
SELECT
    'claude-sonnet-5', 'Claude Sonnet 5', 'anthropic',
    max_context_tokens, max_output_tokens, cost_per_input_token, cost_per_output_token,
    true, true, 'omit', 'native'
FROM llm_models WHERE id = 'claude-sonnet-4-6'
ON CONFLICT (id) DO NOTHING;

-- 3. Ensure claude-opus-4-7 is active + scan-visible. Its temperature_mode='omit'
--    is already set by migration 20260715203756 (runs earlier). This only asserts
--    the activation/visibility flags (no-op if the row is absent — flagged).
UPDATE llm_models
    SET is_active = true, scan_eligible = true
    WHERE id = 'claude-opus-4-7';

-- 4. The two Qwen vLLM rows, durable. ON CONFLICT DO NOTHING so this NEVER
--    clobbers the existing DEV inserts (DEV already carries this api_endpoint).
--    api_endpoint = the vLLM base URL 'http://10.10.0.99:8001' — the BASE with NO
--    /v1 suffix: VllmProvider::new REJECTS a /v1 suffix and appends
--    /v1/chat/completions itself, and the hard gate appends /v1/models — so this
--    ONE column correctly feeds BOTH. An environment whose vLLM lives elsewhere
--    must correct this endpoint (until then the /v1/models gate fails loudly at
--    scan time — never a silent wrong call).
--    zero-ok temperature; guided structured output; per-model timeouts + concurrency.
INSERT INTO llm_models (
    id, display_name, provider, api_endpoint,
    max_context_tokens, is_active, scan_eligible,
    temperature_mode, structured_output_mode, timeout_secs, max_concurrency)
VALUES
    ('Qwen/Qwen2.5-14B-Instruct-AWQ', 'Qwen2.5 14B (AWQ, local)', 'vllm',
     'http://10.10.0.99:8001',
     8192, true, true, 'zero-ok', 'guided', 120, 4)
ON CONFLICT (id) DO NOTHING;

INSERT INTO llm_models (
    id, display_name, provider, api_endpoint,
    max_context_tokens, is_active, scan_eligible,
    temperature_mode, structured_output_mode, timeout_secs, max_concurrency)
VALUES
    ('Qwen/Qwen2.5-32B-Instruct-AWQ', 'Qwen2.5 32B (AWQ, local)', 'vllm',
     'http://10.10.0.99:8001',
     8192, true, true, 'zero-ok', 'guided', 180, 2)
ON CONFLICT (id) DO NOTHING;

-- 5. Retire the old Claude rows FROM THE PICKER only — keep is_active=true so the
--    extraction profiles that resolve them via get_active_model_by_id keep working.
UPDATE llm_models
    SET scan_eligible = false
    WHERE id IN ('claude-opus-4-6', 'claude-sonnet-4-6');
