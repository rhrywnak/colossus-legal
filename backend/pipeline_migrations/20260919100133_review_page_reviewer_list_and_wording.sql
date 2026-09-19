-- review_page_reviewer_list_and_wording: the Review answers page, and a reviewer BENCH
--
-- Created: 2026-09-19 10:01:33
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_REVIEW_PAGE_v1 — THE ONE MIGRATION this task writes (Law 20).
-- Rulings of 2026-09-19 (STOP-A … STOP-H in REVIEW_PAGE_v1_PLAN.md).
-- =============================================================================
--
-- ## Three things, one release
--
-- 1. A Review answers page: one scrollable page, deck order, Marie's current
--    answer under each question with the notes on it and a box to add one.
--    Every string it speaks is a row below.
-- 2. The reviewer stops being ONE login and becomes a LIST. `can_mark_reviewed`
--    was an equality test; it becomes membership.
-- 3. The deck's button row drops Print answers and gains Review answers; the
--    printed-answers PAGE is untouched and is reached from the new page by a
--    ghost link, so `practice_print_answers_label` is REPOINTED rather than
--    retired (ruling STOP-G) — the row keeps its key and says "Print" because it
--    now sits on a page that is already about answers.
--
-- ## Why the reviewer becomes a list, and what that does NOT change
--
-- Chuck reviews Marie's answers. Before trial, so does Roman, and the build had
-- no way to say so without handing the whole queue from one man to the other.
-- Two rows, index-aligned — the SHAPE, not the values this file writes: it seeds
-- each from the singular row it replaces, so a one-name store stays one name
-- until somebody adds a second on the Settings page.
--
--   practice_reviewer_usernames      cpenzien,roman
--   practice_reviewer_display_names  Chuck,Roman
--
-- Comma-separated, which is this store's established list encoding — read by
-- `settings_row_readers::verbatim_list_of`, the case-PRESERVING sibling of the
-- reader `practice_tactic_names` uses. A vocabulary is lower-cased because
-- `Cross` and `cross` are one word; a login is an identity and a display name is
-- printed on a screen, and `Chuck,Roman` must not arrive as `chuck · roman`.
-- The backend refuses to boot if the two lists differ in length — a name list
-- one short would print the wrong attorney's name beside a queue, silently.
--
-- ## Domain note: ONE SHARED CURSOR (Roman, 2026-09-19)
--
-- The queue is still one number every viewer sees. Any listed reviewer's Done
-- reviewing press moves the shared mark and the count clears for everyone —
-- the mark is the LATEST press by anyone on the list, not one person's row.
-- And an answer or a note written by ANY listed reviewer no longer waits: with
-- one name in the exclusion, Roman's own note would have counted as work
-- waiting for Roman. Both are ruled changes to what the queue MEANS, and both
-- are marked STRUCTURAL in `review_cursor.rs`.
--
-- ## Seeded FROM THE STANDING VALUES, never from a literal
--
-- The two new rows are seeded by SELECT from the rows they replace, so a store
-- whose reviewer had already been changed on the Settings page keeps that
-- change. A fresh database seeds them from the values the 2026-09-17 migration
-- inserted. Either way the END state is asserted below.
--
-- ## Retirements (the established pattern)
--
-- `practice_reviewer_username` and `practice_reviewer_display_name` are DELETED
-- once their successors hold their values — the same shape, and the same
-- end-state assertion, that `simple_counts` used on 2026-09-17. Nothing reads
-- the singular rows after this release: `PracticeReadParams` declares the
-- plural pair, and the boot check refuses to start without them.
--
-- ## Idempotent
--
-- ON CONFLICT (key) DO NOTHING on the inserts, a guarded UPDATE, and a DELETE
-- that matches nothing on a re-run. The assertion checks the END state either
-- way (CLAUDE.md rule 25a).
--
-- ## Rolling this back
--
-- Four steps, written out in prose so it exists before anybody needs it at
-- speed. (Prose and not SQL: `settings_store_tests` scans every migration for
-- statements naming a column the table does not have, and a worked example in a
-- comment is indistinguishable to it from a statement that will run.)
--
--   1. Re-create practice_reviewer_username as a text row whose value is the
--      FIRST comma-separated entry of practice_reviewer_usernames.
--   2. Re-create practice_reviewer_display_name the same way, from
--      practice_reviewer_display_names.
--   3. Delete the two plural rows.
--   4. Set practice_print_answers_label back to '🖨 Print answers'.
--
-- The nine wording rows may stay: nothing reads them once the page is gone, and
-- a row nothing reads costs nothing.

-- ─── 1 · The reviewer bench, seeded from the rows it replaces ────────────────

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
SELECT
    'practice_reviewer_usernames',
    value,
    'text',
    value,
    NULL,
    NULL,
    'The signed-in usernames (Authentik) of everyone who reviews Marie''s answers, comma-separated. The review queue is counted against this BENCH: any of them may press Done reviewing, their press moves the one shared mark, and nothing any of them writes counts as work waiting for them. Index-aligned with practice_reviewer_display_names — the backend refuses to start if the two lists differ in length.',
    'api::war_room_progress, api::practice_review_cursor, repositories::review_cursor',
    now(),
    'migration'
  FROM app_settings
 WHERE key = 'practice_reviewer_username'
ON CONFLICT (key) DO NOTHING;

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
SELECT
    'practice_reviewer_display_names',
    value,
    'text',
    value,
    NULL,
    NULL,
    'The reviewers'' names as screens print them, comma-separated, in the SAME ORDER as practice_reviewer_usernames. {reviewer} in the review pill and bar prints them joined by practice_review_name_joiner; the owner chip prints the same line.',
    'frontend WarRoomSummaryCard, warRoomCardView, PracticeReviewBar',
    now(),
    'migration'
  FROM app_settings
 WHERE key = 'practice_reviewer_display_name'
ON CONFLICT (key) DO NOTHING;

-- ─── 2 · The Review answers page's own words ─────────────────────────────────

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('practice_review_title', 'Review answers', 'text', 'Review answers', NULL, NULL,
     'The name of the page, in three places: the control in the deck''s button row, the last crumb above the page, and the eyebrow over its title. One row because they are one name — three rows would let them drift.',
     'frontend PracticeTitleRow, PracticeReviewPage', now(), 'migration'),

    ('practice_review_unanswered', 'Not answered yet — a note here lands on the question and Marie sees it before she answers.', 'text', 'Not answered yet — a note here lands on the question and Marie sees it before she answers.', NULL, NULL,
     'Shown where a question has no answer. It says where a note written there will land, because on every other card the note lands on the answer and a reader cannot see the difference.',
     'frontend PracticeReviewCard', now(), 'migration'),

    ('practice_review_note_placeholder_answer', 'Add a note on this answer…', 'text', 'Add a note on this answer…', NULL, NULL,
     'The add-note box under an ANSWERED question. A note written here lands on the answer that stands now.',
     'frontend PracticeReviewCard', now(), 'migration'),

    ('practice_review_note_placeholder_question', 'Add a note on this question…', 'text', 'Add a note on this question…', NULL, NULL,
     'The add-note box under an UNANSWERED question. A note written here lands on the question itself.',
     'frontend PracticeReviewCard', now(), 'migration'),

    ('practice_review_answered_template', 'Answered {when} · {author}', 'text', 'Answered {when} · {author}', NULL, NULL,
     'The line under an answer on the review page. {when} is the day in the case''s timezone; {author} is the name on the sitting the answer was written in. Composed server-side, like every other date on this surface.',
     'services::practice_page', now(), 'migration'),

    ('practice_review_author_unknown', 'author not recorded', 'text', 'author not recorded', NULL, NULL,
     'Stands in for {author} when the sitting carries no name — sittings from before 2026-08-19 do not. Said out loud rather than left blank: a missing name and a blank name are different facts, and only one of them is worth asking about.',
     'services::practice_page', now(), 'migration'),

    ('practice_review_load_failed', 'The answers for this deck could not be loaded.', 'text', 'The answers for this deck could not be loaded.', NULL, NULL,
     'Shown instead of the page when either of its two reads fails. The technical cause is appended by the page and logged; this is the sentence that says the page is not simply empty.',
     'frontend PracticeReviewPage', now(), 'migration'),

    ('practice_review_empty_deck', 'This scenario has no questions yet, so there is nothing to review.', 'text', 'This scenario has no questions yet, so there is nothing to review.', NULL, NULL,
     'Shown when the deck loaded and is empty. Distinct from the failure above: a deck with no questions and a deck that would not load are different states and must read differently.',
     'frontend PracticeReviewPage', now(), 'migration'),

    ('practice_review_name_joiner', '·', 'text', '·', NULL, NULL,
     'Between two reviewers'' names wherever {reviewer} prints the bench. Stored WITHOUT its spaces — the store trims every value, so the server supplies the space on each side.',
     'api::practice_review_cursor, api::trial_prep', now(), 'migration'),

    ('practice_deck_review_oldest_template', '· oldest waiting since {date}', 'text', '· oldest waiting since {date}', NULL, NULL,
     'Appended to the review bar''s sentence when something is waiting. {date} is the day the OLDEST waiting answer or note arrived — the same moment the War Room summary card names, from the same query, so the two cannot disagree. Withheld when the read returned no date.',
     'frontend PracticeReviewBar', now(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ─── 3 · Print answers → Print (ruling STOP-G) ───────────────────────────────
--
-- The button this row used to label is gone; the row now labels the ghost link
-- at the top of the Review answers page that opens the same paper. It keeps the
-- printer glyph every other print control on this surface carries.
--
-- ## ⚑ Guarded on the NEW value, not on the old one (found 2026-09-19)
--
-- The first draft of this statement read `AND value = 'Print answers'`, and
-- matched NOTHING on DEV — where the row has read `🖨 Print answers` since the
-- L2 migration corrected it. A guard on a value that has already drifted is a
-- statement that silently does nothing, which is the exact failure rule 25a
-- exists for; it was caught by proving this file in a rolled-back transaction
-- against the live store before it shipped, and by nothing else.
--
-- So the guard is `<> '🖨 Print'`: idempotent on a re-run, and the END state is
-- asserted below rather than assumed. The row's MEANING changes in this release
-- — it labelled a button that no longer exists — so a customised value is NOT
-- carried over; repointing it wholesale is what "repoint" means here.

UPDATE app_settings
   SET value         = '🖨 Print',
       default_value = '🖨 Print',
       meaning       = 'The ghost link at the top right of the Review answers page, which opens the printed-answers sheet in a new tab. It used to label a button in the deck''s button row; that button was replaced by Review answers on 2026-09-19 and the paper moved one click further in.',
       consumed_by   = 'frontend PracticeReviewPage',
       updated_at    = now(),
       updated_by    = 'migration'
 WHERE key           = 'practice_print_answers_label'
   AND value        <> '🖨 Print';

-- ─── 4 · Retirements ─────────────────────────────────────────────────────────
--
-- Only after the two inserts above have taken their values. On a store where the
-- successors already existed (a re-run), the inserts did nothing and these rows
-- are already gone — the DELETE matches nothing and the assertion still holds.

DELETE FROM app_settings
 WHERE key IN (
    'practice_reviewer_username',
    'practice_reviewer_display_name');

-- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────────
--
-- Four things are asserted, because four different silent failures are possible
-- here and Postgres reports none of them:
--
--   * an INSERT … SELECT whose source row was missing inserts NOTHING, and the
--     backend would then refuse to boot with a missing key — after the deploy;
--   * a mistyped wording key inserts a row nothing reads and leaves the one the
--     page wants absent;
--   * a DELETE that matched nothing leaves a retired row being served;
--   * two lists of different lengths print the wrong name beside the queue,
--     which is the one failure here that looks like working software.

DO $$
DECLARE
    present   INTEGER;
    retired   INTEGER;
    logins    TEXT;
    names     TEXT;
    printed   TEXT;
BEGIN
    SELECT count(*) INTO present
      FROM app_settings
     WHERE key IN (
        'practice_reviewer_usernames',
        'practice_reviewer_display_names',
        'practice_review_title',
        'practice_review_unanswered',
        'practice_review_note_placeholder_answer',
        'practice_review_note_placeholder_question',
        'practice_review_answered_template',
        'practice_review_author_unknown',
        'practice_review_load_failed',
        'practice_review_empty_deck',
        'practice_review_name_joiner',
        'practice_deck_review_oldest_template');
    IF present <> 12 THEN
        RAISE EXCEPTION
            'review page: expected 12 new settings rows, found %. The backend '
            'declares every one at boot and refuses to start without it.', present;
    END IF;

    SELECT count(*) INTO retired
      FROM app_settings
     WHERE key IN ('practice_reviewer_username', 'practice_reviewer_display_name');
    IF retired <> 0 THEN
        RAISE EXCEPTION 'review page: % retired reviewer rows are still present.', retired;
    END IF;

    SELECT value INTO logins FROM app_settings WHERE key = 'practice_reviewer_usernames';
    SELECT value INTO names  FROM app_settings WHERE key = 'practice_reviewer_display_names';
    IF logins IS NULL OR btrim(logins) = '' OR names IS NULL OR btrim(names) = '' THEN
        RAISE EXCEPTION 'review page: the reviewer bench is missing or blank.';
    END IF;

    -- Index-aligned, and the alignment is what makes a name mean a login.
    IF array_length(string_to_array(logins, ','), 1)
       <> array_length(string_to_array(names, ','), 1) THEN
        RAISE EXCEPTION
            'review page: % logins but % display names. Every reviewer needs a '
            'name in the same position, or the queue prints somebody else''s.',
            array_length(string_to_array(logins, ','), 1),
            array_length(string_to_array(names, ','), 1);
    END IF;

    -- The END state, not "non-blank": the first draft's guard matched nothing on
    -- DEV and left the row reading the old button's words, which a blank check
    -- would have passed. The link says what the assertion says it says.
    SELECT value INTO printed FROM app_settings WHERE key = 'practice_print_answers_label';
    IF printed IS DISTINCT FROM '🖨 Print' THEN
        RAISE EXCEPTION
            'review page: practice_print_answers_label reads %, not the repointed '
            'link label. The UPDATE above matched nothing.', coalesce(printed, '<missing>');
    END IF;
END $$;
