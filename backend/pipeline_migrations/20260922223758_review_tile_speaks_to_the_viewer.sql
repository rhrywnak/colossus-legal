-- review_tile_speaks_to_the_viewer: the review count is the reader's own
--
-- Created: 2026-09-22 22:37:58
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_FOR_YOU_v1, the L2 ruling of 2026-09-22 (option b with a's label):
-- the war room's review tile and pill are drawn ONLY for somebody who may
-- review, and their words name the READER, never the bench. Her own "for Marie"
-- tile is untouched.
-- =============================================================================
--
-- ## Why the words had to move, and not just the visibility
--
-- L2 made the review count per person — read-state is per person, so there is
-- no shared mark left for a shared count to be derived from. The words did not
-- follow it, and the result was measured on a scratch copy of DEV: the witness
-- read HER number under a tile labelled CHUCK, on different decks from his.
-- Same label, same shape, a different fact.
--
-- Hiding the tile from her fixes the witness's case. It does not fix Chuck's:
-- "answers awaiting Chuck's review" is still the wrong sentence when ROMAN is
-- the one reading it, and this build has two reviewers. So the sentence
-- addresses whoever is looking — which is what it now always is.
--
-- ## The KEYS do not change (the REVIEW_PERMISSION precedent, ruling R3)
--
-- Renaming them would mean new rows, a data-moving migration and edits across
-- four blocks and their fixtures, for no change in behaviour. Only the VALUES
-- and their stored `meaning` change here, plus one new row for the tile's owner
-- chip — which cannot be corrected because there was nothing to correct: the
-- chip took the reviewer's display NAME, which is not a wording row at all.
--
-- ## Fresh-database reproduction (Law 20)
--
-- The seeding migration (20260917104454) writes the bench-voiced values; this
-- file runs after it in version order and rewrites all three, so a database
-- built from zero ends on the text below. `{reviewer}` leaves two templates and
-- is required by nothing — `wording_templates::REQUIRED_PLACEHOLDERS` names
-- neither key.

UPDATE app_settings
   SET value         = '{count} answers awaiting your review',
       default_value = '{count} answers awaiting your review',
       meaning       = 'The amber pill on a war room card: answers and notes on this deck that the SIGNED-IN READER has not read. Drawn only for somebody who may review (a listed reviewer or an administrator) — a person who may not review sees no pill, because the number would be about a duty that is not theirs. It speaks in the second person deliberately: since 2026-09-22 the count is per reader, and naming the bench on a number that is the reader''s own told two reviewers the same sentence about two different facts.',
       updated_at    = now(),
       updated_by    = 'migration'
 WHERE key           = 'war_room_card_review_template';

UPDATE app_settings
   SET value         = '{count} answer awaiting your review',
       default_value = '{count} answer awaiting your review',
       meaning       = 'The singular of war_room_card_review_template, used when the count is exactly 1 — "1 answers" does not ship. Same audience and same voice as the plural.',
       updated_at    = now(),
       updated_by    = 'migration'
 WHERE key           = 'war_room_card_review_one';

UPDATE app_settings
   SET value         = 'Answers awaiting your review',
       default_value = 'Answers awaiting your review',
       meaning       = 'The third queue tile''s label on the war room strip. Drawn only for somebody who may review; the number under it is that reader''s own backlog across every deck of the case, and it opens their For you list.',
       updated_at    = now(),
       updated_by    = 'migration'
 WHERE key           = 'war_room_summary_review_label';

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('war_room_summary_review_chip', 'YOU', 'text', 'YOU', NULL, NULL,
     'The small owner chip on the review tile, where the other two tiles print MARIE and ROMAN. It says YOU because the tile is only ever drawn for the person whose backlog it counts. Its predecessor was not a settings row at all — the chip took practice_reviewer_display_names, which named the bench on a number that belongs to the reader.',
     NULL, now(), 'migration')
-- Re-runnable, like every seeding migration in this directory: sqlx applies a
-- file once, but a proof that re-applies it by hand must not die on the second
-- pass, and a database that somehow already carries the row keeps what it has.
ON CONFLICT (key) DO NOTHING;

-- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────────
-- An UPDATE whose WHERE matched nothing is silent in Postgres, and the bench's
-- sentence would go on being served under a build that no longer means it.
DO $$
DECLARE
    corrected INTEGER;
BEGIN
    SELECT count(*) INTO corrected
      FROM app_settings
     WHERE (key = 'war_room_card_review_template'
            AND value = '{count} answers awaiting your review')
        OR (key = 'war_room_card_review_one'
            AND value = '{count} answer awaiting your review')
        OR (key = 'war_room_summary_review_label'
            AND value = 'Answers awaiting your review')
        -- The chip is checked for PRESENCE, not for 'YOU'. The three above
        -- are corrections and must land on the exact sentence this build
        -- reads; the chip is a new row, and the day Roman retunes it in
        -- Settings the value is his. What must be true is that the row exists
        -- and is not blank — a blank would leave the tile with an empty chip.
        OR (key = 'war_room_summary_review_chip' AND btrim(value) <> '');
    IF corrected <> 4 THEN
        RAISE EXCEPTION
            'review tile: expected 4 rows to speak to the viewer, found %', corrected;
    END IF;
    -- And the bench's name is gone from both templates. A correction that
    -- landed on one of the pair would pass the count above.
    IF EXISTS (SELECT 1 FROM app_settings
                WHERE key IN ('war_room_card_review_template', 'war_room_card_review_one')
                  AND value LIKE '%{reviewer}%') THEN
        RAISE EXCEPTION
            'review tile: a review pill still names the bench with {reviewer}';
    END IF;
END $$;
