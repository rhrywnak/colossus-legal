-- t6_subset_modal_wording — the Edit-subset modal's three words.
--
-- Created: 2026-08-31 15:17:42
-- Target: pipeline database (colossus_legal_v2)
--
-- Transcribed from TIMELINE_SUBSET_MOCKUP_v2_2026-08-31.html Screen 3, approved
-- as drawn (design §11 item 3). Light frames only: this app has one palette.
--
-- ## ⚑ TWO OF THESE EXIST BECAUSE THE BANNER LIED
--
-- Defect D2. Save makes two calls — the rename, then the events — and when the
-- first succeeded and the second failed the modal said "That change was not
-- saved". Half of it HAD been saved: the name was already on the row, and a
-- reader who believed the banner would type it again.
--
-- Screen 3 draws the honest version, in one banner and in halves: what saved,
-- in green; what did not, with the server's own reason, in red. These two rows
-- are those halves.
--
-- The events half is a TEMPLATE carrying {status} and {reason} because the
-- reason is the SERVER's sentence — T1 answers 400/409/422 with the offending
-- field and value, and a modal that replaced that with its own words would be
-- throwing away the only part of the message that says what to fix. The template
-- must tolerate an EMPTY reason: a body with no readable message renders the
-- status alone rather than a dangling colon, and that case is tested.
--
-- ## Domain note on the third
--
-- `chronology_subsets_modal_drag_label` is the accessible name of the ⠿ grip.
-- The glyph says nothing to a screen reader, which is the whole reason the row
-- exists — the same rule the picker's ▲▼ and the window's ⧉ ⇲ – × already
-- follow. It says "Drag to move" and not "Move" because the control is not a
-- button that moves the box one step; it is a handle a hand takes hold of.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_saved_name_only_banner',
    'Name and description saved.',
    'text',
    'Name and description saved.',
    NULL, NULL,
    'The GREEN half of the Edit-subset modal''s split banner: what did save when the events did not (mockup Screen 3, defect D2). Domain note: it names both fields because the rename call carries both, and a reader who changed only the description would otherwise wonder which of the two this refers to. Ends in a full stop — it is a complete statement, and the red half that follows it is another.',
    NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_events_not_saved_banner_template',
    'The event list was not saved — the server refused it (HTTP {status}: {reason}). Fix and Save again; nothing you picked has been lost.',
    'text',
    'The event list was not saved — the server refused it (HTTP {status}: {reason}). Fix and Save again; nothing you picked has been lost.',
    NULL, NULL,
    'The RED half of the split banner. {status} is the HTTP code; {reason} is the SERVER''s own sentence, which T1 supplies naming the offending field and value — replacing it with our own words would discard the only part that says what to fix. Domain note: the last clause is the load-bearing one. The modal STAYS OPEN holding every pick and every note, so "nothing you picked has been lost" is a true statement about the screen in front of the reader, and it is there to stop them closing the box and starting again. The template must render sensibly when {reason} is empty — a body with no readable message shows the status alone.',
    NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_subsets_modal_drag_label',
    'Drag to move',
    'text',
    'Drag to move',
    NULL, NULL,
    'The accessible name and tooltip of the ⠿ grip at the left of the Edit-subset modal''s title bar (mockup Screen 3). Domain note: the glyph says nothing to a screen reader, which is why this row exists — the same rule the picker''s ▲▼ and the floating window''s ⧉ ⇲ – × follow. "Drag to move" and not "Move": it is a handle a hand takes hold of, not a button that moves the box one step. The modal is draggable because it used to open jammed against the top of the viewport with its Save button off-screen (defect D7); position is deliberately NOT remembered, because a modal reopens centred.',
    NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ── the END-state assertions (CLAUDE.md rule 25a) ───────────────────────────

DO $$
DECLARE
    n INTEGER;
BEGIN
    -- ── the three new rows ──────────────────────────────────────────────────
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key IN (
            'chronology_subsets_saved_name_only_banner',
            'chronology_subsets_events_not_saved_banner_template',
            'chronology_subsets_modal_drag_label'
     );
    IF n <> 3 THEN
        RAISE EXCEPTION
            'the T6 subset-modal wording must hold all 3 rows, found %', n;
    END IF;

    -- ── the template carries BOTH placeholders ──────────────────────────────
    -- Separately from the count, because a template that lost one renders a
    -- half-sentence — the failure a row count can never see. {reason} may be
    -- filled with an empty string at runtime; it must still be IN the row.
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key = 'chronology_subsets_events_not_saved_banner_template'
       AND value LIKE '%{status}%' AND value LIKE '%{reason}%';
    IF n <> 1 THEN
        RAISE EXCEPTION
            'the events-not-saved template must carry {status} AND {reason}, or the '
            'banner drops the one fact a reader needs to fix the problem';
    END IF;

    -- ── the whole chronology block is still blank-free ──────────────────────
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key LIKE 'chronology\_%' AND (value IS NULL OR btrim(value) = '');
    IF n <> 0 THEN
        RAISE EXCEPTION
            'a chronology row is blank; the boot loader would refuse to start (% rows)', n;
    END IF;
END $$;
