-- timeline_subsets_t4_footer_events_template — the window's footer, corrected.
--
-- Created: 2026-08-31 12:57:28
-- Target: pipeline database (colossus_legal_v2)
--
-- ## ⚑ WHAT WAS WRONG, IN ROMAN'S WORDS
--
-- The floating window's footer read "15 on the chronology · 0 gaps" where the
-- approved mockup (Screen 2) draws "15 events · 2 ⚑". Those are not the same
-- sentence with different numbers in it — they are DIFFERENT NUMBERS wearing the
-- same clothes. {gaps} counts events soft-deleted off the chronology; the ⚑
-- counts rows whose DATE is unsettled. A reader who saw "0 gaps" beside a story
-- carrying two unconfirmed dates was being told something true about a question
-- nobody asked.
--
-- T4 shipped the wrong footer and reported it as a DEVIATED row rather than
-- fixing it. Ruled 2026-08-31: fix it.
--
-- ## ONE ROW SEEDED, ONE ROW RETIRED
--
-- Seeded: chronology_subsets_window_footer_events_template = "{count} events".
--
-- Retired: chronology_subsets_window_footer_template, whose value
-- "{on_chronology} on the chronology · {gaps} gaps" nothing asks for any more.
-- The DELETE is not optional housekeeping — the reach test
-- `no_declared_word_is_left_with_no_asker` refuses a row that is seeded,
-- mirrored and paid for that no screen speaks, and it offers exactly three ways
-- out: wire it up, retire it, or promise a screen. There is no screen coming for
-- this one, so it is retired.
--
-- ## WHY THE ⚑ HALF IS NOT A ROW
--
-- The footer composes " · {n} ⚑" in code and omits it entirely when n is 0. A
-- middle dot, a number and a glyph carry no language — there is nothing in them
-- for an editor to change and nothing for a translator to translate — which is
-- the same split this feature already makes for the title bar's ⧉ ⇲ – ×, where
-- the glyph is in code and its accessible NAME is a stored row. The words in
-- this footer ("events") are the row; the punctuation around the count is not.
--
-- Domain note on {count}: the TOTAL the subset holds, gaps included — the same
-- number the title bar shows, so a reader cannot find two different counts of
-- one story on one window. The gap rows stay marked individually by
-- chronology_subsets_gap_badge_label, which is where a gap is visible and
-- actionable; the footer no longer carries a second tally of them.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_window_footer_events_template',
    '{count} events',
    'text',
    '{count} events',
    NULL, NULL,
    'The floating window''s footer count — "15 events" (mockup v2 Screen 2). {count} is every reference the subset holds, gaps included, which is the SAME number the title bar shows so one window cannot report two counts of one story. Domain note: the footer may also carry " · {n} ⚑" after this, composed in code and omitted when n is zero — a middle dot, a number and a glyph carry no language, so only the word "events" is stored. This row REPLACES chronology_subsets_window_footer_template, which said "{on_chronology} on the chronology · {gaps} gaps" and answered a question nobody was asking.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

-- ── the retirement ──────────────────────────────────────────────────────────
DELETE FROM app_settings
 WHERE key = 'chronology_subsets_window_footer_template';

-- ── the END-state assertions (CLAUDE.md rule 25a) ───────────────────────────
--
-- A statement matching zero rows is silent in Postgres and the old value keeps
-- being served. These assert what must be TRUE when this migration has run —
-- and this migration has BOTH an insert and a delete, so both directions are
-- asserted. A DELETE that matched nothing is exactly as silent as an INSERT
-- that did.

DO $$
DECLARE
    n INTEGER;
BEGIN
    -- ── the new row is there, and carries its placeholder ───────────────────
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key = 'chronology_subsets_window_footer_events_template'
       AND value LIKE '%{count}%';
    IF n <> 1 THEN
        RAISE EXCEPTION
            'the window footer template must exist and carry {count}, found % '
            'matching rows; the footer would render the word "events" with no '
            'number in front of it', n;
    END IF;

    -- ── the old row is GONE ─────────────────────────────────────────────────
    -- Asserted, not assumed. A DELETE naming a key that was already spelled
    -- differently removes nothing and says nothing, and the boot loader would
    -- then keep serving a row this build no longer reads.
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key = 'chronology_subsets_window_footer_template';
    IF n <> 0 THEN
        RAISE EXCEPTION
            'chronology_subsets_window_footer_template must be retired, % rows '
            'remain', n;
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
