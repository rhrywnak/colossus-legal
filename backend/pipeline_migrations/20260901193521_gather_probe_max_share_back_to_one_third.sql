-- gather_probe_max_share: 1/6 -> 1/3. A REVERT, not a better number.
--
-- The 1/6 ruling was made on sound reasoning — `Probate` at 23% and `Personal`
-- at 19% really are as generic as `Court` at 46% — and measurement showed the
-- reasoning did not cover the case that mattered. At a sixth the ceiling is 172
-- and `Phillips` (332) went with them. Measured:
--
--     the seven target cards containing "Phillips"      7 of 7    (100%)
--     corpus-wide cards containing "Phillips"         403 of 1209  (33%)
--
-- A probe can be common and perfectly informative at the same time, and a share
-- of the corpus cannot see the difference. Dropping it cost both acceptance
-- bars: S-11's top-20 went 3 of 7 to 0 of 7, S-9's C-54 went 80 to 112.
--
-- ## This is a revert, and the distinction matters
--
-- 1/3 is not a value chosen after inspecting ranks. It is the value that was in
-- place before a ruling that measurement disproved, restored to what it was.
-- The alternatives the report offered — 1/4, and weighting probes by inverse
-- document frequency instead of dropping them — are deliberately NOT tried
-- here. Choosing among them by which scores best is exactly the tuning this
-- whole sequence has refused to do.
--
-- ## The collapse STAYS
--
-- `$50,000`, `$50,000.00` and `$500,000.00` returned identical id sets and were
-- scoring the same cards three times for one match. That is a defect and its
-- fix stands whatever it costs in rank. It does mean the earlier "5 of 7 in the
-- top 60, 3 of 7 in the top 20" figures at 1/3 were taken BEFORE the collapse
-- and were partly manufactured agreement; the run after this migration is the
-- first honest measurement of a third, and a lower figure is not a regression.

UPDATE app_settings
   SET value         = '1/3',
       default_value = '1/3',
       updated_at    = now(),
       updated_by    = 'migration:gather_probe_max_share_back_to_one_third'
 WHERE key           = 'gather_probe_max_share';

-- Rule 25a: a zero-row UPDATE is silent in Postgres and the old value would
-- keep being served. Assert the END state.
--
-- Unconditional for the reason its predecessor recorded, and the two conditions
-- still both hold: the row has never reached a deployed database (DEV carries
-- zero `gather_%` rows), AND the drift test parses this file for a literal
-- `SET value         = '` that a CASE expression is invisible to.
DO $$
DECLARE
    row_value   TEXT;
    row_default TEXT;
BEGIN
    SELECT value, default_value INTO row_value, row_default
      FROM app_settings WHERE key = 'gather_probe_max_share';

    IF row_default IS NULL THEN
        RAISE EXCEPTION
            'gather_probe_max_share does not exist — this migration reverts a row its predecessors were supposed to seed';
    END IF;

    IF row_default <> '1/3' THEN
        RAISE EXCEPTION
            'gather_probe_max_share.default_value is % after the revert, expected 1/3', row_default;
    END IF;

    IF row_value = '1/6' THEN
        RAISE EXCEPTION
            'gather_probe_max_share is still 1/6 — the revert did not take, and the gather would keep the share that measurement disproved';
    END IF;

    -- The POSITIVE assertion, and it is the one that matters. "The old value is
    -- gone" is necessary and not sufficient: a zero-row UPDATE against a row
    -- whose default was already 1/3 from a partial earlier run would pass every
    -- check above while `value` held something else entirely. Assert the end
    -- state, not the absence of the previous one.
    IF row_value <> '1/3' THEN
        RAISE EXCEPTION
            'gather_probe_max_share is % after the revert, expected 1/3 — the UPDATE did not reach this row',
            row_value;
    END IF;
END $$;
