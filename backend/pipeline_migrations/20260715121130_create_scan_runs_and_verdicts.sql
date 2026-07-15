-- create_scan_runs_and_verdicts: the Theme Scan audit + benchmark tables
--
-- Created: 2026-07-15 12:11:30
-- Target: pipeline database (colossus_legal_v2, applied by the runtime
--         sqlx::migrate::Migrator at backend boot — NOT the compile-time
--         migrate! macro. The file ships with the image; a backend restart
--         re-runs the Migrator and applies it. No cargo rebuild is needed for
--         the migration itself.)
--
-- WHY (LLM Configuration Method, Chunk B / design §6, amendment A4):
--   The scan was stateless — its only audit surface was the log (D2b: "no run
--   table by design"). That blocks the benchmark (driver D1): to compare an Opus
--   scan against a Qwen-14B scan we must record EACH run's resolved parameters,
--   cost, and per-candidate verdicts durably, and the two runs must NOT collide
--   on scenario_fact_refs' (scenario_id, graph_node_id) PK. These two tables are
--   that durable record. `scan_runs` is the per-run header (one row per scan);
--   `scan_run_verdicts` is the per-candidate detail (one row per judged quote).
--
-- WHY A CHILD TABLE, not a JSONB verdict array on scan_runs:
--   the promotion query JOINS one run's verdicts against another's per
--   graph_node_id to compute agreement (relevant + role). That is a relational
--   join — awkward and slow over a JSONB array, natural over rows. ~94 rows per
--   run is trivial relationally. Mirrors the extraction_runs -> extraction_chunks
--   parent/child precedent already in this database.
--
-- WHY resolved_params IS A SNAPSHOT (JSONB), not a reference to the llm_models
--   row: an operator editing a model's params between two benchmark runs would
--   make them incomparable (design 5.9). Snapshotting the RESOLVED params the run
--   actually used freezes the comparison. Same pattern as
--   extraction_runs.processing_config (its second customer).
--
-- NO CHECK CONSTRAINT on proposed_role — deliberate, matching the
--   scenario_fact_refs precedent: the role vocabulary is owned by the Rust
--   FactRole enum (domain/fact_role.rs), NOT the database, so it can evolve
--   without a migration.
--
-- FORWARD-ONLY: the pipeline Migrator applies migrations forward only. There is
--   no down migration. A bad forward migration is corrected by a FURTHER forward
--   migration — never by editing or deleting this file once applied. Both tables
--   are NET-NEW (no existing data touched).

CREATE TABLE scan_runs (
    -- Application-generated UUID (uuid v4), not a DB default: the run id is known
    -- in Rust before the INSERT so it can key the child verdict rows in the same
    -- transaction without a round-trip to read it back.
    run_id           UUID        NOT NULL PRIMARY KEY,

    -- The scenario scanned. Real FK with ON DELETE CASCADE: a run is owned by its
    -- scenario (deleting the scenario discards its scan history), same ownership
    -- shape as scenario_fact_refs -> scenarios and extraction_chunks ->
    -- extraction_runs.
    scenario_id      UUID        NOT NULL REFERENCES scenarios(scenario_id) ON DELETE CASCADE,

    -- The llm_models.id this run judged with. Plain TEXT, NO foreign key ON
    -- PURPOSE: audit/benchmark history must survive an operator later editing or
    -- deactivating that model row. The capability facts that mattered are frozen
    -- in resolved_params below, so the row itself is not needed to interpret the
    -- run.
    model_id         TEXT        NOT NULL,

    -- The RESOLVED (post-resolve/post-constrain) ResolvedLlmParams the run used,
    -- as JSONB. The snapshot, not a reference (see header). Shape:
    -- {"temperature": <number|null>, "timeout_secs": <int>, "max_tokens": <int>}.
    resolved_params  JSONB       NOT NULL,

    -- true = benchmark/measurement run: verdicts are recorded here but the
    -- scenario_fact_refs upsert is SUPPRESSED (A4 — so a second model's run does
    -- not collide with the first on the (scenario_id, graph_node_id) PK).
    -- false = normal workbench scan: also upserts relevant verdicts as undecided
    -- suggestions, exactly as the pre-Chunk-B scan did.
    dry_run          BOOLEAN     NOT NULL,

    -- Outcome tallies. candidates_read = the full ungated pool size (100%-recall
    -- input). The three outcome counts partition it: relevant + irrelevant +
    -- failed = candidates_read.
    candidates_read  INTEGER     NOT NULL,
    relevant_count   INTEGER     NOT NULL,
    irrelevant_count INTEGER     NOT NULL,
    failed_count     INTEGER     NOT NULL,

    -- Summed reported token usage across the run. NULLABLE and NULL-if-absent
    -- (Standing Rule 1 / Chunk B binding): NULL means NO call reported usage
    -- metadata; a value is the sum of the calls that DID report. Never a
    -- fabricated 0 — 0 would be indistinguishable from "reported zero tokens".
    input_tokens     BIGINT,
    output_tokens    BIGINT,

    -- Computed dollar cost = tokens x the model's per-token costs, when both are
    -- known. NULLABLE: NULL for a local vLLM model (no per-token cost) or when
    -- token usage was absent. NUMERIC(12,8) mirrors the llm_models cost columns;
    -- decoded via ::float8 like those (no rust_decimal feature).
    computed_cost    NUMERIC(12,8),

    started_at       TIMESTAMPTZ NOT NULL,
    -- Wall-clock duration of the judging fan-out in milliseconds. BIGINT for
    -- headroom (decodes cleanly to i64); a scan never approaches i32 ms overflow
    -- but BIGINT removes the question.
    duration_ms      BIGINT      NOT NULL
);

CREATE INDEX scan_runs_scenario_id_idx ON scan_runs (scenario_id);

COMMENT ON TABLE scan_runs IS
    'Theme Scan per-run header (LLM Config Chunk B / A4): one row per scan. Records the model, the RESOLVED params snapshot (frozen for benchmark comparability, like extraction_runs.processing_config), outcome tallies, token usage, computed cost, and timing. dry_run=true suppresses the scenario_fact_refs write so two benchmark runs do not collide.';
COMMENT ON COLUMN scan_runs.model_id IS
    'The llm_models.id judged with. Plain TEXT, NO FK — audit history must survive the model row being edited or deactivated; resolved_params freezes what mattered.';
COMMENT ON COLUMN scan_runs.resolved_params IS
    'ResolvedLlmParams snapshot as JSONB {temperature, timeout_secs, max_tokens}. A frozen snapshot, not a reference to the mutable llm_models row (design 5.9).';
COMMENT ON COLUMN scan_runs.dry_run IS
    'true = benchmark run (records verdicts, does NOT upsert scenario_fact_refs). false = workbench scan (also upserts relevant verdicts as undecided suggestions).';
COMMENT ON COLUMN scan_runs.input_tokens IS
    'Summed reported input tokens. NULL = no call reported usage (never a fabricated 0 — Standing Rule 1).';
COMMENT ON COLUMN scan_runs.output_tokens IS
    'Summed reported output tokens. NULL = no call reported usage (never a fabricated 0).';
COMMENT ON COLUMN scan_runs.computed_cost IS
    'tokens x per-token cost when both known; NULL for local vLLM (no per-token cost) or absent usage. Decoded via ::float8 like the llm_models cost columns.';

CREATE TABLE scan_run_verdicts (
    -- Owned by its run: deleting the run discards its verdicts.
    run_id         UUID    NOT NULL REFERENCES scan_runs(run_id) ON DELETE CASCADE,

    -- The Neo4j node id of the candidate quote. Plain TEXT, NO FK (it points into
    -- Neo4j, which Postgres cannot reference), same as scenario_fact_refs.
    graph_node_id  TEXT    NOT NULL,

    -- The judged verdict. relevant/proposed_role/confidence/reason are the model's
    -- output on a SUCCESSFUL parse; all four are NULL when the candidate FAILED
    -- (see `error`). proposed_role carries NO CHECK — the FactRole enum owns the
    -- vocabulary. confidence is REAL (~2-decimal model output; half the width of
    -- DOUBLE PRECISION), matching scenario_fact_refs.confidence.
    relevant       BOOLEAN,
    proposed_role  TEXT,
    confidence     REAL,
    reason         TEXT,

    -- The raw model reply text. Preserved for BOTH successes and parse-failures
    -- (the audit surface the stateless scan previously only logged, and the
    -- Ruling-1 signal for measuring Qwen's prose-JSON compliance). NULL only when
    -- the call itself failed before returning any text (e.g. a network error).
    raw_reply      TEXT,

    -- Standing Rule 1: a failed candidate is DISTINGUISHABLE from a judged one and
    -- carries WHY. NULL = the candidate was judged successfully; NON-NULL = the
    -- per-item failure reason (a bad/unparseable reply, or a call error). The
    -- agreement query selects WHERE error IS NULL to compare only judged verdicts.
    error          TEXT,

    -- One verdict per candidate per run. Two benchmark runs (distinct run_ids)
    -- record the SAME graph_node_id independently — the point of A4.
    PRIMARY KEY (run_id, graph_node_id)
);

COMMENT ON TABLE scan_run_verdicts IS
    'Theme Scan per-candidate verdict detail (LLM Config Chunk B). One row per candidate quote per run. Feeds the promotion benchmark: JOIN two runs on graph_node_id WHERE error IS NULL, compare relevant + proposed_role for the agreement number.';
COMMENT ON COLUMN scan_run_verdicts.graph_node_id IS
    'Neo4j node id of the candidate quote. Plain TEXT, NO FK (points into Neo4j).';
COMMENT ON COLUMN scan_run_verdicts.proposed_role IS
    'Model-proposed role. Vocabulary owned by the FactRole Rust enum, NOT a DB CHECK. NULL when the candidate failed.';
COMMENT ON COLUMN scan_run_verdicts.raw_reply IS
    'Raw model reply, kept for successes and parse-failures (audit + prose-JSON compliance signal). NULL only when the call returned no text.';
COMMENT ON COLUMN scan_run_verdicts.error IS
    'NULL = judged successfully; NON-NULL = the per-item failure reason. Distinguishes failed from judged (Standing Rule 1).';
