-- add_scenario_codes: Add scenario codes
--
-- Created: 2026-08-01 10:33:12
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- Implements the §2a codes law (RATIFIED 2026-08-01), tracker task 1.1: every
-- scenario carries a short stable human handle — S-1, S-2, … — assigned at
-- creation, never changed, never reused, surviving rename and deletion.
--
-- ## Why a COLUMN on scenarios, not a side table
--
-- The sibling mechanism `scenario_candidate_ordinals` (C-1, C-2, …) deliberately
-- lives in its OWN table, because `scenario_fact_refs` is derive-on-read: a row
-- exists there only once a candidate has been ruled on, so putting the ordinal
-- there would force eager row materialization for the whole pool and break
-- `join_facts`' miss-semantics.
--
-- None of that applies here. A `scenarios` row always exists — a scenario with no
-- row is not a scenario — so the code is a plain attribute of an entity that is
-- already materialized. A side table would buy nothing and would add a join to
-- every read.
--
-- ## Why the high-water mark needs its OWN table
--
-- The obvious allocation, `MAX(code_ordinal) + 1` over live rows, is wrong here in
-- a way it is NOT wrong for candidate ordinals. Candidate ordinals are only ever
-- removed by deleting the whole scenario (which discards that scenario's entire id
-- space at once, so nothing can be reused). Scenarios are deleted INDIVIDUALLY by
-- `delete_scenario`, and a MAX over the survivors REWINDS when the
-- highest-numbered scenario is deleted — the next creation would mint a code a
-- deleted scenario already wore. That is precisely the reuse the law forbids: a
-- note or a rehearsal saying "S-7" would silently come to mean a different
-- scenario.
--
-- `case_code_sequences` therefore records the high-water mark as a ROW that
-- survives any scenario deletion. Monotonic by construction: the sequence only
-- ever moves forward, so a code is never reissued even when every scenario that
-- held one is gone. Holes are correct and expected — they are the visible record
-- that something was there.
--
-- A Postgres SEQUENCE was considered and rejected: sequences are global objects,
-- so per-case allocation would need one sequence per case created dynamically,
-- which is unenumerable, awkward to back up, and impossible to seed
-- transactionally in a backfill. A table row is simpler and inspectable with a
-- SELECT.
--
-- ## The never-reuse guarantee, precisely
--
-- Allocation happens INSIDE the insert transaction (see `insert_scenario`): the
-- sequence row is bumped and the returned value written to the new scenario in one
-- atomic act. Postgres' row lock on the `UPDATE … RETURNING` serializes concurrent
-- creations, so two simultaneous inserts cannot read the same next value. The
-- `UNIQUE (case_slug, code_ordinal)` below is the LOUD backstop: if allocation is
-- ever bypassed or races, the second insert fails rather than minting an ambiguous
-- S-7.

-- ─── 1. The code column ─────────────────────────────────────────────────────────
--
-- Added NULLABLE so the backfill below can populate it before the NOT NULL is
-- enforced. INTEGER (not BIGINT): a case holds tens of scenarios, nowhere near i32.
ALTER TABLE scenarios
    ADD COLUMN code_ordinal INTEGER;

-- ─── 2. The per-case high-water mark ────────────────────────────────────────────
CREATE TABLE case_code_sequences (
    -- One row per case. No FK: cases are not a table in this database (the slug is
    -- the case identifier throughout, the same string-id discipline as
    -- scenarios.case_slug and scenario_fact_refs.graph_node_id).
    case_slug    TEXT    PRIMARY KEY,

    -- The highest scenario code EVER issued for this case. Never decreases, not
    -- even when the scenario holding that code is deleted. The next code is
    -- next_ordinal + 1, written back in the same transaction.
    next_ordinal INTEGER NOT NULL DEFAULT 0,

    -- A negative or rewound sequence would mean the never-reuse guarantee had
    -- already failed; refuse it at the database rather than discover it later in a
    -- duplicate code.
    CONSTRAINT case_code_sequences_next_ordinal_non_negative
        CHECK (next_ordinal >= 0)
);

COMMENT ON TABLE case_code_sequences IS
    'Per-case high-water mark for scenario codes (S-n). Monotonic and independent '
    'of the scenarios table so that deleting a scenario can never rewind the '
    'sequence and reissue a code a deleted scenario already wore.';

COMMENT ON COLUMN case_code_sequences.next_ordinal IS
    'Highest scenario code ever issued for this case. The next allocation is '
    'next_ordinal + 1, bumped and read in one UPDATE ... RETURNING inside the '
    'scenario insert transaction.';

-- ─── 3. Backfill existing scenarios in creation order ───────────────────────────
--
-- Creation order, per case, is the only defensible assignment: it matches the
-- order a human already saw these scenarios appear, so S-1 is the oldest scenario
-- of the case rather than an arbitrary UUID-ordered pick. `scenario_id` is the tie
-- breaker so the result is deterministic if two rows share a created_at to the
-- microsecond.
WITH ordered AS (
    SELECT
        scenario_id,
        ROW_NUMBER() OVER (PARTITION BY case_slug ORDER BY created_at, scenario_id)
            AS assigned_ordinal
    FROM scenarios
)
UPDATE scenarios s
   SET code_ordinal = ordered.assigned_ordinal
  FROM ordered
 WHERE s.scenario_id = ordered.scenario_id;

-- ─── 4. Seed the sequence to the backfilled maximum ─────────────────────────────
--
-- Seeded in the SAME migration as the backfill so no window exists in which
-- scenarios carry codes but the sequence still reads 0 — a backend starting in
-- that window would mint S-1 again and collide.
INSERT INTO case_code_sequences (case_slug, next_ordinal)
SELECT case_slug, MAX(code_ordinal)
  FROM scenarios
 WHERE code_ordinal IS NOT NULL
 GROUP BY case_slug
    ON CONFLICT (case_slug) DO UPDATE
       SET next_ordinal = GREATEST(case_code_sequences.next_ordinal, EXCLUDED.next_ordinal);

-- ─── 5. Enforce the invariants now that every row has a code ────────────────────
ALTER TABLE scenarios
    ALTER COLUMN code_ordinal SET NOT NULL;

-- The loud backstop on allocation. Scoped to the case: S-3 in one case and S-3 in
-- another are different scenarios, which is correct — codes are spoken inside a
-- case's context, exactly like candidate ordinals inside a scenario's.
ALTER TABLE scenarios
    ADD CONSTRAINT scenarios_case_code_unique UNIQUE (case_slug, code_ordinal);

COMMENT ON COLUMN scenarios.code_ordinal IS
    'The scenario''s stable human handle, rendered S-{code_ordinal}. Assigned at '
    'creation from case_code_sequences, never changed, never reused. Survives '
    'rename; survives the deletion of other scenarios (the sequence never rewinds).';
