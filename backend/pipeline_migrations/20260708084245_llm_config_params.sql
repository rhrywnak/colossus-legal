-- llm_config_params: add LLM parameter-resolution columns to llm_models
--
-- Created: 2026-07-08 08:42:45
-- Target: pipeline database (colossus_legal_v2, applied by the runtime
--         sqlx::migrate::Migrator at backend boot — NOT the compile-time
--         migrate! macro. The file ships with the image; a backend restart
--         re-runs the Migrator and applies it. No cargo rebuild is needed for
--         the migration itself.)
--
-- WHY (LLM Configuration Method, Chunk A — the model-default layer of the
--   three-layer parameter resolver needs a place to live):
--   The resolver (backend/src/domain/llm_params.rs) resolves temperature /
--   timeout / max_tokens across three layers (user > task > model-default) and
--   then constrains the result against per-model capabilities (temperature
--   omit-vs-zero, output-token ceiling, structured-output support). Those
--   per-model capabilities and the model-default floor are properties OF a
--   model, so they belong on `llm_models` — one row per model, edited at
--   runtime like the existing cost columns.
--
-- All changes are additive — no existing data is deleted. Every new column is
--   nullable. A NULL means "this layer is silent" to the resolver, EXCEPT
--   structured_output_mode where NULL means "unknown capability" (see below and
--   the constraint pass in llm_params.rs). Empty/NULL vs a set value are
--   DISTINGUISHABLE (Standing Rule 1) — the resolver branches on the difference.
--
-- FORWARD-ONLY: the pipeline Migrator applies migrations forward only. There is
--   no down migration. A bad forward migration is corrected by a FURTHER forward
--   migration — never by editing or deleting this file once applied.
--
-- NO CHECK CONSTRAINT on temperature_mode or structured_output_mode — deliberate,
--   matching the scenario-table precedent (role_in_this_scenario, fact status):
--   the vocabulary is OWNED BY the Rust constraint pass (the TemperatureMode /
--   StructuredOutputMode enums in domain/llm_params.rs), NOT by the database. A
--   DB CHECK here would double-own the vocabulary and force a migration every
--   time a mode is added. The code layer is the single owner.

ALTER TABLE llm_models ADD COLUMN IF NOT EXISTS default_temperature    NUMERIC(3,2);
ALTER TABLE llm_models ADD COLUMN IF NOT EXISTS temperature_mode       TEXT;
ALTER TABLE llm_models ADD COLUMN IF NOT EXISTS timeout_secs           INTEGER;
ALTER TABLE llm_models ADD COLUMN IF NOT EXISTS structured_output_mode TEXT;
ALTER TABLE llm_models ADD COLUMN IF NOT EXISTS max_concurrency        INTEGER;

-- Seed capability modes for existing Anthropic rows. Guarded WHERE ... IS NULL
-- so a re-run is idempotent (only unset rows are touched — a value an operator
-- later set by hand is never clobbered).
--
-- temperature_mode = 'zero-ok': existing Anthropic chat models accept an
--   explicit temperature of 0.0. We do NOT hardcode a case-specific omit-required
--   model name here (reusability — another Colossus deployment has different
--   models). Omit-required models (e.g. reasoning models that reject an explicit
--   temperature) are marked 'omit' per-row by the operator via the admin-write
--   chunk (Chunk B), not by this migration.
-- NOTE: omit-required models are marked per-row by the operator later; this
--   migration only establishes the safe 'zero-ok' default for existing rows.
UPDATE llm_models
    SET temperature_mode = 'zero-ok'
    WHERE temperature_mode IS NULL
      AND provider = 'anthropic';

-- structured_output_mode = 'native': Anthropic tool-use gives native structured
--   output. Non-anthropic (e.g. vllm) rows are LEFT NULL on purpose — a local
--   model's structured-output capability is set when that model is actually
--   onboarded. The constraint pass treats a NULL here as 'unknown' (distinct
--   from a known 'none'), so an un-onboarded model can't silently be assumed to
--   support structured output.
UPDATE llm_models
    SET structured_output_mode = 'native'
    WHERE structured_output_mode IS NULL
      AND provider = 'anthropic';

-- timeout_secs, default_temperature, max_concurrency: intentionally left NULL for
--   existing rows. The resolver's model-default layer treats NULL as "this layer
--   silent" and falls through to the task/user layers or the documented system
--   default. No backfill needed.

COMMENT ON COLUMN llm_models.default_temperature IS
    'Model-default temperature (the model-default layer of the three-layer resolver). NULL = layer silent. Decoded via ::float8 like the cost columns.';
COMMENT ON COLUMN llm_models.temperature_mode IS
    'Per-model temperature capability: zero-ok (accepts explicit 0.0) / omit (temperature must be omitted entirely). Vocabulary owned by the TemperatureMode enum in domain/llm_params.rs, NOT a DB CHECK. NULL = unknown.';
COMMENT ON COLUMN llm_models.timeout_secs IS
    'Model-default HTTP timeout in seconds (model-default layer). NULL = layer silent; the resolver falls back to its documented system default.';
COMMENT ON COLUMN llm_models.structured_output_mode IS
    'Per-model structured-output capability: native / guided / none. Vocabulary owned by the StructuredOutputMode enum in domain/llm_params.rs, NOT a DB CHECK. NULL = unknown (distinct from a known none) — an un-onboarded model is not assumed capable.';
COMMENT ON COLUMN llm_models.max_concurrency IS
    'Model-default max concurrent in-flight requests (advisory; wired by a later chunk). NULL = unset.';
