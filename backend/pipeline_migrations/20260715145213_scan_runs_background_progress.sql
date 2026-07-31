-- scan_runs_background_progress: make the Theme Scan a background job
--
-- Created: 2026-07-15 14:52:13
-- Target: pipeline database (colossus_legal_v2, applied by the runtime
--         sqlx::migrate::Migrator at backend boot — NOT the compile-time
--         migrate! macro. The file ships with the image; a backend restart
--         re-runs the Migrator and applies it. No cargo rebuild is needed for
--         the migration itself.)
--
-- WHY (Theme Scan background-job chunk):
--   Chunk B's scan was synchronous — the POST blocked until all ~94 candidates
--   were judged, then INSERTed one `scan_runs` row at the end. A ~94-candidate
--   fan-out takes minutes, which the browser → Traefik → Authentik path times
--   out. This chunk makes the scan a background `tokio` task: the POST inserts a
--   `running` row and returns a run_id immediately; the task updates the row as
--   it judges; a GET polls it. These columns are the progress surface, modelled
--   on the document-processing `chunks_processed`/`percent_complete` pattern.
--
-- COLUMN ROLES:
--   status          — 'running' | 'completed' | 'failed'. Vocabulary owned by the
--                     Rust code (like scenario_fact_refs.status), NOT a DB CHECK,
--                     so it can evolve without a migration.
--   candidates_total — the progress DENOMINATOR, set once at INSERT (known up
--                     front from the candidate-pool read). Immutable for the run.
--                     Deliberately distinct from the Chunk B `candidates_read`
--                     (a completion tally): total is the live denominator, read is
--                     the final count; they match by construction. (Redundancy
--                     noted for a future cleanup — candidates_read is in the
--                     summary contract, so it stays for now.)
--   candidates_judged — the progress NUMERATOR, bumped +1 per judged candidate.
--   error           — the failure reason when status='failed' (NULL otherwise).
--                     Standing Rule 1: a failed run says why.
--   summary_json    — the finished ThemeScanSummary, serialized at completion, so
--                     the GET renders it without re-querying Neo4j on every poll.
--                     A render convenience; scan_run_verdicts stays the source of
--                     truth the agreement query joins on. Mirrors resolved_params.
--   last_progress_at — bumped on every progress write, so the UI can show a
--                     "stalled" hint if it stops advancing. The authoritative
--                     orphan guard is the STARTUP SWEEP (running -> failed on
--                     boot), not a no-progress timer.
--
-- BACKFILL: every PRE-EXISTING scan_runs row was written by the OLD synchronous
--   path, which INSERTs only at the END of a completed scan — so those rows are
--   'completed'. `status` defaults to 'completed' for that reason (the new code
--   INSERTs status='running' EXPLICITLY, overriding the default). candidates_total
--   / candidates_judged backfill from candidates_read (a finished run judged all
--   it read). This keeps the startup sweep from ever flipping a genuinely-finished
--   historical row to 'failed'.
--
-- FORWARD-ONLY: no down migration. A bad forward migration is corrected by a
--   FURTHER forward migration. All changes are additive.

ALTER TABLE scan_runs ADD COLUMN IF NOT EXISTS status            TEXT NOT NULL DEFAULT 'completed';
ALTER TABLE scan_runs ADD COLUMN IF NOT EXISTS candidates_total  INTEGER;
ALTER TABLE scan_runs ADD COLUMN IF NOT EXISTS candidates_judged INTEGER NOT NULL DEFAULT 0;
ALTER TABLE scan_runs ADD COLUMN IF NOT EXISTS error             TEXT;
ALTER TABLE scan_runs ADD COLUMN IF NOT EXISTS summary_json      JSONB;
ALTER TABLE scan_runs ADD COLUMN IF NOT EXISTS last_progress_at  TIMESTAMPTZ;

-- Backfill existing (completed, synchronous-era) rows so the denominator/numerator
-- reflect a finished run. Guarded so a re-run only touches unset rows.
UPDATE scan_runs
    SET candidates_total = candidates_read
    WHERE candidates_total IS NULL;
UPDATE scan_runs
    SET candidates_judged = candidates_read
    WHERE candidates_judged = 0 AND candidates_read > 0 AND status = 'completed';
UPDATE scan_runs
    SET last_progress_at = started_at
    WHERE last_progress_at IS NULL;

-- Index the sweep predicate: the GET reads one run by PK (already covered); the
-- startup sweep scans WHERE status='running' (rare, small) — a partial index
-- keeps that cheap without indexing the common 'completed' rows.
CREATE INDEX IF NOT EXISTS scan_runs_running_idx ON scan_runs (status) WHERE status = 'running';

COMMENT ON COLUMN scan_runs.status IS
    'running | completed | failed. Vocabulary owned by Rust code, not a DB CHECK. A running row at backend startup is orphaned and swept to failed.';
COMMENT ON COLUMN scan_runs.candidates_total IS
    'Progress denominator, set once at INSERT from the candidate-pool read. Matches candidates_read at completion (redundancy noted for future cleanup).';
COMMENT ON COLUMN scan_runs.candidates_judged IS
    'Progress numerator, bumped +1 per judged candidate during the background fan-out.';
COMMENT ON COLUMN scan_runs.error IS
    'Failure reason when status=failed (e.g. "interrupted by restart"); NULL otherwise (Standing Rule 1).';
COMMENT ON COLUMN scan_runs.summary_json IS
    'The finished ThemeScanSummary serialized at completion — a render convenience for the GET so polling does not re-query Neo4j. scan_run_verdicts remains the agreement-query source of truth.';
COMMENT ON COLUMN scan_runs.last_progress_at IS
    'Bumped on every progress write; lets the UI hint stalled. The authoritative orphan guard is the startup sweep, not a timer.';
