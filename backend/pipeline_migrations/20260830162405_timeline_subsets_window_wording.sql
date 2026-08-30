-- timeline_subsets_window_wording — the six words task 3 needs.
--
-- Created: 2026-08-30 16:24:05
-- Target: pipeline database (colossus_legal_v2)
--
-- Two of these close a gap task 2 left open on purpose, and four are declared
-- ahead of the screen that speaks them — the habit T1.2 established.
--
-- ## ⚑ THE ARIA-LABEL LESSON, NOW A ROW INSTEAD OF A COMMENT
--
-- Task 2 shipped the picker's reorder buttons with no accessible name at all.
-- They had carried `aria-label={`${title} — earlier`}` until the rules gate
-- caught it: an aria-label IS a user-visible string — a screen reader says it
-- out loud — so that was hardcoded English under the standing rule, in the one
-- place nothing on screen shows and no screenshot can catch. The English came
-- out, the glyph became the accessible name, and the gap was recorded rather
-- than papered over. `move_earlier_label` and `move_later_label` close it here.
--
-- The general rule, recorded where the next author will look: EVERY aria-label,
-- title, alt and placeholder is a wording row. Neither the reach test (which
-- scans for `cw(` calls, not for string literals), nor tsc, nor vitest can see
-- one. Only a reader can.
--
-- ## The four the floating window needs
--
-- Transcribed from the approved mockup's Screen 1, not invented: the title
-- bar's two controls, its event count, and the amber badge on a row whose event
-- is gone from the chronology.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_move_earlier_label',
    'Move earlier in the story',
    'text',
    'Move earlier in the story',
    NULL, NULL,
    'The accessible name of the picker''s ▲ control, which moves one picked event earlier in the story order. Domain note: it exists because an aria-label is a user-visible string and the standing rule admits no exception for the ones only a screen reader speaks. Says "in the story" and not "up" because the order is a narrative order, not a screen position — the same distinction ruling 2026-08-30 (1) makes between date order and the order somebody chose.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_move_later_label',
    'Move later in the story',
    'text',
    'Move later in the story',
    NULL, NULL,
    'The accessible name of the picker''s ▼ control. The mirror of chronology_subsets_move_earlier_label; edit the two together or one will eventually say "up" while the other says "later".',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_window_minimize_label',
    'Minimize',
    'text',
    'Minimize',
    NULL, NULL,
    'The accessible name and tooltip of the floating window''s – control, which collapses it to its title bar pinned bottom-right (design §5C). Domain note: the glyph is an en-dash on screen and says nothing to a screen reader, which is the whole reason this row exists. American spelling, matching the store''s existing convention.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_window_close_label',
    'Close',
    'text',
    'Close',
    NULL, NULL,
    'The accessible name and tooltip of the floating window''s × control. Domain note: close HIDES the window; the View Timeline button reopens it. It is not a delete and must never read like one.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_window_events_count_template',
    '{count} events',
    'text',
    '{count} events',
    NULL, NULL,
    'The count in the floating window''s title bar — "The $50,000 · 15 events". {count} is the number of references the subset holds, gaps included. Domain note: its VALUE is identical to chronology_subsets_event_count_template, which says the same thing on the Subsets section''s row. Two rows on purpose, per the task instruction: the title bar is a cramped strip beside a name and a selector, and the row is a table cell, so the two are expected to diverge under editing. If they never do, collapse them — see the T3 report''s FINDINGS.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_gap_badge_label',
    'Not on the chronology',
    'text',
    'Not on the chronology',
    NULL, NULL,
    'The amber badge on a floating-window row whose event has been soft-deleted from the chronology (design R1, mockup Screen 1). Domain note: the badge is the SHORT form and chronology_subsets_removed_event_line is the sentence — the badge sits inline beside a title where a clause would not fit, the line sits under it. Both mark the same fact, which the design calls half the value of a subset: the story saying "this happened and it is not on our timeline yet".',
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
    -- ── the six new rows ─────────────────────────────────────────────────────
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key IN (
            'chronology_subsets_move_earlier_label',
            'chronology_subsets_move_later_label',
            'chronology_subsets_window_minimize_label',
            'chronology_subsets_window_close_label',
            'chronology_subsets_window_events_count_template',
            'chronology_subsets_gap_badge_label'
     );
    IF n <> 6 THEN
        RAISE EXCEPTION
            'the timeline-subset window wording must hold all 6 rows, found %', n;
    END IF;

    -- ── the one template still carries the placeholder its caller fills ─────
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key = 'chronology_subsets_window_events_count_template'
       AND value LIKE '%{count}%';
    IF n <> 1 THEN
        RAISE EXCEPTION
            'the window count template must carry {count}, or the title bar '
            'renders a name and the word "events" with no number between them';
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
