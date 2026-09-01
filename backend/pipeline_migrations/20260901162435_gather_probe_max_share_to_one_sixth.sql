-- gather_probe_max_share: 1/3 -> 1/6.
--
-- ## Why, and why it is not because it scored better
--
-- It has not been scored, and it must not be chosen that way — a threshold
-- tuned until a rank moves is a threshold that proves nothing. The reason is
-- the reasoning, and the measured counts are what expose it.
--
-- One third condemned `Court` at 46% of S-11's admitted set. It left standing:
--
--     Phillips        332   32.2%
--     Probate         241   23.4%
--     Personal        192   18.6%
--     Representative  171   16.6%
--     Attorney        151   14.7%
--
-- `Probate` and `Personal` are courthouse furniture in a probate file. They are
-- exactly as generic as `Court`; the argument that condemns one condemns all of
-- them, and one third happened to catch only the three worst. A sixth (172 of
-- 1030) catches the whole group and stops above `Nadia` at 143, which is a
-- party name doing real work.
--
-- ## Why an UPDATE of the existing row rather than a new one
--
-- The key, kind, bounds and meaning are unchanged; only the value moved. A
-- second row would leave two truths in the store and make the reader guess.
--
-- `default_value` moves with `value` because it is what the settings page shows
-- as "what this shipped as", and after this migration 1/6 IS what it shipped as.
--
-- ## Why it overwrites unconditionally
--
-- A migration that stamps over an operator's hand-set value is normally wrong,
-- and a guarded `CASE WHEN value = '1/3'` was written here first. It was
-- replaced for two reasons, in this order:
--
--   1. There is no operator value to preserve. `gather_probe_max_share` has
--      never existed in a deployed database — checked on DEV, which carries
--      zero `gather_%` rows — so the only value it can hold is the one its own
--      predecessor migration seeded, minutes earlier in the same unmerged
--      branch stack.
--   2. `settings_store_tests::the_fixtures_carry_the_values_the_migration_
--      actually_seeds` parses this file off disk for the literal
--      `SET value         = '` and `WHERE key           = '`. A CASE expression
--      is invisible to that parser, so the guarded form would have made the
--      drift check silently stop checking this row — which is a worse failure
--      than the one the guard was protecting against, and a silent one.
--
-- ⚑ THE TWO REASONS ARE A CONJUNCTION, NOT A MENU. Reason 2 alone justifies
-- nothing: a parser that cannot see a guard is an argument for fixing the
-- parser, never for deleting the guard. It is only because reason 1 holds —
-- there is provably no operator value at risk — that trading the guard for
-- visibility costs nothing here.
--
-- Do not cite this migration for a row that HAS been deployed. There the guard
-- is the thing that matters and the parser is the thing that must change: a
-- future correction needs the guard back AND a parser that can see it.

UPDATE app_settings
   SET value         = '1/6',
       default_value = '1/6',
       updated_at    = now(),
       updated_by    = 'migration:gather_probe_max_share_to_one_sixth'
 WHERE key           = 'gather_probe_max_share';

-- Rule 25a: an UPDATE matching zero rows is SILENT in Postgres, and the old
-- value would keep being served with nothing to show for it. Assert the END
-- state, not the statement.
DO $$
DECLARE
    row_value   TEXT;
    row_default TEXT;
BEGIN
    SELECT value, default_value INTO row_value, row_default
      FROM app_settings WHERE key = 'gather_probe_max_share';

    IF row_default IS NULL THEN
        RAISE EXCEPTION
            'gather_probe_max_share does not exist — this migration updates a row that its own predecessor was supposed to seed';
    END IF;

    IF row_default <> '1/6' THEN
        RAISE EXCEPTION
            'gather_probe_max_share.default_value is % after the update, expected 1/6',
            row_default;
    END IF;

    -- The live value must not still be the old default, which would mean the
    -- UPDATE matched nothing and the ruling did not land.
    IF row_value = '1/3' THEN
        RAISE EXCEPTION
            'gather_probe_max_share is still 1/3 — the update did not take, and the gather would keep the share the ruling replaced';
    END IF;

    IF row_value !~ '^[0-9]+/[0-9]+$' THEN
        RAISE EXCEPTION
            'gather_probe_max_share is %, which is not a ratio written n/m', row_value;
    END IF;
END $$;
