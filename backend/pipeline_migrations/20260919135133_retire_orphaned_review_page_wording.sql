-- retire_orphaned_review_page_wording: nine rows the 2026-08-23 ruling orphaned
--
-- Created: 2026-09-19 13:51:33
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_ORPHAN_ROWS_RETIREMENT_v1. THE ONE MIGRATION this task writes, and the
-- whole of it: nine DELETEs and an assertion. No code changes ride with it.
-- =============================================================================
--
-- ## What these rows were
--
-- The wording of the REVIEW PAGE retired on 2026-08-23 — one question with every
-- attempt stacked underneath it, its self-check boxes and its "stronger answer"
-- panel. Roman's ruling that day retired notes and that whole screen from the
-- interface; the screen went, and these nine rows did not.
--
-- They have been dead since. Their own `consumed_by` columns name two things
-- that no longer exist: `frontend PracticeReview` (there is no such component)
-- and `services::practice_review` (there is no such module — `dto::practice_review`
-- is a different thing, and reads none of these).
--
-- ## ⚑ THEY ARE NOT THE v2.1.14 REVIEW PAGE'S ROWS
--
-- CC_TASK_REVIEW_PAGE_v1 added nine rows under the SAME `practice_review_`
-- prefix on 2026-09-19 — `practice_review_title`, `_unanswered`,
-- `_note_placeholder_answer`, `_note_placeholder_question`, `_answered_template`,
-- `_author_unknown`, `_load_failed`, `_empty_deck`, `_name_joiner`.
--
-- Not one of them is in the list below, and this migration names every key
-- EXPLICITLY rather than matching `practice_review%`. A LIKE here would delete
-- the live page's wording and the backend would refuse to start — which is the
-- single worst thing this file could do, and the reason it is written the long
-- way. The assertion checks that too.
--
-- ## Why deleting rather than leaving them
--
-- A row nothing reads costs nothing to serve, and that is exactly the problem:
-- the Settings page lists them, so an operator editing "A stronger answer"
-- changes nothing and has no way to discover that. Nine rows that quietly do not
-- work are worse than nine rows that are not there.
--
-- ## On a fresh database
--
-- The DELETE matches nothing and the assertion passes. That is correct: a store
-- these rows were never seeded into has nothing to retire. The assertion accepts
-- a BEFORE count of 0 (fresh) or 9 (a store with the full set) and refuses
-- anything between — a partial set means this store's history diverged from the
-- one this file was written against, and that is worth stopping for.
--
-- Measured 2026-09-19: DEV holds 9 of 9, PROD holds 9 of 9 (863 app_settings
-- rows each, before this runs).
--
-- ## Why the live-rows guard compares BEFORE with AFTER, and not with 9
--
-- Because 0 live rows is a legitimate state: on a store where
-- `20260919100133` has not run yet — which is DEV today — the live page's rows
-- do not exist, and asserting `= 9` here would refuse a migration that has
-- nothing to do with them. What this file is responsible for is not REMOVING
-- any that are there, so that is what it asserts. Their ABSENCE is the boot
-- check's business, and it refuses to start over it.
--
-- ## Rolling this back
--
-- Re-insert them from `20260819113610_practice_v1_part_b_deck_editor_notes_and_review.sql`,
-- which is where they were seeded. Nothing reads them either way, so a rollback
-- is only needed if the retired screen is ever revived.

DO $$
DECLARE
    before_count INTEGER;
    after_count  INTEGER;
    live_count   INTEGER;
    orphans      TEXT[] := ARRAY[
        'practice_review_asked_as_template',
        'practice_review_attempts_kicker',
        'practice_review_attempt_template',
        'practice_review_boxes_none',
        'practice_review_detail_template',
        'practice_review_no_attempts',
        'practice_review_practice_again',
        'practice_review_progress_template',
        'practice_review_stronger_heading'
    ];
    -- The LIVE page's rows, named here so the guard below is a positive
    -- assertion rather than a hope. If a future edit ever widens the DELETE to a
    -- prefix match, this is what catches it.
    keep         TEXT[] := ARRAY[
        'practice_review_title',
        'practice_review_unanswered',
        'practice_review_note_placeholder_answer',
        'practice_review_note_placeholder_question',
        'practice_review_answered_template',
        'practice_review_author_unknown',
        'practice_review_load_failed',
        'practice_review_empty_deck',
        'practice_review_name_joiner'
    ];
BEGIN
    SELECT count(*) INTO before_count FROM app_settings WHERE key = ANY(orphans);
    SELECT count(*) INTO live_count   FROM app_settings WHERE key = ANY(keep);

    IF before_count NOT IN (0, 9) THEN
        RAISE EXCEPTION
            'orphan retirement: found % of the 9 orphaned review rows, not 0 or 9. '
            'This store''s history differs from the one this migration was written '
            'against — look at which rows are present before deleting any of them.',
            before_count;
    END IF;

    DELETE FROM app_settings WHERE key = ANY(orphans);

    -- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────
    --
    -- A DELETE that matched nothing is silent in Postgres, so the END state is
    -- asserted rather than the statement trusted — and the LIVE page's rows are
    -- asserted to have survived, because the failure this file could cause is
    -- deleting them and taking the backend's boot with it.
    SELECT count(*) INTO after_count FROM app_settings WHERE key = ANY(orphans);
    IF after_count <> 0 THEN
        RAISE EXCEPTION
            'orphan retirement: % orphaned rows are still present after the DELETE.',
            after_count;
    END IF;

    IF (SELECT count(*) FROM app_settings WHERE key = ANY(keep)) <> live_count THEN
        -- The consequence AND the first move. This is the one failure here
        -- that leaves a store the backend will not boot against, so the
        -- message must not stop at naming it: re-seeding is a specific file,
        -- and whoever is reading this is reading it under time pressure.
        RAISE EXCEPTION
            'orphan retirement: the DELETE touched the LIVE review page''s wording '
            '(% rows before, % after). Those rows are declared at boot and the '
            'backend refuses to start without them. This transaction is rolling '
            'back, so nothing is lost; if a store is ever found already missing '
            'them, re-seed from '
            '20260919100133_review_page_reviewer_list_and_wording.sql.',
            live_count, (SELECT count(*) FROM app_settings WHERE key = ANY(keep));
    END IF;

    RAISE NOTICE 'orphan retirement: % rows retired, % live review rows untouched.',
        before_count, live_count;
END $$;
