-- ai_jobs_panel_and_last_run_wording: the words of the jobs panel and of Admin → Data
--
-- Created: 2026-09-24 14:51:51
-- Target: pipeline database
--
-- =============================================================================
-- CC_TASK_MODEL_JOBS_PANEL_v1 (Stage A), mockup of record
-- MODEL_JOBS_OVERVIEW_MOCKUP_v3_2026-09-24, boards 1, 2, 4 and 5.
-- =============================================================================
--
-- ## Why stored rows
--
-- Every sentence on the two surfaces is read from here, so Roman rewords them
-- from the Settings page without a build (Standing Rule 2). The backend composes
-- the finished sentences (Rule 12) and the browser only draws them.
--
-- ## The three blocks
--
--   ai_jobs_*   the panel itself: headings, labels, list footers, markers,
--               the thinking levels and the worst-state warnings.
--   ai_job_*    each job's name, description, how a warning names it, and
--               its cost line.
--   last_run_*  Admin → Data → Last document run, including one plain name per
--               pipeline step in the order the steps run.
--
-- Placeholders ({model}, {who} …) are filled by the backend's `render`, which
-- leaves an unknown one visible rather than silently blank.
--
-- ## Fresh-database reproduction
--
-- Every key is new, so this migration is its only source. Idempotent: ON
-- CONFLICT (key) DO NOTHING, so a re-run changes nothing and a row somebody has
-- reworded is never clobbered.
--
-- GENERATED from one list (the scratchpad generator, per the repo's practice of
-- never typing a wording block by hand four times); the Rust blocks and their
-- test fixtures come from the same list, and a test pins each to this file.

INSERT INTO app_settings
    (key, value, value_kind, default_value, min_value, max_value, meaning,
     consumed_by, updated_at, updated_by)
VALUES
    ('ai_jobs_title', 'Which model and which instructions each AI job uses', 'text', 'Which model and which instructions each AI job uses', NULL, NULL,
     'The heading of the jobs panel on Admin → Overview.',
     NULL, now(), 'migration'),

    ('ai_jobs_intro', 'A change takes effect on the next use. No rebuild, no redeploy, no restart.', 'text', 'A change takes effect on the next use. No rebuild, no redeploy, no restart.', NULL, NULL,
     'The line under the jobs panel heading.',
     NULL, now(), 'migration'),

    ('ai_jobs_model_label', 'Model', 'text', 'Model', NULL, NULL,
     'The small label over each job''s model control.',
     NULL, now(), 'migration'),

    ('ai_jobs_instructions_label', 'Instructions', 'text', 'Instructions', NULL, NULL,
     'The small label over each job''s instructions control.',
     NULL, now(), 'migration'),

    ('ai_jobs_effort_label', 'Thinking', 'text', 'Thinking', NULL, NULL,
     'The small label over the thinking-level control.',
     NULL, now(), 'migration'),

    ('ai_jobs_save_label', 'Save', 'text', 'Save', NULL, NULL,
     'The button that saves one job''s choice.',
     NULL, now(), 'migration'),

    ('ai_jobs_empty_control', '—', 'text', '—', NULL, NULL,
     'Shown where a job has no control of that kind (no model, or no instructions).',
     NULL, now(), 'migration'),

    ('ai_jobs_built_in', 'Built in, not changeable here', 'text', 'Built in, not changeable here', NULL, NULL,
     'Shown where a job''s instructions are part of the program and cannot be picked.',
     NULL, now(), 'migration'),

    ('ai_jobs_fixed_by_server', 'Fixed by the server: {model}', 'text', 'Fixed by the server: {model}', NULL, NULL,
     'Shown instead of a dropdown when the server''s configuration fixes a job''s model. {model} is the model''s name.',
     NULL, now(), 'migration'),

    ('ai_jobs_fixed_meta', 'Set in the server''s configuration. Change it there, or remove that line to choose here.', 'text', 'Set in the server''s configuration. Change it there, or remove that line to choose here.', NULL, NULL,
     'The line under a job whose model the server''s configuration fixes.',
     NULL, now(), 'migration'),

    ('ai_jobs_meta_install', 'Set by the install, never changed', 'text', 'Set by the install, never changed', NULL, NULL,
     'The line under a job nobody has changed in the app.',
     NULL, now(), 'migration'),

    ('ai_jobs_meta_changed', 'Changed by {who}, {when}', 'text', 'Changed by {who}, {when}', NULL, NULL,
     'The line under a one-control job someone changed. {who} and {when} come from the store.',
     NULL, now(), 'migration'),

    ('ai_jobs_meta_model_changed', 'Model changed by {who}, {when}', 'text', 'Model changed by {who}, {when}', NULL, NULL,
     'The line under a job whose model was the last thing changed.',
     NULL, now(), 'migration'),

    ('ai_jobs_meta_instructions_changed', 'Instructions changed by {who}, {when}', 'text', 'Instructions changed by {who}, {when}', NULL, NULL,
     'The line under a job whose instructions were the last thing changed.',
     NULL, now(), 'migration'),

    ('ai_jobs_foot_grounded', 'Not listed: models that are switched off or have no "Quotes checked" tick.', 'text', 'Not listed: models that are switched off or have no "Quotes checked" tick.', NULL, NULL,
     'The last line of a model list that offers only active models with the Quotes checked tick.',
     NULL, now(), 'migration'),

    ('ai_jobs_foot_active', 'Not listed: models that are switched off.', 'text', 'Not listed: models that are switched off.', NULL, NULL,
     'The last line of a model list that offers every active model.',
     NULL, now(), 'migration'),

    ('ai_jobs_foot_anthropic', 'Not listed: models that are switched off, and local models, which the Chat page cannot call.', 'text', 'Not listed: models that are switched off, and local models, which the Chat page cannot call.', NULL, NULL,
     'The last line of the Chat page''s model list.',
     NULL, now(), 'migration'),

    ('ai_jobs_foot_scan', 'Not listed: models that are switched off or not offered for scans.', 'text', 'Not listed: models that are switched off or not offered for scans.', NULL, NULL,
     'The last line of the theme scan''s model list.',
     NULL, now(), 'migration'),

    ('ai_jobs_foot_files', 'New versions of these files arrive with a release. A version in use can''t be edited or deleted.', 'text', 'New versions of these files arrive with a release. A version in use can''t be edited or deleted.', NULL, NULL,
     'The last line of every instructions list.',
     NULL, now(), 'migration'),

    ('ai_jobs_marker_in_use', 'in use', 'text', 'in use', NULL, NULL,
     'Beside the instructions file a job uses now.',
     NULL, now(), 'migration'),

    ('ai_jobs_marker_used_before', 'used before', 'text', 'used before', NULL, NULL,
     'Beside a file the change history shows this job used before.',
     NULL, now(), 'migration'),

    ('ai_jobs_marker_new', 'new, never used', 'text', 'new, never used', NULL, NULL,
     'Beside a newer version than the one in use that the change history has never recorded.',
     NULL, now(), 'migration'),

    ('ai_jobs_marker_earlier', 'earlier version', 'text', 'earlier version', NULL, NULL,
     'Beside an older version than the one in use that the change history has not recorded (the install may have used it).',
     NULL, now(), 'migration'),

    ('ai_jobs_effort_low', 'Low', 'text', 'Low', NULL, NULL,
     'Thinking level low.',
     NULL, now(), 'migration'),

    ('ai_jobs_effort_medium', 'Medium', 'text', 'Medium', NULL, NULL,
     'Thinking level medium.',
     NULL, now(), 'migration'),

    ('ai_jobs_effort_high', 'High', 'text', 'High', NULL, NULL,
     'Thinking level high.',
     NULL, now(), 'migration'),

    ('ai_jobs_effort_xhigh', 'Extra high', 'text', 'Extra high', NULL, NULL,
     'Thinking level xhigh.',
     NULL, now(), 'migration'),

    ('ai_jobs_effort_max', 'Max', 'text', 'Max', NULL, NULL,
     'Thinking level max.',
     NULL, now(), 'migration'),

    ('ai_jobs_effort_unset', 'The model''s own default', 'text', 'The model''s own default', NULL, NULL,
     'Thinking level when none is set.',
     NULL, now(), 'migration'),

    ('ai_jobs_warn_model_off', '{model} is switched off on Prompt Management → Models, so {job} can''t run. Pick another model.', 'text', '{model} is switched off on Prompt Management → Models, so {job} can''t run. Pick another model.', NULL, NULL,
     'Shown under a job whose model is switched off.',
     NULL, now(), 'migration'),

    ('ai_jobs_warn_model_missing', '{model} is not in the list on Prompt Management → Models, so {job} can''t run. Pick another model.', 'text', '{model} is not in the list on Prompt Management → Models, so {job} can''t run. Pick another model.', NULL, NULL,
     'Shown under a job whose model is not in the models list at all.',
     NULL, now(), 'migration'),

    ('ai_jobs_warn_model_unquoted', '{model} has no "Quotes checked" tick on Prompt Management → Models, so {job} can''t run. Pick another model.', 'text', '{model} has no "Quotes checked" tick on Prompt Management → Models, so {job} can''t run. Pick another model.', NULL, NULL,
     'Shown under the Discuss chat when its model lost the Quotes checked tick.',
     NULL, now(), 'migration'),

    ('ai_jobs_warn_model_local', '{model} is a local model, which {job} cannot call. Pick another model.', 'text', '{model} is a local model, which {job} cannot call. Pick another model.', NULL, NULL,
     'Shown under the Chat page when its model is a local one.',
     NULL, now(), 'migration'),

    ('ai_jobs_warn_model_not_scan', '{model} is not offered for scans on Prompt Management → Models, so {job} can''t start on it. Pick another model.', 'text', '{model} is not offered for scans on Prompt Management → Models, so {job} can''t start on it. Pick another model.', NULL, NULL,
     'Shown under the theme scan when its model is not offered for scans.',
     NULL, now(), 'migration'),

    ('ai_jobs_warn_file_missing', '{file} is not in the instructions folder, so {job} can''t run. Pick another version.', 'text', '{file} is not in the instructions folder, so {job} can''t run. Pick another version.', NULL, NULL,
     'Shown under a job whose instructions file is missing.',
     NULL, now(), 'migration'),

    ('ai_jobs_saved', '{job} now uses {value}, from the next use.', 'text', '{job} now uses {value}, from the next use.', NULL, NULL,
     'The confirmation after Save. {job} is the job''s name; {value} what it now uses.',
     NULL, now(), 'migration'),

    ('ai_jobs_saved_reload', 'The next question reloads the case file (about {cost}).', 'text', 'The next question reloads the case file (about {cost}).', NULL, NULL,
     'Added to the confirmation when the change reloads the case file. {cost} is the measured price.',
     NULL, now(), 'migration'),

    ('ai_jobs_saved_reload_unpriced', 'The next question reloads the case file.', 'text', 'The next question reloads the case file.', NULL, NULL,
     'The same, when the price is not known.',
     NULL, now(), 'migration'),

    ('ai_jobs_was', 'was {value}', 'text', 'was {value}', NULL, NULL,
     'In the meta line after a change: what the job used before.',
     NULL, now(), 'migration'),

    ('ai_jobs_switch_back', 'Switch back', 'text', 'Switch back', NULL, NULL,
     'The one-click control that restores what a job used before its last change.',
     NULL, now(), 'migration'),

    ('ai_jobs_refuse_fixed', '{job} takes its model from the server''s configuration. Change it there, or remove that line to choose here.', 'text', '{job} takes its model from the server''s configuration. Change it there, or remove that line to choose here.', NULL, NULL,
     'Refusal: a Save aimed at a model the server''s configuration fixes.',
     NULL, now(), 'migration'),

    ('ai_jobs_refuse_nothing', 'Nothing changed: {job} already uses that.', 'text', 'Nothing changed: {job} already uses that.', NULL, NULL,
     'Refusal: Save with nothing different.',
     NULL, now(), 'migration'),

    ('ai_jobs_refuse_no_history', 'There is nothing to switch back to: {job} has not been changed here.', 'text', 'There is nothing to switch back to: {job} has not been changed here.', NULL, NULL,
     'Refusal: Switch back on a job the change history has never recorded.',
     NULL, now(), 'migration'),

    ('ai_jobs_refuse_not_job', '{setting} is not one of {job}''s settings.', 'text', '{setting} is not one of {job}''s settings.', NULL, NULL,
     'Refusal: a Save naming a setting that belongs to another job.',
     NULL, now(), 'migration'),

    ('ai_jobs_refuse_partial', '{saved} was saved; the rest was not: {reason}', 'text', '{saved} was saved; the rest was not: {reason}', NULL, NULL,
     'Refusal after part of a two-setting Save went through. {saved} names what was saved.',
     NULL, now(), 'migration'),

    ('ai_jobs_refuse_model_in_use', '{model} is used by {jobs}. Pick another model for that job on Admin → Overview first.', 'text', '{model} is used by {jobs}. Pick another model for that job on Admin → Overview first.', NULL, NULL,
     'Refusal on Prompt Management → Models: switching off or deleting a model a job uses.',
     NULL, now(), 'migration'),

    ('ai_jobs_refuse_file_in_use', '{file} is used by {jobs}, so it can''t be changed or deleted.', 'text', '{file} is used by {jobs}, so it can''t be changed or deleted.', NULL, NULL,
     'Refusal on Prompt Management → Prompts: a file a job uses.',
     NULL, now(), 'migration'),

    ('ai_jobs_refuse_files_read_only', 'Instruction files can''t be changed here. New versions of these files arrive with a release.', 'text', 'Instruction files can''t be changed here. New versions of these files arrive with a release.', NULL, NULL,
     'Refusal on Prompt Management → Prompts: any other new, edit or delete (the folder is read-only).',
     NULL, now(), 'migration'),

    ('ai_jobs_files_note', 'New versions of these files arrive with a release.', 'text', 'New versions of these files arrive with a release.', NULL, NULL,
     'The line on Prompt Management → Prompts, which is read-only.',
     NULL, now(), 'migration'),

    ('ai_jobs_used_for_label', 'Used for', 'text', 'Used for', NULL, NULL,
     'The column on Prompt Management → Models and → Prompts that names the jobs using a row.',
     NULL, now(), 'migration'),

    ('ai_jobs_settings_pointer', 'Change this on Admin → Overview → Which model and which instructions each AI job uses.', 'text', 'Change this on Admin → Overview → Which model and which instructions each AI job uses.', NULL, NULL,
     'Shown on the Settings page under a row the Overview panel owns, which is read-only there.',
     NULL, now(), 'migration'),

    ('ai_jobs_others_title', 'Other settings that name a model', 'text', 'Other settings that name a model', NULL, NULL,
     'Heading over settings that name a model but belong to no job yet, so none can hide.',
     NULL, now(), 'migration'),

    ('ai_jobs_others_line', '{setting} names {model}. It is not on this panel yet.', 'text', '{setting} names {model}. It is not on this panel yet.', NULL, NULL,
     'One such setting. {setting} is its meaning line; {model} the model it names.',
     NULL, now(), 'migration'),

    ('ai_jobs_link_models_lead', 'All models:', 'text', 'All models:', NULL, NULL,
     'Words before the link to the models list.',
     NULL, now(), 'migration'),

    ('ai_jobs_link_models', 'Prompt Management → Models', 'text', 'Prompt Management → Models', NULL, NULL,
     'The link to the models list.',
     NULL, now(), 'migration'),

    ('ai_jobs_link_files_lead', 'All instruction files:', 'text', 'All instruction files:', NULL, NULL,
     'Words before the link to the instructions files.',
     NULL, now(), 'migration'),

    ('ai_jobs_link_files', 'Prompt Management → Prompts', 'text', 'Prompt Management → Prompts', NULL, NULL,
     'The link to the instructions files.',
     NULL, now(), 'migration'),

    ('ai_jobs_link_data_lead', 'Document loading numbers:', 'text', 'Document loading numbers:', NULL, NULL,
     'Words before the link to the document numbers.',
     NULL, now(), 'migration'),

    ('ai_jobs_link_data', 'Admin → Data', 'text', 'Admin → Data', NULL, NULL,
     'The link to the document numbers.',
     NULL, now(), 'migration'),

    ('ai_job_discuss_chat_title', 'Discuss chat on an answer', 'text', 'Discuss chat on an answer', NULL, NULL,
     'The Discuss chat job''s name.',
     NULL, now(), 'migration'),

    ('ai_job_discuss_chat_desc', 'Reads the whole record and quotes it.', 'text', 'Reads the whole record and quotes it.', NULL, NULL,
     'The Discuss chat job''s one-line description.',
     NULL, now(), 'migration'),

    ('ai_job_discuss_chat_subject', 'the Discuss chat', 'text', 'the Discuss chat', NULL, NULL,
     'How a warning names the Discuss chat job.',
     NULL, now(), 'migration'),

    ('ai_job_discuss_chat_note', 'Changing the model or the instructions reloads the case file on the next question (about {cost}).', 'text', 'Changing the model or the instructions reloads the case file on the next question (about {cost}).', NULL, NULL,
     'The Discuss chat''s cost line. {cost} is the measured reload price.',
     NULL, now(), 'migration'),

    ('ai_job_discuss_chat_note_unpriced', 'Changing the model or the instructions reloads the case file on the next question.', 'text', 'Changing the model or the instructions reloads the case file on the next question.', NULL, NULL,
     'The Discuss chat''s cost line when the reload price is not known.',
     NULL, now(), 'migration'),

    ('ai_job_discuss_effort_title', 'How hard the Discuss chat thinks', 'text', 'How hard the Discuss chat thinks', NULL, NULL,
     'The thinking-level job''s name.',
     NULL, now(), 'migration'),

    ('ai_job_discuss_effort_desc', 'Medium answers sooner; High thinks longer.', 'text', 'Medium answers sooner; High thinks longer.', NULL, NULL,
     'The thinking-level job''s description.',
     NULL, now(), 'migration'),

    ('ai_job_discuss_effort_subject', 'the Discuss chat', 'text', 'the Discuss chat', NULL, NULL,
     'How a warning names the thinking-level job.',
     NULL, now(), 'migration'),

    ('ai_job_discuss_effort_note', 'No reload.', 'text', 'No reload.', NULL, NULL,
     'The thinking-level job''s cost line: changing it does not reload the case file.',
     NULL, now(), 'migration'),

    ('ai_job_case_story_title', 'Case story given to every chat', 'text', 'Case story given to every chat', NULL, NULL,
     'The case story job''s name.',
     NULL, now(), 'migration'),

    ('ai_job_case_story_desc', 'Parties, timeline, claims, who is who.', 'text', 'Parties, timeline, claims, who is who.', NULL, NULL,
     'The case story job''s description.',
     NULL, now(), 'migration'),

    ('ai_job_case_story_subject', 'the Discuss chat', 'text', 'the Discuss chat', NULL, NULL,
     'How a warning names the case story job (a missing story stops the Discuss chat).',
     NULL, now(), 'migration'),

    ('ai_job_case_story_note', 'Changing it reloads the case file (about {cost}).', 'text', 'Changing it reloads the case file (about {cost}).', NULL, NULL,
     'The case story''s cost line. {cost} is the measured reload price.',
     NULL, now(), 'migration'),

    ('ai_job_case_story_note_unpriced', 'Changing it reloads the case file.', 'text', 'Changing it reloads the case file.', NULL, NULL,
     'The case story''s cost line when the reload price is not known.',
     NULL, now(), 'migration'),

    ('ai_job_answer_analysis_title', 'Answer analysis', 'text', 'Answer analysis', NULL, NULL,
     'The answer analysis job''s name.',
     NULL, now(), 'migration'),

    ('ai_job_answer_analysis_desc', 'The verdict shown under each practice answer.', 'text', 'The verdict shown under each practice answer.', NULL, NULL,
     'The answer analysis job''s description.',
     NULL, now(), 'migration'),

    ('ai_job_answer_analysis_subject', 'the answer analysis', 'text', 'the answer analysis', NULL, NULL,
     'How a warning names the answer analysis job.',
     NULL, now(), 'migration'),

    ('ai_job_chat_page_title', 'Chat page', 'text', 'Chat page', NULL, NULL,
     'The Chat page job''s name.',
     NULL, now(), 'migration'),

    ('ai_job_chat_page_desc', 'The model the Chat page in the top menu starts on. Anyone can switch it there.', 'text', 'The model the Chat page in the top menu starts on. Anyone can switch it there.', NULL, NULL,
     'The Chat page job''s description.',
     NULL, now(), 'migration'),

    ('ai_job_chat_page_subject', 'the Chat page', 'text', 'the Chat page', NULL, NULL,
     'How a warning names the Chat page job.',
     NULL, now(), 'migration'),

    ('ai_job_theme_scan_title', 'Theme scan', 'text', 'Theme scan', NULL, NULL,
     'The theme scan job''s name.',
     NULL, now(), 'migration'),

    ('ai_job_theme_scan_desc', 'The model and instructions the scan screen starts with.', 'text', 'The model and instructions the scan screen starts with.', NULL, NULL,
     'The theme scan job''s description.',
     NULL, now(), 'migration'),

    ('ai_job_theme_scan_subject', 'the theme scan', 'text', 'the theme scan', NULL, NULL,
     'How a warning names the theme scan job.',
     NULL, now(), 'migration'),

    ('last_run_finished_value', '{done} of {total}', 'text', '{done} of {total}', NULL, NULL,
     'The finished-documents number. {done} and {total} are counts.',
     NULL, now(), 'migration'),

    ('last_run_finished_label', 'documents finished', 'text', 'documents finished', NULL, NULL,
     'Under the finished-documents number.',
     NULL, now(), 'migration'),

    ('last_run_quotes_label', 'of quotes found in their source', 'text', 'of quotes found in their source', NULL, NULL,
     'Under the quote-check percentage.',
     NULL, now(), 'migration'),

    ('last_run_quotes_none', '—', 'text', '—', NULL, NULL,
     'Shown instead of a percentage when no quote check has run.',
     NULL, now(), 'migration'),

    ('last_run_when_label', 'last run, {clock}', 'text', 'last run, {clock}', NULL, NULL,
     'Under the last run''s day. {clock} is its time of day.',
     NULL, now(), 'migration'),

    ('last_run_idle', 'No document run in progress. Time estimates appear here while one is running.', 'text', 'No document run in progress. Time estimates appear here while one is running.', NULL, NULL,
     'Shown when no document step is running.',
     NULL, now(), 'migration'),

    ('last_run_running', 'A document run is in progress: {count} documents left, about {time} to go.', 'text', 'A document run is in progress: {count} documents left, about {time} to go.', NULL, NULL,
     'Shown while a document step is running.',
     NULL, now(), 'migration'),

    ('last_run_running_no_estimate', 'A document run is in progress: {count} documents left. No finished document yet to time it by.', 'text', 'A document run is in progress: {count} documents left. No finished document yet to time it by.', NULL, NULL,
     'Shown while a run is in progress but nothing has finished to estimate from.',
     NULL, now(), 'migration'),

    ('last_run_col_step', 'Step, in order', 'text', 'Step, in order', NULL, NULL,
     'Step table column.',
     NULL, now(), 'migration'),

    ('last_run_col_avg', 'Avg time', 'text', 'Avg time', NULL, NULL,
     'Step table column.',
     NULL, now(), 'migration'),

    ('last_run_col_runs', 'Runs', 'text', 'Runs', NULL, NULL,
     'Step table column.',
     NULL, now(), 'migration'),

    ('last_run_col_failed', 'Failed', 'text', 'Failed', NULL, NULL,
     'Step table column.',
     NULL, now(), 'migration'),

    ('last_run_summary', '{runs} step runs, {failed} failed ({detail}).', 'text', '{runs} step runs, {failed} failed ({detail}).', NULL, NULL,
     'Under the step table when a step failed. {detail} lists which and when.',
     NULL, now(), 'migration'),

    ('last_run_summary_clean', '{runs} step runs, none failed.', 'text', '{runs} step runs, none failed.', NULL, NULL,
     'Under the step table when nothing failed.',
     NULL, now(), 'migration'),

    ('last_run_failure_detail', '{step}, {day}', 'text', '{step}, {day}', NULL, NULL,
     'One failed step in the summary: its name and the day it last failed.',
     NULL, now(), 'migration'),

    ('last_run_unknown_step', 'This page has no plain name for the step "{step}", so it is shown by its code name.', 'text', 'This page has no plain name for the step "{step}", so it is shown by its code name.', NULL, NULL,
     'Shown when the store holds a step this page cannot name.',
     NULL, now(), 'migration'),

    ('last_run_empty', 'No document has been run yet.', 'text', 'No document has been run yet.', NULL, NULL,
     'Shown when the store holds no documents.',
     NULL, now(), 'migration'),

    ('last_run_step_upload', 'Upload', 'text', 'Upload', NULL, NULL,
     'Step name, in running order.',
     NULL, now(), 'migration'),

    ('last_run_step_extract_text', 'Read the text', 'text', 'Read the text', NULL, NULL,
     'Step name, in running order.',
     NULL, now(), 'migration'),

    ('last_run_step_llm_extract_pass1', 'First extraction pass', 'text', 'First extraction pass', NULL, NULL,
     'Step name, in running order.',
     NULL, now(), 'migration'),

    ('last_run_step_llm_extract_pass2', 'Second extraction pass', 'text', 'Second extraction pass', NULL, NULL,
     'Step name, in running order.',
     NULL, now(), 'migration'),

    ('last_run_step_verify', 'Check the quotes', 'text', 'Check the quotes', NULL, NULL,
     'Step name, in running order.',
     NULL, now(), 'migration'),

    ('last_run_step_auto_approve', 'Approve automatically', 'text', 'Approve automatically', NULL, NULL,
     'Step name, in running order.',
     NULL, now(), 'migration'),

    ('last_run_step_ingest', 'Load into the case graph', 'text', 'Load into the case graph', NULL, NULL,
     'Step name, in running order.',
     NULL, now(), 'migration'),

    ('last_run_step_ingest_delta', 'Graph update', 'text', 'Graph update', NULL, NULL,
     'Step name, in running order.',
     NULL, now(), 'migration'),

    ('last_run_step_index', 'Search index', 'text', 'Search index', NULL, NULL,
     'Step name, in running order.',
     NULL, now(), 'migration'),

    ('last_run_step_completeness', 'Completeness check', 'text', 'Completeness check', NULL, NULL,
     'Step name, in running order.',
     NULL, now(), 'migration')
ON CONFLICT (key) DO NOTHING;

-- ─── The END-state assertion (CLAUDE.md rule 25a) ────────────────────────────
-- Every key present AND non-blank: the backend declares each one at boot and
-- refuses to start without it.
DO $$
DECLARE
    present INTEGER;
BEGIN
    SELECT count(*) INTO present
      FROM app_settings
     WHERE key IN ('ai_jobs_title',
                   'ai_jobs_intro',
                   'ai_jobs_model_label',
                   'ai_jobs_instructions_label',
                   'ai_jobs_effort_label',
                   'ai_jobs_save_label',
                   'ai_jobs_empty_control',
                   'ai_jobs_built_in',
                   'ai_jobs_fixed_by_server',
                   'ai_jobs_fixed_meta',
                   'ai_jobs_meta_install',
                   'ai_jobs_meta_changed',
                   'ai_jobs_meta_model_changed',
                   'ai_jobs_meta_instructions_changed',
                   'ai_jobs_foot_grounded',
                   'ai_jobs_foot_active',
                   'ai_jobs_foot_anthropic',
                   'ai_jobs_foot_scan',
                   'ai_jobs_foot_files',
                   'ai_jobs_marker_in_use',
                   'ai_jobs_marker_used_before',
                   'ai_jobs_marker_new',
                   'ai_jobs_marker_earlier',
                   'ai_jobs_effort_low',
                   'ai_jobs_effort_medium',
                   'ai_jobs_effort_high',
                   'ai_jobs_effort_xhigh',
                   'ai_jobs_effort_max',
                   'ai_jobs_effort_unset',
                   'ai_jobs_warn_model_off',
                   'ai_jobs_warn_model_missing',
                   'ai_jobs_warn_model_unquoted',
                   'ai_jobs_warn_model_local',
                   'ai_jobs_warn_model_not_scan',
                   'ai_jobs_warn_file_missing',
                   'ai_jobs_saved',
                   'ai_jobs_saved_reload',
                   'ai_jobs_saved_reload_unpriced',
                   'ai_jobs_was',
                   'ai_jobs_switch_back',
                   'ai_jobs_refuse_fixed',
                   'ai_jobs_refuse_nothing',
                   'ai_jobs_refuse_no_history',
                   'ai_jobs_refuse_not_job',
                   'ai_jobs_refuse_partial',
                   'ai_jobs_refuse_model_in_use',
                   'ai_jobs_refuse_file_in_use',
                   'ai_jobs_refuse_files_read_only',
                   'ai_jobs_files_note',
                   'ai_jobs_used_for_label',
                   'ai_jobs_settings_pointer',
                   'ai_jobs_others_title',
                   'ai_jobs_others_line',
                   'ai_jobs_link_models_lead',
                   'ai_jobs_link_models',
                   'ai_jobs_link_files_lead',
                   'ai_jobs_link_files',
                   'ai_jobs_link_data_lead',
                   'ai_jobs_link_data',
                   'ai_job_discuss_chat_title',
                   'ai_job_discuss_chat_desc',
                   'ai_job_discuss_chat_subject',
                   'ai_job_discuss_chat_note',
                   'ai_job_discuss_chat_note_unpriced',
                   'ai_job_discuss_effort_title',
                   'ai_job_discuss_effort_desc',
                   'ai_job_discuss_effort_subject',
                   'ai_job_discuss_effort_note',
                   'ai_job_case_story_title',
                   'ai_job_case_story_desc',
                   'ai_job_case_story_subject',
                   'ai_job_case_story_note',
                   'ai_job_case_story_note_unpriced',
                   'ai_job_answer_analysis_title',
                   'ai_job_answer_analysis_desc',
                   'ai_job_answer_analysis_subject',
                   'ai_job_chat_page_title',
                   'ai_job_chat_page_desc',
                   'ai_job_chat_page_subject',
                   'ai_job_theme_scan_title',
                   'ai_job_theme_scan_desc',
                   'ai_job_theme_scan_subject',
                   'last_run_finished_value',
                   'last_run_finished_label',
                   'last_run_quotes_label',
                   'last_run_quotes_none',
                   'last_run_when_label',
                   'last_run_idle',
                   'last_run_running',
                   'last_run_running_no_estimate',
                   'last_run_col_step',
                   'last_run_col_avg',
                   'last_run_col_runs',
                   'last_run_col_failed',
                   'last_run_summary',
                   'last_run_summary_clean',
                   'last_run_failure_detail',
                   'last_run_unknown_step',
                   'last_run_empty',
                   'last_run_step_upload',
                   'last_run_step_extract_text',
                   'last_run_step_llm_extract_pass1',
                   'last_run_step_llm_extract_pass2',
                   'last_run_step_verify',
                   'last_run_step_auto_approve',
                   'last_run_step_ingest',
                   'last_run_step_ingest_delta',
                   'last_run_step_index',
                   'last_run_step_completeness')
       AND btrim(value) <> '';
    IF present <> 109 THEN
        RAISE EXCEPTION
            'ai jobs panel wording: expected 109 non-blank rows, found %. '
            'The backend declares every one at boot and refuses to start without it.', present;
    END IF;
END $$;
