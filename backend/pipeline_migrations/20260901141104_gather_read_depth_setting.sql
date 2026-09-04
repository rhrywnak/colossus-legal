-- gather_read_depth — how many candidates each half of a ranked gather returns
-- before fusion.
--
-- ## Why this is a row and not a constant
--
-- It shipped as a compiled `200`. That is a per-deployment retrieval LIMIT,
-- which Rule 13 names explicitly, and no `// STRUCTURAL:` claim is honest about
-- it: the number is not protocol and not format, it is a judgement about how
-- many candidates are worth carrying, and L3 exists partly to find out whether
-- 200 is the right one. A value a task is expected to tune is configuration.
--
-- ## What it controls
--
-- Both reads go to the same depth, deliberately: a vector read of 200 fused
-- against a lexical read of 50 would look like the lexical side having no
-- opinion about 150 cards when it was simply never asked.
--
-- Bounds. The floor of 20 is the smallest depth at which the AT-2 top-20 bar is
-- even expressible — below it the bar could not be measured. The ceiling of
-- 2000 is above the whole corpus (1209 Evidence nodes on DEV, 2026-09-01), so
-- it permits "retrieve everything" while still refusing a value that would
-- mean the read is unbounded in all futures.

INSERT INTO app_settings (
    key, value, value_kind, default_value,
    min_value, max_value, meaning, consumed_by, updated_at, updated_by
) VALUES
    -- Key and value on ONE line: `settings_store_tests`' `seeded_value_in`
    -- parses this file off disk for the literal `('key', 'value'`.
    ('gather_read_depth', '200',
     'count', '200',
     20, 2000,
     'How many candidates each half of a scenario''s evidence gather returns before they are fused into one ranked list. Both the vector read and the lexical read go to this same depth. Larger means more of the corpus is considered and a slower gather; smaller means a card has to rank higher in one of the two reads to survive into the list at all.',
     'services::gather_search',
     now(),
     'migration:gather_read_depth_setting')
ON CONFLICT (key) DO NOTHING;

-- Rule 25a: assert the END state. A statement matching zero rows is silent in
-- Postgres, and an INSERT that quietly did nothing would surface later as a
-- boot refusal pointing at code rather than at this file.
DO $$
DECLARE
    row_value   TEXT;
    row_kind    TEXT;
    row_min     NUMERIC;
    row_max     NUMERIC;
BEGIN
    SELECT value, value_kind, min_value, max_value
      INTO row_value, row_kind, row_min, row_max
      FROM app_settings
     WHERE key = 'gather_read_depth';

    IF row_value IS NULL THEN
        RAISE EXCEPTION
            'gather_read_depth was not seeded — the settings boot check will refuse to start';
    END IF;

    IF row_kind <> 'count' THEN
        RAISE EXCEPTION
            'gather_read_depth.value_kind is %, expected count — the reader checks the declared kind and would report a store that has drifted from the code',
            row_kind;
    END IF;

    -- The bounds must be present AND must admit the seeded value, or the first
    -- boot after this migration refuses on a row this file wrote.
    IF row_min IS NULL OR row_max IS NULL THEN
        RAISE EXCEPTION
            'gather_read_depth must carry both bounds; a retrieval depth with no ceiling is how a gather quietly becomes a full table scan';
    END IF;

    IF row_value::numeric < row_min OR row_value::numeric > row_max THEN
        RAISE EXCEPTION
            'gather_read_depth is % but its bounds are [%, %] — the seeded value would refuse its own boot check',
            row_value, row_min, row_max;
    END IF;
END $$;
