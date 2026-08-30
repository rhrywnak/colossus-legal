-- timeline_subsets_screen_wording — the seven words Screens 2 and 3 speak and
-- T1.2 did not seed.
--
-- Created: 2026-08-30 15:33:46
-- Target: pipeline database (colossus_legal_v2)
--
-- ## ⚑ WHY THERE IS A SECOND WORDING MIGRATION FOR ONE FEATURE
--
-- T1.2 declared sixteen rows one commit ahead of the screens that speak them,
-- which is the right habit and worked. It was sixteen short of enough: task 2
-- reached the picker and found eight strings on the approved mockup with no row
-- in the store — a subset's own event count, the add-modal's title, the picked
-- count, the pill's gap clause, two field labels and the per-row note
-- placeholder. Seven rows are seeded here; the eighth ("Open" on the subset row)
-- reuses `chronology_subsets_window_open_timeline`, whose stored words —
-- "Open on the timeline" — are exactly what that link does. Roman ruled both on
-- 2026-08-30.
--
-- The lesson for the next feature, recorded where the next author will be
-- looking: a wording block declared ahead of its screen is guessed, and a guess
-- is short. Count the strings ON the mockup, not the strings the design
-- discusses.
--
-- ## The two the mockup draws that are NOT here, deliberately
--
-- `Edit` on the subset row and `Save subset` in the modal footer. The store
-- already holds `chronology_edit_label` ("✎ Edit") and `chronology_save_label`
-- ("Save"), which say the same things in this app's own habit. Both are recorded
-- as DEVIATED in the T2 report rather than seeded as near-duplicates: two rows
-- meaning "edit" is how one of them goes stale.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_event_count_template',
    '{count} events',
    'text',
    '{count} events',
    NULL, NULL,
    'How many events one subset holds, on its row in the Subsets section (mockup Screen 2). {count} is the number. Domain note: this counts every REFERENCE the subset holds, gaps included — the amber chronology_subsets_gap_count_template line below it says how many of those are gaps. Two numbers and not one, for the reason the floating window''s footer gives: "15 events" over a list showing twelve live lines is the sentence that makes a reader distrust the count.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_form_add_title',
    'Add subset',
    'text',
    'Add subset',
    NULL, NULL,
    'The title of the subset modal when it is creating one (mockup Screen 3 draws the edit variant). Domain note: the EDIT variant reuses chronology_subsets_window_edit ("Edit subset"), which is the same words in the same place. This row exists because chronology_subsets_add_button carries a glyph — "+ Add subset" — and a heading is not a button.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_picked_count_template',
    '{count} picked',
    'text',
    '{count} picked',
    NULL, NULL,
    'How many events are ticked. {count} is the number. Domain note: ONE row for TWO places, because the mockup spells them identically — the pill at the top of the modal ("15 picked · 3 are gaps") and the suffix on each phase header inside the picker ("2008–2009 · 13 events · 9 picked"). A second row would be the one that eventually disagreed with the first.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_pill_gaps_template',
    '{count} are gaps',
    'text',
    '{count} are gaps',
    NULL, NULL,
    'The second half of the picker pill — "15 picked · 3 are gaps". {count} is how many picked events have been removed from the chronology. Domain note: its own row rather than part of the picked template because it is OMITTED at zero; a pill reading "15 picked · 0 are gaps" reports an absence as if it were news. Distinct from chronology_subsets_gap_count_template ("{count} gaps"), which is the amber line on the subset row — same fact, two sentences, because one is a count beside another count and the other stands alone.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_form_name_label',
    'Name',
    'text',
    'Name',
    NULL, NULL,
    'Labels the subset''s name field in the modal (mockup Screen 3). Domain note: NOT chronology_form_title_label ("Title"), which labels an EVENT''s title on a different form. A subset has a name and an event has a title, and the two forms sit one click apart — sharing a row would make one of them wrong the first time either is edited.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_form_description_label',
    'Description — what this story proves, one or two sentences',
    'text',
    'Description — what this story proves, one or two sentences',
    NULL, NULL,
    'Labels the subset''s description field, and instructs while it labels (mockup Screen 3, verbatim). Domain note: the instruction is the point. "Description" alone gets a restatement of the name; "what this story proves" gets the sentence the subset exists to make, which is what the floating window shows the reader above the events. The dash is an EM-DASH (U+2014).',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_note_placeholder',
    'note',
    'text',
    'note',
    NULL, NULL,
    'The placeholder in each picked row''s one-line note field (mockup Screen 3). Domain note: lowercase and bare, because it sits inside a dense list of ticked rows where a sentence would shout; chronology_subsets_picker_hint above the list is where the note is explained. NOT chronology_add_note_placeholder ("Add a note…"), which is the event page''s note box — a different control on a different screen writing to a different table.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

-- ── the END-state assertions (CLAUDE.md rule 25a) ───────────────────────────
--
-- A statement matching zero rows is silent in Postgres and the old value keeps
-- being served. These assert what must be TRUE when this migration has run, not
-- what it did.

DO $$
DECLARE
    n INTEGER;
BEGIN
    -- ── the seven new rows ───────────────────────────────────────────────────
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key IN (
            'chronology_subsets_event_count_template',
            'chronology_subsets_form_add_title',
            'chronology_subsets_picked_count_template',
            'chronology_subsets_pill_gaps_template',
            'chronology_subsets_form_name_label',
            'chronology_subsets_form_description_label',
            'chronology_subsets_note_placeholder'
     );
    IF n <> 7 THEN
        RAISE EXCEPTION
            'the timeline-subset screen wording must hold all 7 rows, found %', n;
    END IF;

    -- ── the three templates still carry the placeholder their caller fills ──
    -- A row seeded without its `{count}` renders a sentence with the number
    -- missing from it, which is quieter than a crash and worse: "picked" with
    -- no count, on a pill whose only job is the count.
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key IN (
            'chronology_subsets_event_count_template',
            'chronology_subsets_picked_count_template',
            'chronology_subsets_pill_gaps_template'
     )
       AND value LIKE '%{count}%';
    IF n <> 3 THEN
        RAISE EXCEPTION
            'all 3 new subset templates must carry {count}, only % do', n;
    END IF;

    -- ── the whole chronology block is still blank-free ──────────────────────
    -- Over the WHOLE prefix and not just this migration's rows, for the reason
    -- the Phase C and T1.2 migrations both gave: a blank value ANYWHERE in the
    -- block stops the boot loader, and a migration that proved only its own half
    -- would let an earlier row go blank between deploys with nothing noticing.
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key LIKE 'chronology\_%' AND (value IS NULL OR btrim(value) = '');
    IF n <> 0 THEN
        RAISE EXCEPTION
            'a chronology row is blank; the boot loader would refuse to start (% rows)', n;
    END IF;
END $$;
