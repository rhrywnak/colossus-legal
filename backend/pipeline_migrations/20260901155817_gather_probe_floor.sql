-- gather_probe_floor — how many trigram probes survive when EVERY probe is over
-- `gather_probe_max_share`.
--
-- The guard it serves is "never run zero probes": a gather whose trigram half
-- searched for nothing would return a lexical ranking built from full text
-- alone and look exactly like one where the trigram half worked and found
-- nothing. That is the same invisible failure as the silent truncation and the
-- 0-row `%` operator, and it is why a floor exists at all.
--
-- ## Why the NUMBER is a row and the GUARD is not
--
-- "Never zero" is the invariant and it is not negotiable. How many above zero
-- is a judgement: three was chosen so the fusion still has something to agree
-- about — a single surviving probe contributes a ranking nothing can
-- corroborate, and corroboration between probes is the entire reason the
-- per-probe lists exist. That is a design argument, not a protocol fact, and
-- Rule 13 sends a threshold with no external anchor to configuration.
--
-- (Contrast the four-character minimum probe length, which stays compiled:
-- pg_trgm decomposes into three-character runs, so the floor there is fixed by
-- the index's own arithmetic and no deployment can move it.)
--
-- Bounds: at least 1, because 0 is the state the guard exists to prevent and
-- storing it would disable the guard through the settings page. At most 25,
-- which is above any probe count measured (31 extracted for S-11, of which 28
-- were kept) while still refusing a value that would make the floor meaningless.

INSERT INTO app_settings (
    key, value, value_kind, default_value,
    min_value, max_value, meaning, consumed_by, updated_at, updated_by
) VALUES
    -- Key and value on ONE line: `settings_store_tests`' `seeded_value_in`
    -- parses this file off disk for the literal `('key', 'value'`.
    ('gather_probe_floor', '3',
     'count', '3',
     1, 25,
     'How many trigram probes are kept when every one of them matches more than the allowed share of a scenario''s admitted evidence. The trigram half must never search for nothing — that would look exactly like a search that worked and found nothing — so the most selective probes are kept even when all of them are too generic. Three rather than one so the fused ranking still has agreement between probes to work with.',
     'services::gather_search',
     now(),
     'migration:gather_probe_floor')
ON CONFLICT (key) DO NOTHING;

-- Rule 25a: assert the END state.
DO $$
DECLARE
    row_value TEXT;
    row_kind  TEXT;
    row_min   NUMERIC;
BEGIN
    SELECT value, value_kind, min_value
      INTO row_value, row_kind, row_min
      FROM app_settings
     WHERE key = 'gather_probe_floor';

    IF row_value IS NULL THEN
        RAISE EXCEPTION
            'gather_probe_floor was not seeded — the settings boot check will refuse to start';
    END IF;

    IF row_kind <> 'count' THEN
        RAISE EXCEPTION
            'gather_probe_floor.value_kind is %, expected count', row_kind;
    END IF;

    -- The floor exists to prevent zero. A minimum of 0 would let the settings
    -- page switch the guard off, which is the one thing it must not permit.
    IF row_min IS NULL OR row_min < 1 THEN
        RAISE EXCEPTION
            'gather_probe_floor.min_value is % — it must be at least 1, or the guard against an empty trigram half could be disabled from the settings page',
            coalesce(row_min::text, 'NULL');
    END IF;

    IF row_value::numeric < 1 THEN
        RAISE EXCEPTION
            'gather_probe_floor is %, which would allow zero probes — the state the guard exists to prevent',
            row_value;
    END IF;
END $$;
