-- retire_the_discuss_dock_settings: remove the "Discuss with AI" dock's rows
--
-- Created: 2026-09-24 15:47:36
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_MODEL_JOBS_PANEL_v1, migration M3 (ruling Q1, 2026-09-24: retire the
-- dock — route, code, practice_discuss_* rows; practice_discussions kept).
-- =============================================================================
--
-- ## Why
--
-- The dock ("Discuss with AI", CC_TASK_QUESTION_CHAT_v1) was replaced by the
-- question chat (CC_TASK_CHAT_ENGINE_v1). No screen reached it any more, but its
-- route still answered and `practice_discuss_default_model` still named a model:
-- a setting that changed nothing a person could see, and a paid model route no
-- page used. The route and its code are removed in the same commit; these are
-- its 25 rows — five parameters (default model, turn cap, prompt file, token cap,
-- thinking dial) and twenty wording rows.
--
-- ## What is KEPT
--
-- `practice_discussions` — the threads people wrote through the dock — is kept
-- whole: the question chat still reads them as context. `app_setting_changes`
-- keeps any history of these keys (none was recorded on PROD or DEV on
-- 2026-09-24): the record of a human decision outlives the row it was about.
--
-- ## Order matters
--
-- The backend that ships with this migration no longer declares these keys. The
-- backend BEFORE it still requires them at boot, so this migration must not run
-- under the old binary — it runs at boot of the new one, which is the only
-- migrator path this project uses.
--
-- ## Fresh-database reproduction
--
-- Earlier migrations seed the rows and this one removes them, so a database built
-- from zero ends in the same state. Re-running deletes nothing further.

DELETE FROM app_settings WHERE key IN (
    'practice_discuss_default_model',
    'practice_discuss_max_turns',
    'practice_discuss_prompt_file',
    'practice_discuss_max_tokens',
    'practice_discuss_effort',
    'practice_discuss_button_label',
    'practice_discuss_title_template',
    'practice_discuss_subtitle_template',
    'practice_discuss_subtitle_draft_template',
    'practice_discuss_context_line',
    'practice_discuss_context_line_draft',
    'practice_discuss_input_placeholder',
    'practice_discuss_send_label',
    'practice_discuss_close_label',
    'practice_discuss_model_label',
    'practice_discuss_footer_template',
    'practice_discuss_cost_billed',
    'practice_discuss_cost_local',
    'practice_discuss_cost_template',
    'practice_discuss_empty',
    'practice_discuss_sending_template',
    'practice_discuss_send_failed',
    'practice_discuss_load_failed',
    'practice_discuss_cap_reached_template',
    'practice_discuss_cap_reached_one'
);

-- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────────
-- No dock row may remain — including one this list forgot, which the LIKE catches
-- (the underscores are escaped so `_` does not match any single character).
DO $$
DECLARE
    remaining INTEGER;
BEGIN
    SELECT count(*) INTO remaining
      FROM app_settings
     WHERE key LIKE 'practice\_discuss\_%';
    IF remaining <> 0 THEN
        RAISE EXCEPTION
            'dock retirement: % practice_discuss_* rows remain in app_settings. '
            'The backend no longer reads them; a leftover is a dead Settings row.', remaining;
    END IF;
END $$;
