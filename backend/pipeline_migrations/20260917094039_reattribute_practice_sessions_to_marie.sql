-- reattribute_practice_sessions_to_marie: the answers were always Marie's
--
-- Created: 2026-09-17 09:40:39
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_REATTRIBUTE_ANSWERS_v1 — a DATA FIX, shipped as one migration (Law 20c,
-- the Qwen display-name precedent): guarded on the old value, END-state asserted,
-- no operator SQL.
-- =============================================================================
--
-- ## Why (found 2026-09-17)
--
-- Every practice sitting is stamped with the login that opened it
-- (`practice_sessions.user_id`, via `services::practice_notes::attribution`).
-- Until today Marie practised signed in as `roman` — a shared login — so every
-- answer she wrote sits under Roman's name, and sittings from before the
-- 2026-08-19 attribution migration carry NULL. On PROD, `/api/users` shows her
-- own account, `docmarie`, first seen as herself on 2026-09-17.
--
-- The review loop (v2.1.10) made that mis-stamp visible: the War Room counts an
-- answer as "new for you" only when SOMEONE ELSE wrote it, so Roman — credited
-- with Marie's answers — was shown none of them as new.
--
-- ## The ruling, on the record
--
-- Roman: "Only Marie answers questions."
--
-- So every sitting stamped `roman`, and every unattributed one, is Marie's.
--
-- ## What this does NOT touch
--
-- Only the SITTINGS were mis-stamped. `practice_deck_changes`, `practice_notes`,
-- the question flags and `known_users` are untouched: Chuck's and Roman's deck
-- edits and notes really are theirs.
--
-- ## From today forward
--
-- Nothing in code changes. Marie now signs in as herself, so every sitting she
-- opens is stamped `docmarie` by the login. This migration only repairs the past.
--
-- ## On a fresh database
--
-- The UPDATE matches zero rows and the assertion passes. That is correct: a
-- store with no mis-stamped sittings has nothing to repair.

UPDATE practice_sessions
   SET user_id   = 'docmarie',
       user_name = 'Marie'
 WHERE user_id = 'roman' OR user_id IS NULL;

-- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────────
--
-- A guard that matched nothing is silent in Postgres, and the War Room would go
-- on hiding every one of Marie's answers from Roman. Assert what must be TRUE
-- afterwards, so a re-run passes and a missed row fails loudly.
DO $$
DECLARE
    remaining INTEGER;
BEGIN
    SELECT count(*) INTO remaining
      FROM practice_sessions
     WHERE user_id = 'roman' OR user_id IS NULL;

    IF remaining <> 0 THEN
        RAISE EXCEPTION
            'reattribute practice sessions: % sittings are still stamped roman or '
            'unattributed after the UPDATE — the answers would stay credited to the '
            'wrong person.', remaining;
    END IF;
END $$;
