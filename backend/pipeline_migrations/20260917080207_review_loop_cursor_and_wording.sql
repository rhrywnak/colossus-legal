-- review_loop_cursor_and_wording: the review loop — Chuck hears Marie, Marie hears Chuck
--
-- Created: 2026-09-17 08:02:07
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_REVIEW_LOOP_v1, ruled on WAR_ROOM_MOCKUP_v3_RULED_2026-09-17 and by
-- CC_GO_REVIEW_LOOP_v1 (seven rulings) and CC_GO_REVIEW_LOOP_v2 (the bar's home).
-- THE ONE MIGRATION this task writes: the read cursor, and every word it adds,
-- changes or retires. No other store write.
-- =============================================================================
--
-- ## Events and read-state are SEPARATE
--
-- The events already exist — `practice_answers`, `practice_deck_changes`,
-- `practice_notes`. This adds NO second event store and NO stored counter:
-- every count on the War Room is derived at read time. The one new table holds
-- READ-STATE only, and only for Chuck's side.
--
-- ## Chuck's side: a READ CURSOR (Slack's `conversations.mark` pattern)
--
-- One row per (user, scenario), moved ONLY by the explicit "Done reviewing"
-- act. No row = everything is new, which is honest rather than a bug: a person
-- who has never marked a deck reviewed has not reviewed it.
--
-- ## Marie's side: per-item, zero new storage (Zulip's pattern)
--
-- Her own answer timestamps are her read marks, so she needs no table here.
--
-- ## Wording: inserted, corrected in place, and retired
--
-- The card and the strip follow the ruled mockup v3. Retired rows are DELETED,
-- not orphaned (GO v1 ruling 3): no pre-v2.1.10 image can run against this
-- store at all — v2.1.9 has no `ignore_missing` and refuses to boot on an
-- applied migration it does not carry — so no older binary will ever look for
-- them.
--
-- The UPDATE alignment below is what `domain::wording::tests::corrected_value_in`
-- parses. Written any other way the correction is invisible to the fixture
-- tests, and they go green while the store holds something else.

-- ─── 1 · The cursor ──────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS practice_review_cursor (
    -- `AuthUser.username`, the same stable id `attribution` stamps on every
    -- practice write (`practice_sessions.user_id`, `practice_notes.author_id`).
    user_id     TEXT        NOT NULL CHECK (btrim(user_id) <> ''),
    scenario_id UUID        NOT NULL
                REFERENCES scenarios(scenario_id) ON DELETE CASCADE,
    -- The moment this user last pressed Done reviewing on this deck.
    looked_at   TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (user_id, scenario_id)
);

COMMENT ON TABLE practice_review_cursor IS
    'Read-state only: when each user last pressed Done reviewing on a scenario''s '
    'practice deck. Moved by that act alone. No row means nothing reviewed yet — '
    'every answer by someone else counts as new. Counts are derived, never stored.';

-- ─── 2 · New words ───────────────────────────────────────────────────────────
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('war_room_card_viewer_new_template', '{count} answers you haven''t reviewed', 'text', '{count} answers you haven''t reviewed', NULL, NULL,
     'The amber pill for the SIGNED-IN viewer: answers by someone else newer than '
     'the moment this viewer last pressed Done reviewing on the deck. Never shown '
     'at zero. Differs per viewer — Marie''s own answers never count for Marie.',
     NULL, now(), 'migration'),

    ('war_room_card_not_started', 'Not started', 'text', 'Not started', NULL, NULL,
     'The gray pill on a deck nobody has answered yet. Replaces "Up to date" there, '
     'which claimed a state the deck had never reached.',
     NULL, now(), 'migration'),

    ('war_room_card_answered_word', 'answered', 'text', 'answered', NULL, NULL,
     'The regular-weight word after the bold count in the prep pane''s headline: '
     '"42 of 42 answered".',
     NULL, now(), 'migration'),

    ('war_room_card_prep_meta_template', 'Chuck {chuck_answered}/{chuck_total} · defense {defense_answered}/{defense_total} · deck {count} q · {date}', 'text', 'Chuck {chuck_answered}/{chuck_total} · defense {defense_answered}/{defense_total} · deck {count} q · {date}', NULL, NULL,
     'The ONE muted line under the answered headline: the split by who asks, the '
     'deck''s visible question count and the day the deck last changed. Not shown '
     'for a scenario with no deck.',
     NULL, now(), 'migration'),

    ('war_room_strip_answered_label', 'Questions answered', 'text', 'Questions answered', NULL, NULL,
     'The first queue tile''s label: visible questions with an answer, across every '
     'scenario of the case.',
     NULL, now(), 'migration'),

    ('war_room_strip_answered_template', '{answered} of {total}', 'text', '{answered} of {total}', NULL, NULL,
     'The first queue tile''s number: answered of all visible questions.',
     NULL, now(), 'migration'),

    ('war_room_strip_waiting_label', 'Waiting for Marie', 'text', 'Waiting for Marie', NULL, NULL,
     'The second queue tile: visible questions with no answer yet, both sides '
     '(Chuck''s and the defense''s).',
     NULL, now(), 'migration'),

    ('war_room_strip_new_for_you_label', 'New answers for you', 'text', 'New answers for you', NULL, NULL,
     'The third queue tile: the sum of every card''s "answers you haven''t '
     'reviewed" pill for the signed-in viewer.',
     NULL, now(), 'migration'),

    ('war_room_strip_candidates_label', 'Candidates for Roman', 'text', 'Candidates for Roman', NULL, NULL,
     'The fourth queue tile: the sum of every card''s candidates to rule.',
     NULL, now(), 'migration'),

    ('practice_deck_review_new_template', '{count} new since you last reviewed', 'text', '{count} new since you last reviewed', NULL, NULL,
     'The review bar under the deck''s title: answers by someone else newer than '
     'the viewer''s last Done reviewing. The bar is not shown at zero.',
     NULL, now(), 'migration'),

    ('practice_deck_review_done_label', 'Done reviewing', 'text', 'Done reviewing', NULL, NULL,
     'The review bar''s button. Moves this viewer''s review mark for this deck to '
     'now, which zeroes the bar and the War Room pill for this viewer only.',
     NULL, now(), 'migration'),

    ('practice_deck_review_failed', 'Could not mark this deck reviewed — nothing was changed.', 'text', 'Could not mark this deck reviewed — nothing was changed.', NULL, NULL,
     'Shown under the review bar when Done reviewing fails. The count stays as it '
     'was, because the mark did not move.',
     NULL, now(), 'migration'),

    ('practice_row_note_add_label', 'Add a note', 'text', 'Add a note', NULL, NULL,
     'Opens the note box on a question''s answers page, for a note on the current '
     'answer.',
     NULL, now(), 'migration'),

    ('practice_row_note_save_label', 'Save note', 'text', 'Save note', NULL, NULL,
     'Saves the note. It then shows under Marie''s deck row and counts toward her '
     '"new or changed" pill until she answers the question again.',
     NULL, now(), 'migration'),

    ('practice_row_note_cancel_label', 'Cancel', 'text', 'Cancel', NULL, NULL,
     'Closes the note box without writing anything.',
     NULL, now(), 'migration'),

    ('practice_row_note_strike_label', 'Strike', 'text', 'Strike', NULL, NULL,
     'Withdraws a note. It stays visible, struck through with its date, and stops '
     'counting toward Marie''s pill.',
     NULL, now(), 'migration'),

    ('practice_row_note_struck_template', 'struck {when}', 'text', 'struck {when}', NULL, NULL,
     'The line under a withdrawn note. {when} is the day it was struck, in the '
     'case''s own timezone.',
     NULL, now(), 'migration'),

    ('practice_row_note_failed', 'The note could not be saved — nothing was written.', 'text', 'The note could not be saved — nothing was written.', NULL, NULL,
     'Shown under the note box, or beside Strike, when the write fails.',
     NULL, now(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ─── 3 · Corrections in place ────────────────────────────────────────────────
-- The scan line is date-only on the card: the model and the relevant count live
-- on the scenario page (task §5).
UPDATE app_settings
SET value         = 'Last scan {date}',
    default_value = 'Last scan {date}',
    meaning       = 'The line under the evidence rows: the day the scenario''s most '
                    'recent scan started. The model and relevant counts are on the '
                    'scenario page.',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'war_room_card_scan_template';

UPDATE app_settings
SET value         = 'Never scanned',
    default_value = 'Never scanned',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'war_room_card_scan_never';

-- The practice link moved to the prep pane's header, arrow and all.
UPDATE app_settings
SET value         = 'Practice →',
    default_value = 'Practice →',
    meaning       = 'The link in the prep pane''s header; the whole pane opens the deck.',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'war_room_card_practice_action';

-- ─── 4 · Retirements ─────────────────────────────────────────────────────────
-- Talking points and watch items leave the card; the deck label, its template
-- and the answered split fold into `war_room_card_prep_meta_template`; the title
-- itself is now the scenario link, so "Open scenario" has no control to label.
DELETE FROM app_settings
 WHERE key IN (
    'war_room_card_talking_points_label',
    'war_room_card_watch_items_label',
    'war_room_card_deck_label',
    'war_room_card_deck_template',
    'war_room_card_answered_split_template',
    'war_room_card_open_action');

-- ─── 5 · The END-state assertion (CLAUDE.md rule 25a) ────────────────────────
--
-- A statement matching zero rows is SILENT in Postgres and the old value keeps
-- being served. This migration inserts, corrects AND deletes, so all three
-- directions are asserted on what must be true afterwards — safe to re-run.
DO $$
DECLARE
    present   INTEGER;
    retired   INTEGER;
    corrected INTEGER;
    cursor_ok INTEGER;
BEGIN
    SELECT count(*) INTO present
      FROM app_settings
     WHERE key IN (
        'war_room_card_viewer_new_template',
        'war_room_card_not_started',
        'war_room_card_answered_word',
        'war_room_card_prep_meta_template',
        'war_room_strip_answered_label',
        'war_room_strip_answered_template',
        'war_room_strip_waiting_label',
        'war_room_strip_new_for_you_label',
        'war_room_strip_candidates_label',
        'practice_deck_review_new_template',
        'practice_deck_review_done_label',
        'practice_deck_review_failed',
        'practice_row_note_add_label',
        'practice_row_note_save_label',
        'practice_row_note_cancel_label',
        'practice_row_note_strike_label',
        'practice_row_note_struck_template',
        'practice_row_note_failed');
    IF present <> 18 THEN
        RAISE EXCEPTION
            'review loop wording: expected 18 new settings rows, found %. The backend '
            'declares every one at boot and refuses to start without it.', present;
    END IF;

    SELECT count(*) INTO corrected
      FROM app_settings
     WHERE (key = 'war_room_card_scan_template'   AND value = 'Last scan {date}')
        OR (key = 'war_room_card_scan_never'      AND value = 'Never scanned')
        OR (key = 'war_room_card_practice_action' AND value = 'Practice →');
    IF corrected <> 3 THEN
        RAISE EXCEPTION
            'review loop wording: expected 3 corrected rows (scan template, scan '
            'never, practice action), found %. A missing row would keep serving '
            'the old card''s words.', corrected;
    END IF;

    SELECT count(*) INTO retired
      FROM app_settings
     WHERE key IN (
        'war_room_card_talking_points_label',
        'war_room_card_watch_items_label',
        'war_room_card_deck_label',
        'war_room_card_deck_template',
        'war_room_card_answered_split_template',
        'war_room_card_open_action');
    IF retired <> 0 THEN
        RAISE EXCEPTION
            'review loop wording: % retired card rows are still present.', retired;
    END IF;

    SELECT count(*) INTO cursor_ok
      FROM information_schema.tables
     WHERE table_name = 'practice_review_cursor';
    IF cursor_ok <> 1 THEN
        RAISE EXCEPTION 'review loop: practice_review_cursor was not created';
    END IF;
END $$;
