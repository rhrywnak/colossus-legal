// =============================================================================
// backend/src/domain/wording_ai_jobs.rs — The words of the jobs panel on Admin → Overview
// =============================================================================
//
// CC_TASK_MODEL_JOBS_PANEL_v1 (Stage A). Every sentence is a stored row, seeded
// by `pipeline_migrations/20260924145151_ai_jobs_panel_and_last_run_wording.sql`, so Roman rewords it from the
// Settings page without a build (Standing Rule 2). The backend fills the
// placeholders with `domain::wording_templates::render` (UNBRACED keys) and the
// browser only draws the finished sentences (Rule 12).
//
// GENERATED from the same list as the migration and the test fixture; a test
// reads the migration off disk and pins all three together (Rule 21).

/// The words of the jobs panel on Admin → Overview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiJobsWording {
    /// The heading of the jobs panel on Admin → Overview.
    pub title: String,
    /// The line under the jobs panel heading.
    pub intro: String,
    /// The small label over each job's model control.
    pub model_label: String,
    /// The small label over each job's instructions control.
    pub instructions_label: String,
    /// The small label over the thinking-level control.
    pub effort_label: String,
    /// The button that saves one job's choice.
    pub save_label: String,
    /// Shown where a job has no control of that kind (no model, or no instructions).
    pub empty_control: String,
    /// Shown where a job's instructions are part of the program and cannot be picked.
    pub built_in: String,
    /// Shown instead of a dropdown when the server's configuration fixes a job's model. {model} is the model's name.
    pub fixed_by_server: String,
    /// The line under a job whose model the server's configuration fixes.
    pub fixed_meta: String,
    /// The line under a job nobody has changed in the app.
    pub meta_install: String,
    /// The line under a one-control job someone changed. {who} and {when} come from the store.
    pub meta_changed: String,
    /// The line under a job whose model was the last thing changed.
    pub meta_model_changed: String,
    /// The line under a job whose instructions were the last thing changed.
    pub meta_instructions_changed: String,
    /// The last line of a model list that offers only active models with the Quotes checked tick.
    pub foot_grounded: String,
    /// The last line of a model list that offers every active model.
    pub foot_active: String,
    /// The last line of the Chat page's model list.
    pub foot_anthropic: String,
    /// The last line of the theme scan's model list.
    pub foot_scan: String,
    /// The last line of every instructions list.
    pub foot_files: String,
    /// Beside the instructions file a job uses now.
    pub marker_in_use: String,
    /// Beside a file the change history shows this job used before.
    pub marker_used_before: String,
    /// Beside a newer version than the one in use that the change history has never recorded.
    pub marker_new: String,
    /// Beside an older version than the one in use that the change history has not recorded (the install may have used it).
    pub marker_earlier: String,
    /// Thinking level low.
    pub effort_low: String,
    /// Thinking level medium.
    pub effort_medium: String,
    /// Thinking level high.
    pub effort_high: String,
    /// Thinking level xhigh.
    pub effort_xhigh: String,
    /// Thinking level max.
    pub effort_max: String,
    /// Thinking level when none is set.
    pub effort_unset: String,
    /// Shown under a job whose model is switched off.
    pub warn_model_off: String,
    /// Shown under a job whose model is not in the models list at all.
    pub warn_model_missing: String,
    /// Shown under the Discuss chat when its model lost the Quotes checked tick.
    pub warn_model_unquoted: String,
    /// Shown under the Chat page when its model is a local one.
    pub warn_model_local: String,
    /// Shown under the theme scan when its model is not offered for scans.
    pub warn_model_not_scan: String,
    /// Shown under a job whose instructions file is missing.
    pub warn_file_missing: String,
    /// The confirmation after Save. {job} is the job's name; {value} what it now uses.
    pub saved: String,
    /// Added to the confirmation when the change reloads the case file. {cost} is the measured price.
    pub saved_reload: String,
    /// The same, when the price is not known.
    pub saved_reload_unpriced: String,
    /// In the meta line after a change: what the job used before.
    pub was: String,
    /// The one-click control that restores what a job used before its last change.
    pub switch_back: String,
    /// Refusal: a Save aimed at a model the server's configuration fixes.
    pub refuse_fixed: String,
    /// Refusal: Save with nothing different.
    pub refuse_nothing: String,
    /// Refusal: Switch back on a job the change history has never recorded.
    pub refuse_no_history: String,
    /// Refusal: a Save naming a setting that belongs to another job.
    pub refuse_not_job: String,
    /// Refusal after part of a two-setting Save went through. {saved} names what was saved.
    pub refuse_partial: String,
    /// Refusal on Prompt Management → Models: switching off or deleting a model a job uses.
    pub refuse_model_in_use: String,
    /// Refusal on Prompt Management → Prompts: a file a job uses.
    pub refuse_file_in_use: String,
    /// Refusal on Prompt Management → Prompts: any other new, edit or delete (the folder is read-only).
    pub refuse_files_read_only: String,
    /// The line on Prompt Management → Prompts, which is read-only.
    pub files_note: String,
    /// The column on Prompt Management → Models and → Prompts that names the jobs using a row.
    pub used_for_label: String,
    /// Shown on the Settings page under a row the Overview panel owns, which is read-only there.
    pub settings_pointer: String,
    /// Heading over settings that name a model but belong to no job yet, so none can hide.
    pub others_title: String,
    /// One such setting. {setting} is its meaning line; {model} the model it names.
    pub others_line: String,
    /// Words before the link to the models list.
    pub link_models_lead: String,
    /// The link to the models list.
    pub link_models: String,
    /// Words before the link to the instructions files.
    pub link_files_lead: String,
    /// The link to the instructions files.
    pub link_files: String,
    /// Words before the link to the document numbers.
    pub link_data_lead: String,
    /// The link to the document numbers.
    pub link_data: String,
}

// KEYS: the stable identifiers. Renaming one is a migration.
pub(crate) const KEY_AI_JOBS_TITLE: &str = "ai_jobs_title";
pub(crate) const KEY_AI_JOBS_INTRO: &str = "ai_jobs_intro";
pub(crate) const KEY_AI_JOBS_MODEL_LABEL: &str = "ai_jobs_model_label";
pub(crate) const KEY_AI_JOBS_INSTRUCTIONS_LABEL: &str = "ai_jobs_instructions_label";
pub(crate) const KEY_AI_JOBS_EFFORT_LABEL: &str = "ai_jobs_effort_label";
pub(crate) const KEY_AI_JOBS_SAVE_LABEL: &str = "ai_jobs_save_label";
pub(crate) const KEY_AI_JOBS_EMPTY_CONTROL: &str = "ai_jobs_empty_control";
pub(crate) const KEY_AI_JOBS_BUILT_IN: &str = "ai_jobs_built_in";
pub(crate) const KEY_AI_JOBS_FIXED_BY_SERVER: &str = "ai_jobs_fixed_by_server";
pub(crate) const KEY_AI_JOBS_FIXED_META: &str = "ai_jobs_fixed_meta";
pub(crate) const KEY_AI_JOBS_META_INSTALL: &str = "ai_jobs_meta_install";
pub(crate) const KEY_AI_JOBS_META_CHANGED: &str = "ai_jobs_meta_changed";
pub(crate) const KEY_AI_JOBS_META_MODEL_CHANGED: &str = "ai_jobs_meta_model_changed";
pub(crate) const KEY_AI_JOBS_META_INSTRUCTIONS_CHANGED: &str = "ai_jobs_meta_instructions_changed";
pub(crate) const KEY_AI_JOBS_FOOT_GROUNDED: &str = "ai_jobs_foot_grounded";
pub(crate) const KEY_AI_JOBS_FOOT_ACTIVE: &str = "ai_jobs_foot_active";
pub(crate) const KEY_AI_JOBS_FOOT_ANTHROPIC: &str = "ai_jobs_foot_anthropic";
pub(crate) const KEY_AI_JOBS_FOOT_SCAN: &str = "ai_jobs_foot_scan";
pub(crate) const KEY_AI_JOBS_FOOT_FILES: &str = "ai_jobs_foot_files";
pub(crate) const KEY_AI_JOBS_MARKER_IN_USE: &str = "ai_jobs_marker_in_use";
pub(crate) const KEY_AI_JOBS_MARKER_USED_BEFORE: &str = "ai_jobs_marker_used_before";
pub(crate) const KEY_AI_JOBS_MARKER_NEW: &str = "ai_jobs_marker_new";
pub(crate) const KEY_AI_JOBS_MARKER_EARLIER: &str = "ai_jobs_marker_earlier";
pub(crate) const KEY_AI_JOBS_EFFORT_LOW: &str = "ai_jobs_effort_low";
pub(crate) const KEY_AI_JOBS_EFFORT_MEDIUM: &str = "ai_jobs_effort_medium";
pub(crate) const KEY_AI_JOBS_EFFORT_HIGH: &str = "ai_jobs_effort_high";
pub(crate) const KEY_AI_JOBS_EFFORT_XHIGH: &str = "ai_jobs_effort_xhigh";
pub(crate) const KEY_AI_JOBS_EFFORT_MAX: &str = "ai_jobs_effort_max";
pub(crate) const KEY_AI_JOBS_EFFORT_UNSET: &str = "ai_jobs_effort_unset";
pub(crate) const KEY_AI_JOBS_WARN_MODEL_OFF: &str = "ai_jobs_warn_model_off";
pub(crate) const KEY_AI_JOBS_WARN_MODEL_MISSING: &str = "ai_jobs_warn_model_missing";
pub(crate) const KEY_AI_JOBS_WARN_MODEL_UNQUOTED: &str = "ai_jobs_warn_model_unquoted";
pub(crate) const KEY_AI_JOBS_WARN_MODEL_LOCAL: &str = "ai_jobs_warn_model_local";
pub(crate) const KEY_AI_JOBS_WARN_MODEL_NOT_SCAN: &str = "ai_jobs_warn_model_not_scan";
pub(crate) const KEY_AI_JOBS_WARN_FILE_MISSING: &str = "ai_jobs_warn_file_missing";
pub(crate) const KEY_AI_JOBS_SAVED: &str = "ai_jobs_saved";
pub(crate) const KEY_AI_JOBS_SAVED_RELOAD: &str = "ai_jobs_saved_reload";
pub(crate) const KEY_AI_JOBS_SAVED_RELOAD_UNPRICED: &str = "ai_jobs_saved_reload_unpriced";
pub(crate) const KEY_AI_JOBS_WAS: &str = "ai_jobs_was";
pub(crate) const KEY_AI_JOBS_SWITCH_BACK: &str = "ai_jobs_switch_back";
pub(crate) const KEY_AI_JOBS_REFUSE_FIXED: &str = "ai_jobs_refuse_fixed";
pub(crate) const KEY_AI_JOBS_REFUSE_NOTHING: &str = "ai_jobs_refuse_nothing";
pub(crate) const KEY_AI_JOBS_REFUSE_NO_HISTORY: &str = "ai_jobs_refuse_no_history";
pub(crate) const KEY_AI_JOBS_REFUSE_NOT_JOB: &str = "ai_jobs_refuse_not_job";
pub(crate) const KEY_AI_JOBS_REFUSE_PARTIAL: &str = "ai_jobs_refuse_partial";
pub(crate) const KEY_AI_JOBS_REFUSE_MODEL_IN_USE: &str = "ai_jobs_refuse_model_in_use";
pub(crate) const KEY_AI_JOBS_REFUSE_FILE_IN_USE: &str = "ai_jobs_refuse_file_in_use";
pub(crate) const KEY_AI_JOBS_REFUSE_FILES_READ_ONLY: &str = "ai_jobs_refuse_files_read_only";
pub(crate) const KEY_AI_JOBS_FILES_NOTE: &str = "ai_jobs_files_note";
pub(crate) const KEY_AI_JOBS_USED_FOR_LABEL: &str = "ai_jobs_used_for_label";
pub(crate) const KEY_AI_JOBS_SETTINGS_POINTER: &str = "ai_jobs_settings_pointer";
pub(crate) const KEY_AI_JOBS_OTHERS_TITLE: &str = "ai_jobs_others_title";
pub(crate) const KEY_AI_JOBS_OTHERS_LINE: &str = "ai_jobs_others_line";
pub(crate) const KEY_AI_JOBS_LINK_MODELS_LEAD: &str = "ai_jobs_link_models_lead";
pub(crate) const KEY_AI_JOBS_LINK_MODELS: &str = "ai_jobs_link_models";
pub(crate) const KEY_AI_JOBS_LINK_FILES_LEAD: &str = "ai_jobs_link_files_lead";
pub(crate) const KEY_AI_JOBS_LINK_FILES: &str = "ai_jobs_link_files";
pub(crate) const KEY_AI_JOBS_LINK_DATA_LEAD: &str = "ai_jobs_link_data_lead";
pub(crate) const KEY_AI_JOBS_LINK_DATA: &str = "ai_jobs_link_data";

/// Every key this block reads, so a missing one is caught at boot BY NAME.
pub const AI_JOBS_WORDING_KEYS: &[&str] = &[
    KEY_AI_JOBS_TITLE,
    KEY_AI_JOBS_INTRO,
    KEY_AI_JOBS_MODEL_LABEL,
    KEY_AI_JOBS_INSTRUCTIONS_LABEL,
    KEY_AI_JOBS_EFFORT_LABEL,
    KEY_AI_JOBS_SAVE_LABEL,
    KEY_AI_JOBS_EMPTY_CONTROL,
    KEY_AI_JOBS_BUILT_IN,
    KEY_AI_JOBS_FIXED_BY_SERVER,
    KEY_AI_JOBS_FIXED_META,
    KEY_AI_JOBS_META_INSTALL,
    KEY_AI_JOBS_META_CHANGED,
    KEY_AI_JOBS_META_MODEL_CHANGED,
    KEY_AI_JOBS_META_INSTRUCTIONS_CHANGED,
    KEY_AI_JOBS_FOOT_GROUNDED,
    KEY_AI_JOBS_FOOT_ACTIVE,
    KEY_AI_JOBS_FOOT_ANTHROPIC,
    KEY_AI_JOBS_FOOT_SCAN,
    KEY_AI_JOBS_FOOT_FILES,
    KEY_AI_JOBS_MARKER_IN_USE,
    KEY_AI_JOBS_MARKER_USED_BEFORE,
    KEY_AI_JOBS_MARKER_NEW,
    KEY_AI_JOBS_MARKER_EARLIER,
    KEY_AI_JOBS_EFFORT_LOW,
    KEY_AI_JOBS_EFFORT_MEDIUM,
    KEY_AI_JOBS_EFFORT_HIGH,
    KEY_AI_JOBS_EFFORT_XHIGH,
    KEY_AI_JOBS_EFFORT_MAX,
    KEY_AI_JOBS_EFFORT_UNSET,
    KEY_AI_JOBS_WARN_MODEL_OFF,
    KEY_AI_JOBS_WARN_MODEL_MISSING,
    KEY_AI_JOBS_WARN_MODEL_UNQUOTED,
    KEY_AI_JOBS_WARN_MODEL_LOCAL,
    KEY_AI_JOBS_WARN_MODEL_NOT_SCAN,
    KEY_AI_JOBS_WARN_FILE_MISSING,
    KEY_AI_JOBS_SAVED,
    KEY_AI_JOBS_SAVED_RELOAD,
    KEY_AI_JOBS_SAVED_RELOAD_UNPRICED,
    KEY_AI_JOBS_WAS,
    KEY_AI_JOBS_SWITCH_BACK,
    KEY_AI_JOBS_REFUSE_FIXED,
    KEY_AI_JOBS_REFUSE_NOTHING,
    KEY_AI_JOBS_REFUSE_NO_HISTORY,
    KEY_AI_JOBS_REFUSE_NOT_JOB,
    KEY_AI_JOBS_REFUSE_PARTIAL,
    KEY_AI_JOBS_REFUSE_MODEL_IN_USE,
    KEY_AI_JOBS_REFUSE_FILE_IN_USE,
    KEY_AI_JOBS_REFUSE_FILES_READ_ONLY,
    KEY_AI_JOBS_FILES_NOTE,
    KEY_AI_JOBS_USED_FOR_LABEL,
    KEY_AI_JOBS_SETTINGS_POINTER,
    KEY_AI_JOBS_OTHERS_TITLE,
    KEY_AI_JOBS_OTHERS_LINE,
    KEY_AI_JOBS_LINK_MODELS_LEAD,
    KEY_AI_JOBS_LINK_MODELS,
    KEY_AI_JOBS_LINK_FILES_LEAD,
    KEY_AI_JOBS_LINK_FILES,
    KEY_AI_JOBS_LINK_DATA_LEAD,
    KEY_AI_JOBS_LINK_DATA,
];

/// Build a [`AiJobsWording`] from the stored rows, or say which key is wrong.
///
/// # Errors
/// Whatever `read` returns for the first key that is missing, mis-kinded or blank.
pub fn build_ai_jobs_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<AiJobsWording, E> {
    Ok(AiJobsWording {
        title: read(KEY_AI_JOBS_TITLE)?,
        intro: read(KEY_AI_JOBS_INTRO)?,
        model_label: read(KEY_AI_JOBS_MODEL_LABEL)?,
        instructions_label: read(KEY_AI_JOBS_INSTRUCTIONS_LABEL)?,
        effort_label: read(KEY_AI_JOBS_EFFORT_LABEL)?,
        save_label: read(KEY_AI_JOBS_SAVE_LABEL)?,
        empty_control: read(KEY_AI_JOBS_EMPTY_CONTROL)?,
        built_in: read(KEY_AI_JOBS_BUILT_IN)?,
        fixed_by_server: read(KEY_AI_JOBS_FIXED_BY_SERVER)?,
        fixed_meta: read(KEY_AI_JOBS_FIXED_META)?,
        meta_install: read(KEY_AI_JOBS_META_INSTALL)?,
        meta_changed: read(KEY_AI_JOBS_META_CHANGED)?,
        meta_model_changed: read(KEY_AI_JOBS_META_MODEL_CHANGED)?,
        meta_instructions_changed: read(KEY_AI_JOBS_META_INSTRUCTIONS_CHANGED)?,
        foot_grounded: read(KEY_AI_JOBS_FOOT_GROUNDED)?,
        foot_active: read(KEY_AI_JOBS_FOOT_ACTIVE)?,
        foot_anthropic: read(KEY_AI_JOBS_FOOT_ANTHROPIC)?,
        foot_scan: read(KEY_AI_JOBS_FOOT_SCAN)?,
        foot_files: read(KEY_AI_JOBS_FOOT_FILES)?,
        marker_in_use: read(KEY_AI_JOBS_MARKER_IN_USE)?,
        marker_used_before: read(KEY_AI_JOBS_MARKER_USED_BEFORE)?,
        marker_new: read(KEY_AI_JOBS_MARKER_NEW)?,
        marker_earlier: read(KEY_AI_JOBS_MARKER_EARLIER)?,
        effort_low: read(KEY_AI_JOBS_EFFORT_LOW)?,
        effort_medium: read(KEY_AI_JOBS_EFFORT_MEDIUM)?,
        effort_high: read(KEY_AI_JOBS_EFFORT_HIGH)?,
        effort_xhigh: read(KEY_AI_JOBS_EFFORT_XHIGH)?,
        effort_max: read(KEY_AI_JOBS_EFFORT_MAX)?,
        effort_unset: read(KEY_AI_JOBS_EFFORT_UNSET)?,
        warn_model_off: read(KEY_AI_JOBS_WARN_MODEL_OFF)?,
        warn_model_missing: read(KEY_AI_JOBS_WARN_MODEL_MISSING)?,
        warn_model_unquoted: read(KEY_AI_JOBS_WARN_MODEL_UNQUOTED)?,
        warn_model_local: read(KEY_AI_JOBS_WARN_MODEL_LOCAL)?,
        warn_model_not_scan: read(KEY_AI_JOBS_WARN_MODEL_NOT_SCAN)?,
        warn_file_missing: read(KEY_AI_JOBS_WARN_FILE_MISSING)?,
        saved: read(KEY_AI_JOBS_SAVED)?,
        saved_reload: read(KEY_AI_JOBS_SAVED_RELOAD)?,
        saved_reload_unpriced: read(KEY_AI_JOBS_SAVED_RELOAD_UNPRICED)?,
        was: read(KEY_AI_JOBS_WAS)?,
        switch_back: read(KEY_AI_JOBS_SWITCH_BACK)?,
        refuse_fixed: read(KEY_AI_JOBS_REFUSE_FIXED)?,
        refuse_nothing: read(KEY_AI_JOBS_REFUSE_NOTHING)?,
        refuse_no_history: read(KEY_AI_JOBS_REFUSE_NO_HISTORY)?,
        refuse_not_job: read(KEY_AI_JOBS_REFUSE_NOT_JOB)?,
        refuse_partial: read(KEY_AI_JOBS_REFUSE_PARTIAL)?,
        refuse_model_in_use: read(KEY_AI_JOBS_REFUSE_MODEL_IN_USE)?,
        refuse_file_in_use: read(KEY_AI_JOBS_REFUSE_FILE_IN_USE)?,
        refuse_files_read_only: read(KEY_AI_JOBS_REFUSE_FILES_READ_ONLY)?,
        files_note: read(KEY_AI_JOBS_FILES_NOTE)?,
        used_for_label: read(KEY_AI_JOBS_USED_FOR_LABEL)?,
        settings_pointer: read(KEY_AI_JOBS_SETTINGS_POINTER)?,
        others_title: read(KEY_AI_JOBS_OTHERS_TITLE)?,
        others_line: read(KEY_AI_JOBS_OTHERS_LINE)?,
        link_models_lead: read(KEY_AI_JOBS_LINK_MODELS_LEAD)?,
        link_models: read(KEY_AI_JOBS_LINK_MODELS)?,
        link_files_lead: read(KEY_AI_JOBS_LINK_FILES_LEAD)?,
        link_files: read(KEY_AI_JOBS_LINK_FILES)?,
        link_data_lead: read(KEY_AI_JOBS_LINK_DATA_LEAD)?,
        link_data: read(KEY_AI_JOBS_LINK_DATA)?,
    })
}

#[cfg(test)]
#[path = "wording_ai_jobs_tests.rs"]
pub(crate) mod tests;
