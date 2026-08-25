-- chronology_wording_and_phase_window: the words the timeline speaks, and one number
--
-- Created: 2026-08-25
-- Target: pipeline database (colossus_legal_v2)
--
-- CASE_CHRONOLOGY_DESIGN_v2 Phase B, §B8. Twenty-nine strings and one parameter.
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
-- ## The glyphs are IN the strings, deliberately
--
-- `⚠`, `💬`, `⤢`, `⇲`, `◌`, `←` and `↕` are part of the words. Putting them in
-- the row means a component contains no user-visible character at all, which is
-- what the no-wording-in-code law is for, and it means Roman can drop a glyph he
-- dislikes without a rebuild.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_page_title',
    'Case Timeline',
    'text',
    'Case Timeline',
    NULL, NULL,
    'The timeline page''s own title. It is the name of the thing, not a description of it.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_count_template',
    '{events} events across {phases} phases',
    'text',
    '{events} events across {phases} phases',
    NULL, NULL,
    'The subtitle under the title when no filter is on. {events} and {phases} are counted from what the page actually received, never stored.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_filtered_count_template',
    'Showing {phase} · {shown} of {total} events',
    'text',
    'Showing {phase} · {shown} of {total} events',
    NULL, NULL,
    'The subtitle when a phase filter is on (design R16). Domain note: this line is how a filtered page stays HONEST. A page quietly showing six of twenty-two events with nothing saying so is the failure this template exists to prevent.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_search_placeholder',
    'Search events, facts, notes…',
    'text',
    'Search events, facts, notes…',
    NULL, NULL,
    'The placeholder in the timeline''s search box. It names what is searched so nobody has to guess.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_all_tags_label',
    'All',
    'text',
    'All',
    NULL, NULL,
    'The chip that clears the tag filter and shows every tag again.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_dates_label',
    'Dates',
    'text',
    'Dates',
    NULL, NULL,
    'The label on the timeline''s date-range filter.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_date_from_label',
    'From',
    'text',
    'From',
    NULL, NULL,
    'The earliest-date field of the date-range filter.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_date_to_label',
    'To',
    'text',
    'To',
    NULL, NULL,
    'The latest-date field of the date-range filter.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_expand_label',
    '⤢ Expand',
    'text',
    '⤢ Expand',
    NULL, NULL,
    'The always-visible control on every phase header (design R16, R17). Domain note: pressing it does NOT open a new page — it applies that phase as the active filter on the same page, which is what keeps the product two levels deep. Hover-only controls are a named anti-pattern, so this is always visible and muted.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_show_all_phases_label',
    '⇲ Show all phases',
    'text',
    '⇲ Show all phases',
    NULL, NULL,
    'How a reader leaves an expanded phase. The ✕ on the phase filter chip does the same thing, deliberately: one mechanism, two places to reach it.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_scroll_hint_template',
    '↕ scroll window — shows {count} at a time (size configurable in settings)',
    'text',
    '↕ scroll window — shows {count} at a time (size configurable in settings)',
    NULL, NULL,
    'The line above a phase''s scroll window (design R6). {count} is the stored chronology_phase_window_events. Domain note: the parenthetical is there so a reader who finds four too few knows the number is theirs to change.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_phase_count_template',
    '{range} · {count} events',
    'text',
    '{range} · {count} events',
    NULL, NULL,
    'The meta line beside a phase''s name. {range} is that phase''s own stored date_range and {count} is how many events the page is showing in it.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_no_document_label',
    '⚠ no document yet',
    'text',
    '⚠ no document yet',
    NULL, NULL,
    'The amber mark on an event whose document is not in the system (design R12). Domain note: four events carry it today, and their linkless state IS the to-scan to-do list. It is a mark, never a dead link — the ten dead links this redesign was written after are exactly what it replaces.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_link_unchecked_label',
    '◌ not checked',
    'text',
    '◌ not checked',
    NULL, NULL,
    'Shown on a link whose target lives in a store this build has no resolver for. Domain note: this is NOT no-document-yet. Missing means looked for and not there; unchecked means nobody looked. Rendering the second as the first would tell a reader a document is absent when the truth is that nothing checked.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_note_count_template',
    '💬 {count} notes',
    'text',
    '💬 {count} notes',
    NULL, NULL,
    'The note badge on an event card carrying more than one note.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_note_count_one',
    '💬 1 note',
    'text',
    '💬 1 note',
    NULL, NULL,
    'The note badge when an event carries exactly one note. A separate row rather than a plural rule in code, so the wording stays data.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_no_pinpoint_label',
    'no pinpoint',
    'text',
    'no pinpoint',
    NULL, NULL,
    'Shown beside a document link that names no page or paragraph (design R9). Domain note: the absence is MARKED rather than left blank, because an unpinpointed link is a job somebody still has to do.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_loading_label',
    'Loading the timeline…',
    'text',
    'Loading the timeline…',
    NULL, NULL,
    'Shown while the timeline''s one request is in flight.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_error_template',
    'The case timeline could not be loaded ({reason}). Try reloading the page.',
    'text',
    'The case timeline could not be loaded ({reason}). Try reloading the page.',
    NULL, NULL,
    'Shown when the timeline request fails. {reason} carries the thrown message. Domain note: this replaces a page that swallowed the failure and rendered nothing at all.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_empty_label',
    'No events in this case yet.',
    'text',
    'No events in this case yet.',
    NULL, NULL,
    'Shown when the case genuinely holds no chronology events.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_no_matches_label',
    'No events match these filters.',
    'text',
    'No events match these filters.',
    NULL, NULL,
    'Shown when the case holds events but the active filters match none. Domain note: a DIFFERENT sentence from the empty case on purpose — nothing here and your filters hid everything send a reader to two different places.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_unknown_phase_template',
    'Event {id} names a phase this build does not know ({phase}). It is shown here so it can be corrected.',
    'text',
    'Event {id} names a phase this build does not know ({phase}). It is shown here so it can be corrected.',
    NULL, NULL,
    'The loud row for an event whose phase has no phase row. Domain note: it RENDERS rather than vanishing. The home band used to count such an event nowhere and show nothing, and an event nobody can see is an event nobody can fix.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_back_label',
    '← Case Timeline',
    'text',
    '← Case Timeline',
    NULL, NULL,
    'The event page''s breadcrumb back to the list. It returns to the list with whatever filter was on, so a reader does not lose their place.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_documents_heading',
    'Documents',
    'text',
    'Documents',
    NULL, NULL,
    'The event page''s document-links panel.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_notes_heading',
    'Notes',
    'text',
    'Notes',
    NULL, NULL,
    'The event page''s notes panel.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_history_heading',
    'History',
    'text',
    'History',
    NULL, NULL,
    'The event page''s history panel.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_no_history_label',
    'No changes recorded yet',
    'text',
    'No changes recorded yet',
    NULL, NULL,
    'Shown in the history panel when an event has no history rows. Domain note: the panel is rendered EMPTY rather than hidden. Every event is in this state until the write endpoints land, and a missing panel would read as a feature that does not exist rather than one with nothing in it yet.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_no_notes_label',
    'No notes yet',
    'text',
    'No notes yet',
    NULL, NULL,
    'Shown in the notes panel when an event carries no notes.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_band_mismatch_template',
    '{shown} of {total} events are in a phase this page can show.',
    'text',
    '{shown} of {total} events are in a phase this page can show.',
    NULL, NULL,
    'The home timeline band''s count-mismatch marker. Domain note: the band groups by phase, so an event whose phase has no pill was previously counted nowhere and dropped without a word. This line is what it says instead.',
    NULL,
    NOW(),
    'migration')
ON CONFLICT (key) DO NOTHING;

-- The scroll window's size. A PARAMETER, not wording: nobody reads this number
-- on a screen, they read its effect. It is declared in REQUIRED_KEYS and the
-- key and value sit on ONE line because `settings_store_tests` parses required
-- rows with a stricter reader than the wording tests use.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('chronology_phase_window_events', '4', 'count', '4', 1, 40, 'How many events one phase''s scroll window shows before it scrolls (design R6). Domain note: a stored number precisely so the answer is data. Four is the mockup''s number; if Roman watches Marie read and decides six is better, that is one edit here and the next page load obeys — no rebuild, no deploy. The bounds keep it a WINDOW: below one there is nothing to see, and above forty the window is the page and the setting has stopped meaning anything.', NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ⚑ ROW-COUNT ASSERTION, ruled 2026-08-25.
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
            'chronology_page_title',
            'chronology_count_template',
            'chronology_filtered_count_template',
            'chronology_search_placeholder',
            'chronology_all_tags_label',
            'chronology_dates_label',
            'chronology_date_from_label',
            'chronology_date_to_label',
            'chronology_expand_label',
            'chronology_show_all_phases_label',
            'chronology_scroll_hint_template',
            'chronology_phase_count_template',
            'chronology_no_document_label',
            'chronology_link_unchecked_label',
            'chronology_note_count_template',
            'chronology_note_count_one',
            'chronology_no_pinpoint_label',
            'chronology_loading_label',
            'chronology_error_template',
            'chronology_empty_label',
            'chronology_no_matches_label',
            'chronology_unknown_phase_template',
            'chronology_back_label',
            'chronology_documents_heading',
            'chronology_notes_heading',
            'chronology_history_heading',
            'chronology_no_history_label',
            'chronology_no_notes_label',
            'chronology_band_mismatch_template',
            'chronology_phase_window_events'
     );
    IF n <> 30 THEN
        RAISE EXCEPTION
            'the chronology block must hold all 30 rows, found %', n;
    END IF;

    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key LIKE 'chronology\_%' AND (value IS NULL OR btrim(value) = '');
    IF n <> 0 THEN
        RAISE EXCEPTION
            'a chronology row is blank; the boot loader would refuse to start (% rows)', n;
    END IF;
END $$;
