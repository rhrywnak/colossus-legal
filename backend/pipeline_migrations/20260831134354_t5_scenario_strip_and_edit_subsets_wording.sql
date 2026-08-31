-- t5_scenario_strip_and_edit_subsets_wording — the header strip's one new word,
-- the Timeline-subsets section's nine, and the retirement of two T3 rows.
--
-- Created: 2026-08-31 13:43:54
-- Target: pipeline database (colossus_legal_v2)
--
-- All ten values are TRANSCRIBED from TIMELINE_SUBSET_MOCKUP_v2_2026-08-31.html
-- Screens 1 and 4, approved as drawn (design §11 items 1 and 4). Light frames
-- only: this app has one palette and no dark theme, settled by the T4 ruling.
--
-- ## ⚑ THE TOOLTIP REPLACES A SENTENCE THAT WAS ON THE PAGE
--
-- `scenario_rehearsal_disabled_tooltip` is not a new idea; it is a shorter,
-- calmer form of one already in the store. `scenario_rehearsal_link_blocked_reason`
-- ("Not in rehearsal — this scenario is {status}. Switch it to Ready on this
-- page first.") is rendered TWICE by `ScenarioHeaderTiers` today — once as the
-- disabled control's tooltip and once as a visible line beside it. Screen 1
-- removes the visible line and keeps the tooltip.
--
-- The old row is NOT retired. It still carries `{status}`, which names the state
-- the scenario is actually in, and the rehearsal gate elsewhere may still want
-- that sentence in full. The new row is the SHORT form the mockup writes on the
-- button, with no placeholder: a tooltip on a control that is disabled because
-- it is Draft does not need to tell the reader it is Draft — the segmented
-- control two inches to the left already says so.
--
-- ## The nine for Screen 4
--
-- Roman ruled on 2026-08-31 that the Edit PAGE the mockup drew does not exist in
-- this app and will not be built for T5 — the section goes at the foot of the
-- scenario DETAIL page instead. The words are unaffected by that: they describe
-- the section, not its container, which is why they are seeded verbatim from the
-- drawing and why the component that speaks them can move to a real Edit page
-- later without a migration.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('scenario_rehearsal_disabled_tooltip',
    'Draft scenarios do not rehearse — switch to Ready first',
    'text',
    'Draft scenarios do not rehearse — switch to Ready first',
    NULL, NULL,
    'The title attribute on the header strip''s disabled "Rehearsal view" control (mockup Screen 1). Domain note: the SHORT form. scenario_rehearsal_link_blocked_reason says the same thing at length and carries {status}; this one has no placeholder because a tooltip on a control disabled for being Draft need not repeat the word Draft — the segmented control beside it already shows the state. The visible copy of the long sentence leaves the page with this row''s arrival; the long row itself stays, because the rehearsal gate elsewhere still speaks it.',
    NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('scenario_edit_subsets_section_title',
    'Timeline subsets',
    'text',
    'Timeline subsets',
    NULL, NULL,
    'The heading of the section that attaches and detaches timeline subsets (mockup Screen 4). Domain note: "subsets" and not "timelines" — a subset is a NAMED SELECTION of events off the one case chronology, and calling it a timeline would imply the case has several.',
    NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('scenario_edit_subsets_section_hint',
    'Attach the stories this scenario tells in dates. Attached subsets appear behind the View Timeline button on every page of this scenario. A subset can be carried by several scenarios; detaching never deletes it.',
    'text',
    'Attach the stories this scenario tells in dates. Attached subsets appear behind the View Timeline button on every page of this scenario. A subset can be carried by several scenarios; detaching never deletes it.',
    NULL, NULL,
    'The muted line under the section heading. Domain note: it teaches three things a reader cannot infer from the controls — WHERE an attached subset shows up, that attachment is MANY-TO-MANY, and that Detach is not Delete. The third sentence exists because the button says "Detach" beside a list of things the reader may have spent an hour building, and the fear it answers is the reason detaching was undiscoverable before (defect D10).',
    NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('scenario_edit_subsets_attached_state',
    'Attached',
    'text',
    'Attached',
    NULL, NULL,
    'The state word on a subset row this scenario carries (mockup Screen 4, ".srow.on .st"). Domain note: the ✓ glyph the mockup draws beside it is furniture and lives in code, the same split the timeline window''s ⧉ ⇲ – × already use. Capitalised where its opposite is not, because the mockup draws it that way: the attached state is the notable one.',
    NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('scenario_edit_subsets_not_attached_state',
    'not attached',
    'text',
    'not attached',
    NULL, NULL,
    'The state word on a subset row this scenario does not carry. The mirror of scenario_edit_subsets_attached_state; edit the two together. Lower-case and muted, as the mockup draws it — not carrying a subset is the ordinary case and should not shout.',
    NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('scenario_edit_subsets_attach_button',
    'Attach',
    'text',
    'Attach',
    NULL, NULL,
    'The button that links a subset to this scenario. Domain note: an EXPLICIT button and not a ✓-toggle, which is defect D10 — the old toggle was invisible as a control, so nobody found detaching. It writes immediately (POST /cases/:slug/scenarios/:id/subsets); Save scenario has nothing to do with it.',
    NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('scenario_edit_subsets_detach_button',
    'Detach',
    'text',
    'Detach',
    NULL, NULL,
    'The button that unlinks a subset from this scenario. Domain note: "Detach" and never "Remove" or "Delete" — the subset is untouched and still on the timeline, and the section''s hint says so in as many words. The write is the one HARD delete in the feature (a scenario''s fact about a subset, not the subset''s content), which is why it needs no confirm dialog.',
    NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('scenario_edit_subsets_preview_link',
    'Preview',
    'text',
    'Preview',
    NULL, NULL,
    'The link that opens the floating timeline window on one subset WITHOUT attaching it (mockup Screen 4). Domain note: it exists so a reader can see the story before carrying it — the alternative was attach, look, detach, which writes twice to answer a question. In preview the window''s footer hides "Edit subset", because the reader is deciding whether to take this story, not editing it.',
    NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('scenario_edit_subsets_create_link',
    'Create a new subset on the timeline',
    'text',
    'Create a new subset on the timeline',
    NULL, NULL,
    'The link out to the timeline page, where subsets are made. Domain note: it names the DESTINATION rather than the act ("on the timeline") because the reader is about to leave this page in a new tab and should know where they are going. The + and → the mockup draws around it are furniture and live in code.',
    NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES ('scenario_edit_subsets_create_hint',
    'opens the timeline in a new tab; come back and Attach',
    'text',
    'opens the timeline in a new tab; come back and Attach',
    NULL, NULL,
    'The muted aside beside the create link. Domain note: it promises a NEW TAB and then tells the reader what to do when they return, because building a subset is a several-minute act on another screen and the thing they came here to do is still waiting. Lower-case opening, as the mockup draws it — it is an aside, not a sentence of its own.',
    NULL, NOW(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ── the retirement (Roman's ruling, 2026-08-31) ─────────────────────────────
--
-- The T3 "Timeline: [chips] Attach…" row comes off every view page — that row
-- IS defect D6 — so `ScenarioTimelineRow.tsx` stops being mounted and these two
-- rows lose their only caller. Retired the way the T4 footer row was: DELETEd
-- here and removed from the keys registry, the domain struct, the wire DTO and
-- the fixture, so `no_declared_word_is_left_with_no_asker` stays green.
DELETE FROM app_settings
 WHERE key IN ('chronology_scenario_timeline_row_label',
               'chronology_scenario_attach_link');

-- ── the END-state assertions (CLAUDE.md rule 25a) ───────────────────────────
--
-- A statement matching zero rows is silent in Postgres and the old value keeps
-- being served. This migration INSERTS and DELETES, so both directions are
-- asserted: a DELETE that matched nothing is exactly as silent as an INSERT
-- that did.

DO $$
DECLARE
    n INTEGER;
BEGIN
    -- ── the ten new rows ────────────────────────────────────────────────────
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key IN (
            'scenario_rehearsal_disabled_tooltip',
            'scenario_edit_subsets_section_title',
            'scenario_edit_subsets_section_hint',
            'scenario_edit_subsets_attached_state',
            'scenario_edit_subsets_not_attached_state',
            'scenario_edit_subsets_attach_button',
            'scenario_edit_subsets_detach_button',
            'scenario_edit_subsets_preview_link',
            'scenario_edit_subsets_create_link',
            'scenario_edit_subsets_create_hint'
     );
    IF n <> 10 THEN
        RAISE EXCEPTION
            'the T5 scenario strip and edit-subsets wording must hold all 10 rows, found %', n;
    END IF;

    -- ── the two T3 rows are GONE ────────────────────────────────────────────
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key IN ('chronology_scenario_timeline_row_label',
                   'chronology_scenario_attach_link');
    IF n <> 0 THEN
        RAISE EXCEPTION
            'the two T3 timeline-row keys must be retired, % remain', n;
    END IF;

    -- ── the long rehearsal sentence SURVIVES ────────────────────────────────
    -- Asserted because the obvious misreading of "the sentence is removed from
    -- the page" is to delete its row. The page stops SHOWING it; the store
    -- keeps it, and the rehearsal gate still reads it.
    SELECT COUNT(*) INTO n FROM app_settings
     WHERE key = 'scenario_rehearsal_link_blocked_reason';
    IF n <> 1 THEN
        RAISE EXCEPTION
            'scenario_rehearsal_link_blocked_reason must still exist (found %) — '
            'the page stops showing it; the store keeps it', n;
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
