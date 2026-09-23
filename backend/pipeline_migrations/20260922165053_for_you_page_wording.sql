-- for_you_page_wording: every word the "For you" page speaks
--
-- Created: 2026-09-22 16:50:53
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_FOR_YOU_v1, layer L1. The ONE migration this layer writes: the
-- twenty-five rows `domain::wording_for_you` declares, and nothing else. No
-- schema, no back-fill, no correction of an existing row.
-- =============================================================================
--
-- ## Why every visible string on this page is a row
--
-- Ruling Q5: the nav label "For you" is structural and lives in `navItems.ts`
-- with the rest of the furniture; EVERY other string a person reads on this
-- page is stored here, so retuning a sentence is a Settings edit and a restart,
-- never a rebuild. The backend refuses to start if one of these rows is
-- missing, naming the key — which is why they are seeded before the code that
-- declares them can ship.
--
-- ## Domain note: the sentences are the mockup's
--
-- The values below are `FOR_YOU_MOCKUP_v1_2026-09-22.html` boards 1-3 and 5,
-- reproduced rather than paraphrased, with two exceptions recorded in the build
-- report: the reviewer's subtitle drops the mockup's "Pressing Done reviewing
-- on a deck clears its rows" (that control is layer L2 and does not exist yet),
-- and the empty state's third line reads "Last one:" rather than "Last note:",
-- because the same page serves a reviewer whose last item is usually an answer.
--
-- ## Domain note: names are values, not code
--
-- `for_you_witness_name` is "Marie" HERE and nowhere in the source. The same
-- discipline the war room already follows, for the same reason: a second case
-- on this build has a different witness and must not need a new binary.
--
-- ## Fresh-database reproduction (Law 20)
--
-- These rows are INSERTed with `ON CONFLICT (key) DO NOTHING`, so a database
-- built from zero ends with exactly these values and a database that somehow
-- already carries one keeps what it has. The assertion at the foot counts what
-- is present afterwards, because an INSERT that matched nothing is silent in
-- Postgres and the boot check would then fail on a key this file claims to have
-- seeded.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('for_you_title', 'For you', 'text', 'For you', NULL, NULL,
     'The page''s title, and the heading above the list. The one place that answers "what is waiting for me?" across every scenario in the case.',
     NULL, now(), 'migration'),

    ('for_you_subtitle_witness', '{who}''s notes on your answers. Newest first. A row clears when you open it.', 'text', '{who}''s notes on your answers. Newest first. A row clears when you open it.', NULL, NULL,
     'The line under the title when the WITNESS is reading. {who} is the reviewer bench, joined as the war room joins it.',
     NULL, now(), 'migration'),

    ('for_you_subtitle_reviewer', '{who}''s new answers and her notes. Newest first. A row clears when you open it.', 'text', '{who}''s new answers and her notes. Newest first. A row clears when you open it.', NULL, NULL,
     'The same line when a REVIEWER is reading. {who} is the witness, named by for_you_witness_name.',
     NULL, now(), 'migration'),

    ('for_you_not_your_list', 'Nothing is waiting for you here. This page carries the answers and notes that pass between the witness and the reviewers.', 'text', 'Nothing is waiting for you here. This page carries the answers and notes that pass between the witness and the reviewers.', NULL, NULL,
     'What the page says to a signed-in person who is neither a reviewer nor the witness. Not an error: an honest sentence naming whose page this is.',
     NULL, now(), 'migration'),

    ('for_you_tab_unread_template', 'Unread · {count}', 'text', 'Unread · {count}', NULL, NULL,
     'The first tab: what is waiting. {count} is the number of unread items on this person''s side.',
     NULL, now(), 'migration'),

    ('for_you_tab_everything_template', 'Everything · {count}', 'text', 'Everything · {count}', NULL, NULL,
     'The second tab: the same list with the already-read rows still in it, so a note that has been opened can be found again.',
     NULL, now(), 'migration'),

    ('for_you_group_today', 'TODAY', 'text', 'TODAY', NULL, NULL,
     'The heading over rows written today, in the case''s timezone (practice_case_timezone). The day is decided on the server; the browser never compares dates.',
     NULL, now(), 'migration'),

    ('for_you_group_yesterday', 'YESTERDAY', 'text', 'YESTERDAY', NULL, NULL,
     'The heading over rows written the day before today, in the case''s timezone.',
     NULL, now(), 'migration'),

    ('for_you_group_earlier', 'EARLIER', 'text', 'EARLIER', NULL, NULL,
     'The heading over everything older than yesterday. One heading, not a date per day: the list is read newest-first and the exact day is on each row.',
     NULL, now(), 'migration'),

    ('for_you_deck_line_template', '{code} · {deck} — “{question}”', 'text', '{code} · {deck} — “{question}”', NULL, NULL,
     'A row''s first line: which deck this came from and which question it is about. {code} is the scenario code (S-11), {deck} its name, {question} the question''s text.',
     NULL, now(), 'migration'),

    ('for_you_deck_line_no_question_template', '{code} · {deck}', 'text', '{code} · {deck}', NULL, NULL,
     'The same first line for a note left on a whole scenario rather than on one question — there is no question to quote.',
     NULL, now(), 'migration'),

    ('for_you_body_answer_template', 'Answered: “{text}”', 'text', 'Answered: “{text}”', NULL, NULL,
     'The body of a row about a new answer. {text} is the answer as the witness wrote it.',
     NULL, now(), 'migration'),

    ('for_you_body_change_template', 'Now reads: “{text}”', 'text', 'Now reads: “{text}”', NULL, NULL,
     'The body of a row about a reworded or edited question. {text} is the question''s new wording. A row about a note shows the note itself and uses no template.',
     NULL, now(), 'migration'),

    ('for_you_byline_template', '{who} · {what}', 'text', '{who} · {what}', NULL, NULL,
     'The small line under a row. {who} is the person who wrote it, by display name — never a login. {what} is one of the for_you_byline_* clauses, which is what makes each row say what KIND of thing it is.',
     NULL, now(), 'migration'),

    ('for_you_byline_note_on_answer_witness', 'on your answer of {when}', 'text', 'on your answer of {when}', NULL, NULL,
     '{what} for a note left on the witness''s own answer, as SHE reads it. {when} is the day that answer was given.',
     NULL, now(), 'migration'),

    ('for_you_byline_note_on_answer_reviewer', 'note on her answer', 'text', 'note on her answer', NULL, NULL,
     'The same note as a REVIEWER reads it: about her answer, not his. Its own row because the two sides are two different sentences about one event.',
     NULL, now(), 'migration'),

    ('for_you_byline_note_on_question', 'on the question', 'text', 'on the question', NULL, NULL,
     '{what} for a note left on the question itself rather than on one attempt at it.',
     NULL, now(), 'migration'),

    ('for_you_byline_answer', 'new answer', 'text', 'new answer', NULL, NULL,
     '{what} for a new answer from the witness.',
     NULL, now(), 'migration'),

    ('for_you_byline_change', 'changed the question', 'text', 'changed the question', NULL, NULL,
     '{what} for a reworded or edited question. Only those two kinds of deck change count as waiting; moving, hiding and unhiding do not change what was asked.',
     NULL, now(), 'migration'),

    ('for_you_byline_read_suffix_template', '· read by you {when}', 'text', '· read by you {when}', NULL, NULL,
     'Appended to a row''s byline on the Everything tab when this person has already read it. Never shown on the Unread tab, where by construction there is no moment to name.',
     NULL, now(), 'migration'),

    ('for_you_empty_title', 'Nothing waiting for you.', 'text', 'Nothing waiting for you.', NULL, NULL,
     'The empty state''s first line, shown when this person''s side holds no unread item.',
     NULL, now(), 'migration'),

    ('for_you_empty_hint_template', '{who}''s notes and answers appear here as they arrive.', 'text', '{who}''s notes and answers appear here as they arrive.', NULL, NULL,
     'The empty state''s second line. {who} is the other side, so the page says where the next row will come from rather than only that there is none.',
     NULL, now(), 'migration'),

    ('for_you_empty_last_template', 'Last one: {when}.', 'text', 'Last one: {when}.', NULL, NULL,
     'The empty state''s third line. {when} is the day of the most recent item on this side, read or not. Withheld entirely when there has never been one, rather than rendered with an empty date.',
     NULL, now(), 'migration'),

    ('for_you_witness_name', 'Marie', 'text', 'Marie', NULL, NULL,
     'What this page calls the witness (practice_witness_username). The reviewers'' display names are a settings row already (practice_reviewer_display_names) and hers was not; a login on screen is a database identifier shown to a lawyer.',
     NULL, now(), 'migration'),

    ('for_you_unknown_author', 'Someone', 'text', 'Someone', NULL, NULL,
     'The name a row shows for an item written before this application recorded who wrote it (before 2026-08-19). Such rows wait for nobody — the item-seen migration marks them read for everyone — but they still appear on the Everything tab, and a blank where a name goes reads as a page that failed to load.',
     NULL, now(), 'migration'),

    -- A PRACTICE row, seeded here because this task is what made it necessary:
    -- the question page shows it when the read-clear the list asked for did not
    -- land. Its key is practice_row_*, so the assertion below — which counts
    -- for_you_% rows — does not count it, deliberately.
    ('practice_row_read_failed', 'This question could not be marked as read — it is still on your For you list.', 'text', 'This question could not be marked as read — it is still on your For you list.', NULL, NULL,
     'Shown on a question opened from the For you list when the write that marks it read failed. The question itself is on screen and readable — only the bookkeeping failed — so this is a line rather than a barrier, and it says the row is still waiting on the list, which is the true consequence.',
     NULL, now(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────────
-- A seeding INSERT that matched zero rows is silent. The boot loader reads all
-- twenty-five of these by name and REFUSES TO START if one is missing, so a
-- silent partial seed here is a deploy that takes the service down — this turns
-- it into a failed migration instead, which is a deploy that does not happen.
DO $$
DECLARE
    seeded INTEGER;
BEGIN
    SELECT count(*) INTO seeded
      FROM app_settings
     WHERE key LIKE 'for_you\_%' AND value_kind = 'text' AND btrim(value) <> '';
    IF seeded <> 25 THEN
        RAISE EXCEPTION
            'for you wording: expected 25 non-blank text rows, found %', seeded;
    END IF;
END $$;
