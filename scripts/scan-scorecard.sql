-- =============================================================================
-- scan-scorecard.sql — does the scan agree with Roman's own rulings?
-- =============================================================================
--
-- The standing post-change instrument for task 2.15. Run it after EVERY change to
-- the judging prompt, the model, or the pre-filter: it scores one scan run against
-- the scenario's ruled ledger and says, in one table, how much of what Roman
-- included the scan caught and how much of what he dropped it let through.
--
-- Source: CC_REPORT_2_15_MEASUREMENT.md §6. It lived in a report until now; this
-- file is its home, because executable tooling belongs in the repo (Roman's
-- convention), not in a document nobody can run.
--
-- READ-ONLY. It writes nothing and takes no locks worth naming.
--
-- -----------------------------------------------------------------------------
-- USAGE
-- -----------------------------------------------------------------------------
--
-- The scenario and run ids are psql variables. From the DB host:
--
--   sudo podman exec -i <postgres-container> psql -U <user> -d colossus_legal_v2 \
--     -v scenario="'<scenario-uuid>'" -v run="'<run-uuid>'" \
--     -f scan-scorecard.sql
--
-- Or interactively, inside psql:
--
--   \set scenario '<scenario-uuid>'
--   \set run      '<run-uuid>'
--   \i scripts/scan-scorecard.sql
--
-- Find the two ids with:
--
--   SELECT scenario_id, code, name FROM scenarios ORDER BY code;
--   SELECT run_id, model_id, status, started_at, candidates_read, candidates_total
--     FROM scan_runs WHERE scenario_id = '<scenario-uuid>'
--     ORDER BY started_at DESC;
--
-- DATABASE: colossus_legal_v2 (the pipeline database). Both tables live there.
--
-- -----------------------------------------------------------------------------
-- HOW TO READ THE OUTPUT
-- -----------------------------------------------------------------------------
--
-- One row per confidence threshold. Only the first row (0.0) describes what the
-- scan ACTUALLY did; the rest answer "what if we had also required confidence >= X".
--
--   includes_kept          Roman said INCLUDE, the scan said relevant.   Higher is better.
--   includes_lost          Roman said INCLUDE, the scan did not.         Lower is better.
--   drops_kept_wrongly     Roman said DROP, the scan said relevant.      Lower is better.
--   drops_correctly_killed Roman said DROP, the scan agreed.             Higher is better.
--
-- The 2026-08-06 baseline on S-2, for comparison: 16 kept, 36 lost, 0 wrongly
-- kept, 3 correctly killed — and the threshold sweep was FLAT on the last column,
-- which is why the confidence threshold was struck from the design.
--
-- -----------------------------------------------------------------------------
-- TWO THINGS THIS QUERY DOES DELIBERATELY, AND WHY
-- -----------------------------------------------------------------------------
--
-- 1. It joins on `graph_node_id`, NEVER on `scenario_fact_refs.source_run_id`.
--    Measured 2026-08-06: 82 of S-2's 83 refs carry a NULL `source_run_id` even
--    though `scan_run_merges` records four merges against those same nodes — a
--    later human edit re-upserts the row without preserving the column. Scoring
--    through it would silently compare almost nothing.
--
-- 2. It scores only `included` / `dropped` rows — the GOLD LABELS. `undecided`
--    is not a negative; it is a question nobody has answered, and counting it as
--    one would flatter or damn the scan by accident. The known limitation, stated
--    so nobody over-reads a good number: on 2026-08-06 the ledger held 52
--    includes against 3 drops, so this measures RECALL well and PRECISION barely.
--    Ruling the undecided rows is what fixes that, and only Roman can do it.
--
-- A note on collapsed duplicates (task 2.15): a scan folds byte-identical quotes
-- into one judged group but writes a verdict row for EVERY member, precisely so
-- this join still finds a ruled twin. If that ever changes, this scorecard starts
-- reporting folded rows as `includes_lost`.

WITH ruled AS (
    -- The gold labels: only what a human actually decided.
    SELECT graph_node_id, status
    FROM scenario_fact_refs
    WHERE scenario_id = :scenario
      AND status IN ('included', 'dropped')
), v AS (
    -- One run's verdicts. `error IS NULL` is not applied here on purpose: a
    -- candidate the model could not judge is a candidate the scan did not catch,
    -- and hiding it would overstate recall.
    SELECT graph_node_id, relevant, confidence
    FROM scan_run_verdicts
    WHERE run_id = :run
), t(thr) AS (
    VALUES (0.0), (0.4), (0.5), (0.6), (0.7), (0.8), (0.9)
)
SELECT
    t.thr AS min_confidence,
    count(*) FILTER (
        WHERE r.status = 'included'
          AND v.relevant AND v.confidence >= t.thr
    ) AS includes_kept,
    count(*) FILTER (
        WHERE r.status = 'included'
          AND NOT (coalesce(v.relevant, false) AND v.confidence >= t.thr)
    ) AS includes_lost,
    count(*) FILTER (
        WHERE r.status = 'dropped'
          AND v.relevant AND v.confidence >= t.thr
    ) AS drops_kept_wrongly,
    count(*) FILTER (
        WHERE r.status = 'dropped'
          AND NOT (coalesce(v.relevant, false) AND v.confidence >= t.thr)
    ) AS drops_correctly_killed
FROM t
CROSS JOIN ruled r
LEFT JOIN v ON v.graph_node_id = r.graph_node_id
GROUP BY t.thr
ORDER BY t.thr;
