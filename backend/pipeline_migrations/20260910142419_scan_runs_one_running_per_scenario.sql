-- scan_runs_one_running_per_scenario: One running scan per scenario
--
-- Created: 2026-09-10 14:24:19
-- Target: pipeline database (colossus_legal_v2)
-- Task: CC_TASK_SCAN_SERVER_STATE_v1, part A1
--
-- Make "at most one `running` scan per scenario" a fact the DATABASE holds,
-- not a hope the UI carries.
--
-- ## Why the database and not the handler
--
-- The refusal that answers a second Run with a 409 lives in
-- `services::theme_scan_start::start_theme_scan` — it reads for a `running` row
-- before it writes the stub. That check is a read followed by a write, so two
-- POSTs that arrive inside the same few milliseconds can both read "none
-- running" and both promote. The window is small and the cost is not: two
-- background tasks judging the same pool spend twice the budget, write two sets
-- of verdicts, and race each other's progress bumps on a row neither owns.
--
-- A partial unique index closes the window without a lock the application has to
-- remember to take. The second promote raises 23505, and
-- `promote_scan_run_running` maps that ONE constraint name to the same 409 the
-- pre-check returns (never a 500) — so the fast path stays a clean refusal with
-- a run_id, and the racing path lands on the same answer.
--
-- ## The sweep has to come first, and what it is copied from
--
-- A partial unique index cannot be built over a scenario that already holds two
-- `running` rows: `CREATE UNIQUE INDEX` validates the existing rows and fails.
-- Any such row is an ORPHAN by definition — the `tokio` task that owned it did
-- not survive whatever left the row behind — and the codebase already has one
-- authoritative statement of what to do with it:
-- `repositories::pipeline_repository::scan_runs::sweep_running_scan_runs`, the
-- startup guard. This UPDATE is that statement, verbatim: the same three columns,
-- the same `status = 'failed'`, and the same `error` literal
-- ('interrupted by restart', the `INTERRUPTED_BY_RESTART` const in that module).
-- Copied rather than paraphrased so a reader comparing the two sees one rule.
--
-- ## What DEV actually held when this was written (measured, not assumed)
--
-- Read on 2026-09-10 against `colossus_legal_v2` on 10.10.100.200:
--
--     status    | count            scenario_id | running_rows
--   ------------+-------          -------------+--------------
--    completed  |     9            (0 rows)
--    failed     |     4
--
-- ZERO `running` rows, not three. The sweep below therefore updates nothing on
-- DEV today, and that is a legitimate outcome rather than a failure — which is
-- exactly why the assertion at the foot of this file asserts the END STATE (no
-- `running` rows remain, and the index exists) rather than a swept row count.
-- A migration that demanded "N rows changed" would fail on the very database it
-- was written for. PROD, or any environment killed mid-scan, is where the sweep
-- has work to do.
--
-- ## `cancelled` needs no schema change
--
-- Part C introduces a fourth status token. `scan_runs.status` carries NO CHECK
-- constraint (verified 2026-09-10: `pg_constraint` on the table lists only the
-- primary key and the `scenario_id` foreign key), and the vocabulary is owned by
-- the `SCAN_STATUS_*` consts in Rust. So the new value is code, not DDL. The
-- index below is deliberately keyed on `'running'` alone: a `cancelled` run is
-- terminal and any number of them may sit under one scenario.
--
-- ## Reproducible from a fresh database
--
-- Both statements are idempotent-safe: the UPDATE matches nothing on a database
-- with no orphans, and the index carries IF NOT EXISTS. Re-running this file
-- changes nothing and still passes its assertion.

-- 1. Sweep the orphans. The same values `sweep_running_scan_runs` writes at boot.
UPDATE scan_runs
   SET status           = 'failed',
       error            = 'interrupted by restart',
       last_progress_at = NOW()
 WHERE status = 'running';

-- 2. The invariant itself. PARTIAL (the WHERE clause) so it constrains only the
--    live state: a scenario may hold any number of completed, failed or
--    cancelled runs, and at most one running one.
CREATE UNIQUE INDEX IF NOT EXISTS scan_runs_one_running_per_scenario
    ON scan_runs (scenario_id)
    WHERE status = 'running';

COMMENT ON INDEX scan_runs_one_running_per_scenario IS
    'At most one running scan per scenario. The backstop behind the 409 refusal in services::theme_scan_start; a violation surfaces as 23505 on the promote UPDATE and is mapped to that same 409, never a 500.';

-- The END-state assertion (CLAUDE.md 25a). Asserted rather than trusted: an
-- UPDATE matching zero rows is silent in Postgres, so a sweep that failed to
-- clear an orphan would leave the index build as the only thing that noticed —
-- and `IF NOT EXISTS` on a pre-existing index would swallow that too. These two
-- checks together are what makes "this file ran and the invariant now holds"
-- observable rather than assumed.
DO $$
DECLARE
    still_running INTEGER;
    have_index    INTEGER;
BEGIN
    SELECT count(*) INTO still_running FROM scan_runs WHERE status = 'running';

    IF still_running <> 0 THEN
        RAISE EXCEPTION
            'scan_runs_one_running_per_scenario: expected 0 running rows after the '
            'orphan sweep, found %. The UPDATE above did not take effect — do NOT '
            'assume the unique index is protecting anything.', still_running;
    END IF;

    SELECT count(*) INTO have_index
      FROM pg_indexes
     WHERE schemaname = 'public'
       AND tablename  = 'scan_runs'
       AND indexname  = 'scan_runs_one_running_per_scenario';

    IF have_index <> 1 THEN
        RAISE EXCEPTION
            'scan_runs_one_running_per_scenario: the partial unique index is absent '
            'after this migration (found % matching index rows). The one-running-'
            'per-scenario invariant has no backstop.', have_index;
    END IF;

    RAISE NOTICE
        'scan_runs_one_running_per_scenario: 0 running rows, partial unique index in place';
END $$;
