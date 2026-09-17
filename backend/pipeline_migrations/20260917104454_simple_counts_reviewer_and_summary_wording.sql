-- simple_counts_reviewer_and_summary_wording: one summary card, three owned queues, one truth
--
-- Created: 2026-09-17 10:44:54
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_SIMPLE_COUNTS_v1, ruled on WAR_ROOM_SUMMARY_CARD_RULED_2026-09-17 and by
-- CC_GO_SIMPLE_COUNTS_v1 (six rulings). THE ONE MIGRATION this task writes.
-- =============================================================================
--
-- ## The ruling this implements
--
-- Three global counts, identical for every viewer, each owned by a named person:
-- unanswered questions (Marie), answers requiring review (the reviewer), and
-- candidates to rule (Roman). The per-viewer "new answers for you" count is gone.
-- Anyone can read answers; only the REVIEWER's Done reviewing press moves the
-- shared review queue.
--
-- ## Who the reviewer is lives in two rows, not in code
--
-- practice_reviewer_username is the login the queue is counted against and the
-- only login offered Done reviewing. practice_reviewer_display_name is what
-- screens print: the owner chip, and the reviewer placeholder in the pill and bar
-- templates (GO ruling 5). Changing attorney is two Settings edits.
--
-- ## Retired rows are DELETED (GO ruling 4)
--
-- The four-tile strip, the viewer pill and the bar's "since you last reviewed"
-- line have no screen left to render them.
--
-- ## Idempotent
--
-- ON CONFLICT (key) DO NOTHING, and the DELETE matches nothing on a re-run; the
-- assertion below checks the END state either way (CLAUDE.md rule 25a).

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('practice_reviewer_username', 'cpenzien', 'text', 'cpenzien', NULL, NULL,
     'The signed-in username (Authentik) of the attorney who reviews Marie''s answers. The review queue is counted against THIS person''s Done reviewing marks, and only this person is offered the Done reviewing button. Changing attorney: edit this row and practice_reviewer_display_name.',
     'api::war_room_progress, api::practice_review_cursor', now(), 'migration'),

    ('practice_reviewer_display_name', 'Chuck', 'text', 'Chuck', NULL, NULL,
     'The reviewer''s name as screens print it: the owner chip on the review cell, and {reviewer} in the review pill and bar.',
     'frontend WarRoomSummaryCard, warRoomCardView, PracticeReviewBar', now(), 'migration'),

    ('war_room_summary_answered_label', 'Questions answered', 'text', 'Questions answered', NULL, NULL,
     'The top row of the War Room summary card: the label beside the answered bar.',
     NULL, now(), 'migration'),

    ('war_room_summary_answered_rest_template', 'of {total} · {pct}%', 'text', 'of {total} · {pct}%', NULL, NULL,
     'After the bold answered number: {total} visible questions across the case and {pct}, the whole-number percentage answered.',
     NULL, now(), 'migration'),

    ('war_room_summary_unanswered_label', 'Unanswered questions', 'text', 'Unanswered questions', NULL, NULL,
     'Marie''s queue: visible questions with no answer, across every scenario. The same number for every viewer.',
     NULL, now(), 'migration'),

    ('war_room_summary_review_label', 'Answers requiring review', 'text', 'Answers requiring review', NULL, NULL,
     'The reviewer''s queue: current answers newer than the reviewer''s Done reviewing on that scenario, not written by the reviewer. The same number for every viewer.',
     NULL, now(), 'migration'),

    ('war_room_summary_candidates_label', 'Candidates to rule', 'text', 'Candidates to rule', NULL, NULL,
     'Roman''s queue: scan proposals nobody has ruled, summed over every scenario.',
     NULL, now(), 'migration'),

    ('war_room_owner_marie', 'Marie', 'text', 'Marie', NULL, NULL,
     'The owner chip on the unanswered-questions cell. (The review cell''s chip prints practice_reviewer_display_name.)',
     NULL, now(), 'migration'),

    ('war_room_owner_roman', 'Roman', 'text', 'Roman', NULL, NULL,
     'The owner chip on the candidates-to-rule cell.',
     NULL, now(), 'migration'),

    ('war_room_summary_unanswered_context_template', 'across {n} scenarios · {codes} untouched', 'text', 'across {n} scenarios · {codes} untouched', NULL, NULL,
     'Context under Marie''s count: {n} scenarios still have unanswered questions; {codes} lists up to three scenarios with a deck and no answer at all.',
     NULL, now(), 'migration'),

    ('war_room_summary_unanswered_context_one', 'across {n} scenario · {codes} untouched', 'text', 'across {n} scenario · {codes} untouched', NULL, NULL,
     'The singular of the line above, when exactly one scenario has unanswered questions.',
     NULL, now(), 'migration'),

    ('war_room_summary_unanswered_context_none_untouched', 'across {n} scenarios', 'text', 'across {n} scenarios', NULL, NULL,
     'Context under Marie''s count when every scenario with a deck has at least one answer.',
     NULL, now(), 'migration'),

    ('war_room_summary_unanswered_context_none_untouched_one', 'across {n} scenario', 'text', 'across {n} scenario', NULL, NULL,
     'The singular of the line above.',
     NULL, now(), 'migration'),

    ('war_room_summary_review_context_template', 'oldest waiting since {date} · {code} has {n}', 'text', 'oldest waiting since {date} · {code} has {n}', NULL, NULL,
     'Context under the review count: the day of the oldest answer still awaiting review, and the scenario with the most waiting.',
     NULL, now(), 'migration'),

    ('war_room_summary_candidates_pile_template', '{codes} {n}', 'text', '{codes} {n}', NULL, NULL,
     'One pile in the line under the candidates count; the four largest pile sizes are listed, and scenarios with the same count share one pile.',
     NULL, now(), 'migration'),

    ('war_room_summary_list_joiner', '·', 'text', '·', NULL, NULL,
     'Between piles in the summary card''s candidates line. The page adds the spaces around it.',
     NULL, now(), 'migration'),

    ('war_room_summary_tie_joiner', '&', 'text', '&', NULL, NULL,
     'Between two scenario codes that share one pile size. The page adds the spaces around it.',
     NULL, now(), 'migration'),

    ('war_room_summary_code_joiner', ',', 'text', ',', NULL, NULL,
     'Between the untouched scenario codes. The page adds the space after it.',
     NULL, now(), 'migration'),

    ('war_room_summary_unanswered_zero', 'every question answered', 'text', 'every question answered', NULL, NULL,
     'Context under Marie''s count when it is zero — a satisfied queue says so.',
     NULL, now(), 'migration'),

    ('war_room_summary_review_zero', 'nothing waiting', 'text', 'nothing waiting', NULL, NULL,
     'Context under the review count when it is zero.',
     NULL, now(), 'migration'),

    ('war_room_summary_candidates_zero', 'nothing to rule', 'text', 'nothing to rule', NULL, NULL,
     'Context under the candidates count when it is zero.',
     NULL, now(), 'migration'),

    ('war_room_card_review_template', '{count} answers awaiting {reviewer}''s review', 'text', '{count} answers awaiting {reviewer}''s review', NULL, NULL,
     'The amber pill on a scenario card: answers awaiting the reviewer on that scenario. The same for every viewer; {reviewer} is practice_reviewer_display_name.',
     NULL, now(), 'migration'),

    ('war_room_card_review_one', '{count} answer awaiting {reviewer}''s review', 'text', '{count} answer awaiting {reviewer}''s review', NULL, NULL,
     'The singular of the pill above, used when exactly one answer is waiting.',
     NULL, now(), 'migration'),

    ('practice_deck_review_awaiting_template', '{count} answers awaiting {reviewer}''s review', 'text', '{count} answers awaiting {reviewer}''s review', NULL, NULL,
     'The review bar under the deck title, shown to everyone; only the reviewer is offered Done reviewing. Not shown at zero.',
     NULL, now(), 'migration'),

    ('practice_deck_review_awaiting_one', '{count} answer awaiting {reviewer}''s review', 'text', '{count} answer awaiting {reviewer}''s review', NULL, NULL,
     'The singular of the bar line above.',
     NULL, now(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ─── Retirements ─────────────────────────────────────────────────────────────
DELETE FROM app_settings
 WHERE key IN (
    'war_room_strip_answered_label',
    'war_room_strip_answered_template',
    'war_room_strip_waiting_label',
    'war_room_strip_new_for_you_label',
    'war_room_strip_candidates_label',
    'war_room_card_viewer_new_template',
    'war_room_card_viewer_new_one',
    'practice_deck_review_new_template',
    'practice_deck_review_new_one');

-- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────────
--
-- An INSERT with a mistyped key and a DELETE that matched nothing are both silent
-- in Postgres, so both directions are asserted — plus the reviewer row's value,
-- because a blank login would zero the review queue for everyone without an error.
DO $$
DECLARE
    present  INTEGER;
    retired  INTEGER;
    reviewer TEXT;
BEGIN
    SELECT count(*) INTO present
      FROM app_settings
     WHERE key IN (
        'practice_reviewer_username',
        'practice_reviewer_display_name',
        'war_room_summary_answered_label',
        'war_room_summary_answered_rest_template',
        'war_room_summary_unanswered_label',
        'war_room_summary_review_label',
        'war_room_summary_candidates_label',
        'war_room_owner_marie',
        'war_room_owner_roman',
        'war_room_summary_unanswered_context_template',
        'war_room_summary_unanswered_context_one',
        'war_room_summary_unanswered_context_none_untouched',
        'war_room_summary_unanswered_context_none_untouched_one',
        'war_room_summary_review_context_template',
        'war_room_summary_candidates_pile_template',
        'war_room_summary_list_joiner',
        'war_room_summary_tie_joiner',
        'war_room_summary_code_joiner',
        'war_room_summary_unanswered_zero',
        'war_room_summary_review_zero',
        'war_room_summary_candidates_zero',
        'war_room_card_review_template',
        'war_room_card_review_one',
        'practice_deck_review_awaiting_template',
        'practice_deck_review_awaiting_one');
    IF present <> 25 THEN
        RAISE EXCEPTION
            'simple counts: expected 25 new settings rows, found %. The backend '
            'declares every one at boot and refuses to start without it.', present;
    END IF;

    SELECT count(*) INTO retired
      FROM app_settings
     WHERE key IN (
        'war_room_strip_answered_label',
        'war_room_strip_answered_template',
        'war_room_strip_waiting_label',
        'war_room_strip_new_for_you_label',
        'war_room_strip_candidates_label',
        'war_room_card_viewer_new_template',
        'war_room_card_viewer_new_one',
        'practice_deck_review_new_template',
        'practice_deck_review_new_one');
    IF retired <> 0 THEN
        RAISE EXCEPTION 'simple counts: % retired rows are still present.', retired;
    END IF;

    SELECT value INTO reviewer FROM app_settings WHERE key = 'practice_reviewer_username';
    IF reviewer IS NULL OR btrim(reviewer) = '' THEN
        RAISE EXCEPTION 'simple counts: practice_reviewer_username is missing or blank.';
    END IF;
END $$;
