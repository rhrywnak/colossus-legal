-- timeline_subsets_compact_wording — the two words the "Dates only" view speaks.
--
-- Created: 2026-09-02 08:46:26
-- Target: pipeline database (colossus_legal_v2)
--
-- Both values are TRANSCRIBED from CC_TASK_TIMELINE_SUBSETS_COMPACT_v1 §C1,
-- which names them in Roman's own words: the control "reads 'Dates only' when
-- the window is showing details, and 'Show details' when it is compact". Not
-- invented here, and no mockup was drawn because this screen SUBTRACTS from
-- the approved Screen 2 (design §11 item 2) and adds one text button in a
-- footer that already carries two.
--
-- ## Domain note: why this view exists, and why it is two words and not one
--
-- The subset window is right for reference and wrong for the night before
-- Marie testifies. The other side's play is "she cannot keep the events
-- straight" — so what she rehearses is DATES against titles she already knows,
-- and every fact paragraph, tag pill and story note between them is a row she
-- has to read past to reach the next date.
--
-- The control is a TOGGLE and therefore has two labels, not one. A single
-- label ("Dates only") that stayed put once pressed would leave the reader
-- with no word on screen for the way back — she would be looking at a button
-- naming the state she is already in. Each label names what pressing it DOES,
-- which is the same rule the ⧉ / ⇲ pair follows two commits back.
--
-- ## ⚑ These are the mirror of each other. Edit them together.
--
-- If one is reworded and the other is not, the button changes vocabulary
-- halfway through a toggle — "Dates only" going to "Show the whole row" — and
-- nothing in this build would fail. That is exactly the class of drift the
-- precision-caption pair (`month · approx.` / `year · approx.`) records, and
-- the reason both rows say so in their `meaning`.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_window_dates_only',
    'Dates only',
    'text',
    'Dates only',
    NULL, NULL,
    'The floating window''s footer control while the window is showing FULL rows; pressing it strips every row to its date and title. Domain note: the view it opens is for rehearsal, not reference — the night before testimony the witness knows the order and is refreshing the dates, and the fact paragraphs, tag pills and story notes are what push fifteen events off the window. It says "dates only" and not "compact" or "collapse" because it names what survives rather than what happens to the row. The MIRROR of chronology_subsets_window_show_details, which is the same button once it has been pressed — edit the two together or the toggle changes vocabulary halfway through.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_window_show_details',
    'Show details',
    'text',
    'Show details',
    NULL, NULL,
    'The floating window''s footer control while the window is COMPACT; pressing it brings back the description strip, the fact paragraphs, the tag pills and the story notes. The mirror of chronology_subsets_window_dates_only — same button, other state — and the reason the pair exists rather than one fixed label: a button that kept saying "Dates only" after it had been pressed would name the state the reader is already in and leave no word on screen for the way back. Domain note: what it restores is DETAIL, never a row the compact view had dropped — the compact view hides children and drops nothing, so the gap badge on an event removed from the chronology is visible in both.',
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
    -- ── the two new rows, at the values this build expects ──────────────────
    -- Asserted on key AND value together. A row count alone would pass on a
    -- pre-existing row holding some other sentence, which is the state an
    -- `ON CONFLICT DO NOTHING` can quietly produce.
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE (key = 'chronology_subsets_window_dates_only'   AND value = 'Dates only')
        OR (key = 'chronology_subsets_window_show_details' AND value = 'Show details');
    IF n <> 2 THEN
        RAISE EXCEPTION
            'the compact-view wording must hold both rows at their seeded values, found %', n;
    END IF;

    -- ── neither label carries a placeholder ─────────────────────────────────
    -- The mirror of the T4 template assertion, inverted. These two are the only
    -- kind of row this block seeds that must NOT be filled: `cw` returns them
    -- verbatim to a button's text, so a stray {count} would ship a literal
    -- brace to screen with nothing failing on the way.
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key IN ('chronology_subsets_window_dates_only',
                   'chronology_subsets_window_show_details')
       AND value LIKE '%{%';
    IF n <> 0 THEN
        RAISE EXCEPTION
            'a compact-view label carries a placeholder; it is rendered verbatim '
            'and would put a literal brace on the button (% rows)', n;
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
