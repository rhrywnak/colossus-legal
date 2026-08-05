-- add_tier_and_sort_ordinal_to_scenario_fact_refs: weight and human order
--
-- Created: 2026-08-05 09:22:29
-- Target: pipeline database (colossus_legal_v2, applied at backend boot by the
--         runtime sqlx::migrate::Migrator — forward-only, no down migration)
--
-- Task 2.13 slice 1, from SCENARIO_FACTS_REDESIGN_v1 (SIGNED 2026-08-05).
--
-- WHY THESE TWO COLUMNS
--
--   The Scenario facts list remembers WHERE every fact lives and nothing about
--   what any of them is worth. Roman's ruling: "I have a list of facts. Some are
--   good, others are crap. How do I prepare for trial with this list?" Weight
--   (which facts carry the scenario) and order (the sequence is the argument)
--   are the two answers that need storage; both are per-SCENARIO judgments about
--   a SHARED fact, so they belong on the reference row, exactly as
--   `role_in_this_scenario` does — never on the graph node, which other
--   scenarios also point at.
--
-- ## `tier` — NO CHECK CONSTRAINT, deliberately
--
--   This follows THIS table's documented precedent rather than the more common
--   house style. The `status` migration (20260706162558) states the rule:
--
--       "NO CHECK CONSTRAINT — deliberate, matching THIS table's existing
--        precedent for `role_in_this_scenario` … the vocabulary is evolvable …
--        Validated in code."
--
--   The three-tier invariant is enforced by the `FactTier` Rust enum
--   (`domain::fact_tier`) with the exact `FactStatus` discipline — `TryFrom<&str>`
--   at the read boundary, `code()` at the write boundary, a loud typed error on an
--   unknown token. A fourth tier later is then a code change with a version bump,
--   not a migration. Contrast the sibling `scenarios` table, whose `direction` /
--   `status` DO use CHECK: those are stable lifecycle fields, this is an evolvable
--   interaction vocabulary. Different volatility, different choice.
--
--   NOT NULL DEFAULT 'backup' mirrors `status`'s shape: every row has a definite
--   tier, and the default is the neutral middle one the signed design names for a
--   newly-included fact. Existing rows take it without a backfill statement — a
--   fact nobody has weighed IS backup, so the default is the truth rather than a
--   placeholder standing in for one.
--
-- ## `sort_ordinal` — INTEGER and NULLABLE
--
--   INTEGER, emphatically NOT NUMERIC. beta.364 died on a NUMERIC column: there is
--   no `rust_decimal` / `bigdecimal` in this tree, so sqlx cannot decode NUMERIC at
--   all and the failure arrives at runtime on the read path. INTEGER decodes
--   natively to `i32`.
--
--   NULLABLE because "the human has not placed this one" is a real state and not a
--   defect: NULL sorts to the END of the list, behind every fact somebody has
--   deliberately positioned, and keeps its existing C-ordinal order there. A
--   backfilled 0 for every row would erase the difference between "placed first"
--   and "never placed", which is precisely the distinction the drag exists to
--   create (Standing Rule 1).
--
--   Values are assigned SPARSELY, in steps of 1024, so a drag between two
--   neighbours writes the midpoint of their ordinals and touches exactly ONE row.
--   Every card is independent (the signed design's first law) — there is no
--   whole-list renumber, and no card's stored position changes because a different
--   card moved.
--
-- FORWARD-ONLY: the pipeline Migrator applies migrations forward only. There is
--   no down migration. A bad forward migration is corrected by a FURTHER forward
--   migration (alter/drop) — never by editing or deleting this file once applied.

ALTER TABLE scenario_fact_refs
    ADD COLUMN tier TEXT NOT NULL DEFAULT 'backup';

ALTER TABLE scenario_fact_refs
    ADD COLUMN sort_ordinal INTEGER;

COMMENT ON COLUMN scenario_fact_refs.tier IS
    'How much this fact carries THIS scenario: carries / backup (default) / background. '
    'A per-scenario human judgment about a shared fact, like role_in_this_scenario. '
    'Vocabulary validated in code by the FactTier enum, NOT a DB CHECK — evolvable, '
    'matching this table''s precedent for status and role_in_this_scenario.';

COMMENT ON COLUMN scenario_fact_refs.sort_ordinal IS
    'The human''s explicit position for this fact in THIS scenario, assigned sparsely '
    'in steps of 1024 so a drag writes one row (the midpoint of its new neighbours). '
    'NULL means the human has never placed this fact: it sorts after every placed one, '
    'keeping its C-ordinal order there. NULL and 0 are different states, never collapsed.';
