-- review_permission_display_only: the reviewer rows become DISPLAY-only
--
-- Created: 2026-09-22 13:53:53
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_REVIEW_PERMISSION_v1, ruling R3.
-- =============================================================================
--
-- ## What changed in the code, and why these two rows now read differently
--
-- `practice_reviewer_usernames` did two unrelated jobs: it decided who may press
-- "Done reviewing", and it decided whose names the war room prints. Roman took
-- himself off the list so the dashboard would stop naming him, and lost the
-- ability to review with it (2026-09-22).
--
-- Permission now comes from `services::review_permission::may_review`: a listed
-- reviewer OR a member of the deployment's admin group. These rows keep only
-- their DISPLAY job, plus the one queue job SQL can do — an answer or a note
-- written by a LISTED reviewer does not wait for review (an unlisted
-- administrator's own work does, until a Done is pressed; ruling R2).
--
-- ## The KEYS DO NOT CHANGE (ruling R3)
--
-- Renaming them would mean new rows, a data-moving migration and edits across
-- `practice_params`, `settings_practice`, `settings_groups`, the war room, the
-- chat gather and three fixtures — for no change in behaviour. Only the stored
-- `meaning` (the help text the Settings page prints under each row) changes
-- here; the group's LABEL is a code constant and changes with the code.
--
-- ## Fresh-database reproduction
--
-- The seeding migrations write the old `meaning`; this file runs after them in
-- version order and rewrites both rows, so a database built from zero ends on
-- the text below. Nothing else in the store is touched: no value, no default,
-- no new key, no schema.

UPDATE app_settings
SET meaning       = 'The signed-in usernames (Authentik) of the reviewers SHOWN ON THE WAR ROOM, comma-separated. This list is DISPLAY, not permission: since 2026-09-22 anyone in the admin group may press Done reviewing whether or not they are listed here, and any permitted press clears the queue for everyone. What the list still decides: whose names the dashboard and the deck bar print, and whose answers and notes do not count as work waiting for review. Index-aligned with practice_reviewer_display_names — the backend refuses to start if the two lists differ in length.',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_reviewer_usernames';

UPDATE app_settings
SET meaning       = 'The shown reviewers'' names as screens print them, comma-separated, in the SAME ORDER as practice_reviewer_usernames. {reviewer} in the review pill and bar prints them joined by practice_review_name_joiner; the owner chip prints the same line. Display only: removing a name here removes it from the war room and takes nothing away from an administrator''s ability to review.',
    updated_at    = NOW(),
    updated_by    = 'migration'
WHERE key           = 'practice_reviewer_display_names';

-- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────────
-- An UPDATE whose WHERE matched nothing is silent in Postgres, and the old text
-- would go on being served under a build that no longer means it.
DO $$
DECLARE
    corrected INTEGER;
BEGIN
    SELECT count(*) INTO corrected
      FROM app_settings
     WHERE (key = 'practice_reviewer_usernames'
            AND meaning LIKE 'The signed-in usernames (Authentik) of the reviewers SHOWN ON THE WAR ROOM%')
        OR (key = 'practice_reviewer_display_names'
            AND meaning LIKE 'The shown reviewers'' names as screens print them%');
    IF corrected <> 2 THEN
        RAISE EXCEPTION
            'review permission: expected 2 reviewer rows to carry the display-only '
            'meaning, found %', corrected;
    END IF;
END $$;
