-- gather_subject_filter — which parties a ranked gather may reach.
--
-- ## Why this is a row and not a constant
--
-- It decides WHAT THE SEARCH CAN SEE, which is the single most consequential
-- knob in the gather cascade, and the answer is not the same for every
-- scenario the case will ever have. More immediately: when a card is missing
-- from a gather, the first diagnostic question is "is this a filter problem or
-- a ranking problem?", and that question is only answerable by a human who can
-- flip this to `off` and look. A compiled-in default would make the diagnosis
-- a rebuild.
--
-- ## The three values
--
--   strict  — the subject only. This is the behaviour before the cascade, kept
--             as the CONSERVATION BASELINE: every card `strict` returns must
--             still appear in the ranked list under any other mode.
--   widened — the subject plus every party the scenario's linked allegations
--             name. THE DEFAULT, and the value AT-2 turns on: four of the seven
--             $50,000 admissions S-11 must reach are filed ABOUT Emil Awad
--             alone, so `strict` recovers three of seven however good the
--             ranking is, because the read never sees the other four.
--   off     — no party filter. A diagnostic, not a mode to run in.
--
-- The vocabulary lives in `domain::gather_filter::GatherSubjectFilter` and is
-- validated at boot: a row holding anything else is a startup refusal naming
-- the three legal spellings, not a silent fall back to a default that searches
-- a different pool.
--
-- `value_kind` is `text` rather than a new kind: the store's kinds describe how
-- to PARSE a value (float, count, ratio, text), and this is text that code then
-- interprets — the same shape as `theme_scan_default_model`, which is likewise
-- a closed vocabulary stored as text and validated in code. `min_value` and
-- `max_value` stay NULL because a bound on a word means nothing.

INSERT INTO app_settings (
    key, value, value_kind, default_value,
    min_value, max_value, meaning, consumed_by, updated_at, updated_by
) VALUES
    -- Key and value on ONE line, deliberately: `settings_store_tests`'
    -- `seeded_value_in` parses this file off disk looking for the literal
    -- `('key', 'value'` so the test fixture and this migration cannot drift
    -- apart. Split across two lines it finds nothing and the row reads as
    -- unseeded (CLAUDE.md §4 rule 21).
    ('gather_subject_filter', 'widened',
     'text', 'widened',
     NULL, NULL,
     'Which parties a scenario''s evidence gather may reach: strict (the subject only — the behaviour before the ranked gather), widened (the subject plus every party the scenario''s linked allegations name — the default), or off (no party filter, for diagnosing whether a missing card is a filter problem or a ranking problem).',
     'services::gather_search',
     now(),
     'migration:gather_subject_filter_setting')
ON CONFLICT (key) DO NOTHING;

-- Rule 25a: a statement that matches zero rows is silent in Postgres, and an
-- INSERT that quietly did nothing would leave the boot check to fail later with
-- a missing-key error pointing at code rather than at this file. Assert the END
-- state — the row exists, holds a legal value, and is declared text.
DO $$
DECLARE
    row_value      TEXT;
    row_kind       TEXT;
    row_default    TEXT;
BEGIN
    SELECT value, value_kind, default_value
      INTO row_value, row_kind, row_default
      FROM app_settings
     WHERE key = 'gather_subject_filter';

    IF row_value IS NULL THEN
        RAISE EXCEPTION
            'gather_subject_filter was not seeded — the settings boot check will refuse to start';
    END IF;

    IF row_kind <> 'text' THEN
        RAISE EXCEPTION
            'gather_subject_filter.value_kind is %, expected text — the reader checks the declared kind and would report a store that has drifted from the code',
            row_kind;
    END IF;

    -- Both the live value and the shipped default must be in the vocabulary.
    -- The default is checked too because it is displayed beside the current
    -- value so a human can see what they changed FROM; an illegal default would
    -- offer them an illegal value to go back to.
    IF row_value NOT IN ('strict', 'widened', 'off') THEN
        RAISE EXCEPTION
            'gather_subject_filter holds %, which is not one of strict, widened, off',
            row_value;
    END IF;

    IF row_default NOT IN ('strict', 'widened', 'off') THEN
        RAISE EXCEPTION
            'gather_subject_filter.default_value is %, which is not one of strict, widened, off',
            row_default;
    END IF;
END $$;
