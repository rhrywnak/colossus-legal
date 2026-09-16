-- war_room_status_card_wording: every word the War Room's status card speaks
--
-- Created: 2026-09-16 13:01:21
-- Target: pipeline database
--
-- =============================================================================
-- Twenty-two rows for one card
-- =============================================================================
--
-- CC_TASK_WAR_ROOM_v1, ruled on WAR_ROOM_CARD_MOCKUP_v1_2026-09-16 and amended by
-- CC_GO_WAR_ROOM_v1 (Q3, Q4, Q5) and CC_GO_WAR_ROOM_v3 (the changed pill).
--
-- The card used to say "Ready" on all eleven scenarios. It now reports where each
-- scenario stands: evidence ruled → linked into the Matrix → prep written → deck
-- built → answered. Every label, template and pill on it is a row here, so Roman
-- can retune any of them on the Settings page with no build.
--
-- ## Templates carry NUMBERS, the browser fills them
--
-- The payload sends `{ linked, total }`, `{ relevant, total }` and the like as
-- numbers, and the browser fills `{name}` placeholders from these rows. The
-- DATE inside a template is formatted by the browser too — the sentence is the
-- store's, the date format is the reader's (the same split the scan header makes).
--
-- ## Why the answered line is TWO rows
--
-- The mockup sets "0 of 17" in bold and the rest of the line in regular weight.
-- One template would make the screen split a stored sentence at a guessed
-- boundary; two rows let each half be styled without parsing either.
--
-- ## The two em-dash rows (Q4)
--
-- "0 of 0" reads as a failure to a lawyer. A scenario with nothing stuck shows
-- the label and an em dash, and so does a scenario with no deck yet. Two rows,
-- because they are two different facts that happen to share a glyph today.
--
-- ## The changed pill (GO v3)
--
-- ONE template, no viewer identity: "{count} new or changed for Marie". The name
-- is case data in a row (Rule 2), not code.
--
-- Idempotent: `ON CONFLICT (key) DO NOTHING`, so a re-run changes nothing and a
-- row Roman has edited is never clobbered.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── The two section headers ──────────────────────────────────────────────
    ('war_room_card_evidence_heading', 'Evidence', 'text', 'Evidence', NULL, NULL,
     'The header of the card''s middle pane: how far the scenario''s evidence has '
     'been ruled and linked. Rendered in capitals by the page style.',
     NULL, now(), 'system (seed)'),

    ('war_room_card_prep_heading', 'Prep & rehearsal', 'text', 'Prep & rehearsal', NULL, NULL,
     'The header of the card''s right pane: the talking points, the watch list, '
     'the practice deck and how much of it has been answered.',
     NULL, now(), 'system (seed)'),

    -- ── EVIDENCE pane ────────────────────────────────────────────────────────
    ('war_room_card_facts_included_label', 'Facts included', 'text', 'Facts included', NULL, NULL,
     'Label for the count of candidate facts a human has ruled Included for this '
     'scenario — the same count the scenario page''s fact list shows.',
     NULL, now(), 'system (seed)'),

    ('war_room_card_candidates_label', 'Candidates to rule', 'text', 'Candidates to rule', NULL, NULL,
     'Label for the count of cards the latest scan proposed that nobody has '
     'ruled yet. Shown in the warning colour when it is above zero: work is owed.',
     NULL, now(), 'system (seed)'),

    ('war_room_card_matrix_linked_label', 'Matrix linked', 'text', 'Matrix linked', NULL, NULL,
     'Label for how many of the statements the extraction could not link to an '
     'accusation a human has since linked into the Matrix.',
     NULL, now(), 'system (seed)'),

    ('war_room_card_matrix_linked_template', '{linked} of {total}', 'text', '{linked} of {total}', NULL, NULL,
     'The Matrix-linked value. {linked} is how many a human has linked, {total} is '
     'how many the extraction left unlinked — the same two numbers as the '
     'scenario page''s "N of M linked." line.',
     NULL, now(), 'system (seed)'),

    ('war_room_card_matrix_linked_none', '—', 'text', '—', NULL, NULL,
     'The Matrix-linked value when nothing in the pool was left unlinked. An em '
     'dash rather than "0 of 0", which reads as a failure (ruling Q4).',
     NULL, now(), 'system (seed)'),

    ('war_room_card_scan_template', 'Scan: {model} · {date} · {relevant} relevant of {total}', 'text',
     'Scan: {model} · {date} · {relevant} relevant of {total}', NULL, NULL,
     'The line under the evidence rows naming the scenario''s most recent scan: '
     'the model''s display name, the day it started, and how many of the '
     'candidates it judged relevant.',
     NULL, now(), 'system (seed)'),

    ('war_room_card_scan_never', 'Scan: never run', 'text', 'Scan: never run', NULL, NULL,
     'The scan line when no scan has ever run on this scenario. Shown in the '
     'warning colour: until a scan runs, nothing has been proposed to rule.',
     NULL, now(), 'system (seed)'),

    -- ── PREP & REHEARSAL pane ────────────────────────────────────────────────
    ('war_room_card_talking_points_label', 'Talking points', 'text', 'Talking points', NULL, NULL,
     'Label for the number of talking points written for this scenario.',
     NULL, now(), 'system (seed)'),

    ('war_room_card_watch_items_label', 'Watch items', 'text', 'Watch items', NULL, NULL,
     'Label for the number of watch-list items on this scenario.',
     NULL, now(), 'system (seed)'),

    ('war_room_card_deck_label', 'Deck', 'text', 'Deck', NULL, NULL,
     'Label for the practice deck: how many questions it holds and when it was '
     'last built. Hidden questions are not counted.',
     NULL, now(), 'system (seed)'),

    ('war_room_card_deck_template', '{count} questions · {date}', 'text', '{count} questions · {date}', NULL, NULL,
     'The deck value. {count} is the visible questions, {date} the day the deck '
     'last changed — the same date the practice page''s "as of" line uses.',
     NULL, now(), 'system (seed)'),

    ('war_room_card_deck_none', '—', 'text', '—', NULL, NULL,
     'The deck value when the scenario has no visible questions yet. An em dash '
     'rather than "0 questions", for the same reason as the Matrix row (Q4).',
     NULL, now(), 'system (seed)'),

    ('war_room_card_answered_count_template', '{answered} of {total}', 'text', '{answered} of {total}', NULL, NULL,
     'The bold opening of the answered line: how many of the deck''s visible '
     'questions have an answer. Not shown when the deck is empty.',
     NULL, now(), 'system (seed)'),

    ('war_room_card_answered_split_template', 'answered · Chuck {chuck_answered}/{chuck_total} · defense {defense_answered}/{defense_total}', 'text',
     'answered · Chuck {chuck_answered}/{chuck_total} · defense {defense_answered}/{defense_total}', NULL, NULL,
     'The rest of the answered line, split by who asks the question: Chuck''s '
     'side, and the defense''s. The page joins it to the bold opening with one space.',
     NULL, now(), 'system (seed)'),

    ('war_room_card_changed_template', '{count} new or changed for Marie', 'text',
     '{count} new or changed for Marie', NULL, NULL,
     'The amber pill: questions added or reworded since Marie last answered on '
     'this deck. Counted per question — answering one clears it. Never shown on '
     'a deck she has not answered at all.',
     NULL, now(), 'system (seed)'),

    ('war_room_card_up_to_date', 'Up to date', 'text', 'Up to date', NULL, NULL,
     'The green pill: nothing on the deck has changed since Marie last answered it.',
     NULL, now(), 'system (seed)'),

    -- ── The action row ───────────────────────────────────────────────────────
    ('war_room_card_open_action', 'Open scenario', 'text', 'Open scenario', NULL, NULL,
     'The first link on the card: opens the scenario page.',
     NULL, now(), 'system (seed)'),

    ('war_room_card_practice_action', 'Practice', 'text', 'Practice', NULL, NULL,
     'Opens this scenario''s practice deck.',
     NULL, now(), 'system (seed)'),

    ('war_room_card_timeline_action', 'Timeline', 'text', 'Timeline', NULL, NULL,
     'Opens the timeline window. Shown only on a scenario that carries a '
     'timeline subset.',
     NULL, now(), 'system (seed)'),

    ('war_room_card_delete_action', 'Delete', 'text', 'Delete', NULL, NULL,
     'The last control on the card, deliberately quieter than the others. It '
     'opens the confirmation dialog; nothing is deleted from the card itself.',
     NULL, now(), 'system (seed)')
ON CONFLICT (key) DO NOTHING;

-- ─── The row-count assertion (CLAUDE.md rule 25a) ─────────────────────────────
--
-- A statement matching zero rows is SILENT in Postgres, and `ON CONFLICT DO
-- NOTHING` makes that possible on purpose. A key mistyped above inserts nothing,
-- this migration reports success, and the backend then refuses to start naming
-- the key and not this file. Assert the END state.
DO $$
DECLARE
    present INTEGER;
BEGIN
    SELECT count(*) INTO present
      FROM app_settings
     WHERE key IN (
        'war_room_card_evidence_heading',
        'war_room_card_prep_heading',
        'war_room_card_facts_included_label',
        'war_room_card_candidates_label',
        'war_room_card_matrix_linked_label',
        'war_room_card_matrix_linked_template',
        'war_room_card_matrix_linked_none',
        'war_room_card_scan_template',
        'war_room_card_scan_never',
        'war_room_card_talking_points_label',
        'war_room_card_watch_items_label',
        'war_room_card_deck_label',
        'war_room_card_deck_template',
        'war_room_card_deck_none',
        'war_room_card_answered_count_template',
        'war_room_card_answered_split_template',
        'war_room_card_changed_template',
        'war_room_card_up_to_date',
        'war_room_card_open_action',
        'war_room_card_practice_action',
        'war_room_card_timeline_action',
        'war_room_card_delete_action');

    IF present <> 22 THEN
        RAISE EXCEPTION
            'war room status card wording: expected 22 settings rows after this '
            'migration, found %. The backend declares all 22 keys at boot '
            '(WAR_ROOM_WORDING_KEYS) and refuses to start when one is missing, so '
            'this migration fails here rather than at the next deploy.', present;
    END IF;
END $$;
