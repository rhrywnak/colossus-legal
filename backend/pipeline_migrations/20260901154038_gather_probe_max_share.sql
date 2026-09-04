-- gather_probe_max_share — the share of the admitted set above which a trigram
-- probe is dropped as saying nothing.
--
-- ## Why a probe that matches everything is worse than no probe
--
-- The trigram half runs one query per probe and fuses the per-probe rankings, so
-- a card several probes agree on outranks one a single probe found. That is the
-- whole point of the fan-out — and it is exactly what a probe matching most of
-- the corpus destroys, because it agrees with everything.
--
-- Measured on the S-11 gather, 2026-09-01, against an admitted set of 1030:
--
--   Court     534 of 1030   (52%)     $50,000    73 of 1030   (7%)
--   Probate   over the read depth     Hanley     64
--   Awad      over the read depth     Tighe      35
--
-- Eleven of S-11's thirty-one probes were capitalised legal common nouns —
-- Court, Probate, Attorney, Defendant, Plaintiff, Personal, Representative,
-- County, August, Upon, Everything, Eventually — each contributing a near
-- uniform list to the fusion, competing with the figure the scenario turns on.
--
-- ## Why a SHARE and not a count
--
-- It scales with the admitted set rather than the corpus. A scenario whose
-- widening admits 40 cards and one whose widening admits 1030 need the same
-- rule, and a fixed count would be far too strict for the first and useless for
-- the second.
--
-- ## Why a ROW and not a constant
--
-- It is a threshold, which Rule 13 sends to configuration; and more to the
-- point, one third is a first value chosen from one case's numbers. Whoever
-- finds it wrong should be able to try another without a rebuild — and the
-- architect, not the implementer, decides when it moves, which is only possible
-- if it is data.
--
-- Stored as a ratio (`1/3`) rather than a float so the intent survives exactly:
-- `0.333` is a number somebody rounded, `1/3` is the value that was meant.

INSERT INTO app_settings (
    key, value, value_kind, default_value,
    min_value, max_value, meaning, consumed_by, updated_at, updated_by
) VALUES
    -- Key and value on ONE line: `settings_store_tests`' `seeded_value_in`
    -- parses this file off disk for the literal `('key', 'value'`.
    ('gather_probe_max_share', '1/3',
     'ratio', '1/3',
     NULL, NULL,
     'A trigram probe matching more than this share of a scenario''s admitted evidence is dropped before it is read: a probe that matches most of the pool agrees with everything and so distinguishes nothing, while crowding out the figures and names the scenario actually turns on. Lower is stricter. Probes matching nothing are always kept — a term the corpus does not contain is information, not noise — and the most selective probes are always kept even if every one of them is over this share.',
     'services::gather_search',
     now(),
     'migration:gather_probe_max_share')
ON CONFLICT (key) DO NOTHING;

-- Rule 25a: assert the END state. A statement matching zero rows is silent in
-- Postgres, and an INSERT that quietly did nothing would surface later as a boot
-- refusal pointing at code rather than at this file.
DO $$
DECLARE
    row_value   TEXT;
    row_kind    TEXT;
    row_default TEXT;
    numerator   INTEGER;
    denominator INTEGER;
BEGIN
    SELECT value, value_kind, default_value
      INTO row_value, row_kind, row_default
      FROM app_settings
     WHERE key = 'gather_probe_max_share';

    IF row_value IS NULL THEN
        RAISE EXCEPTION
            'gather_probe_max_share was not seeded — the settings boot check will refuse to start';
    END IF;

    IF row_kind <> 'ratio' THEN
        RAISE EXCEPTION
            'gather_probe_max_share.value_kind is %, expected ratio — the reader checks the declared kind and would report a store that has drifted from the code',
            row_kind;
    END IF;

    IF row_value !~ '^[0-9]+/[0-9]+$' OR row_default !~ '^[0-9]+/[0-9]+$' THEN
        RAISE EXCEPTION
            'gather_probe_max_share must be written n/m; value is % and default is %',
            row_value, row_default;
    END IF;

    numerator   := split_part(row_value, '/', 1)::integer;
    denominator := split_part(row_value, '/', 2)::integer;

    -- A zero denominator is a division by nothing; a share above one drops
    -- nothing at all and would look like the feature is off rather than broken.
    IF denominator = 0 THEN
        RAISE EXCEPTION 'gather_probe_max_share has a zero denominator';
    END IF;

    IF numerator > denominator THEN
        RAISE EXCEPTION
            'gather_probe_max_share is %, a share above 1 — no probe could ever exceed it, so the rule would silently do nothing',
            row_value;
    END IF;
END $$;
