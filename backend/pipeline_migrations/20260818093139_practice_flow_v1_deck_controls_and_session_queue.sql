-- practice_flow_v1_deck_controls_and_session_queue: Practice flow v1
--
-- Created: 2026-08-18 09:31:39
-- Target: pipeline database (colossus_legal_v2)
--
-- Task CC_TASK_PRACTICE_FLOW_V1_v1, spec PRACTICE_MOCKUP_v3_2026-08-18.html,
-- on Roman's three rulings: flag-and-skip rather than in-place edit · the
-- questions list OPEN by default · build the mockup as drawn.
--
-- Marie can now read the deck before she starts, keep a question out of THIS
-- sitting, tell Roman and Chuck what is wrong with one, walk out of a sitting
-- and come back to it, and skip a question that does not fit mid-drill.
--
-- ## Additive only
--
-- Six nullable columns, one widened CHECK, and a block of `app_settings` rows.
-- No column is dropped or re-typed and no existing row changes value.

-- ─── 1 · The flag, on the QUESTION ────────────────────────────────────────────
--
-- ## Why the note lives on the question and not on the session
--
-- Roman's ruling: a flag OUTLIVES the sitting. It is Marie telling Roman and
-- Chuck that a question is wrong — which is a fact about the deck, not about
-- one evening. It stays until they change the question on the seed or in the
-- v1 deck editor, and it prints at the foot of every Chuck's sheet until then.
--
-- ## Domain note: why the question TEXT is not editable here
--
-- The deck text is what was proved verbatim against the mockup and what the
-- one-sentence read is prompted on. A text change mid-week, with no record of
-- who made it or why, is the pairing-editor problem again. So the flag carries
-- the complaint and the row id; the edit stays a human act on the seed.
ALTER TABLE practice_questions ADD COLUMN IF NOT EXISTS flag_note  TEXT        NULL;
ALTER TABLE practice_questions ADD COLUMN IF NOT EXISTS flagged_at TIMESTAMPTZ NULL;
ALTER TABLE practice_questions ADD COLUMN IF NOT EXISTS flagged_by TEXT        NULL;

COMMENT ON COLUMN practice_questions.flag_note IS
    'One line from Marie saying what is wrong with this question. NULL = not '
    'flagged. Outlives the sitting; cleared only by a human.';
COMMENT ON COLUMN practice_questions.flagged_at IS
    'When the flag was written. NULL exactly when flag_note is NULL — the two '
    'are set and cleared together, so a row cannot carry a time for a flag it '
    'does not have.';
COMMENT ON COLUMN practice_questions.flagged_by IS
    'The authenticated username that wrote the flag. Stored rather than derived '
    'so "who flagged this" survives the log window.';

-- ─── 2 · The sitting, on the SESSION ─────────────────────────────────────────
--
-- ## Why the queue is stored and not held in the browser
--
-- The sitting has its own address now
-- (`/practice/:scenarioId/session/:sessionId`), which means a reload, a browser
-- Back, and a closed laptop all have to land somewhere sensible. A queue that
-- lived only in React state would be gone at the first reload, and "resume"
-- would silently deal a DIFFERENT set of questions than the one she started —
-- the same five questions in another order is not the same sitting.
--
-- `queue` is the dealt question ids IN ORDER, including the re-queues that
-- "Ask me this one again later" appends. `count` is what she chose off the
-- pills, kept because the queue can grow past it and the choice is still worth
-- knowing. `skipped_today` is the ids she kept out on the start screen — for
-- the record, not for the queue, since they were never dealt.
--
-- ## Why resume is DERIVED rather than stored as a cursor
--
-- A stored `idx` is a second truth about how far she got, and it goes wrong the
-- first time a write lands and the cursor update does not. The answers ARE the
-- record: resume is the queue minus the questions that already have an answer
-- row in this session — `skipped` rows included, because a question she skipped
-- is one she has dealt with. Nothing to keep in step, nothing to repair.
ALTER TABLE practice_sessions ADD COLUMN IF NOT EXISTS count         INTEGER NULL;
ALTER TABLE practice_sessions ADD COLUMN IF NOT EXISTS queue         JSONB   NULL;
ALTER TABLE practice_sessions ADD COLUMN IF NOT EXISTS skipped_today JSONB   NULL;

COMMENT ON COLUMN practice_sessions.queue IS
    'The dealt question ids in order, re-queues appended. NULL on sessions '
    'started before flow v1 — those resume as an empty queue, which ends them.';

-- ─── 3 · The third mark ──────────────────────────────────────────────────────
--
-- ## Why `skipped` is a MARK and not a boolean beside it
--
-- "Skip this one — doesn't fit" writes a real answer row: the question was
-- dealt, it was put in front of her, and she said it does not belong. That is
-- an outcome, and the sheet has one column for outcomes. A separate boolean
-- would make `mark` lie ('fine' for a question she never answered) and would
-- put two sources of truth in the one place Chuck reads.
--
-- Every reader of `mark` treats it as neither fine nor repeat: it is not
-- counted in "N to repeat", and it gets its own clause in the headline.
ALTER TABLE practice_answers DROP CONSTRAINT IF EXISTS practice_answers_mark_check;
ALTER TABLE practice_answers ADD  CONSTRAINT practice_answers_mark_check
    CHECK (mark IN ('fine', 'repeat', 'skipped'));

-- ─── 4 · The words ───────────────────────────────────────────────────────────
--
-- Every string v3 adds. Not one of them is written in JSX or in Rust: Marie
-- reads these sentences alone the night before she testifies, and a wrong one
-- is a witness being coached by a typo (the same reason the v0 block exists).
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    -- ── S0 · the deck, listed ────────────────────────────────────────────────
    ('practice_deck_heading', 'The questions', 'text', 'The questions', NULL, NULL,
     'The bold label over the question list on the start screen.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_deck_count_template',
     '· {n} — {george} George''s side · {chuck} Chuck', 'text',
     '· {n} — {george} George''s side · {chuck} Chuck', NULL, NULL,
     'The count beside the deck heading, filled from the questions the filter is '
     'showing — not from the whole deck. Leads with the separator because the '
     'renderer supplies the joining space.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_deck_skipped_suffix_template', '· {k} skipped today', 'text',
     '· {k} skipped today', NULL, NULL,
     'Appended to the deck count when k > 0. A separate row so the common case '
     'renders no empty clause and no stray separator.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_deck_hide_link', 'Hide the questions', 'text', 'Hide the questions',
     NULL, NULL,
     'Folds the list for this page-load only — deliberately NOT persisted. Roman''s '
     'ruling: the list is open by default because he and Chuck are the readers this '
     'week. Whether Marie should see the deck before a drill at all is Chuck''s call; '
     'the fold is the compromise until he rules.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_deck_show_link', 'Show the questions', 'text', 'Show the questions',
     NULL, NULL, 'The same link once the list is folded.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_deck_instruction_template',
     'Read them first. If one doesn''t fit, {skip} keeps it out of this sitting; {flag} tells Roman and Chuck what''s wrong with it — it stays in the deck until they change it.',
     'text',
     'Read them first. If one doesn''t fit, {skip} keeps it out of this sitting; {flag} tells Roman and Chuck what''s wrong with it — it stays in the deck until they change it.',
     NULL, NULL,
     'The one sentence of instruction under the deck heading. {skip} and {flag} are '
     'filled with the two control labels below and rendered bold, so the sentence '
     'and the buttons cannot drift apart when one is renamed.',
     'frontend PracticeDeckList', NOW(), 'migration'),

    -- ── S0 · the two controls on a row ───────────────────────────────────────
    ('practice_skip_today_label', 'Skip today', 'text', 'Skip today', NULL, NULL,
     'Keeps a question out of THIS sitting. Domain note: session-scoped on '
     'purpose — it is not a judgment on the question, which is what Flag is for.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_skipped_today_label', 'Skipped today ✓', 'text', 'Skipped today ✓',
     NULL, NULL, 'The same control once the row is out.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_flag_label', 'Flag', 'text', 'Flag', NULL, NULL,
     'Opens the one-line note on the row.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_flag_edit_label', 'Edit flag', 'text', 'Edit flag', NULL, NULL,
     'The same control once a note is stored.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_flag_placeholder',
     'What''s wrong with it? One line — Roman and Chuck read this.', 'text',
     'What''s wrong with it? One line — Roman and Chuck read this.', NULL, NULL,
     'The placeholder in the flag input. Says who reads it, which is the whole '
     'reason the control is not an edit box.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_flag_save_label', 'Save flag', 'text', 'Save flag', NULL, NULL,
     'Writes the note to the question.', 'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_flag_cancel_label', 'Cancel', 'text', 'Cancel', NULL, NULL,
     'Closes the flag input without writing.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_flag_shown_template', 'flagged: “{note}”', 'text',
     'flagged: “{note}”', NULL, NULL,
     'The stored note as it renders under the source line. The ⚑ is drawn by the '
     'stylesheet, not stored here, because it is decoration and not language.',
     'frontend PracticeDeckList', NOW(), 'migration'),
    ('practice_nothing_left_label', 'Nothing left to ask', 'text',
     'Nothing left to ask', NULL, NULL,
     'What Start reads when every question in the filter is skipped today. A '
     'disabled button that still said "Start" would be a screen refusing without '
     'saying why.',
     'frontend PracticeStart', NOW(), 'migration'),

    -- ── S0 · the unfinished sitting ──────────────────────────────────────────
    ('practice_unfinished_label', 'Unfinished session', 'text', 'Unfinished session',
     NULL, NULL, 'The bold opening of the blue resume box.',
     'frontend PracticeStart', NOW(), 'migration'),
    ('practice_unfinished_detail_template',
     '· {when} · {who} · {answered} of {total} answered.', 'text',
     '· {when} · {who} · {answered} of {total} answered.', NULL, NULL,
     'The rest of the resume line. {total} is the stored queue length, so the '
     'number does not move when she resumes.',
     'frontend PracticeStart', NOW(), 'migration'),
    ('practice_resume_label', 'Resume', 'text', 'Resume', NULL, NULL,
     'Re-enters the open sitting at the next undealt question.',
     'frontend PracticeStart', NOW(), 'migration'),
    ('practice_start_over_label', 'Start over', 'text', 'Start over', NULL, NULL,
     'Closes the open sitting and returns a clean start card. Domain note: this '
     'never deletes an answer — the closed session keeps its own Chuck''s sheet.',
     'frontend PracticeStart', NOW(), 'migration'),
    ('practice_start_over_hint',
     'Starting over keeps what you answered on Chuck''s sheet.', 'text',
     'Starting over keeps what you answered on Chuck''s sheet.', NULL, NULL,
     'The sub-line beside Start over, saying the destructive-sounding control is '
     'not destructive.',
     'frontend PracticeStart', NOW(), 'migration'),

    -- ── S1 / S2 · the top bar ────────────────────────────────────────────────
    ('practice_back_label', '◂ Back to start', 'text', '◂ Back to start', NULL, NULL,
     'The marked exit from a sitting (NN/g heuristic 3 — user control and '
     'freedom; no dead ends).',
     'frontend PracticeTopBar', NOW(), 'migration'),
    ('practice_back_hint_question', 'your answers so far are kept', 'text',
     'your answers so far are kept', NULL, NULL,
     'The grey hint beside Back on the QUESTION screen. Says the exit is safe.',
     'frontend PracticeTopBar', NOW(), 'migration'),
    ('practice_back_hint_reveal', 'this answer is already on Chuck''s sheet', 'text',
     'this answer is already on Chuck''s sheet', NULL, NULL,
     'The grey hint beside Back on the REVEAL screen. Different sentence because '
     'the fact is different: the row was written when she pressed Answer.',
     'frontend PracticeTopBar', NOW(), 'migration'),
    ('practice_skip_question_label', 'Skip this one — doesn''t fit', 'text',
     'Skip this one — doesn''t fit', NULL, NULL,
     'Mid-sitting skip. Writes a row marked skipped; no read is called and no '
     'tokens are spent.',
     'frontend PracticeTopBar', NOW(), 'migration'),
    ('practice_end_session_label', 'End session ▸', 'text', 'End session ▸',
     NULL, NULL, 'Closes the sitting and shows Chuck''s sheet with what was answered.',
     'frontend PracticeTopBar', NOW(), 'migration'),
    ('practice_skipped_answer_text', '(skipped — doesn''t fit)', 'text',
     '(skipped — doesn''t fit)', NULL, NULL,
     'What the answer row stores when she skips mid-sitting. Domain note: a '
     'stored phrase and never an empty string, so a skipped question and an '
     'unanswered one stay different rows — the same rule the "(nothing typed)" '
     'phrase follows.',
     'services::practice_answers', NOW(), 'migration'),

    -- ── S3 · Chuck's sheet ───────────────────────────────────────────────────
    ('practice_mark_skipped', 'skipped', 'text', 'skipped', NULL, NULL,
     'The third mark, in the muted style. Neither fine nor repeat.',
     'frontend PracticeSheet', NOW(), 'migration'),
    ('practice_sheet_skipped_clause_template', '{s} skipped.', 'text',
     '{s} skipped.', NULL, NULL,
     'Appended to the sheet headline when s > 0.',
     'services::practice_sheet', NOW(), 'migration'),
    ('practice_sheet_ended_early_clause', 'Ended early.', 'text', 'Ended early.',
     NULL, NULL,
     'Appended to the sheet headline when the sitting was ended before its queue '
     'was exhausted. Domain note: this is a fact about the sitting, not a fault — '
     'the sheet says what happened and grades nothing.',
     'services::practice_sheet', NOW(), 'migration'),
    ('practice_flag_summary_heading', 'Flagged before the session', 'text',
     'Flagged before the session', NULL, NULL,
     'The bold heading of the flag list at the foot of Chuck''s sheet.',
     'frontend PracticeSheet', NOW(), 'migration'),
    ('practice_flag_summary_hint',
     '— questions Marie said don''t fit; Roman/Chuck decide what to do with them:',
     'text',
     '— questions Marie said don''t fit; Roman/Chuck decide what to do with them:',
     NULL, NULL, 'The sentence after the flag-list heading.',
     'frontend PracticeSheet', NOW(), 'migration'),
    ('practice_flag_summary_item_template', '{id} — “{question}” → {note}', 'text',
     '{id} — “{question}” → {note}', NULL, NULL,
     'One flagged question, as it prints. {id} is a POSITION on its own side '
     '(G2 = George''s second question), not the row''s uuid — a uuid is no use on '
     'printed paper. {question} is the verbatim text, which is what Roman greps '
     'the seed for.',
     'frontend PracticeSheet', NOW(), 'migration');
