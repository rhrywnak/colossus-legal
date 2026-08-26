-- chronology_write_wording: every word the timeline's WRITE controls speak,
-- and the one number the document picker reads
--
-- Created: 2026-08-26
-- Target: pipeline database (colossus_legal_v2)
--
-- CASE_CHRONOLOGY_DESIGN_v2 Phase C. Thirty-nine strings and one parameter.
--
-- ## ⚑ BOTH HALVES, OR NEITHER
--
-- A wording key is real only when all THREE parties agree: this migration holds
-- the row, `domain::wording_chronology` DECLARES the key, and the frontend asks
-- for it. Boot refuses to start if a declared key has no row here;
-- `dto::chronology_wording_reach_tests` refuses if the frontend asks for a name
-- no field carries. Seeding a row this build declares nowhere is the .407 defect
-- — seven rows, a clean boot, and a page that rendered blank.
--
-- ## Why a form's LABELS are wording and not markup
--
-- Phase B put the page's sentences in rows and left its controls to Phase C
-- because Phase C is where the controls arrive. Every label, placeholder,
-- button and confirm-less undo line below is a row for the same reason the
-- read surface's are: Roman changes a word Marie stumbles over by editing one
-- row, and the next page load obeys — no rebuild, no deploy, no CC.
--
-- ## The glyphs are IN the strings, deliberately
--
-- `✎`, `🗑`, `✕`, `🔍`, `+` and `~` are part of the words. Putting them in the
-- row means a component contains no user-visible character at all, which is
-- what the no-wording-in-code law is for, and it means Roman can drop a glyph
-- he dislikes without a rebuild.
--
-- ## What is NOT here
--
-- A confirm-dialog sentence. There is none, by ruling R10: delete acts at once
-- and the undo line that replaces the card in place IS the safety. A stored
-- "are you sure?" would be a row for a dialog this design refuses to draw.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_add_event_label',
    '+ Add event',
    'text',
    '+ Add event',
    NULL, NULL,
    'The control that opens the Add-event form on the timeline list (mockup Screen 1). Domain note: it is drawn for every authenticated reader — R2 makes Roman, Chuck and Marie equal authors, so there is no version of this page where the button exists for one of them and not another.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_edit_label',
    '✎ Edit',
    'text',
    '✎ Edit',
    NULL, NULL,
    'The edit control on every event card and on the event page. Domain note: ALWAYS VISIBLE and muted, per R17 — hover-only controls are a named anti-pattern, and CaseFleet''s rows carry a visible pencil.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_delete_label',
    '🗑 Delete',
    'text',
    '🗑 Delete',
    NULL, NULL,
    'The delete control on every event card and on the event page. Domain note: pressing it deletes IMMEDIATELY with no confirm dialog (R10) — the undo line that replaces the card is the safety, which is the pattern already ruled on the practice page.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_deleted_line_label',
    'Deleted —',
    'text',
    'Deleted —',
    NULL, NULL,
    'The line that replaces a deleted event IN PLACE until the reader navigates away (R10). Domain note: the trailing em dash is part of the stored words because the Undo control follows it on the same line — the component contains no user-visible character of its own.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_undo_label',
    'Undo',
    'text',
    'Undo',
    NULL, NULL,
    'Restores an event that was just deleted. Domain note: nothing is ever hard-deleted, so this is always able to succeed — the row is still there with deleted_at set, and undo clears it.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_form_add_title',
    'Add event',
    'text',
    'Add event',
    NULL, NULL,
    'The heading over the event form when it is creating a new event (mockup Screen 3).',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_form_edit_title',
    'Edit event',
    'text',
    'Edit event',
    NULL, NULL,
    'The heading over the event form when it is editing an existing event. Domain note: the same form as Add, pre-filled — one form, two headings, so a reader never has to learn a second layout.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_form_date_label',
    'Date',
    'text',
    'Date',
    NULL, NULL,
    'The event form''s date field. Domain note: date and title are the only two fields required forever (R11); everything else on this form may be left empty.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_form_precision_label',
    'Precision',
    'text',
    'Precision',
    NULL, NULL,
    'The event form''s date-precision select. Domain note: precision says WHICH PARTS of the date are known. Printing 1 March 2010 for a source that said March 2010 would be a fabricated day, which is what this control exists to prevent.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_precision_day_label',
    'Exact day',
    'text',
    'Exact day',
    NULL, NULL,
    'The day-precision option. Stored rather than derived from the token so the vocabulary the database holds (day) and the words a human reads (Exact day) can differ without either becoming a code constant.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_precision_month_label',
    'Month only',
    'text',
    'Month only',
    NULL, NULL,
    'The month-precision option: the source stated a month and a year, and the day is padding the screen must not print.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_precision_year_label',
    'Year only',
    'text',
    'Year only',
    NULL, NULL,
    'The year-precision option: the source stated only a year.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_form_approximate_label',
    'Approximate (~)',
    'text',
    'Approximate (~)',
    NULL, NULL,
    'The event form''s approximate checkbox. Domain note: separate from precision on purpose. Precision says which parts of the date are known; approximate says the whole thing is a best estimate. Three seeded events carry a full day-precision date that is nonetheless a guess.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_form_title_label',
    'Title',
    'text',
    'Title',
    NULL, NULL,
    'The event form''s title field.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_form_title_placeholder',
    'One short line — this is what the list shows',
    'text',
    'One short line — this is what the list shows',
    NULL, NULL,
    'The placeholder inside the event form''s title field. It names what the value is used for rather than repeating the label.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_form_fact_label',
    'What happened (one plain sentence or two)',
    'text',
    'What happened (one plain sentence or two)',
    NULL, NULL,
    'The event form''s fact field. Domain note: optional by R11 but ENCOURAGED, which is why the label asks a question rather than naming a column.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_form_fact_placeholder',
    'Write it so anyone can check it against the source.',
    'text',
    'Write it so anyone can check it against the source.',
    NULL, NULL,
    'The placeholder inside the fact field. Domain note: Marie reads and writes this chronology; a row must read as one plain sentence a non-lawyer can check against the source in one click.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_form_tags_label',
    'Tags',
    'text',
    'Tags',
    NULL, NULL,
    'The event form''s tag picker. Domain note: the chips are drawn from chronology_tags, so a sixth tag is a row and not a build (R7).',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_form_phase_label',
    'Phase',
    'text',
    'Phase',
    NULL, NULL,
    'The event form''s phase select. Domain note: the options are the rows of chronology_phases, and the column is a foreign key onto them — an unknown phase is refused by name rather than stored.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_form_documents_label',
    'Documents',
    'text',
    'Documents',
    NULL, NULL,
    'The event form''s document section, where the author searches the document store and picks a target (R9).',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_document_search_placeholder',
    '🔍 Search documents and pick one…',
    'text',
    '🔍 Search documents and pick one…',
    NULL, NULL,
    'The placeholder in the document picker. Domain note: links are HUMAN-MADE in v1 (R9) — nothing converts a prose reference into a link automatically, so this search is how every link is born.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_document_search_empty_label',
    'No documents match that search.',
    'text',
    'No documents match that search.',
    NULL, NULL,
    'Shown when a document search matched no document. Domain note: a DIFFERENT state from having typed nothing yet, so an author can tell a fruitless search from an untouched box.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_pinpoint_placeholder',
    'pinpoint (page / ¶) optional — a link without one is marked',
    'text',
    'pinpoint (page / ¶) optional — a link without one is marked',
    NULL, NULL,
    'The placeholder in the pinpoint field. Domain note: it states the consequence out loud, because an unpinpointed link is marked on every screen that renders it and doubles as the to-scan to-do list (R9).',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_save_label',
    'Save',
    'text',
    'Save',
    NULL, NULL,
    'The event form''s save control. It is the one guarded write path''s front door.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_cancel_label',
    'Cancel',
    'text',
    'Cancel',
    NULL, NULL,
    'Closes the event form without writing anything.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_saving_label',
    'Saving…',
    'text',
    'Saving…',
    NULL, NULL,
    'Shown on a write control while its request is in flight. Domain note: it labels a moment, so it is not a sentence — but it is still a stored row, because a control whose label changes in code is a control whose words drifted.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_add_note_placeholder',
    'Add a note…',
    'text',
    'Add a note…',
    NULL, NULL,
    'The placeholder in the event page''s add-note box. Domain note: notes are individual and attributed (R8) — three writers never overwrite one shared blob.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_add_note_button_label',
    'Add',
    'text',
    'Add',
    NULL, NULL,
    'The control that writes the typed note.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_link_document_label',
    '+ Link a document…',
    'text',
    '+ Link a document…',
    NULL, NULL,
    'Opens the document picker on the event page (mockup Screen 2''s deviated row).',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_remove_link_label',
    '✕ Remove',
    'text',
    '✕ Remove',
    NULL, NULL,
    'Removes one document link from an event. Domain note: a link is addressed by (event, target type, target id) — the three columns the author actually picked — so no surrogate id had to be invented for a row a human can point at.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_delete_note_label',
    '✕ Delete note',
    'text',
    '✕ Delete note',
    NULL, NULL,
    'Deletes one note. Domain note: soft, like everything else here, and only its own author may press it — R8''s attributed-notes model means a note belongs to the person who signed it.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_history_line_template',
    '{when} · {who} · {what}',
    'text',
    '{when} · {who} · {what}',
    NULL, NULL,
    'One line of an event''s history panel. {when} is the change date, {who} the acting user from the login, {what} the stored action in its display word. Domain note: history is APPEND-ONLY — nothing ever updates or deletes a history row, so this list only ever grows.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_history_created_label',
    'created',
    'text',
    'created',
    NULL, NULL,
    'How the stored action created reads on screen.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_history_updated_label',
    'edited',
    'text',
    'edited',
    NULL, NULL,
    'How the stored action updated reads on screen. Domain note: the stored token and the display word differ on purpose. The database says updated because that is what happened to the row; a human reads edited because that is what the person did.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_history_deleted_label',
    'deleted',
    'text',
    'deleted',
    NULL, NULL,
    'How the stored action deleted reads on screen. Domain note: a deleted event still has a history, and this line is what makes the delete attributable and recoverable (R10).',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_history_restored_label',
    'restored',
    'text',
    'restored',
    NULL, NULL,
    'How the stored action restored reads on screen. It is what pressing Undo writes.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_history_unknown_template',
    '{action}',
    'text',
    '{action}',
    NULL, NULL,
    'The fallback display for a history action this build has no word for. Domain note: the raw token renders rather than an empty line or a swallowed row — a vocabulary drift must be visible on the one screen where somebody could notice it.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_picker_capped_template',
    'Showing {shown} of {total} matches — narrow the search to see the rest.',
    'text',
    'Showing {shown} of {total} matches — narrow the search to see the rest.',
    NULL, NULL,
    'The line under a capped document-picker list (design R9). Domain note: the cap is never SILENT. A short list that looked complete is how somebody links the wrong document with no idea a better match was cut off, so the picker says how many it is showing of how many matched. The number itself is chronology_document_picker_max.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_write_failed_template',
    'That change was not saved — {reason}',
    'text',
    'That change was not saved — {reason}',
    NULL, NULL,
    'Shown when a write fails. Domain note: {reason} carries the server''s own message, so an author reads why rather than watching a button do nothing. This row CAN be stored (unlike the page''s load-failure line) because the wording store already arrived with the read that drew the form.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

-- The document picker's result cap. A PARAMETER, not wording: nobody reads this
-- number on a screen, they read its effect. It is declared in REQUIRED_KEYS, and
-- the key and value sit on ONE line because `settings_store_tests` parses
-- required rows with a stricter reader than the wording tests use.
--
-- Domain note: the cap is never SILENT. The picker's response carries how many
-- documents matched alongside the page it returns, and the surface says so when
-- the two differ — a truncated list that looked complete is how somebody links
-- the wrong document with no idea a better match was cut off.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_document_picker_max', '20', 'count', '20', 1, 200, 'How many documents the timeline''s document picker offers for one search (design R9). Domain note: a stored number so the answer is data. Twenty fills the picker without filling the screen; if Roman finds himself scrolling past twenty to reach the document he wants, that is one edit here and the next search obeys. The bounds keep it a PICKER: below one it offers nothing, and above two hundred it has stopped being a short list.', NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ⚑ ROW-COUNT ASSERTION (CLAUDE.md 25a, ruled 2026-08-25).
--
-- A seed that matches zero rows is SILENT in Postgres, and a wording key with no
-- row makes the BACKEND REFUSE TO START — a deploy taking DEV down, discovered by
-- the outage rather than by the migration. This block turns that into a failed
-- migration instead. It asserts the END STATE, so it is equally true on a first
-- run and on a re-run where ON CONFLICT did nothing.
DO $$
DECLARE
    n INTEGER;
BEGIN
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key IN (
            'chronology_add_event_label',
            'chronology_edit_label',
            'chronology_delete_label',
            'chronology_deleted_line_label',
            'chronology_undo_label',
            'chronology_form_add_title',
            'chronology_form_edit_title',
            'chronology_form_date_label',
            'chronology_form_precision_label',
            'chronology_precision_day_label',
            'chronology_precision_month_label',
            'chronology_precision_year_label',
            'chronology_form_approximate_label',
            'chronology_form_title_label',
            'chronology_form_title_placeholder',
            'chronology_form_fact_label',
            'chronology_form_fact_placeholder',
            'chronology_form_tags_label',
            'chronology_form_phase_label',
            'chronology_form_documents_label',
            'chronology_document_search_placeholder',
            'chronology_document_search_empty_label',
            'chronology_pinpoint_placeholder',
            'chronology_save_label',
            'chronology_cancel_label',
            'chronology_saving_label',
            'chronology_add_note_placeholder',
            'chronology_add_note_button_label',
            'chronology_link_document_label',
            'chronology_remove_link_label',
            'chronology_delete_note_label',
            'chronology_history_line_template',
            'chronology_history_created_label',
            'chronology_history_updated_label',
            'chronology_history_deleted_label',
            'chronology_history_restored_label',
            'chronology_history_unknown_template',
            'chronology_write_failed_template',
            'chronology_picker_capped_template',
            'chronology_document_picker_max'
     );
    IF n <> 40 THEN
        RAISE EXCEPTION
            'the chronology write block must hold all 40 rows, found %', n;
    END IF;

    -- The blank check is deliberately over the WHOLE chronology prefix rather
    -- than this migration's own rows: a blank value anywhere in the block stops
    -- the boot loader, and a migration that proved only its own half would let
    -- an earlier row go blank between deploys without anything noticing.
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key LIKE 'chronology\_%' AND (value IS NULL OR btrim(value) = '');
    IF n <> 0 THEN
        RAISE EXCEPTION
            'a chronology row is blank; the boot loader would refuse to start (% rows)', n;
    END IF;
END $$;
