// =============================================================================
// backend/src/domain/wording_last_run.rs — The words of Admin → Data → Last document run
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

/// The words of Admin → Data → Last document run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastRunWording {
    /// The finished-documents number. {done} and {total} are counts.
    pub finished_value: String,
    /// Under the finished-documents number.
    pub finished_label: String,
    /// Under the quote-check percentage.
    pub quotes_label: String,
    /// Shown instead of a percentage when no quote check has run.
    pub quotes_none: String,
    /// Under the last run's day. {clock} is its time of day.
    pub when_label: String,
    /// Shown when no document step is running.
    pub idle: String,
    /// Shown while a document step is running.
    pub running: String,
    /// Shown while a run is in progress but nothing has finished to estimate from.
    pub running_no_estimate: String,
    /// Step table column.
    pub col_step: String,
    /// Step table column.
    pub col_avg: String,
    /// Step table column.
    pub col_runs: String,
    /// Step table column.
    pub col_failed: String,
    /// Under the step table when a step failed. {detail} lists which and when.
    pub summary: String,
    /// Under the step table when nothing failed.
    pub summary_clean: String,
    /// One failed step in the summary: its name and the day it last failed.
    pub failure_detail: String,
    /// Shown when the store holds a step this page cannot name.
    pub unknown_step: String,
    /// Shown when the store holds no documents.
    pub empty: String,
    /// Step name, in running order.
    pub step_upload: String,
    /// Step name, in running order.
    pub step_extract_text: String,
    /// Step name, in running order.
    pub step_llm_extract_pass1: String,
    /// Step name, in running order.
    pub step_llm_extract_pass2: String,
    /// Step name, in running order.
    pub step_verify: String,
    /// Step name, in running order.
    pub step_auto_approve: String,
    /// Step name, in running order.
    pub step_ingest: String,
    /// Step name, in running order.
    pub step_ingest_delta: String,
    /// Step name, in running order.
    pub step_index: String,
    /// Step name, in running order.
    pub step_completeness: String,
}

// KEYS: the stable identifiers. Renaming one is a migration.
pub(crate) const KEY_LAST_RUN_FINISHED_VALUE: &str = "last_run_finished_value";
pub(crate) const KEY_LAST_RUN_FINISHED_LABEL: &str = "last_run_finished_label";
pub(crate) const KEY_LAST_RUN_QUOTES_LABEL: &str = "last_run_quotes_label";
pub(crate) const KEY_LAST_RUN_QUOTES_NONE: &str = "last_run_quotes_none";
pub(crate) const KEY_LAST_RUN_WHEN_LABEL: &str = "last_run_when_label";
pub(crate) const KEY_LAST_RUN_IDLE: &str = "last_run_idle";
pub(crate) const KEY_LAST_RUN_RUNNING: &str = "last_run_running";
pub(crate) const KEY_LAST_RUN_RUNNING_NO_ESTIMATE: &str = "last_run_running_no_estimate";
pub(crate) const KEY_LAST_RUN_COL_STEP: &str = "last_run_col_step";
pub(crate) const KEY_LAST_RUN_COL_AVG: &str = "last_run_col_avg";
pub(crate) const KEY_LAST_RUN_COL_RUNS: &str = "last_run_col_runs";
pub(crate) const KEY_LAST_RUN_COL_FAILED: &str = "last_run_col_failed";
pub(crate) const KEY_LAST_RUN_SUMMARY: &str = "last_run_summary";
pub(crate) const KEY_LAST_RUN_SUMMARY_CLEAN: &str = "last_run_summary_clean";
pub(crate) const KEY_LAST_RUN_FAILURE_DETAIL: &str = "last_run_failure_detail";
pub(crate) const KEY_LAST_RUN_UNKNOWN_STEP: &str = "last_run_unknown_step";
pub(crate) const KEY_LAST_RUN_EMPTY: &str = "last_run_empty";
pub(crate) const KEY_LAST_RUN_STEP_UPLOAD: &str = "last_run_step_upload";
pub(crate) const KEY_LAST_RUN_STEP_EXTRACT_TEXT: &str = "last_run_step_extract_text";
pub(crate) const KEY_LAST_RUN_STEP_LLM_EXTRACT_PASS1: &str = "last_run_step_llm_extract_pass1";
pub(crate) const KEY_LAST_RUN_STEP_LLM_EXTRACT_PASS2: &str = "last_run_step_llm_extract_pass2";
pub(crate) const KEY_LAST_RUN_STEP_VERIFY: &str = "last_run_step_verify";
pub(crate) const KEY_LAST_RUN_STEP_AUTO_APPROVE: &str = "last_run_step_auto_approve";
pub(crate) const KEY_LAST_RUN_STEP_INGEST: &str = "last_run_step_ingest";
pub(crate) const KEY_LAST_RUN_STEP_INGEST_DELTA: &str = "last_run_step_ingest_delta";
pub(crate) const KEY_LAST_RUN_STEP_INDEX: &str = "last_run_step_index";
pub(crate) const KEY_LAST_RUN_STEP_COMPLETENESS: &str = "last_run_step_completeness";

/// Every key this block reads, so a missing one is caught at boot BY NAME.
pub const LAST_RUN_WORDING_KEYS: &[&str] = &[
    KEY_LAST_RUN_FINISHED_VALUE,
    KEY_LAST_RUN_FINISHED_LABEL,
    KEY_LAST_RUN_QUOTES_LABEL,
    KEY_LAST_RUN_QUOTES_NONE,
    KEY_LAST_RUN_WHEN_LABEL,
    KEY_LAST_RUN_IDLE,
    KEY_LAST_RUN_RUNNING,
    KEY_LAST_RUN_RUNNING_NO_ESTIMATE,
    KEY_LAST_RUN_COL_STEP,
    KEY_LAST_RUN_COL_AVG,
    KEY_LAST_RUN_COL_RUNS,
    KEY_LAST_RUN_COL_FAILED,
    KEY_LAST_RUN_SUMMARY,
    KEY_LAST_RUN_SUMMARY_CLEAN,
    KEY_LAST_RUN_FAILURE_DETAIL,
    KEY_LAST_RUN_UNKNOWN_STEP,
    KEY_LAST_RUN_EMPTY,
    KEY_LAST_RUN_STEP_UPLOAD,
    KEY_LAST_RUN_STEP_EXTRACT_TEXT,
    KEY_LAST_RUN_STEP_LLM_EXTRACT_PASS1,
    KEY_LAST_RUN_STEP_LLM_EXTRACT_PASS2,
    KEY_LAST_RUN_STEP_VERIFY,
    KEY_LAST_RUN_STEP_AUTO_APPROVE,
    KEY_LAST_RUN_STEP_INGEST,
    KEY_LAST_RUN_STEP_INGEST_DELTA,
    KEY_LAST_RUN_STEP_INDEX,
    KEY_LAST_RUN_STEP_COMPLETENESS,
];

/// Build a [`LastRunWording`] from the stored rows, or say which key is wrong.
///
/// # Errors
/// Whatever `read` returns for the first key that is missing, mis-kinded or blank.
pub fn build_last_run_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<LastRunWording, E> {
    Ok(LastRunWording {
        finished_value: read(KEY_LAST_RUN_FINISHED_VALUE)?,
        finished_label: read(KEY_LAST_RUN_FINISHED_LABEL)?,
        quotes_label: read(KEY_LAST_RUN_QUOTES_LABEL)?,
        quotes_none: read(KEY_LAST_RUN_QUOTES_NONE)?,
        when_label: read(KEY_LAST_RUN_WHEN_LABEL)?,
        idle: read(KEY_LAST_RUN_IDLE)?,
        running: read(KEY_LAST_RUN_RUNNING)?,
        running_no_estimate: read(KEY_LAST_RUN_RUNNING_NO_ESTIMATE)?,
        col_step: read(KEY_LAST_RUN_COL_STEP)?,
        col_avg: read(KEY_LAST_RUN_COL_AVG)?,
        col_runs: read(KEY_LAST_RUN_COL_RUNS)?,
        col_failed: read(KEY_LAST_RUN_COL_FAILED)?,
        summary: read(KEY_LAST_RUN_SUMMARY)?,
        summary_clean: read(KEY_LAST_RUN_SUMMARY_CLEAN)?,
        failure_detail: read(KEY_LAST_RUN_FAILURE_DETAIL)?,
        unknown_step: read(KEY_LAST_RUN_UNKNOWN_STEP)?,
        empty: read(KEY_LAST_RUN_EMPTY)?,
        step_upload: read(KEY_LAST_RUN_STEP_UPLOAD)?,
        step_extract_text: read(KEY_LAST_RUN_STEP_EXTRACT_TEXT)?,
        step_llm_extract_pass1: read(KEY_LAST_RUN_STEP_LLM_EXTRACT_PASS1)?,
        step_llm_extract_pass2: read(KEY_LAST_RUN_STEP_LLM_EXTRACT_PASS2)?,
        step_verify: read(KEY_LAST_RUN_STEP_VERIFY)?,
        step_auto_approve: read(KEY_LAST_RUN_STEP_AUTO_APPROVE)?,
        step_ingest: read(KEY_LAST_RUN_STEP_INGEST)?,
        step_ingest_delta: read(KEY_LAST_RUN_STEP_INGEST_DELTA)?,
        step_index: read(KEY_LAST_RUN_STEP_INDEX)?,
        step_completeness: read(KEY_LAST_RUN_STEP_COMPLETENESS)?,
    })
}

#[cfg(test)]
#[path = "wording_last_run_tests.rs"]
pub(crate) mod tests;
