-- defect_sweep: the sittings nobody closed, the questions with no handle, and
-- one sentence that stopped being true
--
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_DEFECT_SWEEP_v1, ruled by CC_GO_DEFECT_SWEEP_v1 (six rulings).
-- THREE store changes in ONE migration, because they ship in one release and
-- each is a single guarded statement. Two of the six defects need no store
-- change at all (the Neo4j indexes are created by the app's own boot migrator;
-- the flag's retirement removes code, never columns) and one was closed as
-- not-reproduced.
-- =============================================================================

-- ─── 1 · The sittings nobody ever closed (defect 1) ──────────────────────────
--
-- ## What was measured on DEV, 2026-09-17
--
-- 26 open sittings (`ended_at IS NULL`): 25 fossils of the retired multi-session
-- flow (2026-08-19 → 08-23) and exactly ONE from the one-page flow. The row
-- count was never the problem — `open_session_for_answers` REUSES the newest
-- unended row, so the one-page flow opens one per user per scenario and no more.
--
-- What broke is what `ended_at` MEANS to the one thing that reads it.
-- `practice::last_ended_session` feeds the deck's "what changed since your last
-- sitting" box, and with `max(ended_at)` frozen at 2026-08-20 11:58 that box was
-- measuring from a month earlier: 144 deck changes qualified, and the number
-- could only grow.
--
-- ## The stamp, and the consequence Roman accepted (ruling 1b)
--
-- Each sitting is closed at its OWN last answer, falling back to `started_at`
-- when it holds none — never `NOW()`, which would record a month of silence as a
-- month of practice. 18 of the 26 hold no answers at all.
--
-- Closing them moves `last_ended_session` forward, which EMPTIES the changed-box
-- on the deck page. That was put to Roman as the visible consequence and
-- ACCEPTED: the box belongs to the retired sitting path, and Marie's real signal
-- since v2.1.10 is the per-question badge, which reads no sittings at all.
--
-- Today's sittings are deliberately NOT closed: somebody may be answering into
-- one right now, and ending it under her is the one outcome worse than the bug.
-- From here `api::practice_one_page::post_answer_session` does this day by day,
-- through `practice_sitting_close::close_sittings_before_today`.
-- The timezone comes from `practice_case_timezone`, the same row
-- `close_sittings_before_today` is handed at runtime — never a literal here
-- (Standing Rule 2). A migration that spelled the zone out would be a second
-- source of truth for the case's own calendar, and it would be the one nobody
-- edits when the case moves.
UPDATE practice_sessions s
   SET ended_at = COALESCE(
           (SELECT MAX(a.answered_at) FROM practice_answers a
             WHERE a.session_id = s.id),
           s.started_at)
 WHERE s.ended_at IS NULL
   AND (s.started_at AT TIME ZONE
        (SELECT value FROM app_settings WHERE key = 'practice_case_timezone'))::date
       < (NOW() AT TIME ZONE
          (SELECT value FROM app_settings WHERE key = 'practice_case_timezone'))::date;

-- ─── 2 · A handle for every question (defect 4) ──────────────────────────────
--
-- ## Why the `x` namespace and not `g6`, `c9` (ruling 4)
--
-- `deck_key` is what `seed_practice_deck --update` matches on — BY KEY, never by
-- text, because text is what a re-wording changes. That made the old NULL a loud
-- failure and a minted file-namespace key a silent one:
--
--   * NULL: `--update` REFUSES the whole run on a stored row it cannot match by
--     text (`UpdateError::StoredRowUnmatched`). Measured: 35 such rows, ALL in
--     one scenario, which means `--update` refuses on that deck today. A
--     redirect also cannot anchor to an un-keyed question, because `follows_key`
--     resolves against a cross question's `deck_key`.
--   * `g6`: the architect's next file question takes `g6` too, `--update`
--     matches by key, and one question's text is rewritten with another's —
--     silently, on a deck Marie practises from.
--
-- `x` belongs to no side, so the deck file can never write it; a row whose key
-- the file does not mention is already "LEFT ALONE and listed in the report".
--
-- Numbered per scenario by `sort_order` so the result is deterministic and a
-- re-run on a fresh database produces the same keys, and continued past any `x`
-- key the scenario already holds so this can never collide with a key
-- `deck_key_mint` has already issued.
WITH numbered AS (
    SELECT q.id,
           q.scenario_id,
           'x' || (
               COALESCE(
                   (SELECT MAX(NULLIF(regexp_replace(e.deck_key, '^x', ''), '')::int)
                      FROM practice_questions e
                     WHERE e.scenario_id = q.scenario_id
                       AND e.deck_key ~ '^x[0-9]+$'),
                   0)
               + ROW_NUMBER() OVER (PARTITION BY q.scenario_id ORDER BY q.sort_order, q.id)
           )::text AS minted
      FROM practice_questions q
     WHERE q.deck_key IS NULL
)
UPDATE practice_questions t
   SET deck_key = numbered.minted,
       updated_at = NOW()
  FROM numbered
 WHERE t.id = numbered.id;

-- ─── 3 · The review bar stops saying "answers" (defect 5) ────────────────────
--
-- The queue now counts a question waiting on an unstruck NOTE as well as one
-- waiting on an answer (`review_cursor::NOTE_WAITING`), so "{count} answers
-- awaiting {reviewer}'s review" became false the moment that shipped: the count
-- is questions, and some of them are waiting on something Marie wrote rather
-- than on an answer. Ruled 2026-09-17 — "the count must never be a mild lie".
--
-- The keys do not change, so nothing in the frontend moves; only the sentences.
UPDATE app_settings
   SET value         = '{count} questions awaiting {reviewer}''s review',
       default_value = '{count} questions awaiting {reviewer}''s review',
       updated_at    = NOW(),
       updated_by    = 'migration'
 WHERE key           = 'practice_deck_review_awaiting_template';

UPDATE app_settings
   SET value         = '{count} question awaiting {reviewer}''s review',
       default_value = '{count} question awaiting {reviewer}''s review',
       updated_at    = NOW(),
       updated_by    = 'migration'
 WHERE key           = 'practice_deck_review_awaiting_one';

-- ─── 4 · The END-state assertions (CLAUDE.md rule 25a) ───────────────────────
--
-- A statement matching zero rows is SILENT in Postgres and the old value keeps
-- being served. All three of the above are UPDATEs over rows that must exist, so
-- all three are assertable — and each failure sentence names what an operator
-- would have to look at.
DO $$
DECLARE
    still_open   INTEGER;
    unkeyed      INTEGER;
    duplicates   INTEGER;
    reworded     INTEGER;
BEGIN
    -- 1 · No sitting from an earlier day is still open. Today's may be.
    SELECT count(*) INTO still_open
      FROM practice_sessions
     WHERE ended_at IS NULL
       AND (started_at AT TIME ZONE
            (SELECT value FROM app_settings WHERE key = 'practice_case_timezone'))::date
           < (NOW() AT TIME ZONE
              (SELECT value FROM app_settings WHERE key = 'practice_case_timezone'))::date;
    IF still_open <> 0 THEN
        RAISE EXCEPTION
            'defect sweep: % sittings from an earlier day are still open — the '
            'deck''s "what changed since your last sitting" box would keep '
            'measuring from whenever the last one ended.', still_open;
    END IF;

    -- 2 · Every question carries a handle, and no scenario has two the same.
    SELECT count(*) INTO unkeyed FROM practice_questions WHERE deck_key IS NULL;
    IF unkeyed <> 0 THEN
        RAISE EXCEPTION
            'defect sweep: % questions still carry no deck_key — seed_practice_deck '
            '--update refuses a whole run on one of these, and no redirect can '
            'anchor to it.', unkeyed;
    END IF;

    SELECT count(*) INTO duplicates FROM (
        SELECT scenario_id, deck_key FROM practice_questions
         WHERE deck_key IS NOT NULL
         GROUP BY scenario_id, deck_key HAVING count(*) > 1) d;
    IF duplicates <> 0 THEN
        RAISE EXCEPTION
            'defect sweep: % (scenario, deck_key) pairs are duplicated — two '
            'questions sharing a handle is the state --update would later '
            'resolve by guessing.', duplicates;
    END IF;

    -- 3 · Both review sentences count QUESTIONS, not answers.
    SELECT count(*) INTO reworded
      FROM app_settings
     WHERE key IN ('practice_deck_review_awaiting_template',
                   'practice_deck_review_awaiting_one')
       AND value LIKE '%question%'
       AND value NOT LIKE '%answer%';
    IF reworded <> 2 THEN
        RAISE EXCEPTION
            'defect sweep: expected both review sentences to count questions, '
            'found % — the bar would name answers over a count that includes '
            'notes.', reworded;
    END IF;
END $$;
