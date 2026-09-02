-- timeline_subsets_empty_story_wording — the sentence an empty story says.
--
-- Created: 2026-09-02 09:10:05
-- Target: pipeline database (colossus_legal_v2)
--
-- TRANSCRIBED from CC_TASK_TIMELINE_SUBSETS_POLISH_v1 §P1, which gives the
-- value in full: "This story has no events yet." Not invented here, and no
-- mockup was drawn — Screen 2 (design §11 item 2) has no empty state because
-- nobody drew one, which is the defect this row closes.
--
-- ## Domain note: what the window said before, and why a blank band is a bug
--
-- `SubsetWindowBody` maps the subset's events into the scrolling body. With
-- zero events that map produces nothing, so the body rendered as a blank padded
-- strip: the window opened, the title bar named the story, the footer said
-- "0 events", and the space between them said NOTHING. Seen on DEV through the
-- section's Preview.
--
-- A reader cannot tell that state from a window that failed to load — which is
-- the Standing Rule 1 failure exactly, two operationally distinct states with
-- one observable. The loading line and the error line beside it are both rows
-- already (`chronology_subsets_window_loading_label`, and the error's own
-- sentence comes from the server); this is the third state of the same slot and
-- it had no words.
--
-- ## Why "yet", and why "story" rather than "subset"
--
-- "yet" because an empty subset is a story somebody has started and not
-- finished picking events for — it is a stage of authoring, not a fault, and
-- the sentence should not read as an error beside the amber one that is.
-- "story" because that is the word this window uses for a subset throughout
-- (`chronology_subsets_window_loading_label` says "Loading the story…"); the
-- reader of this window is a witness, not the person who built the subset.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_window_no_events',
    'This story has no events yet.',
    'text',
    'This story has no events yet.',
    NULL, NULL,
    'What the floating window''s body says when the subset carries no events at all (design §11 item 2''s missing empty state, filed 2026-09-02 from DEV). Domain note: it says "yet" because an empty subset is a story somebody has started and not finished picking events for — a stage of authoring, not a fault — and it must not read as an error beside the amber sentence that is one. It says "story" because that is this window''s word for a subset throughout; its reader is a witness rehearsing, not the person who built it. Without this row the body rendered blank, which a reader cannot tell from a window that failed to load. The footer still says "0 events" from its own template; the two are not the same sentence and neither replaces the other.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

-- ── the END-state assertions (CLAUDE.md rule 25a) ───────────────────────────
--
-- A statement matching zero rows is silent in Postgres and the old value keeps
-- being served. These assert what must be TRUE when this migration has run,
-- not what this migration attempted.

DO $$
DECLARE
    n INTEGER;
BEGIN
    -- ── the row, at the value this build expects ────────────────────────────
    -- On key AND value together. A row count alone would pass on a pre-existing
    -- row holding some other sentence, which is the state an
    -- `ON CONFLICT DO NOTHING` can quietly produce.
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key = 'chronology_subsets_window_no_events'
       AND value = 'This story has no events yet.';
    IF n <> 1 THEN
        RAISE EXCEPTION
            'the empty-story sentence must be seeded at its stated value, found %', n;
    END IF;

    -- ── it carries no placeholder ───────────────────────────────────────────
    -- `cw` returns this row verbatim into the window's body: there is no `fill`
    -- on this path, so a stray {count} would put a literal brace on screen with
    -- nothing failing on the way. The footer's "{count} events" is the template
    -- beside it, and confusing the two is exactly how a brace would arrive.
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key = 'chronology_subsets_window_no_events' AND value LIKE '%{%';
    IF n <> 0 THEN
        RAISE EXCEPTION
            'the empty-story sentence carries a placeholder; it is rendered '
            'verbatim and would put a literal brace in the window (% rows)', n;
    END IF;

    -- ── the whole chronology block is still blank-free ──────────────────────
    -- Over the WHOLE prefix and not just this migration's row, for the reason
    -- every chronology wording migration since Phase C has given: a blank value
    -- ANYWHERE in the block stops the boot loader, and a migration that proved
    -- only its own half would let an earlier row go blank between deploys with
    -- nothing noticing.
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key LIKE 'chronology\_%' AND (value IS NULL OR btrim(value) = '');
    IF n <> 0 THEN
        -- The count alone tells an operator that something is wrong and not
        -- WHICH row. In CI the migration log is often the only artefact they
        -- get, so the query that names the offender travels with the failure.
        RAISE EXCEPTION
            'a chronology row is blank; the boot loader would refuse to start '
            '(% rows). To name them: SELECT key FROM app_settings WHERE key '
            'LIKE ''chronology\_%%'' AND (value IS NULL OR btrim(value) = '''')', n;
    END IF;
END $$;
