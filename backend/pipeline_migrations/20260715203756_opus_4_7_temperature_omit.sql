-- opus_4_7_temperature_omit: mark claude-opus-4-7 as temperature-omit
--
-- Created: 2026-07-15 20:37:56
-- Target: pipeline database (colossus_legal_v2, applied by the runtime
--         sqlx::migrate::Migrator at backend boot — NOT the compile-time
--         migrate! macro. The file ships with the image; a backend restart
--         re-runs the Migrator and applies it. No cargo rebuild is needed for
--         the migration itself.)
--
-- WHY (temperature-mode fix):
--   `claude-opus-4-7` DEPRECATED the `temperature` parameter — every call that
--   sends an explicit temperature 400s ("temperature is deprecated for this
--   model"). Chunk A's migration (20260708084245) seeded `temperature_mode =
--   'zero-ok'` for ALL existing anthropic rows, so opus-4-7 is currently marked
--   zero-ok and the provider sends `temperature = 0` to it. The code fix
--   (pipeline/providers.rs → domain::llm_params::construction_temperature) now
--   HONORS this column: `omit` → send no temperature key. This migration flips
--   opus-4-7 to `omit` so the code and the row agree. Both halves are required.
--
-- ORDERING: this file's timestamp (20260715203756) is LATER than Chunk A's seed
--   (20260708084245), so the Migrator applies Chunk A's `zero-ok` seed FIRST and
--   this override SECOND — opus-4-7 ends `omit`.
--
-- SCOPE: only `claude-opus-4-7` is flipped here — the one confirmed
--   temperature-deprecated model in the registry. The extraction models
--   (claude-sonnet-4-6 / claude-opus-4-6) stay `zero-ok` (they accept temp 0), so
--   extraction's effective temperature is UNCHANGED (construction_temperature
--   resolves them to Some(0.0) via the deterministic default). If a build1 audit
--   finds OTHER newer temperature-deprecated Anthropic models, add them to this
--   file with the same guarded UPDATE before it is applied.
--
-- IDEMPOTENT: keyed on the exact id, and a no-op when the id is absent or already
--   'omit'. FORWARD-ONLY: no down migration; correct a mistake with a further
--   forward migration.

UPDATE llm_models
    SET temperature_mode = 'omit'
    WHERE id = 'claude-opus-4-7'
      AND temperature_mode IS DISTINCT FROM 'omit';
