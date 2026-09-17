-- read_budget_and_effort: the read's token budget vs Opus 5's thinking
--
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_READ_V4_BUDGET_FIX_v1, ruled by CC_GO_READ_V4_BUDGET_v1 (five rulings).
-- A NEW migration, never an edit: the rows it changes are already applied on DEV
-- (Law 20).
-- =============================================================================
--
-- ## The live failure this repairs (DEV, 2026-09-17 18:06:39Z)
--
-- The first answer read on prompt v4, Opus 5, cap 1024. Attempt 1 completed (699
-- output tokens) but broke the 12-word `call` ceiling, so it was re-requested, as
-- designed. Attempt 2 came back TRUNCATED: "Produced 1024 output tokens against a
-- configured cap of 1024", two content blocks (text x1, thinking x1),
-- stop_reason=max_tokens. Two attempts is the ceiling, so Marie read
-- "I can't read this one." The stored row shows read_attempts=2, read_ms=26003,
-- read_ok NULL.
--
-- Two causes, two rows:
--
-- 1. THE BUDGET. 1024 has no room for adaptive thinking PLUS a JSON verdict. The
--    verdict itself is small — the last three v3 reads produced 166, 323 and 539
--    output tokens — so 4096 is headroom for the thinking, not for the answer.
--
-- 2. THE THINKING. The read sent no `effort` key, which means the API's own
--    default (`high`), and on Opus 5 that is adaptive thinking ON, counted against
--    the same max_tokens as the answer (the 2026-08-28 incident, one budget
--    smaller). The read is a strict-JSON verdict against a 12-word ceiling; it has
--    nothing to gain from a long deliberation, and the witness waits through it.
--    `low` is the documented remedy — turning thinking DOWN is supported, turning
--    it off is not — and it is what the extraction family already sends for a far
--    bigger job.
--
-- The dock gets the same dial in the same migration (GO ruling 5): it sends no
-- effort either, and a long thinking pass there truncates a reply mid-sentence and
-- stores it as a turn.
--
-- The 12-word `call` ceiling is NOT touched: the re-request handled it correctly.
--
-- The UPDATE alignment below is what `domain::wording::tests::corrected_value_in`
-- parses.

-- ─── 1 · The read's budget ───────────────────────────────────────────────────
UPDATE app_settings
SET value         = '4096',
    default_value = '4096',
    meaning       = 'The read''s output cap. The reply is a small JSON verdict — the '
                    'last v3 reads produced 166-539 output tokens — so this is headroom '
                    'for a model that THINKS before it answers, not for the answer. At '
                    '1024 the first v4 read was truncated by a thinking block '
                    '(2026-09-17). Domain note: 4096 is under every active Anthropic '
                    'row''s ceiling but ABOVE the two 2048-token vLLM rows, so pointing '
                    'practice_read_model at a local model makes the read abstain by '
                    'name — constrain REFUSES a cap above a model''s ceiling rather than '
                    'clamping it, and a named abstain is the correct behaviour (ruled '
                    '2026-09-17).',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_read_max_tokens';

-- ─── 2 · The two effort dials ────────────────────────────────────────────────
INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('practice_read_effort', 'low', 'text', 'low', NULL, NULL,
     'How much the model may THINK before it writes one answer read: low, medium, '
     'high, xhigh, max, or absent to send no effort key at all (the API default, '
     'which is high). Thinking tokens count against practice_read_max_tokens, and '
     'at high they truncated the first v4 read. The read is a 12-word verdict with '
     'a why and up to three pointers; low is the documented remedy and what '
     'extraction already sends. Set it to absent to restore the pre-2026-09-17 '
     'behaviour without a deploy.',
     'services::practice_read_setup', now(), 'migration'),

    ('practice_discuss_effort', 'low', 'text', 'low', NULL, NULL,
     'The same dial for the Discuss with AI dock. Same vocabulary, same reason: a '
     'long thinking pass spends the reply''s budget and the reader''s wait, and a '
     'truncated reply is STORED as a turn. Set it to absent to send no effort key.',
     'services::practice_discuss_run', now(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ─── 3 · The END-state assertion (CLAUDE.md rule 25a) ────────────────────────
--
-- A statement matching zero rows is SILENT in Postgres and the old value keeps
-- being served — here that means the read stays truncated and nobody is told.
DO $$
DECLARE
    budget  TEXT;
    efforts INTEGER;
BEGIN
    SELECT value INTO budget FROM app_settings WHERE key = 'practice_read_max_tokens';
    IF budget IS DISTINCT FROM '4096' THEN
        RAISE EXCEPTION
            'read budget: practice_read_max_tokens is %, expected 4096 — the read '
            'would still be truncated by a thinking block.', budget;
    END IF;

    SELECT count(*) INTO efforts
      FROM app_settings
     WHERE key IN ('practice_read_effort', 'practice_discuss_effort')
       AND value = 'low';
    IF efforts <> 2 THEN
        RAISE EXCEPTION
            'read budget: expected both effort rows at low, found % — the backend '
            'reads both keys at boot and EXITS when either is missing or carries a '
            'word outside low/medium/high/xhigh/max/absent.', efforts;
    END IF;
END $$;
