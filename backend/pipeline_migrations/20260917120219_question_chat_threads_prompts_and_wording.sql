-- question_chat_threads_prompts_and_wording: Discuss with AI — a saved conversation on any question
--
-- Created: 2026-09-17 12:02:19
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_QUESTION_CHAT_v1 + ADDENDUM_v1, ruled by CC_GO_QUESTION_CHAT_v1.
-- THE ONE MIGRATION: the thread table, the dock's settings rows, every new
-- wording row, and the read prompt's move to v4.
-- =============================================================================
--
-- ## One thread per question, as rows
--
-- A question's discussion is simply its rows in `practice_discussions`, ordered by
-- time. Everyone (Marie, Chuck, Roman) reads and writes the same thread. A model
-- turn is stamped with the model that wrote it and what it cost (tokens, ms), so
-- a mid-thread model switch stays legible and spend stays observable.
--
-- ## What is never stored here
--
-- Marie's unsaved DRAFT answer. It rides a single message to the model and is
-- discarded (GO ruling 11): a draft she has not committed to is not a record.

-- ─── 1 · The thread ──────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS practice_discussions (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    question_id    UUID        NOT NULL
                   REFERENCES practice_questions(id) ON DELETE CASCADE,
    -- The signed-in username (attribution's stable id). For a model turn: the
    -- user whose message it answers.
    author_user_id TEXT        NOT NULL CHECK (btrim(author_user_id) <> ''),
    -- The name the dock prints: the person, or the model's display name.
    author_name    TEXT        NOT NULL CHECK (btrim(author_name) <> ''),
    role           TEXT        NOT NULL CHECK (role IN ('user', 'model')),
    -- Which model wrote a model turn; NULL on every user turn.
    model_id       TEXT,
    text           TEXT        NOT NULL CHECK (btrim(text) <> ''),
    -- What a model turn cost. NULL on user turns, and when a provider reports none.
    input_tokens   INTEGER,
    output_tokens  INTEGER,
    ms             INTEGER,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT practice_discussions_model_iff_model_turn
        CHECK ((role = 'model') = (model_id IS NOT NULL))
);

CREATE INDEX IF NOT EXISTS idx_practice_discussions_question
    ON practice_discussions (question_id, created_at);

COMMENT ON TABLE practice_discussions IS
    'Discuss with AI: one saved conversation per practice question, visible to the '
    'whole team. Model turns carry model_id and cost. Drafts are never stored.';

-- ─── 2 · Settings and wording ────────────────────────────────────────────────
-- Idempotent: ON CONFLICT (key) DO NOTHING, so a re-run changes nothing and a row
-- somebody edited is never clobbered.
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('practice_discuss_default_model', 'claude-opus-5', 'text', 'claude-opus-5', NULL, NULL,
     'The model the discussion dock starts on. Anyone can switch mid-thread; every model reply is stamped with the model that wrote it. Must be an active Anthropic llm_models row.',
     NULL, now(), 'migration'),

    ('practice_discuss_max_turns', '40', 'count', '40', 1, 500,
     'The most model replies one question''s discussion may hold, across everyone. A guard against runaway spend; reaching it refuses the message with a named 409.',
     NULL, now(), 'migration'),

    ('practice_discuss_prompt_file', 'practice_discuss_prompt_v1.md', 'text', 'practice_discuss_prompt_v1.md', NULL, NULL,
     'The discussion''s system prompt, in the template directory beside the read''s prompts.',
     NULL, now(), 'migration'),

    ('practice_discuss_max_tokens', '4096', 'count', '4096', 64, 32000,
     'The output cap of one discussion reply. Headroom for a model that thinks before it answers; constrain REFUSES a cap above the model''s own ceiling rather than clamping it.',
     NULL, now(), 'migration'),

    ('practice_discuss_button_label', 'Discuss with AI', 'text', 'Discuss with AI', NULL, NULL,
     'The button on a question page that opens the discussion dock. Static words — never a model name (ruled amendment 1).',
     NULL, now(), 'migration'),

    ('practice_discuss_title_template', 'Discuss · Question {n}', 'text', 'Discuss · Question {n}', NULL, NULL,
     'The dock''s title. {n} is the question''s position in the scenario''s deck.',
     NULL, now(), 'migration'),

    ('practice_discuss_subtitle_template', '{code}', 'text', '{code}', NULL, NULL,
     'The line under the dock''s title when her saved answer (or none) is what the model sees. {code} is the scenario code.',
     NULL, now(), 'migration'),

    ('practice_discuss_subtitle_draft_template', '{code} · your draft answer is visible to the model', 'text', '{code} · your draft answer is visible to the model', NULL, NULL,
     'The line under the title when her unsaved draft goes with the message instead of her saved answer.',
     NULL, now(), 'migration'),

    ('practice_discuss_context_line', 'The model sees: the question · its tactic · your current answer · the talking points and receipts · the sworn pair · the latest analysis · the scenario''s attack · the team''s notes · for redirects, the question it repairs', 'text', 'The model sees: the question · its tactic · your current answer · the talking points and receipts · the sworn pair · the latest analysis · the scenario''s attack · the team''s notes · for redirects, the question it repairs', NULL, NULL,
     'The small line at the top of the thread saying what the model is given with every message.',
     NULL, now(), 'migration'),

    ('practice_discuss_context_line_draft', 'The model sees: the question · its tactic · your unsaved draft · the talking points and receipts · the sworn pair · the latest analysis · the scenario''s attack · the team''s notes · for redirects, the question it repairs', 'text', 'The model sees: the question · its tactic · your unsaved draft · the talking points and receipts · the sworn pair · the latest analysis · the scenario''s attack · the team''s notes · for redirects, the question it repairs', NULL, NULL,
     'The same line while her unsaved draft is what goes with the message.',
     NULL, now(), 'migration'),

    ('practice_discuss_input_placeholder', 'Ask about this question or your answer…', 'text', 'Ask about this question or your answer…', NULL, NULL,
     'The message box''s placeholder.',
     NULL, now(), 'migration'),

    ('practice_discuss_send_label', 'Send', 'text', 'Send', NULL, NULL,
     'The send button.',
     NULL, now(), 'migration'),

    ('practice_discuss_close_label', 'Close the discussion', 'text', 'Close the discussion', NULL, NULL,
     'The close button''s accessible name (it shows as ✕).',
     NULL, now(), 'migration'),

    ('practice_discuss_model_label', 'Model', 'text', 'Model', NULL, NULL,
     'The model picker''s accessible name.',
     NULL, now(), 'migration'),

    ('practice_discuss_footer_template', 'Thread is saved on this question · visible to Marie, Chuck and Roman · {cost} on {model}', 'text', 'Thread is saved on this question · visible to Marie, Chuck and Roman · {cost} on {model}', NULL, NULL,
     'The dock''s footer. {cost} is the cost word for the chosen model''s billing class; {model} its display name.',
     NULL, now(), 'migration'),

    ('practice_discuss_cost_billed', '$ per turn', 'text', '$ per turn', NULL, NULL,
     'The {cost} word for a billed (API) model.',
     NULL, now(), 'migration'),

    ('practice_discuss_cost_local', '$0 per turn', 'text', '$0 per turn', NULL, NULL,
     'The {cost} word for a local model.',
     NULL, now(), 'migration'),

    ('practice_discuss_cost_template', '{input} in · {output} out · {seconds} s', 'text', '{input} in · {output} out · {seconds} s', NULL, NULL,
     'The small line under a model reply: what that one call cost — input and output tokens, and seconds. Spend stays observable without SQL.',
     NULL, now(), 'migration'),

    ('practice_discuss_empty', 'No discussion yet — ask the first question.', 'text', 'No discussion yet — ask the first question.', NULL, NULL,
     'Shown in the thread before anyone has written.',
     NULL, now(), 'migration'),

    ('practice_discuss_sending_template', 'Waiting for {model}…', 'text', 'Waiting for {model}…', NULL, NULL,
     'Shown while a reply is on its way.',
     NULL, now(), 'migration'),

    ('practice_discuss_send_failed', 'No reply came back. Your message is saved above — send again to retry.', 'text', 'No reply came back. Your message is saved above — send again to retry.', NULL, NULL,
     'Shown when the model call fails after the message was stored.',
     NULL, now(), 'migration'),

    ('practice_discuss_load_failed', 'The discussion could not be loaded.', 'text', 'The discussion could not be loaded.', NULL, NULL,
     'Shown when the thread cannot be read.',
     NULL, now(), 'migration'),

    ('practice_discuss_cap_reached_template', 'This question has reached its limit of {max} model replies.', 'text', 'This question has reached its limit of {max} model replies.', NULL, NULL,
     'Shown when the per-question reply cap (practice_discuss_max_turns) is reached; the message is not sent.',
     NULL, now(), 'migration'),

    ('practice_discuss_cap_reached_one', 'This question has reached its limit of {max} model reply.', 'text', 'This question has reached its limit of {max} model reply.', NULL, NULL,
     'The singular of the line above, when the cap is 1.',
     NULL, now(), 'migration'),

    ('war_room_summary_unanswered_changed_clause', '{n} new or changed', 'text', '{n} new or changed', NULL, NULL,
     'Appended to Marie''s context line on the summary card: the sum of every card''s new-or-changed pill. Not shown at zero.',
     NULL, now(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ─── 3 · The read's prompt moves to v4 (v3 stays on disk) ────────────────────
--
-- v4 tells the model about the three sections the payload gained, and that a
-- standing note outranks it. Pointing this row back at v3 is the whole rollback of
-- the prompt half (the payload's new sections then go unexplained, not unsent).
UPDATE app_settings
SET value         = 'practice_read_prompt_v4.md',
    default_value = 'practice_read_prompt_v4.md',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_read_prompt_file';

-- ─── 4 · The END-state assertion (CLAUDE.md rule 25a) ────────────────────────
DO $$
DECLARE
    present   INTEGER;
    prompt    TEXT;
    table_ok  INTEGER;
BEGIN
    SELECT count(*) INTO present
      FROM app_settings
     WHERE key IN (
        'practice_discuss_default_model',
        'practice_discuss_max_turns',
        'practice_discuss_prompt_file',
        'practice_discuss_max_tokens',
        'practice_discuss_button_label',
        'practice_discuss_title_template',
        'practice_discuss_subtitle_template',
        'practice_discuss_subtitle_draft_template',
        'practice_discuss_context_line',
        'practice_discuss_context_line_draft',
        'practice_discuss_input_placeholder',
        'practice_discuss_send_label',
        'practice_discuss_close_label',
        'practice_discuss_model_label',
        'practice_discuss_footer_template',
        'practice_discuss_cost_billed',
        'practice_discuss_cost_local',
        'practice_discuss_cost_template',
        'practice_discuss_empty',
        'practice_discuss_sending_template',
        'practice_discuss_send_failed',
        'practice_discuss_load_failed',
        'practice_discuss_cap_reached_template',
        'practice_discuss_cap_reached_one',
        'war_room_summary_unanswered_changed_clause');
    IF present <> 25 THEN
        RAISE EXCEPTION
            'question chat: expected 25 new settings rows, found %. The backend '
            'declares every one at boot and refuses to start without it.', present;
    END IF;

    SELECT value INTO prompt FROM app_settings WHERE key = 'practice_read_prompt_file';
    IF prompt IS DISTINCT FROM 'practice_read_prompt_v4.md' THEN
        RAISE EXCEPTION 'question chat: practice_read_prompt_file is %, expected practice_read_prompt_v4.md', prompt;
    END IF;

    SELECT count(*) INTO table_ok FROM information_schema.tables WHERE table_name = 'practice_discussions';
    IF table_ok <> 1 THEN
        RAISE EXCEPTION 'question chat: practice_discussions was not created';
    END IF;
END $$;
