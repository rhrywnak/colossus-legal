-- practice_hotfix_attribution_from_login_and_case_timezone: who did it comes
-- from the login, and "today" is the case's day
--
-- Created: 2026-08-19 13:51:56
-- Target: pipeline database (colossus_legal_v2)
--
-- Task CC_TASK_PRACTICE_V1_HOTFIX_WORKFLOW_v1 §1 and §3.2, on Roman's order
-- after testing .402.
--
-- ## What was wrong, in the words of the person who hit it
--
-- "Edit did nothing until a hidden 'Who is editing?' selector was set, and
-- nothing said so." The premise behind that selector — that this build has one
-- shared login and therefore cannot know who is editing — was simply WRONG.
-- Chuck and Marie have had logins since March. Every request already arrives
-- with an authenticated user; the selector was asking a question the server
-- could already answer, and then silently refusing to work until it was answered.
--
-- So: the selectors go, and every attribution column is written from the session.
--
-- ## Additive only
--
-- Four nullable columns, two settings rows deleted, one added, six wording rows
-- added. Nothing is dropped or re-typed; every `ADD COLUMN` carries
-- `IF NOT EXISTS`.


-- ─── 1 · A sitting knows whose it is ─────────────────────────────────────────
--
-- ## Why this column did not exist, and why it has to now
--
-- Practice was built for one witness on one laptop, so "whose sitting is this"
-- had one answer and nobody had to store it. It stops having one answer the
-- moment "Changed since YOUR last sitting" is on the screen: that sentence is a
-- comparison against a particular person's last sitting, and with no user on the
-- row it compares against anybody's — so Chuck opening the page would clear the
-- box for Marie, who has not read a word of it.
--
-- Two columns, for the reason the flag columns give: `user_id` is the identity
-- that survives a display-name change, and `user_name` is what a screen prints.
-- Storing only the id would mean joining to Authentik to render a sentence;
-- storing only the name would lose the person the day somebody is renamed.
--
-- NULL on every sitting started before this migration. That is honest — nobody
-- recorded who they belonged to — and the readers treat NULL as "not this user",
-- so an old sitting never satisfies a per-user comparison.
-- One statement per line, unaligned: the disk/code guard matches the literal
-- prefix `ALTER TABLE <table> ADD COLUMN `, and padding the table name to line
-- the columns up makes every one of these invisible to it.
ALTER TABLE practice_sessions ADD COLUMN IF NOT EXISTS user_id TEXT NULL;
ALTER TABLE practice_sessions ADD COLUMN IF NOT EXISTS user_name TEXT NULL;

COMMENT ON COLUMN practice_sessions.user_id IS
    'The signed-in username (Authentik) this sitting belongs to. NULL on every '
    'sitting opened before 2026-08-19, which is why the per-user reads treat '
    'NULL as "somebody else".';


-- ─── 2 · Every attribution gains its stable id ───────────────────────────────
--
-- The existing columns (`changed_by`, `author`, `flagged_by`) now hold the
-- DISPLAY NAME — what the screen prints beside the note or the change. The new
-- `_id` columns hold the username, which is what identifies the person when a
-- display name changes.
--
-- ## One inconsistency this creates, stated rather than hidden
--
-- `practice_questions.flagged_by` has been written with the USERNAME since Part
-- A (`put_question_flag` passed `user.username`). Rows written before today
-- therefore carry a username in a column that from today carries a display name.
-- On DEV, where every flag was written by `roman`, the two are close enough to
-- read; there is no backfill because there is nothing to backfill FROM — the
-- display name of a past flag was never recorded. New flags carry both.
ALTER TABLE practice_deck_changes ADD COLUMN IF NOT EXISTS changed_by_id TEXT NULL;
ALTER TABLE practice_notes ADD COLUMN IF NOT EXISTS author_id TEXT NULL;
ALTER TABLE practice_questions ADD COLUMN IF NOT EXISTS flagged_by_id TEXT NULL;

COMMENT ON COLUMN practice_deck_changes.changed_by_id IS
    'The signed-in username. `changed_by` beside it is the display name the '
    'screen prints. NULL on rows written before 2026-08-19.';


-- ─── 2b · A fourth mark: hidden before she was ever asked ────────────────────
--
-- A question can be hidden from the deck while an open sitting still has it
-- QUEUED and not yet dealt. When the sitting reaches it there is nothing to ask,
-- and none of the three existing marks is true: she did not answer it, she did
-- not set it aside (`skipped` is her act, and this was Chuck's), and it is not
-- fine. So the sheet says what actually happened.
--
-- `DROP CONSTRAINT IF EXISTS` then `ADD`, which is the shape flow v1 used when
-- it widened this same vocabulary from two values to three.
ALTER TABLE practice_answers DROP CONSTRAINT IF EXISTS practice_answers_mark_check;
ALTER TABLE practice_answers ADD  CONSTRAINT practice_answers_mark_check
    CHECK (mark IN ('fine', 'repeat', 'skipped', 'hidden'));


-- ─── 3 · The two selector vocabularies go ────────────────────────────────────
--
-- `practice_note_authors` and `practice_editor_authors` existed ONLY to fill the
-- two dropdowns. With attribution coming from the login there is nothing to
-- fill, nothing to validate against, and — this is the part that matters — an
-- allow-list of display names would now be a way to lock a real signed-in user
-- out of writing a note about a witness's testimony, silently, because their
-- name is spelled differently in Authentik than in a settings row.
--
-- DELETE and not "leave them lying about": a stored row nothing reads is the
-- defect this repo has hit before (the unreachable QuestionLine, 2026-08-10).
DELETE FROM app_settings WHERE key IN ('practice_note_authors', 'practice_editor_authors');


-- ─── 4 · "Today" is the CASE's day, not UTC ──────────────────────────────────
--
-- Owed from Part A and reported twice. Every timestamp on these tables is UTC,
-- and the row status compared them in UTC — so at 20:00 EDT every answer Marie
-- gave that evening flipped from `answered today` to `last: Wed 19 Aug`, four
-- hours before her day ended. She is practising in the evening. It is the only
-- time she practises.
--
-- ## Why a settings ROW and why Postgres does the comparing
--
-- The zone is case data (Roman's ruling: `America/Detroit`), so Rule 2 puts it
-- in the store rather than in code. And the comparison happens in SQL —
-- `(answered_at AT TIME ZONE $tz)::date` — rather than in Rust, because Postgres
-- already carries the full tz database and this backend does not: the
-- alternative was adding `chrono-tz` as a dependency to answer a question the
-- database can already answer exactly.
--
-- A zone name Postgres does not know makes the read FAIL LOUDLY (`invalid time
-- zone`) rather than silently falling back to UTC, which is the behaviour worth
-- having: a silent fallback here is the bug this row exists to fix, wearing a
-- disguise.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('practice_case_timezone', 'America/Detroit', 'text', 'America/Detroit',
     NULL, NULL,
     'The IANA zone this case''s days are counted in. It decides when "answered '
     'today" becomes "last: <date>" on a deck row, and when the unfinished-session '
     'line says "today". Case data, not a deployment value: the witness practises '
     'in the evening in Michigan, and comparing in UTC ended her day at 20:00 '
     'local. Postgres does the comparing, so any zone it knows is valid and one '
     'it does not know fails the read loudly rather than falling back to UTC.',
     'repositories::pipeline_repository::practice_flow', NOW(), 'migration'),

    -- ── §2 · edit mode is a MODE ─────────────────────────────────────────────
    ('practice_editor_busy_hint', 'Finish editing first', 'text',
     'Finish editing first', NULL, NULL,
     'The hint on every control the deck editor disables — Start, the count '
     'pills, the side cards, the fold, Resume and Start over. Hard rule from '
     'this task: no control on a practice page may silently do nothing, so a '
     'disabled control carries its reason.',
     'frontend PracticeStart', NOW(), 'migration'),
    ('practice_editor_discard_confirm_template',
     'Discard the unsaved edit on Q{n}?', 'text',
     'Discard the unsaved edit on Q{n}?', NULL, NULL,
     'Asked when the editor is left with an inline edit still open. Saved '
     'changes are already written and are not at risk; this is only about the '
     'one row still in the fields, and it names which.',
     'frontend PracticeDeckList', NOW(), 'migration'),

    -- ── §3 · the walk ────────────────────────────────────────────────────────
    ('practice_answer_empty_hint',
     'Type your answer first — or press "I don''t recall."', 'text',
     'Type your answer first — or press "I don''t recall."', NULL, NULL,
     'Why Answer is disabled on an empty box. Domain note: it names the OTHER '
     'control deliberately — "I don''t recall." is a complete answer and stays '
     'one click, and a witness looking at a disabled Answer button needs telling '
     'that saying nothing is not the same as having nothing to say.',
     'frontend PracticeQuestion', NOW(), 'migration'),
    ('practice_mark_hidden_before_asked', 'hidden before asked', 'text',
     'hidden before asked', NULL, NULL,
     'The mark on a sheet row for a question that was hidden from the deck while '
     'this sitting still had it queued. Not `skipped` — she never set it aside, '
     'and never saw it. The sheet says what happened.',
     'services::practice_sheet', NOW(), 'migration'),
    ('practice_review_asked_as_template', 'asked as: “{text}”', 'text',
     'asked as: “{text}”', NULL, NULL,
     'Under one attempt on the review page, when the question has been re-worded '
     'since. The header shows the CURRENT question; this says what she was '
     'actually asked that time, which is what her answer answers.',
     'frontend PracticeReview', NOW(), 'migration'),
    ('practice_answer_already_recorded',
     'That question is already answered in this sitting — this tab is behind. Reload to see it.',
     'text',
     'That question is already answered in this sitting — this tab is behind. Reload to see it.',
     NULL, NULL,
     'Shown when a second tab answers a question the first already answered. It '
     'names the CAUSE (this tab is behind) rather than blaming her, because two '
     'tabs open on one sitting is a thing a person does, not a mistake.',
     'frontend PracticeQuestion', NOW(), 'migration');


-- ─── 5 · Three strings the pickers were the only readers of ──────────────────
--
-- `practice_editor_as_label` ("Editing as"), `practice_editor_as_unset`
-- ("— who? —") and `practice_notes_author_unset` ("— who is writing? —") had one
-- reader each, and both readers are deleted by §3. A stored row nothing reads is
-- the defect this repo already has on file (the unreachable QuestionLine,
-- 2026-08-10), so they go with the controls rather than after them.
--
-- Their domain blocks drop the keys in the same change, which is what makes this
-- safe: boot REFUSES to start on a key a block declares and the store does not
-- hold, so a DELETE without the matching block edit would take the backend down.
DELETE FROM app_settings
 WHERE key IN ('practice_editor_as_label',
               'practice_editor_as_unset',
               'practice_notes_author_unset');


-- ─── 6 · The one string the pickers' removal OWES ────────────────────────────
--
-- Save on the notes panel used to be disabled for two reasons — no author, and
-- no text — and said neither. One reason is gone with the picker; the other is
-- still real, and §1's hard rule is that no control may be disabled without
-- carrying why.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('practice_notes_empty_hint', 'Type the note first', 'text',
     'Type the note first', NULL, NULL,
     'Why Save is disabled on an empty note box. The server refuses a blank note '
     'anyway; this says so before the click rather than after it.',
     'frontend PracticeNotes', NOW(), 'migration');
