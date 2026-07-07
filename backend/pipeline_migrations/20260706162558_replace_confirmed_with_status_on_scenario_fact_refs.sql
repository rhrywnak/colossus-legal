-- replace_confirmed_with_status_on_scenario_fact_refs: swap the two-state
--   `confirmed BOOLEAN` for a three-state `status TEXT`
--
-- Created: 2026-07-06 16:25:58
-- Target: pipeline database (colossus_legal_v2, applied by the runtime
--         sqlx::migrate::Migrator at backend boot — NOT the compile-time
--         migrate! macro. The file ships with the image; a backend restart
--         re-runs the Migrator and applies it. No cargo rebuild is needed for
--         the migration itself.)
--
-- WHY (Phase 1a.1 — the candidate workbench needs THREE states):
--   A human working a scenario either INCLUDES a candidate (it becomes a
--   confirmed fact), DROPS it (a scenario-scoped exclusion — the graph node is
--   untouched and still visible to other scenarios), or leaves it UNDECIDED (the
--   Theme Scan later judges only the undecided remainder). That is three
--   mutually-exclusive states, which the old `confirmed BOOLEAN` cannot encode.
--
--   `confirmed` was effectively write-only (two writers, zero behavioral readers,
--   dropped at the DTO boundary, zero frontend exposure — established by the
--   Phase-1a.1 read-and-report), so a clean replace is safe. Colossus runs a
--   SINGLE backend container and migrates at boot before serving, so the
--   add-backfill-drop in one file is atomic (no old binary lingers writing
--   `confirmed` after the column is gone).
--
-- BACKFILL MAPPING:
--   confirmed = TRUE  -> 'included'   (a human had deliberately curated it)
--   confirmed = FALSE -> 'undecided'  (a Theme Scan suggestion awaiting review)
--   'dropped' starts EMPTY — no existing row is a drop; the drop action arrives
--   in a later chunk (1a.3). The `FactStatus::Dropped` variant exists in code
--   with no producer yet, on purpose.
--
-- NOT NULL DEFAULT 'undecided' mirrors the old `confirmed NOT NULL DEFAULT FALSE`
--   shape: every row has a definite status, and the default is the neutral
--   "not yet decided" state (the `confirmed = FALSE` analogue).
--
-- NO CHECK CONSTRAINT — deliberate, matching THIS table's existing precedent for
--   `role_in_this_scenario` (its create-migration comment: the vocabulary is
--   "intentionally NOT a DB CHECK or enum … Validated in code"). The three-state
--   invariant is enforced by the `FactStatus` Rust enum, not the database, so the
--   workbench vocabulary can evolve (e.g. a future `needs_review`) without a
--   migration. This is intentionally UNLIKE the sibling `scenarios` table, whose
--   `direction`/`status` DO use CHECK — those are stable lifecycle fields;
--   `scenario_fact_refs.status` is an evolvable interaction vocabulary.
--
-- FORWARD-ONLY: the pipeline Migrator applies migrations forward only. There is
--   no down migration. A bad forward migration is corrected by a FURTHER forward
--   migration (alter/drop) — never by editing or deleting this file once applied.

ALTER TABLE scenario_fact_refs
    ADD COLUMN status TEXT NOT NULL DEFAULT 'undecided';

UPDATE scenario_fact_refs
    SET status = CASE WHEN confirmed THEN 'included' ELSE 'undecided' END;

ALTER TABLE scenario_fact_refs
    DROP COLUMN confirmed;

COMMENT ON COLUMN scenario_fact_refs.status IS
    'Workbench state of this candidate in this scenario: undecided (default; the Theme Scan judges only these) / included (a confirmed fact) / dropped (scenario-scoped exclusion). Vocabulary validated in code by the FactStatus enum, NOT a DB CHECK — evolvable like role_in_this_scenario.';
