-- t5_scenario_header_button_labels — the last three words in the strip.
--
-- Created: 2026-08-31 14:34:04
-- Target: pipeline database (colossus_legal_v2)
--
-- ## ⚑ WHAT THESE CLOSE
--
-- `ScenarioHeaderStrip.tsx` shipped in T5 with three HARDCODED English labels —
-- "✎ Edit", "Rehearsal view" and "Delete". They were carried forward verbatim
-- from `ScenarioHeaderTiers.tsx`, the header T5 deletes, where they had been
-- literals for as long as that component existed. T5.2 named them as literals in
-- its own prose and T5.1 budgeted no rows for them, so they came across as they
-- were and were reported as OWED rather than fixed quietly.
--
-- Roman approved these three on 2026-08-31 (round two). After this migration
-- there is ZERO English in the strip, which the report proves by grep.
--
-- ## Why the ✎ is NOT in the value
--
-- The stored word is "Edit"; the pencil stays in code. That is the same split
-- the floating window's ⧉ ⇲ – × already make and the same one the Timeline
-- subsets section makes with its ✓ and its + → : a glyph carries no language —
-- there is nothing in it for an editor to reword and nothing for a translator to
-- translate — while the word beside it is exactly the thing somebody might want
-- to change. Putting the glyph in the row would also mean an editor who wanted
-- "Rename" had to know to keep a pencil in front of it.
--
-- ## Domain note on the block these join
--
-- These are `scenario_header_*` and not `scenario_edit_subsets_*`: they belong
-- to the header strip, which renders on three surfaces, and not to the Timeline
-- subsets section, which renders on one. A future task that retires the strip
-- retires these with it and leaves the section's ten alone.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('scenario_header_edit_label',
    'Edit',
    'text',
    'Edit',
    NULL, NULL,
    'The label on the scenario header strip''s control that opens the identity editor (mockup v2 Screen 1, row 2). Domain note: the ✎ glyph the strip draws in front of it lives in code — the word is what an editor would change, the pencil is furniture. It edits the scenario''s IDENTITY (name, definition, attack, theme, motivation, allegations) and never its status: declaring a scenario Ready is a recorded human act with its own control, two inches to the left on row 1.',
    NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('scenario_header_rehearsal_view_label',
    'Rehearsal view',
    'text',
    'Rehearsal view',
    NULL, NULL,
    'The label on the strip control that opens Marie''s testimony-prep view for THIS scenario. Domain note: the control is INERT unless the scenario is Ready, and scenario_rehearsal_disabled_tooltip says why on hover — before .390 it looked alive at every status and, clicked on a Draft scenario, silently delivered a different scenario''s rehearsal. The label is the same either way; what changes is whether it is a link or a disabled span.',
    NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('scenario_header_delete_label',
    'Delete',
    'text',
    'Delete',
    NULL, NULL,
    'The label on the strip''s destructive control. Domain note: a visible button rather than a kebab menu — Roman overruled the menu twice on 2026-08-07 — because the confirm dialog, which names the scenario and stays open on failure, is the guard, and DISTANCE does the rest: the status control is in row 1 and this sits at the far end of row 2, so nothing destructive is adjacent to anything routine.',
    NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ── the END-state assertions (CLAUDE.md rule 25a) ───────────────────────────
--
-- A statement matching zero rows is silent in Postgres and the old value keeps
-- being served. These assert what must be TRUE when this migration has run.

DO $$
DECLARE
    n INTEGER;
BEGIN
    -- ── the three new rows ──────────────────────────────────────────────────
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key IN (
            'scenario_header_edit_label',
            'scenario_header_rehearsal_view_label',
            'scenario_header_delete_label'
     );
    IF n <> 3 THEN
        RAISE EXCEPTION
            'the scenario header button labels must hold all 3 rows, found %', n;
    END IF;

    -- ── none of them carries a GLYPH ────────────────────────────────────────
    -- Asserted because the tempting edit is to paste the pencil into the value
    -- and delete it from the code. A glyph in the row would make an editor who
    -- wanted "Rename" responsible for knowing to keep a ✎ in front of it, and it
    -- would put a character no translator can act on into a translatable string.
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key IN (
            'scenario_header_edit_label',
            'scenario_header_rehearsal_view_label',
            'scenario_header_delete_label'
     )
       AND value ~ '[^[:ascii:]]';
    IF n <> 0 THEN
        RAISE EXCEPTION
            'a scenario header button label carries a non-ASCII character (% rows) — '
            'the glyphs belong in the component, the words belong here', n;
    END IF;

    -- ── the scenario and chronology blocks are still blank-free ─────────────
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE (key LIKE 'scenario\_%' OR key LIKE 'chronology\_%')
       AND (value IS NULL OR btrim(value) = '');
    IF n <> 0 THEN
        RAISE EXCEPTION
            'a scenario or chronology row is blank; the boot loader would refuse to start (% rows)', n;
    END IF;
END $$;
