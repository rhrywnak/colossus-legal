-- remove_reviewer_authored_ssa_answer: 185 of 185 answered was never true
--
-- Created: 2026-09-20 16:13:42
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_REVIEW_COUNTS_HONEST_v1 — a DATA FIX, shipped as one migration
-- (Law 20c, the `reattribute_practice_sessions_to_marie` precedent): guarded on
-- the shape, tolerant of zero rows, END state asserted, no operator SQL.
-- =============================================================================
--
-- ## Why (found 2026-09-19)
--
-- PROD's dashboard read 185 of 185 answered and 184 requiring review at the same
-- moment. Both numbers were computed correctly from the same store, and they
-- disagreed because ONE answer was written by a reviewer:
--
--   · `war_room_status::deck_counts` counts an answer however it got there
--     (`war_room_status.rs:117`), so the junk answer counted as answered;
--   · `review_cursor::ANSWER_WAITING` excludes answers written by anyone on the
--     reviewer bench (`review_cursor.rs:203-205`), so it did not wait.
--
-- The answer sits on S-13's SSA question — "What did the SSA tell you in
-- November 2025?" — and Marie has never answered it. Nobody typed it as her.
--
-- ## Why DELETE and not reattribute
--
-- The 2026-09-17 migration reattributed sittings because those answers really
-- were Marie's, typed by her under a shared login. This one was not typed by
-- her at all. Restamping it would make the dashboard's 185 honest and the
-- RECORD false, which is the worse of the two.
--
-- ## What this does NOT delete
--
-- Nothing but the matching `practice_answers` rows. The sitting stays (a
-- childless session is not a lie about anything, and the 09-18 cleanup that
-- proposed removing them was killed). The question stays. `practice_notes` rows
-- whose `answer_id` names a deleted answer are removed by the schema's own
-- CASCADE (`20260819113610…sql:182`) — the count is reported below before the
-- delete runs, because a note quietly vanishing is exactly the silent failure
-- Standing Rule 1 exists to stop.
--
-- ## The consequence, stated so it is not read as a regression
--
-- After this lands on PROD the dashboard reads **184 of 185 answered** and
-- Marie's Unanswered reads **1** — the SSA question she has genuinely never
-- answered. That is the point. Answering it through the app returns it to N/N.
--
-- ## On a fresh database
--
-- The DELETE matches zero rows and every assertion passes. A store without the
-- junk row has nothing to repair, and a new environment must not fail here.
--
-- ## Why zero rows WARNS when the question already has answers (ruled 2026-09-20)
--
-- Tolerating zero is what makes this safe on a fresh database, and it is also
-- what would hide a mis-aimed predicate on PROD: a DELETE that matched nothing
-- and an END state of nothing are the same two numbers whether the store was
-- already clean or the author login was wrong. The two cases ARE distinguishable
-- by one more read — a fresh store has no answers on that question at all,
-- while a store where the predicate missed has some. So zero matches plus a
-- non-empty question raises a WARNING naming both counts. Never an exception:
-- a legitimately repaired PROD reaches exactly that state on the second deploy.

DO $$
DECLARE
    target_question CONSTANT UUID := '77fde47d-92da-409b-9276-ea55cd5750c5';
    target_author   CONSTANT TEXT := 'roman';
    matching        INTEGER;
    on_question     INTEGER;
    cascading_notes INTEGER;
    removed         INTEGER;
    remaining       INTEGER;
BEGIN
    SELECT count(*) INTO matching
      FROM practice_answers a
      JOIN practice_sessions s ON s.id = a.session_id
     WHERE a.question_id = target_question
       AND s.user_id = target_author;

    -- 0 is a clean no-op (DEV, and any fresh database). 1 is PROD. More than one
    -- means the store is not the store this fix was written against, and a
    -- DELETE would take rows nobody has looked at.
    IF matching > 1 THEN
        RAISE EXCEPTION
            'junk answer: % answers on question % are authored by %, and this fix '
            'was ruled against exactly one. Nothing was deleted — look at the rows '
            'before widening it.', matching, target_question, target_author;
    END IF;

    SELECT count(*) INTO on_question
      FROM practice_answers a WHERE a.question_id = target_question;

    IF matching = 0 AND on_question > 0 THEN
        RAISE WARNING
            'junk answer: no answer on question % is authored by %, but the '
            'question carries % answer(s). Either this store was already repaired '
            '— which is the expected second-deploy state — or the author login is '
            'wrong and nothing was removed. Check who wrote them before trusting '
            'the answered count.', target_question, target_author, on_question;
    END IF;

    SELECT count(*) INTO cascading_notes
      FROM practice_notes n
     WHERE n.answer_id IN (
            SELECT a.id
              FROM practice_answers a
              JOIN practice_sessions s ON s.id = a.session_id
             WHERE a.question_id = target_question
               AND s.user_id = target_author);

    -- Said out loud BEFORE the delete: practice_notes.answer_id cascades, so
    -- notes on this answer go with it and Postgres reports nothing.
    RAISE NOTICE
        'junk answer: % matching answer(s), carrying % note(s) that the schema''s '
        'CASCADE will remove with them.', matching, cascading_notes;

    DELETE FROM practice_answers a
     USING practice_sessions s
     WHERE s.id = a.session_id
       AND a.question_id = target_question
       AND s.user_id = target_author;

    GET DIAGNOSTICS removed = ROW_COUNT;
    RAISE NOTICE 'junk answer: % answer row(s) deleted.', removed;

    -- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────
    --
    -- A DELETE that matched nothing is silent in Postgres. This asserts the END
    -- state, so a re-run passes and a row the predicate missed fails loudly.
    SELECT count(*) INTO remaining
      FROM practice_answers a
      JOIN practice_sessions s ON s.id = a.session_id
     WHERE a.question_id = target_question
       AND s.user_id = target_author;

    IF remaining <> 0 THEN
        RAISE EXCEPTION
            'junk answer: % reviewer-authored answer(s) remain on question % after '
            'the DELETE — the dashboard would go on reporting an answer nobody '
            'typed.', remaining, target_question;
    END IF;
END $$;
