-- for_you_l3_deck_threshold_replies_and_board_4_marks: the last layer's rows
--
-- Created: 2026-09-22 23:12:42
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_FOR_YOU_v1, layer L3. Ten rows, in three groups, and no schema:
--
--   1. `practice_for_you_deck_threshold` — how many UNREAD items a deck must
--      hold before the reviewer's list shows it as ONE row (ruled: default 5).
--   2. four `for_you_*` rows — what a deck row says, and how a reply reads in
--      a list where there is room for only one line.
--   3. five `practice_row_*` rows — the reply control, and the board-4 mark on
--      a deck's question rows.
--
-- The reply COLUMN (`practice_notes.answers_note_id`) was created by L0's
-- migration (20260922153654) and has shipped unused since. Nothing is added to
-- the schema here; this layer only gives it a writer and words.
-- =============================================================================
--
-- ## Why the threshold is a row and not a constant
--
-- Where the line falls is a judgement about one person's day, and the two sides
-- of it are different pages: below it the list names each errand, at or above
-- it the list names the deck and sends the reader to the review page. Roman
-- moves it, restarts, and reads the page again. `min_value` is 1 — at 0 every
-- deck holding anything at all would collapse to a single row, and the page
-- could no longer name an individual item anywhere.
--
-- ## Domain note: what the board-4 mark is FOR
--
-- FOR_YOU_MOCKUP_v1 board 4 puts "Chuck left a note" on the question's own row
-- in the deck list, so the witness working down her deck can see which
-- questions have something waiting WITHOUT going to the For you page first. It
-- is the same per-person unread state the list is built from, read one deck at
-- a time, which is why these are `practice_row_*` rows: they are spoken by the
-- deck, not by the list.
--
-- ## Fresh-database reproduction (Law 20)
--
-- Every row is INSERTed with `ON CONFLICT (key) DO NOTHING`, so a database
-- built from zero ends with exactly these values, a database that already
-- carries one keeps what it has, and a proof that re-applies this file by hand
-- does not die on the second pass. The assertion at the foot counts what is
-- present AFTERWARDS: an INSERT matching nothing is silent in Postgres, and the
-- boot loader reads all ten of these by name and refuses to start without them.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── 1. The threshold (a COUNT row, with bounds) ──────────────────────────
    ('practice_for_you_deck_threshold', '5', 'count', '5', 1, 100,
     'How many UNREAD items one deck must hold before the For you page shows it as a single row ("S-11 · The $50,000 — 17 answers waiting") instead of one row per item. Below this number the items are listed individually, because each one is a separate errand; at or above it they are one deck to sit down with, and the row opens that deck''s review page. Counts UNREAD items only (ruled 2026-09-22), and applies to a REVIEWER''s list only — the witness''s list is always per item, because she answers one question at a time. Raise it to see more individual rows; set it to 1 to group every deck that holds anything.',
     NULL, now(), 'migration'),

    -- ── 2. The deck row, and a reply as a list row ───────────────────────────
    ('for_you_deck_body_template', '{count} answers waiting', 'text', '{count} answers waiting', NULL, NULL,
     'The body of a grouped DECK row on the For you page: how many unread items that deck holds. Its first line is the deck line without a question (for_you_deck_line_no_question_template), because the row stands for several questions at once.',
     NULL, now(), 'migration'),

    ('for_you_deck_body_one', '{count} answer waiting', 'text', '{count} answer waiting', NULL, NULL,
     'The singular of for_you_deck_body_template, used when the count is exactly 1 — which is reachable the moment practice_for_you_deck_threshold is set to 1, and is exactly when "1 answers waiting" would otherwise be on screen.',
     NULL, now(), 'migration'),

    ('for_you_deck_byline_template', 'oldest {when}', 'text', 'oldest {when}', NULL, NULL,
     'The small line under a grouped deck row. {when} is the OLDEST item waiting on that deck, which is the fact that says how long it has been sitting there — a deck of seventeen answers from this morning is a different afternoon from a deck of seventeen going back three weeks.',
     NULL, now(), 'migration'),

    ('for_you_body_reply_template', 'Reply to “{parent}”: “{text}”', 'text', 'Reply to “{parent}”: “{text}”', NULL, NULL,
     'How a REPLY reads as one row of the For you list: {parent} is the note being answered, {text} the answer to it. The question page draws the same exchange in two lines, one under the other, because it has the room; a list row has one line and must carry both halves or neither, since "Yes, that''s right" on its own says nothing.',
     NULL, now(), 'migration'),

    -- ── 3. The reply control, and the board-4 mark ───────────────────────────
    ('practice_row_note_reply_label', 'Reply', 'text', 'Reply', NULL, NULL,
     'Opens the reply box under one note, on the answers page and on the Review answers page. A reply IS a note — one table, one kind of thing, no second concept — and this control is what makes the pair visible as an exchange rather than as two unrelated notes a day apart.',
     NULL, now(), 'migration'),

    ('practice_row_waiting_note_template', '{who} left a note', 'text', '{who} left a note', NULL, NULL,
     'The mark on a question''s row in the deck list when a NOTE on it is unread by the person reading (FOR_YOU_MOCKUP_v1 board 4). {who} is whoever wrote the newest unread one. It disappears the moment she opens the question, because opening it is what marks it read.',
     NULL, now(), 'migration'),

    ('practice_row_waiting_answer_template', '{who} answered', 'text', '{who} answered', NULL, NULL,
     'The same mark for an unread ANSWER — which is what a REVIEWER reading the deck list sees, since answers wait for the reviewers and notes from the bench wait for the witness. One list, two readers, two marks.',
     NULL, now(), 'migration'),

    ('practice_row_waiting_change', 'the question changed', 'text', 'the question changed', NULL, NULL,
     'The same mark for an unread REWORDING of the question. No {who}: what matters to the witness is not who edited it but that the question she is about to answer is not the one she answered last time.',
     NULL, now(), 'migration'),

    ('practice_row_waiting_more_template', '+{count} more', 'text', '+{count} more', NULL, NULL,
     'Appended to the mark when the row holds more than one unread item: the mark names the newest, and this says how many others are under it. {count} is the number BESIDES the one named, so it is never "+0 more".',
     NULL, now(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────────
-- A seeding INSERT that matched zero rows is silent, and the boot loader reads
-- every one of these by name: a partial seed is a service that will not start.
-- This turns that into a migration that fails instead, which is a deploy that
-- does not happen.
DO $$
DECLARE
    seeded INTEGER;
BEGIN
    SELECT count(*) INTO seeded
      FROM app_settings
     WHERE key IN ('for_you_deck_body_template',
                   'for_you_deck_body_one',
                   'for_you_deck_byline_template',
                   'for_you_body_reply_template',
                   'practice_row_note_reply_label',
                   'practice_row_waiting_note_template',
                   'practice_row_waiting_answer_template',
                   'practice_row_waiting_change',
                   'practice_row_waiting_more_template')
       AND value_kind = 'text'
       AND btrim(value) <> '';
    IF seeded <> 9 THEN
        RAISE EXCEPTION
            'for you L3 wording: expected 9 non-blank text rows, found %', seeded;
    END IF;

    -- The threshold is checked SEPARATELY, and by more than its presence: it is
    -- the one row here that is a number, and a number with bounds. A row seeded
    -- without them would let a Settings edit to 0 through, and 0 collapses every
    -- deck into a single row — a page that can no longer name an item at all.
    IF NOT EXISTS (SELECT 1 FROM app_settings
                    WHERE key = 'practice_for_you_deck_threshold'
                      AND value_kind = 'count'
                      AND min_value >= 1
                      AND max_value IS NOT NULL
                      AND btrim(value) <> '') THEN
        RAISE EXCEPTION
            'for you L3: practice_for_you_deck_threshold is missing, blank, not a count, or unbounded below 1';
    END IF;
END $$;
