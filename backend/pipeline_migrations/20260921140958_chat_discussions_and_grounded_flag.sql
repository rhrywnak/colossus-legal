-- chat_discussions_and_grounded_flag: the chat engine's memory, its model flag, its settings
--
-- Created: 2026-09-21 14:09:58
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_CHAT_ENGINE_v1 (+ GO, + ADDENDUM_1) — THE ONE MIGRATION this task writes.
-- Additive only: three new tables, one new column, new settings rows. The only
-- existing rows touched are the one-time `grounded` backfill below, asserted.
-- `practice_discussions` (the old dock's shared thread) is NOT touched — its rows
-- are READ by the new panel as "Earlier team discussion" (ADDENDUM_1), never moved.
-- =============================================================================
--
-- ## Reproduction on a fresh database (Law 20)
--
-- Applied by the pipeline Migrator at boot; every statement is idempotent
-- (IF NOT EXISTS / ON CONFLICT DO NOTHING), so a re-run changes nothing and a row
-- somebody has since edited is never clobbered. The 14-digit prefix sorts after
-- every existing migration. (The chain as a whole does not yet apply from zero —
-- the pre-existing eight-digit prefixes sort first; that defect is tracked
-- separately and is not this file's.)

-- ─── 1 · The grounded flag ──────────────────────────────────────────────────
--
-- `grounded = true` means a call to this model goes through the Anthropic Messages
-- API, with Citations (every quoted passage platform-checked against the supplied
-- document) and server-side compaction. The chat refuses an ungrounded model at
-- save and at boot (ruling D3): without platform-enforced quotes, the "no citation,
-- no claim" rule the chat's prompt lives under cannot hold.
--
-- DEFAULT false — the cautious side, as `billing_class` chose 'billed': a model
-- nobody has classified is one we cannot promise quotes from. The backfill is
-- seeded from `provider` as the best evidence available today (measured on DEV
-- 2026-09-21: 6 anthropic rows, 3 vllm rows); after this migration the column is
-- authored, not inferred.
ALTER TABLE llm_models
    ADD COLUMN IF NOT EXISTS grounded BOOLEAN NOT NULL DEFAULT false;

COMMENT ON COLUMN llm_models.grounded IS
    'Calls to this model go through the Anthropic Messages API with Citations and '
    'server-side compaction. false = the chat refuses it (no platform-enforced quotes). '
    'Backfilled once from provider=anthropic (2026-09-21); authored thereafter.';

UPDATE llm_models SET grounded = true WHERE provider = 'anthropic' AND grounded = false;

-- ─── 2 · Threads: one per (anchor, user), readable by all three users ─────────
--
-- `anchor_id` is TEXT with no foreign key (ruling D5): a question id is a UUID, a
-- document id is TEXT, and one column serves every anchor type. The price is no
-- cascade — a thread whose anchor is deleted is an orphan, which the backend
-- COUNTS AT BOOT and logs by name rather than letting it vanish silently.
CREATE TABLE IF NOT EXISTS discussions (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    -- 'question' only, until the scenario and document anchors ship (tasks 2, 3).
    anchor_type  TEXT        NOT NULL CHECK (anchor_type IN ('question')),
    anchor_id    TEXT        NOT NULL CHECK (btrim(anchor_id) <> ''),
    -- The owner's signed-in username. Only the owner writes; everyone reads.
    username     TEXT        NOT NULL CHECK (btrim(username) <> ''),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT discussions_one_per_user UNIQUE (anchor_type, anchor_id, username)
);

COMMENT ON TABLE discussions IS
    'Discuss with AI v2: one thread per (anchor, user). Every thread is readable by '
    'everyone on the case and is given to the model when a sibling thread is answered.';

-- ─── 3 · Messages: append-only, stored VERBATIM ─────────────────────────────
--
-- `content` is the RAW API content-block array. It must be: the next turn replays
-- the history, and thinking blocks carry signatures the provider checks, tool_use
-- ids must match their tool_result, and a compaction block replaces everything
-- before it. A row is never edited after insert.
CREATE TABLE IF NOT EXISTS discussion_messages (
    id                    UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    discussion_id         UUID        NOT NULL REFERENCES discussions(id) ON DELETE CASCADE,
    seq                   INTEGER     NOT NULL CHECK (seq >= 1),
    role                  TEXT        NOT NULL CHECK (role IN ('user', 'assistant')),
    content               JSONB       NOT NULL,
    -- What the thread shows. NULL on a tool-result turn (nothing to show).
    rendered_text         TEXT,
    -- Verified citation cards for an assistant turn; NULL = none on this turn.
    citations             JSONB,
    -- The model that wrote an assistant turn; NULL on every user turn.
    model                 TEXT,
    stop_reason           TEXT,
    input_tokens          INTEGER,
    output_tokens         INTEGER,
    cache_creation_tokens INTEGER,
    cache_read_tokens     INTEGER,
    ms                    INTEGER,
    -- A named failure marker (refused / truncated / stalled / failed). NULL = succeeded.
    -- A failed assistant turn is kept for the record and NOT replayed to the model.
    failure               TEXT,
    -- The failure's full sentence (which bound, which stop reason, which transport
    -- error), so an operator reads WHY from the row itself, not only from the logs.
    failure_detail        TEXT,
    -- The resolved configuration the turn ran under (model, output cap, effort,
    -- cache TTL, tool rounds, compaction trigger, headroom, prompt and narrative
    -- files) — on the LAST assistant row of each turn. Settings change; this
    -- records what applied, the counterpart of extraction_runs.processing_config.
    run_config            JSONB,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT discussion_messages_seq_unique UNIQUE (discussion_id, seq),
    CONSTRAINT discussion_messages_model_iff_assistant
        CHECK ((role = 'assistant') = (model IS NOT NULL))
);

COMMENT ON TABLE discussion_messages IS
    'Append-only turns of a discussions thread; content is the verbatim API block array.';

-- ─── 4 · Who has seen how far (the unread badge, ruling D4) ─────────────────
CREATE TABLE IF NOT EXISTS discussion_reads (
    discussion_id UUID    NOT NULL REFERENCES discussions(id) ON DELETE CASCADE,
    username      TEXT    NOT NULL CHECK (btrim(username) <> ''),
    last_seen_seq INTEGER NOT NULL CHECK (last_seen_seq >= 0),
    PRIMARY KEY (discussion_id, username)
);

-- ─── 5 · The chat's parameters ──────────────────────────────────────────────
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('question_chat_model', 'claude-opus-5', 'text', 'claude-opus-5', NULL, NULL,
     'The model the question chat answers with. Must be an ACTIVE and GROUNDED llm_models row: a save naming any other is refused, and the backend refuses to boot if the row stops qualifying.',
     NULL, now(), 'migration'),
    ('question_chat_prompt_file', 'question_chat_prompt_v1.md', 'text', 'question_chat_prompt_v1.md', NULL, NULL,
     'The question chat''s system prompt, in the template directory beside the read''s prompts. The backend refuses to boot if the file is not deployed.',
     NULL, now(), 'migration'),
    ('chat_case_narrative_file', 'case_narrative_v1.md', 'text', 'case_narrative_v1.md', NULL, NULL,
     'The case narrative every chat is given (parties, timeline, claims, who is who), in the template directory. The backend refuses to boot if the file is not deployed.',
     NULL, now(), 'migration'),
    ('question_chat_max_tokens', '16000', 'count', '16000', 256, 64000,
     'The output cap of one chat call, thinking included. Headroom for a model that thinks before it answers; the API refuses a cap above the model''s own ceiling rather than clamping it.',
     NULL, now(), 'migration'),
    ('question_chat_effort', 'high', 'text', 'high', NULL, NULL,
     'How much the model may think before one chat reply: low, medium, high, xhigh, max, or absent (send no effort key).',
     NULL, now(), 'migration'),
    ('question_chat_max_tool_rounds', '6', 'count', '6', 1, 20,
     'The most model calls one chat reply may take while it fetches the record with tools. Past it the reply is abandoned with a named error rather than looping.',
     NULL, now(), 'migration'),
    ('question_chat_context_headroom_tokens', '80000', 'count', '80000', 0, 500000,
     'Room kept free inside the model''s context window for the conversation and the reply. A package that leaves less than this is refused by name before any call is made.',
     NULL, now(), 'migration'),
    ('question_chat_cache_ttl', '1h', 'text', '1h', NULL, NULL,
     'How long the cached document package lives: 5m or 1h. A practice sitting has gaps longer than five minutes, so 1h (GO ruling Q3).',
     NULL, now(), 'migration'),
    ('question_chat_compaction_trigger_tokens', '700000', 'count', '700000', 0, 2000000,
     'Input size at which the provider compacts the conversation server-side. 0 turns compaction off; otherwise at least 50000. Kept well above the document package, because compaction summarizes documents too and a summarized document can no longer be quoted.',
     NULL, now(), 'migration'),
    ('question_chat_max_turns', '200', 'count', '200', 1, 2000,
     'The most model replies one person''s thread on one question may hold. A guard against runaway spend; reaching it refuses the message with a named 409.',
     NULL, now(), 'migration'),
    ('question_chat_client_idle_timeout_secs', '150', 'count', '150', 30, 900,
     'How long the browser waits with no word from the server before it gives up on a reply. Longer than the server''s own stall detector, so the server always names the failure first.',
     NULL, now(), 'migration'),
    -- How the model is told who asks, by the question's KIND (cross / direct /
    -- redirect — the practice_questions.kind vocabulary). Model-facing prose, not
    -- screen wording: which is why they are parameters and not practice_chat_* rows.
    ('question_chat_asker_cross', 'opposing counsel, on cross-examination', 'text', 'opposing counsel, on cross-examination', NULL, NULL,
     'How the question chat tells the model who asks a cross-examination question.',
     NULL, now(), 'migration'),
    ('question_chat_asker_direct', 'the witness''s own lawyer, on direct examination', 'text', 'the witness''s own lawyer, on direct examination', NULL, NULL,
     'How the question chat tells the model who asks a direct-examination question.',
     NULL, now(), 'migration'),
    ('question_chat_asker_redirect', 'the witness''s own lawyer, on redirect', 'text', 'the witness''s own lawyer, on redirect', NULL, NULL,
     'How the question chat tells the model who asks a redirect question.',
     NULL, now(), 'migration'),
    ('question_chat_error_preview_chars', '500', 'count', '500', 50, 20000,
     'How much of a provider''s error body, or of a malformed stream event, a chat failure quotes in its stored detail and log line. Read at boot (the engine is built once).',
     NULL, now(), 'migration'),
    ('question_chat_ai_display_name', 'The AI', 'text', 'The AI', NULL, NULL,
     'The name the model''s messages carry — on screen, and in the other threads the model is shown.',
     NULL, now(), 'migration'),
    ('question_chat_chars_per_token', '3', 'count', '3', 1, 10,
     'Characters per token the context-size guard assumes. Deliberately low (English prose runs nearer 4) so the guard refuses early rather than late; the API still refuses an oversized request by name.',
     NULL, now(), 'migration'),
    -- Case data: the witness's name as threads show it and as the AI is told it.
    -- The login (practice_witness_username) is not a name — on DEV its Authentik
    -- display name is the login itself — so the name is its own row. One value,
    -- edited on its own: any name is valid for any login, so there is no pair
    -- invariant to deadlock on (Law 23(b)).
    ('chat_witness_display_name', 'Marie', 'text', 'Marie', NULL, NULL,
     'The witness''s name as the discussion panel shows it and the AI is told it. The witness is the login in practice_witness_username.',
     NULL, now(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ─── 6 · The panel's words (mockup DISCUSS_CHAT_MOCKUP_v2_2026-09-21) ─────────
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('practice_chat_open_label', 'Discuss this answer', 'text', 'Discuss this answer', NULL, NULL,
     'The button beside Try again that opens the discussion panel.', NULL, now(), 'migration'),
    ('practice_chat_open_hint', 'The read is the quick verdict. The discussion is where the strategy lives — ask why, argue back, or work out a better shape.', 'text', 'The read is the quick verdict. The discussion is where the strategy lives — ask why, argue back, or work out a better shape.', NULL, NULL,
     'The line under the Discuss button.', NULL, now(), 'migration'),
    ('practice_chat_your_thread', 'Your thread', 'text', 'Your thread', NULL, NULL,
     'The switcher button while your own thread is open.', NULL, now(), 'migration'),
    ('practice_chat_thread_of_template', '{name}''s thread', 'text', '{name}''s thread', NULL, NULL,
     'The switcher button while someone else''s thread is open. {name} is their name.', NULL, now(), 'migration'),
    ('practice_chat_switch_label', 'Switch thread', 'text', 'Switch thread', NULL, NULL,
     'The switcher button''s accessible name.', NULL, now(), 'migration'),
    ('practice_chat_visibility_template', '{others} can read this thread', 'text', '{others} can read this thread', NULL, NULL,
     'The header line on your own thread. {others} is everyone else on the case, joined in words.', NULL, now(), 'migration'),
    ('practice_chat_resumed_template', 'resumed from {date}', 'text', 'resumed from {date}', NULL, NULL,
     'Appended to the header line when the thread began on an earlier day. {date} is that day.', NULL, now(), 'migration'),
    ('practice_chat_readonly_line', 'Read-only — you can read this thread, and the AI reads it too, but only its owner writes in it.', 'text', 'Read-only — you can read this thread, and the AI reads it too, but only its owner writes in it.', NULL, NULL,
     'The header line and the composer''s replacement on someone else''s thread.', NULL, now(), 'migration'),
    ('practice_chat_earlier_readonly_line', 'Read-only — the team''s discussion of this question from before each person had a thread. The AI reads it too.', 'text', 'Read-only — the team''s discussion of this question from before each person had a thread. The AI reads it too.', NULL, NULL,
     'The header line and the composer''s replacement on the Earlier team discussion.', NULL, now(), 'migration'),
    ('practice_chat_and', 'and', 'text', 'and', NULL, NULL,
     'The word that joins the last two names in {others}.', NULL, now(), 'migration'),
    ('practice_chat_grounded_chip_template', '{model} · grounded', 'text', '{model} · grounded', NULL, NULL,
     'The model chip. {model} is the model''s short name. Grounded: every quoted passage is checked against the stored document.', NULL, now(), 'migration'),
    ('practice_chat_expand_label', 'Expand discussion to full screen', 'text', 'Expand discussion to full screen', NULL, NULL,
     'The expand button''s accessible name.', NULL, now(), 'migration'),
    ('practice_chat_collapse_label', 'Collapse to side panel', 'text', 'Collapse to side panel', NULL, NULL,
     'The collapse button''s accessible name.', NULL, now(), 'migration'),
    ('practice_chat_back_label', 'Back', 'text', 'Back', NULL, NULL,
     'The Back control on the full-screen strip.', NULL, now(), 'migration'),
    ('practice_chat_back_aria', 'Back to question', 'text', 'Back to question', NULL, NULL,
     'The Back control''s accessible name.', NULL, now(), 'migration'),
    ('practice_chat_resize_label', 'Drag to resize', 'text', 'Drag to resize', NULL, NULL,
     'The divider''s tooltip.', NULL, now(), 'migration'),
    ('practice_chat_strip_answer_template', 'your answer: “{answer}”', 'text', 'your answer: “{answer}”', NULL, NULL,
     'On the full-screen strip, after the question. {answer} is her saved answer.', NULL, now(), 'migration'),
    ('practice_chat_input_placeholder', 'Ask about this question, the documents behind it, or the strategy…', 'text', 'Ask about this question, the documents behind it, or the strategy…', NULL, NULL,
     'The message box''s placeholder.', NULL, now(), 'migration'),
    ('practice_chat_message_label', 'Message', 'text', 'Message', NULL, NULL,
     'The message box''s accessible name.', NULL, now(), 'migration'),
    ('practice_chat_send_label', 'Send', 'text', 'Send', NULL, NULL,
     'The send button.', NULL, now(), 'migration'),
    ('practice_chat_switcher_own_template', 'Your thread · {count} messages', 'text', 'Your thread · {count} messages', NULL, NULL,
     'Your row in the switcher. {count} is its number of messages.', NULL, now(), 'migration'),
    ('practice_chat_switcher_own_one', 'Your thread · 1 message', 'text', 'Your thread · 1 message', NULL, NULL,
     'Your row in the switcher at exactly one message.', NULL, now(), 'migration'),
    ('practice_chat_switcher_own_empty', 'Your thread · no messages yet', 'text', 'Your thread · no messages yet', NULL, NULL,
     'Your row in the switcher before you have written.', NULL, now(), 'migration'),
    ('practice_chat_switcher_other_template', '{name} · {count} messages', 'text', '{name} · {count} messages', NULL, NULL,
     'Someone else''s row in the switcher.', NULL, now(), 'migration'),
    ('practice_chat_switcher_other_one', '{name} · 1 message', 'text', '{name} · 1 message', NULL, NULL,
     'Someone else''s row at exactly one message.', NULL, now(), 'migration'),
    ('practice_chat_switcher_other_empty', '{name} · no messages yet', 'text', '{name} · no messages yet', NULL, NULL,
     'Someone else''s row before they have written.', NULL, now(), 'migration'),
    ('practice_chat_unread_template', '{count} new', 'text', '{count} new', NULL, NULL,
     'The unread badge on a switcher row. {count} is how many messages you have not seen.', NULL, now(), 'migration'),
    ('practice_chat_readonly_mark', 'Read-only', 'text', 'Read-only', NULL, NULL,
     'The small mark under a switcher row you cannot write in.', NULL, now(), 'migration'),
    ('practice_chat_switcher_footer', 'Everyone on the case can read all threads. The AI reads them too, so insight crosses between you.', 'text', 'Everyone on the case can read all threads. The AI reads them too, so insight crosses between you.', NULL, NULL,
     'The switcher''s footer — the visibility rule, said plainly.', NULL, now(), 'migration'),
    ('practice_chat_earlier_label', 'Earlier team discussion', 'text', 'Earlier team discussion', NULL, NULL,
     'The switcher row for the old shared thread (ADDENDUM_1).', NULL, now(), 'migration'),
    ('practice_chat_earlier_meta_template', '{count} messages · last {date}', 'text', '{count} messages · last {date}', NULL, NULL,
     'Under the Earlier team discussion row. {date} is its last message''s day.', NULL, now(), 'migration'),
    ('practice_chat_earlier_meta_one', '1 message · {date}', 'text', '1 message · {date}', NULL, NULL,
     'The same at exactly one message.', NULL, now(), 'migration'),
    ('practice_chat_empty', 'No messages yet — ask the first question.', 'text', 'No messages yet — ask the first question.', NULL, NULL,
     'Shown in a thread before anyone has written.', NULL, now(), 'migration'),
    ('practice_chat_waiting', 'Thinking…', 'text', 'Thinking…', NULL, NULL,
     'Shown while a reply is on its way and nothing has arrived yet.', NULL, now(), 'migration'),
    ('practice_chat_tool_line', 'Checking the record…', 'text', 'Checking the record…', NULL, NULL,
     'Shown while the model is reading documents or answer history mid-reply.', NULL, now(), 'migration'),
    ('practice_chat_send_failed', 'No reply came back. Your message is saved above — send again to retry.', 'text', 'No reply came back. Your message is saved above — send again to retry.', NULL, NULL,
     'Shown when the reply fails after your message was stored.', NULL, now(), 'migration'),
    ('practice_chat_stalled', 'The reply stopped arriving. Your message is saved above — send again to retry.', 'text', 'The reply stopped arriving. Your message is saved above — send again to retry.', NULL, NULL,
     'Shown when the browser hears nothing from the server for too long.', NULL, now(), 'migration'),
    ('practice_chat_refused', 'The model declined to answer that. Your message is saved above — try putting it another way.', 'text', 'The model declined to answer that. Your message is saved above — try putting it another way.', NULL, NULL,
     'Shown when the model refuses a message.', NULL, now(), 'migration'),
    ('practice_chat_truncated', 'The reply ran past its length limit and is not shown. Your message is saved above — send again to retry.', 'text', 'The reply ran past its length limit and is not shown. Your message is saved above — send again to retry.', NULL, NULL,
     'Shown when a reply is cut off at question_chat_max_tokens.', NULL, now(), 'migration'),
    ('practice_chat_load_failed', 'The discussion could not be loaded.', 'text', 'The discussion could not be loaded.', NULL, NULL,
     'Shown when the threads cannot be read.', NULL, now(), 'migration'),
    ('practice_chat_read_mark_failed', 'Your place in this thread could not be saved — its messages may show as new again.', 'text', 'Your place in this thread could not be saved — its messages may show as new again.', NULL, NULL,
     'Shown when opening a thread could not move your read mark (the unread badge would stay lit).', NULL, now(), 'migration'),
    ('practice_chat_cap_reached_template', 'This thread has reached its limit of {max} replies.', 'text', 'This thread has reached its limit of {max} replies.', NULL, NULL,
     'Shown when question_chat_max_turns is reached; the message is not sent.', NULL, now(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ─── 7 · The END state, asserted (CLAUDE.md rule 25a) ───────────────────────
DO $$
DECLARE
    params  INTEGER;
    words   INTEGER;
    ungrounded INTEGER;
BEGIN
    SELECT count(*) INTO ungrounded FROM llm_models WHERE provider = 'anthropic' AND NOT grounded;
    IF ungrounded <> 0 THEN
        RAISE EXCEPTION 'grounded backfill: % anthropic llm_models rows are still not grounded', ungrounded;
    END IF;

    SELECT count(*) INTO params FROM app_settings
     WHERE key LIKE 'question_chat\_%'
        OR key IN ('chat_case_narrative_file', 'chat_witness_display_name');
    IF params <> 18 THEN
        RAISE EXCEPTION 'chat parameter rows: expected 18, found %', params;
    END IF;

    SELECT count(*) INTO words FROM app_settings WHERE key LIKE 'practice_chat\_%';
    IF words <> 42 THEN
        RAISE EXCEPTION 'chat wording rows: expected 42, found %', words;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM llm_models m
          JOIN app_settings s ON s.key = 'question_chat_model' AND s.value = m.id
         WHERE m.is_active AND m.grounded
    ) THEN
        RAISE EXCEPTION 'question_chat_model names no active grounded llm_models row — '
                        'point it at one (the backend would refuse to boot)';
    END IF;
END $$;
