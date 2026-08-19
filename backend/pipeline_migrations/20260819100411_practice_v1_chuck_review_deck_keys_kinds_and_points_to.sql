-- practice_v1_chuck_review_deck_keys_kinds_and_points_to: Practice v1 — the deck
-- gains a stable key and a KIND, the log gains what she would point to
--
-- Created: 2026-08-19 10:04:11
-- Target: pipeline database (colossus_legal_v2)
--
-- Task CC_TASK_PRACTICE_V1_CHUCK_REVIEW_v1, Part A (items A2–A8). Chuck reads
-- the deck on Thursday morning and may not be available next week, so the parts
-- that let him CHANGE it — and the parts that let Marie practise one question
-- at a time and see what happened to it — are what this migration makes
-- representable.
--
-- ## Additive only, and why every statement is guarded
--
-- Four columns, one unique index, one constraint swap, one backfill, twelve new
-- `app_settings` rows and two edits to existing ones. Nothing is dropped and no
-- existing column is re-typed. Every `ADD COLUMN` carries `IF NOT EXISTS` and
-- the one constraint is dropped-if-exists before it is added, which is the same
-- shape the flow v1 migration used when it widened the `mark` vocabulary.
--
-- ## Why the deck needs a KEY at all
--
-- Chuck re-orders the deck in the editor (Part B) and the architect re-orders it
-- in `practice_decks/S-5.yaml`. Without a stable handle those two orders can
-- only be reconciled by matching on TEXT — which stops working the first time
-- he re-words a question, and silently inserts a duplicate rather than updating
-- the row he meant. `deck_key` is that handle: written by a human in the file,
-- unique per scenario, and never re-used.


-- ─── 1 · The deck's stable key ───────────────────────────────────────────────
--
-- NULLABLE on purpose. S-5's ten rows were seeded before this column existed
-- and they are matched ONCE by exact text to receive their keys (the seed's
-- `--update` path); until that runs they carry NULL, and a NULL key is an
-- honest "this row predates the key" rather than a made-up one.
--
-- The UNIQUE index is per SCENARIO, not global: `g1` means "George's first" in
-- every deck, and a global unique would make the second scenario's file wrong
-- for no reason. Postgres treats NULLs as distinct in a unique index, so the
-- un-keyed rows do not collide with each other while they wait.
ALTER TABLE practice_questions ADD COLUMN IF NOT EXISTS deck_key TEXT NULL;

ALTER TABLE practice_questions DROP CONSTRAINT IF EXISTS practice_questions_deck_key_nonblank;
ALTER TABLE practice_questions ADD  CONSTRAINT practice_questions_deck_key_nonblank
    CHECK (deck_key IS NULL OR btrim(deck_key) <> '');

CREATE UNIQUE INDEX IF NOT EXISTS practice_questions_deck_key_unique
    ON practice_questions (scenario_id, deck_key);


-- ─── 2 · What KIND of question it is ─────────────────────────────────────────
--
-- `side` says who is speaking; it cannot say what the question DOES. Chuck asks
-- two different kinds of question and they are answered by opposite rules: a
-- direct question opens a subject, a REDIRECT repairs one George just damaged,
-- and it exists only because of the George question it follows. The read judges
-- them differently (prompt v2), the mixed queue deals them differently, and the
-- screen tags them differently — three behaviours that cannot hang off `side`.
--
-- Added nullable, backfilled, then made NOT NULL. Adding it NOT NULL with a
-- default would have written 'cross' onto Chuck's five direct questions and
-- called it done.
ALTER TABLE practice_questions ADD COLUMN IF NOT EXISTS kind TEXT NULL;

UPDATE practice_questions
   SET kind = CASE WHEN side = 'george' THEN 'cross' ELSE 'direct' END
 WHERE kind IS NULL;

ALTER TABLE practice_questions ALTER COLUMN kind SET DEFAULT 'cross';
ALTER TABLE practice_questions ALTER COLUMN kind SET NOT NULL;

ALTER TABLE practice_questions DROP CONSTRAINT IF EXISTS practice_questions_kind_check;
ALTER TABLE practice_questions ADD  CONSTRAINT practice_questions_kind_check
    CHECK (kind IN ('cross', 'direct', 'redirect'));

-- A redirect answers ONE George question, named by that question's `deck_key`.
--
-- Deliberately not a foreign key to `practice_questions(id)`. Two reasons: the
-- file is what authors the relationship and the file speaks in keys, not uuids;
-- and the seed writes the George row and its redirect in the same transaction,
-- where an FK would force an ordering the file does not have to obey. The seed
-- REFUSES a `follows` naming a key the deck does not carry, which is the check
-- an FK would have bought, made at the moment a human can still fix the file.
ALTER TABLE practice_questions ADD COLUMN IF NOT EXISTS follows_key TEXT NULL;

ALTER TABLE practice_questions DROP CONSTRAINT IF EXISTS practice_questions_follows_only_redirect;
ALTER TABLE practice_questions ADD  CONSTRAINT practice_questions_follows_only_redirect
    CHECK (follows_key IS NULL OR kind = 'redirect');

COMMENT ON COLUMN practice_questions.kind IS
    'cross (George''s attack as a question) | direct (Chuck opens a subject) | '
    'redirect (Chuck repairs the subject George just damaged). Widening this '
    'vocabulary is a migration plus a code change, both loud.';
COMMENT ON COLUMN practice_questions.deck_key IS
    'The stable handle the deck FILE uses (g1, c3, r2). Unique per scenario. '
    'NULL on rows seeded before 2026-08-19; the seed''s --update path matches '
    'those by exact text, once, to give them one.';


-- The exhibit ONE question stands on, named the way Marie would name it aloud.
--
-- ## Why this is a column and not the `receipt` line already on the row
--
-- `receipt` is a paragraph of provenance ("Built from what Phillips told the
-- court: “…” — Hearing…, p. 34"). It is written to be READ under a question. The
-- picker needs something else: a short handle she can choose off a list, in the
-- register of a witness pointing at a document. Deriving one from the other would
-- mean the machine parsing authored prose and printing what it cut out — and the
-- Chuck rows prove it cannot: their receipts end in "establishes point 1", which
-- is not a document at all.
--
-- NULL is the ordinary state. A question that stands on no document of its own —
-- every Chuck direct, every redirect — carries none, and contributes nothing to
-- the list.
ALTER TABLE practice_questions ADD COLUMN IF NOT EXISTS source_line TEXT NULL;

ALTER TABLE practice_questions DROP CONSTRAINT IF EXISTS practice_questions_source_line_nonblank;
ALTER TABLE practice_questions ADD  CONSTRAINT practice_questions_source_line_nonblank
    CHECK (source_line IS NULL OR btrim(source_line) <> '');

COMMENT ON COLUMN practice_questions.source_line IS
    'The exhibit this question stands on, as Marie would name it — the handle '
    'the "I''d point to…" picker offers. Authored in the deck file; NULL on every '
    'question that stands on no document of its own.';


-- ─── 3 · What she would point to ─────────────────────────────────────────────
--
-- Under the answer box she may name the exhibits she would reach for. Optional,
-- never required, and never a grade: the point is that Chuck can see whether
-- the receipt she reached for is the one that actually answers the question.
--
-- ## Why the TEXT is stored and not a reference
--
-- The list she picks from is composed from the deck and the seeded receipts —
-- authored phrases like "your certified letter, 16 Nov 2009". Storing a pointer
-- would mean Chuck's sheet re-renders itself the day somebody re-words a
-- receipt, which would change what a printed record says she said. An answer is
-- a moment; what she pointed to at that moment is part of it.
--
-- JSONB and not TEXT[]: she may pick several, the count is small, nothing joins
-- on it, and every other "one act, several values" column on these tables
-- (`self_check`, `queue`, `skipped_today`) is already jsonb.
ALTER TABLE practice_answers ADD COLUMN IF NOT EXISTS points_to JSONB NULL;

COMMENT ON COLUMN practice_answers.points_to IS
    'The receipts she said she would point to, as the authored phrases she was '
    'shown. NULL = the control was never opened; [] = she opened it and picked '
    'nothing. Two different facts, kept different.';


-- ─── 4 · The read moves to v2 ────────────────────────────────────────────────
--
-- Roman's ruling of 2026-08-19 morning: on CROSS the right answer is the short
-- counter PLUS ONE ANCHOR, and a paragraph on cross is `That's redirect — save
-- it for Chuck.` rather than a volunteering fault; on DIRECT and REDIRECT there
-- is no length fault at all. That is a change to what the model is TOLD, so it
-- is a new prompt file and a pointer moved — v1 stays on disk, readable, and
-- one settings edit away from being the live prompt again.
--
-- MOUNT: `practice_read_prompt_v2.md` must be on the template mount before the
-- service restarts. The report carries the push block.
-- The alignment of this UPDATE is not decoration. `domain::wording::tests`'
-- `corrected_value_in` looks for the literal `SET value         = '` and
-- `key           = '` when it works out which value a key ENDS UP with after a
-- later migration corrects it. Written any other way, the correction is
-- invisible to the fixture tests and they go green while the store holds
-- something else — which is the exact drift those tests exist to catch.
UPDATE app_settings
SET value         = 'practice_read_prompt_v2.md',
    default_value = 'practice_read_prompt_v2.md',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_read_prompt_file';


-- ─── 5 · One arrow, not two ──────────────────────────────────────────────────
--
-- `<details><summary>` draws its own disclosure marker; the stored label carried
-- a second ▸ of its own, so the drawer rendered two. The marker is the arrow —
-- it turns when the drawer opens, which a character in a string cannot do — so
-- the LABEL is what loses its arrow.
UPDATE app_settings
SET value         = 'Show a stronger answer',
    default_value = 'Show a stronger answer',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_stronger_summary';


-- ─── 6 · The words ───────────────────────────────────────────────────────────
--
-- Everything Part A adds that a human reads. Not one of them is written in JSX
-- or in Rust: Marie reads these alone the night before she testifies, and Roman
-- edits their tone after watching her use the tool — which is a Settings edit
-- and a restart, never a build.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── The way into ONE question ────────────────────────────────────────────
    ('practice_row_practice_this_label', 'Practice this one ▸', 'text',
     'Practice this one ▸', NULL, NULL,
     'The control on a deck row that opens a one-question sitting. The question '
     'text on the row is the same link; this is the visible half, on the row''s '
     'control side, for a reader who does not know the text is clickable.',
     'frontend PracticeDeckList', NOW(), 'migration'),

    -- ── What happened to this question, on its row ───────────────────────────
    ('practice_row_answered_today_template', 'answered today · {mark}', 'text',
     'answered today · {mark}', NULL, NULL,
     'The status under a row she has answered TODAY. {mark} is the stored mark '
     'word — fine or repeat — so the row and Chuck''s sheet use one vocabulary.',
     'services::practice_page', NOW(), 'migration'),
    ('practice_row_skipped_today', 'skipped today', 'text', 'skipped today',
     NULL, NULL,
     'The status under a row whose newest attempt today was a mid-sitting skip. '
     'Distinct from Skip today on the start card, which writes no row at all.',
     'services::practice_page', NOW(), 'migration'),
    ('practice_row_earlier_template', 'last: {when} · {mark}', 'text',
     'last: {when} · {mark}', NULL, NULL,
     'The status under a row whose newest attempt was on an earlier day. Named '
     'as a date rather than "answered" because the tense is what she needs: a '
     'row answered last Tuesday is not a row she has done tonight.',
     'services::practice_page', NOW(), 'migration'),
    ('practice_row_attempt_suffix_template', '· attempt {n}', 'text',
     '· attempt {n}', NULL, NULL,
     'Appended to the row status when she has answered this question more than '
     'once. Withdrawn entirely at one attempt — "attempt 1" on every row is '
     'noise, and the number only means something once it is above one.',
     'services::practice_page', NOW(), 'migration'),

    -- ── The redirect ─────────────────────────────────────────────────────────
    ('practice_redirect_tag', 'redirect', 'text', 'redirect', NULL, NULL,
     'The small tag beside the Chuck pill on a redirect question. It wears '
     'Chuck''s pill because Chuck asks it; the tag says WHY he is asking it.',
     'frontend PracticeQuestion', NOW(), 'migration'),
    ('practice_redirect_stronger_line',
     'Tell it — this is Chuck''s time.', 'text',
     'Tell it — this is Chuck''s time.', NULL, NULL,
     'What the stronger-answer drawer shows on a REDIRECT that carries no '
     'stored example. Domain note: the honest "no receipt for this one — that''s '
     'a Chuck question" line is WRONG here. A redirect is not a question '
     'somebody failed to write an answer for; it is the one place in the drill '
     'where the right answer is length, and the drawer has to say so.',
     'frontend PracticeReveal', NOW(), 'migration'),

    -- ── "I'd point to…" ──────────────────────────────────────────────────────
    ('practice_points_to_label', 'I''d point to…', 'text', 'I''d point to…',
     NULL, NULL,
     'The control under the answer box that opens this scenario''s receipts. '
     'Optional, never required — a witness who names nothing has not failed a '
     'step.',
     'frontend PracticeQuestion', NOW(), 'migration'),
    ('practice_points_to_done_label', 'Done', 'text', 'Done', NULL, NULL,
     'Closes the receipt list. It is a fold, not a save: what she picked is sent '
     'with the answer, so there is nothing here for a separate write to lose.',
     'frontend PracticeQuestion', NOW(), 'migration'),
    ('practice_points_to_reveal_prefix', 'You''d point to:', 'text',
     'You''d point to:', NULL, NULL,
     'Introduces the receipts she picked, on the reveal. Addressed to her, in '
     'the second person, because everything else on that screen is.',
     'frontend PracticeReveal', NOW(), 'migration'),
    ('practice_points_to_sheet_prefix', 'would point to:', 'text',
     'would point to:', NULL, NULL,
     'The same list on Chuck''s sheet, in the third person — he is reading about '
     'her, not to her.',
     'frontend PracticeSheet', NOW(), 'migration'),

    -- ── The unfinished sitting ───────────────────────────────────────────────
    ('practice_unfinished_today_word', 'today', 'text', 'today', NULL, NULL,
     'Stands where the date goes in the unfinished-session line when the sitting '
     'was started today. "today 09:57" is what a person says; "Tue 19 Aug 09:57" '
     'about something twenty minutes old is not.',
     'services::practice_page', NOW(), 'migration');
