-- merge_model_unification_source_run_id_ordinals_selection
--
-- Created: 2026-07-19 15:21:07
-- Target: pipeline database (colossus_legal_v2)
--
-- Schema half of the unified merge model (SCENARIO_MERGE_MODEL_UNIFIED_DESIGN_v1
-- v1.3). Three ADDITIVE changes, no destructive operation anywhere:
--
--   1. scenario_fact_refs.source_run_id — which scan run's judgment a fact ref
--      carries (NULL = human-authored). Design §9.2 / ratification point 8.
--   2. scenario_candidate_ordinals     — the persisted, scenario-scoped, append-only
--      candidate id (C-1, C-2, …). Design §7 / ruling §10.1.
--   3. scan_run_merges.selected_node_ids — which picks a merge event applied.
--      Design §3.1.
--
-- The dry_run column on scan_runs is deliberately NOT dropped here: it is
-- deprecated (nothing branches on it after this batch, new runs write false) and
-- its drop rides Chunk B's migration, which already touches scan_runs (§3.4).

-- ─── 1. scenario_fact_refs.source_run_id ────────────────────────────────────────
--
-- A present `confidence` proves SOME scan judgment was applied, but not WHICH
-- run's — insufficient for the per-run "applied" state and weak provenance for a
-- trial-preparation system. This column closes that gap.
--
-- NULLABLE and NULL-by-default on purpose: every existing row, and every future
-- human-authored row (include/drop/undrop via upsert_fact_ref), genuinely has no
-- source run. NULL here is the honest "authored by a human", not a placeholder —
-- the same absence-is-meaningful discipline as the NULL `confidence` column.
--
-- ON DELETE SET NULL: if a run is ever deleted, the fact keeps the judgment it was
-- given while honestly recording that the run it came from is gone. Note this is
-- now DEFENCE-IN-DEPTH only: deleting a merged run is refused in code with a 409
-- (design §10.2), because one delete would otherwise destroy both provenance
-- mechanisms at once (this column via SET NULL, and scan_run_merges via CASCADE).
ALTER TABLE scenario_fact_refs
    ADD COLUMN source_run_id UUID REFERENCES scan_runs(run_id) ON DELETE SET NULL;

COMMENT ON COLUMN scenario_fact_refs.source_run_id IS
    'The scan run whose judgment this fact ref carries, or NULL when the row was '
    'authored by a human (include/drop/undrop). Written by the merge SQL inside the '
    'undecided-gated tail, so a curated row''s provenance freezes with its status. '
    'Presence/absence mirrors the card''s judgment-strip semantics: absence IS the '
    '"human / unscored" signal. Deleting a merged run is refused in code (409); the '
    'ON DELETE SET NULL is defence-in-depth for the unmerged-run path.';

-- Reverse lookup: "which fact refs came from THIS run" drives both the per-run
-- "applied" state (GET /scan-runs/:run_id) and the delete-restriction pre-check.
-- Without an index both are sequential scans of the whole table; the table is small
-- today (~94 rows/scenario) but the query runs on every run-detail open. Partial
-- (WHERE NOT NULL) because human-authored rows are the majority and are never the
-- target of this lookup — a smaller index serving exactly the rows it is for.
CREATE INDEX scenario_fact_refs_source_run_id_idx
    ON scenario_fact_refs (source_run_id)
    WHERE source_run_id IS NOT NULL;

-- ─── 2. scenario_candidate_ordinals ─────────────────────────────────────────────
--
-- The human's handle on a candidate fact: "look at C-14." Speakable, writable in a
-- margin note, stable for the life of the scenario. Replaces the truncated-hash
-- chip (which was stable but not simple — a hash fragment is not speakable).
--
-- ## Why a SEPARATE table rather than a column on scenario_fact_refs
--
-- `scenario_fact_refs` is derive-on-read by RATIFIED decision: a row exists there
-- if and only if a candidate has been ruled on or scored, and gather never writes
-- it. Putting the ordinal there would force eager row materialization for the whole
-- pool, breaking that invariant and, with it, `join_facts`' miss-semantics ("a ref
-- pointing at a dead graph node") — every pool member would suddenly have a ref.
-- Splitting identity from state keeps both contracts intact: this table memoizes
-- IDENTITY (which candidate is C-14), `scenario_fact_refs` records STATE (what the
-- human decided). Gather may write here precisely because an ordinal is not user
-- state.
--
-- Domain note: ordinals are never reused, never renumbered, never resorted-into. A
-- dropped candidate keeps its id forever — drop excludes, it never deletes, and
-- "we looked at C-31 and dropped it" must stay sayable. When duplicate graph nodes
-- are retired by the pipeline fix, their ordinals leave HOLES in the sequence.
-- Holes are correct: a gap is honest history, and renumbering to close one would
-- break every prior reference in a notebook or a transcript.
CREATE TABLE scenario_candidate_ordinals (
    -- Owned by its scenario, exactly like scenario_fact_refs: deleting the scenario
    -- discards its id space. Across scenarios, ordinals are independent — C-14 in S2
    -- and C-14 in S5 are different facts, which is acceptable because curation and
    -- rehearsal both happen inside one scenario's context.
    scenario_id   UUID        NOT NULL REFERENCES scenarios(scenario_id) ON DELETE CASCADE,

    -- The Neo4j node id. Plain TEXT with NO foreign key — it points into Neo4j,
    -- which Postgres cannot reference. Same shape as scenario_fact_refs.
    graph_node_id TEXT        NOT NULL,

    -- The human-facing sequence number, rendered as C-{ordinal}. Starts at 1 per
    -- scenario. INTEGER (not BIGINT): a scenario's pool is ~94 candidates today and
    -- is bounded by the case's evidence count — nowhere near i32.
    ordinal       INTEGER     NOT NULL,

    -- When this candidate first entered the pool. Bound from Rust Utc::now(), not a
    -- DB default, matching the scan_runs.started_at / scan_run_merges.merged_at
    -- house pattern — the application owns the timestamp. Also the honest answer to
    -- "when did this candidate first appear", which the ordinal alone only implies.
    assigned_at   TIMESTAMPTZ NOT NULL,

    -- Identity: one ordinal per (scenario, node). This is what makes assignment
    -- IDEMPOTENT — re-running gather hits ON CONFLICT DO NOTHING and never
    -- double-assigns. Same composite-PK shape as scenario_fact_refs, and the same
    -- (scenario_id, graph_node_id) identity key that already protects include/drop
    -- across pool reloads now protects the id too.
    PRIMARY KEY (scenario_id, graph_node_id),

    -- No two candidates in one scenario share an ordinal. This is the LOUD guard on
    -- the assignment statement: two gathers racing to assign the same next ordinal
    -- collide here and one fails visibly, rather than silently minting a duplicate
    -- "C-14" that would make the human's handle ambiguous (Standing Rule 1).
    UNIQUE (scenario_id, ordinal)
);

COMMENT ON TABLE scenario_candidate_ordinals IS
    'The persisted, scenario-scoped, append-only candidate identifier (C-1, C-2, …) — '
    'the human''s speakable handle on a candidate fact, shown identically on the '
    'Candidate Facts card and the scan-results row. Separate from scenario_fact_refs '
    'so that table''s derive-on-read contract (a row exists only for a ruled or scored '
    'candidate) stays intact: this table memoizes IDENTITY, that one records STATE. '
    'Assigned at gather, never reused, never renumbered; holes left by retired '
    'duplicate nodes are correct and are never closed.';

COMMENT ON COLUMN scenario_candidate_ordinals.ordinal IS
    'The human-facing sequence number, rendered C-{ordinal}. Unique per scenario, '
    'append-only (new candidates take MAX+1), never reused or renumbered — a prior '
    'reference to "C-14" must stay valid for the life of the scenario.';

COMMENT ON COLUMN scenario_candidate_ordinals.assigned_at IS
    'When this candidate first entered the scenario''s pool. Bound from the '
    'application (Utc::now()), not a DB default — house pattern.';

-- ─── 3. scan_run_merges.selected_node_ids ───────────────────────────────────────
--
-- The merge audit table records that a merge happened and how many rows it applied.
-- It could not say WHICH picks the human chose — and in the pick-keyed model the
-- selection IS the human's decision, so an audit trail without it records the event
-- but not the act.
--
-- NULLABLE with no backfill, deliberately: rows written before this migration
-- genuinely do not know their selection. NULL reads "selection not recorded"; a
-- backfilled '[]' would read "the human selected nothing", a different and false
-- claim (Standing Rule 1 — the two states stay distinguishable). Every merge
-- written from now on carries the array.
--
-- JSONB rather than TEXT[]: matches scan_runs.resolved_params' precedent for
-- application-shaped payloads, and leaves room to record per-pick detail later
-- without another migration.
--
-- rows_affected is KEPT alongside it: the two are different numbers. The selection
-- is what the human checked; rows_affected is what the status-preserving reconcile
-- actually wrote (picks whose target was already included/dropped are preserved and
-- therefore not counted). Keeping both makes the guard's effect auditable.
ALTER TABLE scan_run_merges
    ADD COLUMN selected_node_ids JSONB;

COMMENT ON COLUMN scan_run_merges.selected_node_ids IS
    'The graph_node_ids the human CHECKED for this merge, as a JSON array. NULL on '
    'rows written before the column existed — "selection not recorded", deliberately '
    'distinct from an empty array ("selected nothing"). Differs from rows_affected: '
    'this is what was chosen, that is what the status-preserving reconcile wrote.';
