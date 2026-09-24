-- ai_job_columns_on_app_settings: which AI job each setting belongs to
--
-- Created: 2026-09-24 14:51:50
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_MODEL_JOBS_PANEL_v1 (Stage A), mockup of record
-- MODEL_JOBS_OVERVIEW_MOCKUP_v3_2026-09-24, board 1.
-- =============================================================================
--
-- ## Why two columns and not a list in the code
--
-- Admin → Overview shows "which model and which instructions each AI job uses".
-- Which settings make up a job is a fact about THIS store, so it lives beside the
-- rows it describes. The panel is built by reading every row whose `ai_job` is
-- set; a future job seeded with `ai_job` appears on the panel with no code
-- change (Law 22: no job may stay configurable only in Settings).
--
--   ai_job  — the job the row belongs to, a stable token (e.g. 'answer_analysis').
--             NULL = not a job setting, which is every other row.
--   ai_role — what the row chooses for that job: 'model', 'instructions' or
--             'effort'. NULL exactly when ai_job is NULL.
--
-- Free TEXT rather than a CHECK, for the reason `value_kind` gives in
-- 20260801225147: the vocabulary lives in code (`services::ai_jobs`), which
-- refuses an unknown role by name, and a CHECK would have to migrate in lockstep.
--
-- ## Fresh-database reproduction
--
-- `ADD COLUMN IF NOT EXISTS` and plain UPDATEs keyed by setting key: a re-run
-- changes nothing, and a database built from zero gets the same nine rows once
-- the migrations that seed those keys have run (all of them precede this file).

ALTER TABLE app_settings
    ADD COLUMN IF NOT EXISTS ai_job  TEXT,
    ADD COLUMN IF NOT EXISTS ai_role TEXT;

COMMENT ON COLUMN app_settings.ai_job IS
    'The AI job this setting belongs to (Admin → Overview panel). NULL = not a job setting.';
COMMENT ON COLUMN app_settings.ai_role IS
    'What the setting chooses for its job: model | instructions | effort. NULL exactly when ai_job is NULL.';

UPDATE app_settings SET ai_job = 'discuss_chat',    ai_role = 'model'        WHERE key = 'question_chat_model';
UPDATE app_settings SET ai_job = 'discuss_chat',    ai_role = 'instructions' WHERE key = 'question_chat_prompt_file';
UPDATE app_settings SET ai_job = 'discuss_effort',  ai_role = 'effort'       WHERE key = 'question_chat_effort';
UPDATE app_settings SET ai_job = 'case_story',      ai_role = 'instructions' WHERE key = 'chat_case_narrative_file';
UPDATE app_settings SET ai_job = 'answer_analysis', ai_role = 'model'        WHERE key = 'practice_read_model';
UPDATE app_settings SET ai_job = 'answer_analysis', ai_role = 'instructions' WHERE key = 'practice_read_prompt_file';
UPDATE app_settings SET ai_job = 'chat_page',       ai_role = 'model'        WHERE key = 'chat_default_model';
UPDATE app_settings SET ai_job = 'theme_scan',      ai_role = 'model'        WHERE key = 'theme_scan_default_model';
UPDATE app_settings SET ai_job = 'theme_scan',      ai_role = 'instructions' WHERE key = 'theme_scan_prompt_file';

-- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────────
-- An UPDATE that matches zero rows is silent in Postgres. Assert the exact nine
-- (key, job, role) triples, and that no other row carries a job.
DO $$
DECLARE
    matched INTEGER;
    tagged  INTEGER;
BEGIN
    SELECT count(*) INTO matched
      FROM app_settings
     WHERE (key, ai_job, ai_role) IN (
        ('question_chat_model',       'discuss_chat',    'model'),
        ('question_chat_prompt_file', 'discuss_chat',    'instructions'),
        ('question_chat_effort',      'discuss_effort',  'effort'),
        ('chat_case_narrative_file',  'case_story',      'instructions'),
        ('practice_read_model',       'answer_analysis', 'model'),
        ('practice_read_prompt_file', 'answer_analysis', 'instructions'),
        ('chat_default_model',        'chat_page',       'model'),
        ('theme_scan_default_model',  'theme_scan',      'model'),
        ('theme_scan_prompt_file',    'theme_scan',      'instructions'));
    SELECT count(*) INTO tagged FROM app_settings WHERE ai_job IS NOT NULL;
    IF matched <> 9 OR tagged <> 9 THEN
        RAISE EXCEPTION
            'ai jobs: expected exactly 9 job settings, matched % and tagged %. '
            'A key named here is missing from app_settings, or another row carries a job.',
            matched, tagged;
    END IF;
END $$;
