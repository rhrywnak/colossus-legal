-- add_confidence_to_scenario_fact_refs: add a per-fact model-confidence column
--
-- Created: 2026-07-06 11:27:45
-- Target: pipeline database (colossus_legal_v2, applied by the runtime
--         sqlx::migrate::Migrator at backend boot — NOT the compile-time
--         migrate! macro. The file ships with the image; a backend restart
--         re-runs the Migrator and applies it. No cargo rebuild is needed for
--         the migration itself.)
--
-- WHY (D2a substrate for the Theme Scan feature, D2b):
--   The Theme Scan asks the model, per candidate quote, to propose a role and a
--   confidence in [0.0, 1.0]. `scenario_fact_refs` already carries the role
--   (`role_in_this_scenario`) and the model's reason (`note`), and can hold a
--   suggestion via `confirmed = FALSE` — but it has NO column for the numeric
--   confidence. This migration adds it.
--
--   NULLABLE on purpose: confidence is written ONLY by the Theme Scan (D2b).
--   Human-curated refs (the existing `.../scenarios/:id/facts` route) and every
--   pre-scan row leave it NULL. Nullable means existing rows and the existing
--   facts route are unaffected — no backfill, no default, no behavior change.
--
--   REAL (single-precision float, 4 bytes) not DOUBLE PRECISION: the model emits
--   ~2-decimal confidence; REAL's ~7 significant digits is ample and the column
--   is half the width. Deliberate width choice.
--
-- FORWARD-ONLY: the pipeline Migrator applies migrations forward only. There is
--   no down migration. A bad forward migration is corrected by a FURTHER forward
--   migration (alter/drop) — never by editing or deleting this file once applied.

ALTER TABLE scenario_fact_refs
    ADD COLUMN confidence REAL;

COMMENT ON COLUMN scenario_fact_refs.confidence IS
    'Theme Scan model confidence [0.0,1.0] for the proposed role. NULL for human-added refs (facts route) and pre-scan rows.';
