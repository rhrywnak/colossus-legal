-- timeline_subsets_t4_window_wording — the six words T4 needs.
--
-- Created: 2026-08-31 11:22:13
-- Target: pipeline database (colossus_legal_v2)
--
-- All six are TRANSCRIBED from TIMELINE_SUBSET_MOCKUP_v2_2026-08-31.html —
-- Screen 2 (light and dark) and Screen 5, approved as drawn by Roman the same
-- morning (design §11). Not invented: each value below appears in the mockup's
-- own markup, and the comment on each row says where.
--
-- ## ⚑ EVERY ARIA-LABEL AND TITLE IS A ROW — the T3 lesson, still standing
--
-- The rule the 08-30 migration recorded holds here and is why the first two
-- rows exist at all: an aria-label is a user-visible string because a screen
-- reader says it out loud, and it is the one class of string no screenshot,
-- no tsc run and no reach test can catch. `chronology_subsets_window_popout_label`
-- and `chronology_subsets_window_popin_label` are the accessible names of the
-- ⧉ and ⇲ glyphs, which say nothing on their own.
--
-- ## The four that are read on screen
--
-- The date-to-confirm badge, the two precision captions, and the divider
-- template. The precision captions are the SECOND line of the date column
-- (mockup `.fev .d small` — "2009 · month · approx.") and the divider template
-- is the year rule when a story crosses a phase boundary (mockup `.ydiv`,
-- "2009 · probate").
--
-- ## Domain note: what the ⚑ badge actually marks in THIS build
--
-- The mockup draws the badge on the Milster handoff because Roman entered that
-- date from recollection. Nothing in the data says so: measured on DEV
-- 2026-08-31, no chronology_events row carries "to confirm" in its fact, no
-- event_link label or pinpoint carries it, and `attributes` holds only tags and
-- legacy source ids. So the badge ships on `approximate = true` alone, which on
-- "The $50,000" marks exactly the two rows the mockup marks. If a first-class
-- "date to confirm" flag is later added to the event, this row's meaning is
-- where the change starts. See the T4 report, NEEDS A RULING.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_window_popout_label',
    'Pop out to its own window',
    'text',
    'Pop out to its own window',
    NULL, NULL,
    'The accessible name and tooltip of the floating window''s ⧉ control, which reopens the same story as a separate always-on-top desktop window (design §11 item 5, mockup Screen 5). Domain note: it says "its own window" and not "full screen" or "detach" because that is literally what happens — the panel becomes a real OS window the reader can drag to a second monitor and keep beside the app. Transcribed from the mockup''s own title attribute on the .pop button.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_window_popin_label',
    'Back into the page',
    'text',
    'Back into the page',
    NULL, NULL,
    'The accessible name and tooltip of the ⇲ control in the popped-out window''s bar, which closes that window and restores the in-page one where it was. Domain note: the mirror of chronology_subsets_window_popout_label — edit the two together. It says "back into the page" and not "close" because × is beside it and DOES close: ⇲ returns the story to where it came from, × puts it away entirely. Transcribed from the mockup''s Screen 5 bar.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_date_to_confirm_badge',
    'date to confirm',
    'text',
    'date to confirm',
    NULL, NULL,
    'The amber pill beside an approximate date in the floating window (mockup Screen 2, .fev .flag). Domain note: it marks the DATE, not the fact — the event happened, and what is unsettled is when. That distinction is the whole reason the badge exists: an approximate date rendered plainly is read as a fact by the next person to quote it, and this story is quoted in a brief. NOT the same row as chronology_subsets_gap_badge_label, which marks an event soft-deleted from the chronology; a row can carry either, both, or neither.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_precision_month_label',
    'month · approx.',
    'text',
    'month · approx.',
    NULL, NULL,
    'The caption under a month-precision approximate date in the floating window — the second line of the date column reads "2009 · month · approx." (mockup Screen 2, .fev .d small). Domain note: it says the SOURCE only stated a month, so "1 April 2009" would be a fabricated day. That is the class of mistake the precision vocabulary exists to prevent, and the caption is where a reader is told. The separator is a middle dot (U+00B7) with a space either side, matching the mockup and the store''s other compound captions.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_precision_year_label',
    'year · approx.',
    'text',
    'year · approx.',
    NULL, NULL,
    'The caption under a year-precision approximate date in the floating window — "2009 · year · approx." (mockup Screen 2). The mirror of chronology_subsets_precision_month_label; edit the two together or one will eventually say "approx." while the other says "approximate".',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_year_phase_divider_template',
    '{year} · {phase}',
    'text',
    '{year} · {phase}',
    NULL, NULL,
    'The floating window''s divider when the story crosses a phase boundary — "2009 · probate" (mockup Screen 2, .ydiv). {year} is the calendar year the rows below belong to; {phase} is the phase''s own label, lower-cased as the mockup draws it. Domain note: a divider carrying ONLY the phase was T3''s rule and is withdrawn — a story told in dates needs the year on every rule, and the phase is the extra fact on the one rule where it changed. When the year turns inside a single phase the divider is the bare year and this template is not used.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

-- ── the END-state assertions (CLAUDE.md rule 25a) ───────────────────────────
--
-- A statement matching zero rows is silent in Postgres and the old value keeps
-- being served. These assert what must be TRUE when this migration has run.

DO $$
DECLARE
    n INTEGER;
BEGIN
    -- ── the six new rows ────────────────────────────────────────────────────
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key IN (
            'chronology_subsets_window_popout_label',
            'chronology_subsets_window_popin_label',
            'chronology_subsets_date_to_confirm_badge',
            'chronology_subsets_precision_month_label',
            'chronology_subsets_precision_year_label',
            'chronology_subsets_year_phase_divider_template'
     );
    IF n <> 6 THEN
        RAISE EXCEPTION
            'the T4 timeline-subset window wording must hold all 6 rows, found %', n;
    END IF;

    -- ── the divider template still carries BOTH placeholders ────────────────
    -- Asserted separately from the row count because a template that lost one
    -- of its two fills is a row that EXISTS and renders a half-sentence — the
    -- failure mode a count can never see.
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key = 'chronology_subsets_year_phase_divider_template'
       AND value LIKE '%{year}%' AND value LIKE '%{phase}%';
    IF n <> 1 THEN
        RAISE EXCEPTION
            'the year-phase divider template must carry {year} AND {phase}, or a '
            'cross-phase rule renders one of the two facts and drops the other';
    END IF;

    -- ── the whole chronology block is still blank-free ──────────────────────
    -- Over the WHOLE prefix and not just this migration's rows, for the reason
    -- every chronology wording migration since Phase C has given: a blank value
    -- ANYWHERE in the block stops the boot loader, and a migration that proved
    -- only its own half would let an earlier row go blank between deploys with
    -- nothing noticing.
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key LIKE 'chronology\_%' AND (value IS NULL OR btrim(value) = '');
    IF n <> 0 THEN
        RAISE EXCEPTION
            'a chronology row is blank; the boot loader would refuse to start (% rows)', n;
    END IF;
END $$;
