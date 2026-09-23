-- deck_review_bar_speaks_to_the_viewer: the deck's review count is the reader's
--
-- Created: 2026-09-23 07:10:48
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_FOR_YOU_v1, the ruling of 2026-09-23 on BUILD L3 §6: the deck's
-- review bar follows the war room's — drawn ONLY for somebody who may review,
-- and its words name the READER, never the bench.
-- =============================================================================
--
-- ## Why this is a second migration and not an edit of the first
--
-- `20260922223758` corrected the WAR ROOM's three strings, and DEV has since
-- applied it: its checksum is recorded, and `sqlx` refuses to boot against a
-- file whose digest moved. So the deck bar's correction ships here, in a file
-- nothing has applied yet. (Ruled 2026-09-23. The guard is
-- `backend/src/frozen_migrations.rs`, which pins the six digests DEV holds.)
--
-- ## What was wrong, measured rather than argued
--
-- On a copy of DEV, one deck, one moment: Chuck read `awaiting 2`, Roman
-- `awaiting 1`, Marie `awaiting 1` — three different numbers, because since
-- CC_TASK_FOR_YOU_v1 L2 read-state is per person and the bar's count is
-- whatever THIS reader has not read. All three sat under one sentence:
-- "N questions awaiting Chuck · Roman's review".
--
-- That is the same defect the war room ruling names, one screen over: the
-- witness reading HER number under a label that says CHUCK. Worse here, because
-- the witness is not a reviewer at all — the bar counts what waits on the
-- REVIEWERS' side, which is not a duty she has and not a number she can act on.
--
-- ## Two halves, and why the words alone would not do
--
-- The sentence now addresses whoever is looking. The other half is in the code:
-- a viewer who may not review is served no count at all (the deck read skips
-- the query) and the bar is not drawn. Correcting only the words would leave
-- her reading "2 questions awaiting your review" about answers she wrote.
--
-- ## The KEYS do not change (the REVIEW_PERMISSION precedent, ruling R3)
--
-- Renaming them would mean new rows, a data-moving migration and edits across
-- the block, its fixture and the wire mirror, for no change in behaviour. Only
-- the VALUES and their stored `meaning` change here.
--
-- ## Fresh-database reproduction (Law 20)
--
-- `20260917104454` seeds the bench-voiced values and `20260917153315` corrects
-- "answers" to "questions"; this file runs after both in version order and
-- rewrites the pair, so a database built from zero ends on the text below.
-- `{reviewer}` leaves both templates and is required by nothing —
-- `wording_templates::REQUIRED_PLACEHOLDERS` names neither key.

UPDATE app_settings
   SET value         = '{count} questions awaiting your review',
       default_value = '{count} questions awaiting your review',
       meaning       = 'The amber sentence on a deck''s review bar: questions on this deck carrying something the SIGNED-IN READER has not read — a new answer, or a note from the witness. Drawn only for somebody who may review (a listed reviewer or an administrator); a person who may not review sees no bar at all, because the number counts a duty that is not theirs. It speaks in the second person deliberately: since 2026-09-22 the count is per reader, and naming the bench on a number that is the reader''s own told three people the same sentence about three different facts.',
       updated_at    = now(),
       updated_by    = 'migration'
 WHERE key           = 'practice_deck_review_awaiting_template';

UPDATE app_settings
   SET value         = '{count} question awaiting your review',
       default_value = '{count} question awaiting your review',
       meaning       = 'The singular of practice_deck_review_awaiting_template, used when the count is exactly 1 — "1 questions" does not ship. Same audience and same voice as the plural.',
       updated_at    = now(),
       updated_by    = 'migration'
 WHERE key           = 'practice_deck_review_awaiting_one';

-- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────────
-- An UPDATE whose WHERE matched nothing is silent in Postgres, and the bench's
-- sentence would go on being served under a build that no longer means it.
DO $$
DECLARE
    corrected INTEGER;
    stale     TEXT;
BEGIN
    SELECT count(*) INTO corrected
      FROM app_settings
     WHERE (key = 'practice_deck_review_awaiting_template'
            AND value = '{count} questions awaiting your review')
        OR (key = 'practice_deck_review_awaiting_one'
            AND value = '{count} question awaiting your review');
    IF corrected <> 2 THEN
        -- Name the ROW, not just the count: an operator reading "found 1" in a
        -- deploy log would otherwise have to open psql to learn which of the
        -- pair fell short, and the pair is the whole point of this file.
        SELECT string_agg(key, ', ' ORDER BY key) INTO stale
          FROM app_settings
         WHERE key IN ('practice_deck_review_awaiting_template',
                       'practice_deck_review_awaiting_one')
           AND value NOT IN ('{count} questions awaiting your review',
                             '{count} question awaiting your review');
        RAISE EXCEPTION
            'deck review bar: expected 2 rows to speak to the viewer, found % — still wrong: %',
            corrected, coalesce(stale, '(a row is missing from app_settings entirely)');
    END IF;

    -- And the bench's name is gone from BOTH. A correction that landed on one
    -- of the pair would pass the count above — which is exactly how a screen
    -- ends up saying one thing in the singular and another in the plural.
    SELECT string_agg(key, ', ' ORDER BY key) INTO stale
      FROM app_settings
     WHERE key IN ('practice_deck_review_awaiting_template',
                   'practice_deck_review_awaiting_one')
       AND value LIKE '%{reviewer}%';
    IF stale IS NOT NULL THEN
        RAISE EXCEPTION
            'deck review bar: % still names the bench with {reviewer}', stale;
    END IF;
END $$;
