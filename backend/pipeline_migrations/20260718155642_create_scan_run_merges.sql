-- create_scan_run_merges: the scan-run → scenario merge audit table
--
-- Created: 2026-07-18 15:56:42
-- Target: pipeline database (colossus_legal_v2, applied by the runtime
--         sqlx::migrate::Migrator at backend boot — NOT the compile-time
--         migrate! macro. The file ships with the image; a backend restart
--         re-runs the Migrator and applies it. No cargo rebuild is needed for
--         the migration itself.)
--
-- WHY (Merge provenance, workflow layer on set-as-basis):
--   A Merge (set-as-basis) promotes one stored scan run's relevant picks into a
--   scenario's candidate facts. Until now the merge left NO durable trace: the
--   run detail could not tell it had been merged, so it showed a naked "Merge"
--   button that implied nothing had happened. This table is that trace — one row
--   per merge EVENT.
--
-- WHY AN EVENT TABLE, not a `merged_at` column on scan_runs:
--   Re-merge is legitimate — the reconcile is status-preserving, so promoting a
--   run again is a real workflow (refresh the undecided suggestions without
--   overruling human include/drop). A single `merged_at`/boolean on scan_runs
--   assumes merge-happens-once and cannot represent that history. A child event
--   table records EACH merge, so "merged 2×, last at …" is answerable by COUNT +
--   MAX(merged_at). Same parent/child shape as scan_runs -> scan_run_verdicts.
--
-- FORWARD-ONLY: the pipeline Migrator applies migrations forward only. There is
--   no down migration. A bad forward migration is corrected by a FURTHER forward
--   migration — never by editing this file once applied. NET-NEW table (no
--   existing data touched).

CREATE TABLE scan_run_merges (
    -- Application-generated UUID (uuid v4), not a DB default: minted in Rust
    -- before the INSERT (house pattern — same as scan_runs.run_id).
    merge_id      UUID        NOT NULL PRIMARY KEY,

    -- The run that was merged. Real FK with ON DELETE CASCADE: a merge event is
    -- owned by its run — deleting the run (which already cascades its verdicts)
    -- discards its merge history too. Same ownership shape as
    -- scan_run_verdicts -> scan_runs.
    run_id        UUID        NOT NULL REFERENCES scan_runs(run_id) ON DELETE CASCADE,

    -- The scenario merged into. FK with ON DELETE CASCADE, mirroring
    -- scan_runs.scenario_id: deleting the scenario discards its scan+merge
    -- history. Denormalized alongside run_id (the run already carries its
    -- scenario) so a per-scenario audit read needs no join through scan_runs.
    scenario_id   UUID        NOT NULL REFERENCES scenarios(scenario_id) ON DELETE CASCADE,

    -- When the merge committed. Bound from Rust Utc::now() (not a DB default),
    -- matching the scan_runs.started_at house pattern — the application owns the
    -- timestamp. MAX(merged_at) per run is the "last merged" the run detail shows.
    merged_at     TIMESTAMPTZ NOT NULL,

    -- How many candidate-fact rows this merge inserted or refreshed as undecided
    -- suggestions — the same count the merge endpoint returns. Picks preserved as
    -- existing human included/dropped curation are NOT counted (the merge SQL's
    -- ON CONFLICT … WHERE status='undecided' skips them). A legitimate 0 (the run
    -- had no relevant picks, or all targets were already curated) is a real
    -- recorded event, distinct from "never merged" (no row at all) — Standing
    -- Rule 1.
    rows_affected INTEGER     NOT NULL
);

-- Per-run lookup: the run detail reads COUNT(*) + MAX(merged_at) WHERE run_id = $1.
CREATE INDEX scan_run_merges_run_id_idx ON scan_run_merges (run_id);

COMMENT ON TABLE scan_run_merges IS
    'Scan-run → scenario merge audit (set-as-basis provenance): one row per merge EVENT, so re-merge is a history (COUNT + MAX(merged_at)), never a merged-once boolean. Parent/child on scan_runs like scan_run_verdicts.';
COMMENT ON COLUMN scan_run_merges.merged_at IS
    'When the merge committed, bound from Rust Utc::now() (no DB default), matching scan_runs.started_at. MAX per run = the "last merged" shown on the run detail.';
COMMENT ON COLUMN scan_run_merges.rows_affected IS
    'Candidate-fact rows inserted/refreshed as undecided by this merge (the count the merge endpoint returns). A recorded 0 (no relevant picks / all already curated) is distinct from never-merged (no row) — Standing Rule 1.';
