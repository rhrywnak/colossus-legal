// =============================================================================
// backend/src/domain/wording_ai_job_rows.rs — Each AI job's name, description and cost line
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

/// Each AI job's name, description and cost line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiJobRowsWording {
    /// The Discuss chat job's name.
    pub discuss_chat_title: String,
    /// The Discuss chat job's one-line description.
    pub discuss_chat_desc: String,
    /// How a warning names the Discuss chat job.
    pub discuss_chat_subject: String,
    /// The Discuss chat's cost line. {cost} is the measured reload price.
    pub discuss_chat_note: String,
    /// The Discuss chat's cost line when the reload price is not known.
    pub discuss_chat_note_unpriced: String,
    /// The thinking-level job's name.
    pub discuss_effort_title: String,
    /// The thinking-level job's description.
    pub discuss_effort_desc: String,
    /// How a warning names the thinking-level job.
    pub discuss_effort_subject: String,
    /// The thinking-level job's cost line: changing it does not reload the case file.
    pub discuss_effort_note: String,
    /// The case story job's name.
    pub case_story_title: String,
    /// The case story job's description.
    pub case_story_desc: String,
    /// How a warning names the case story job (a missing story stops the Discuss chat).
    pub case_story_subject: String,
    /// The case story's cost line. {cost} is the measured reload price.
    pub case_story_note: String,
    /// The case story's cost line when the reload price is not known.
    pub case_story_note_unpriced: String,
    /// The answer analysis job's name.
    pub answer_analysis_title: String,
    /// The answer analysis job's description.
    pub answer_analysis_desc: String,
    /// How a warning names the answer analysis job.
    pub answer_analysis_subject: String,
    /// The Chat page job's name.
    pub chat_page_title: String,
    /// The Chat page job's description.
    pub chat_page_desc: String,
    /// How a warning names the Chat page job.
    pub chat_page_subject: String,
    /// The theme scan job's name.
    pub theme_scan_title: String,
    /// The theme scan job's description.
    pub theme_scan_desc: String,
    /// How a warning names the theme scan job.
    pub theme_scan_subject: String,
}

// KEYS: the stable identifiers. Renaming one is a migration.
pub(crate) const KEY_AI_JOB_DISCUSS_CHAT_TITLE: &str = "ai_job_discuss_chat_title";
pub(crate) const KEY_AI_JOB_DISCUSS_CHAT_DESC: &str = "ai_job_discuss_chat_desc";
pub(crate) const KEY_AI_JOB_DISCUSS_CHAT_SUBJECT: &str = "ai_job_discuss_chat_subject";
pub(crate) const KEY_AI_JOB_DISCUSS_CHAT_NOTE: &str = "ai_job_discuss_chat_note";
pub(crate) const KEY_AI_JOB_DISCUSS_CHAT_NOTE_UNPRICED: &str = "ai_job_discuss_chat_note_unpriced";
pub(crate) const KEY_AI_JOB_DISCUSS_EFFORT_TITLE: &str = "ai_job_discuss_effort_title";
pub(crate) const KEY_AI_JOB_DISCUSS_EFFORT_DESC: &str = "ai_job_discuss_effort_desc";
pub(crate) const KEY_AI_JOB_DISCUSS_EFFORT_SUBJECT: &str = "ai_job_discuss_effort_subject";
pub(crate) const KEY_AI_JOB_DISCUSS_EFFORT_NOTE: &str = "ai_job_discuss_effort_note";
pub(crate) const KEY_AI_JOB_CASE_STORY_TITLE: &str = "ai_job_case_story_title";
pub(crate) const KEY_AI_JOB_CASE_STORY_DESC: &str = "ai_job_case_story_desc";
pub(crate) const KEY_AI_JOB_CASE_STORY_SUBJECT: &str = "ai_job_case_story_subject";
pub(crate) const KEY_AI_JOB_CASE_STORY_NOTE: &str = "ai_job_case_story_note";
pub(crate) const KEY_AI_JOB_CASE_STORY_NOTE_UNPRICED: &str = "ai_job_case_story_note_unpriced";
pub(crate) const KEY_AI_JOB_ANSWER_ANALYSIS_TITLE: &str = "ai_job_answer_analysis_title";
pub(crate) const KEY_AI_JOB_ANSWER_ANALYSIS_DESC: &str = "ai_job_answer_analysis_desc";
pub(crate) const KEY_AI_JOB_ANSWER_ANALYSIS_SUBJECT: &str = "ai_job_answer_analysis_subject";
pub(crate) const KEY_AI_JOB_CHAT_PAGE_TITLE: &str = "ai_job_chat_page_title";
pub(crate) const KEY_AI_JOB_CHAT_PAGE_DESC: &str = "ai_job_chat_page_desc";
pub(crate) const KEY_AI_JOB_CHAT_PAGE_SUBJECT: &str = "ai_job_chat_page_subject";
pub(crate) const KEY_AI_JOB_THEME_SCAN_TITLE: &str = "ai_job_theme_scan_title";
pub(crate) const KEY_AI_JOB_THEME_SCAN_DESC: &str = "ai_job_theme_scan_desc";
pub(crate) const KEY_AI_JOB_THEME_SCAN_SUBJECT: &str = "ai_job_theme_scan_subject";

/// Every key this block reads, so a missing one is caught at boot BY NAME.
pub const AI_JOB_ROWS_WORDING_KEYS: &[&str] = &[
    KEY_AI_JOB_DISCUSS_CHAT_TITLE,
    KEY_AI_JOB_DISCUSS_CHAT_DESC,
    KEY_AI_JOB_DISCUSS_CHAT_SUBJECT,
    KEY_AI_JOB_DISCUSS_CHAT_NOTE,
    KEY_AI_JOB_DISCUSS_CHAT_NOTE_UNPRICED,
    KEY_AI_JOB_DISCUSS_EFFORT_TITLE,
    KEY_AI_JOB_DISCUSS_EFFORT_DESC,
    KEY_AI_JOB_DISCUSS_EFFORT_SUBJECT,
    KEY_AI_JOB_DISCUSS_EFFORT_NOTE,
    KEY_AI_JOB_CASE_STORY_TITLE,
    KEY_AI_JOB_CASE_STORY_DESC,
    KEY_AI_JOB_CASE_STORY_SUBJECT,
    KEY_AI_JOB_CASE_STORY_NOTE,
    KEY_AI_JOB_CASE_STORY_NOTE_UNPRICED,
    KEY_AI_JOB_ANSWER_ANALYSIS_TITLE,
    KEY_AI_JOB_ANSWER_ANALYSIS_DESC,
    KEY_AI_JOB_ANSWER_ANALYSIS_SUBJECT,
    KEY_AI_JOB_CHAT_PAGE_TITLE,
    KEY_AI_JOB_CHAT_PAGE_DESC,
    KEY_AI_JOB_CHAT_PAGE_SUBJECT,
    KEY_AI_JOB_THEME_SCAN_TITLE,
    KEY_AI_JOB_THEME_SCAN_DESC,
    KEY_AI_JOB_THEME_SCAN_SUBJECT,
];

/// Build a [`AiJobRowsWording`] from the stored rows, or say which key is wrong.
///
/// # Errors
/// Whatever `read` returns for the first key that is missing, mis-kinded or blank.
pub fn build_ai_job_rows_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<AiJobRowsWording, E> {
    Ok(AiJobRowsWording {
        discuss_chat_title: read(KEY_AI_JOB_DISCUSS_CHAT_TITLE)?,
        discuss_chat_desc: read(KEY_AI_JOB_DISCUSS_CHAT_DESC)?,
        discuss_chat_subject: read(KEY_AI_JOB_DISCUSS_CHAT_SUBJECT)?,
        discuss_chat_note: read(KEY_AI_JOB_DISCUSS_CHAT_NOTE)?,
        discuss_chat_note_unpriced: read(KEY_AI_JOB_DISCUSS_CHAT_NOTE_UNPRICED)?,
        discuss_effort_title: read(KEY_AI_JOB_DISCUSS_EFFORT_TITLE)?,
        discuss_effort_desc: read(KEY_AI_JOB_DISCUSS_EFFORT_DESC)?,
        discuss_effort_subject: read(KEY_AI_JOB_DISCUSS_EFFORT_SUBJECT)?,
        discuss_effort_note: read(KEY_AI_JOB_DISCUSS_EFFORT_NOTE)?,
        case_story_title: read(KEY_AI_JOB_CASE_STORY_TITLE)?,
        case_story_desc: read(KEY_AI_JOB_CASE_STORY_DESC)?,
        case_story_subject: read(KEY_AI_JOB_CASE_STORY_SUBJECT)?,
        case_story_note: read(KEY_AI_JOB_CASE_STORY_NOTE)?,
        case_story_note_unpriced: read(KEY_AI_JOB_CASE_STORY_NOTE_UNPRICED)?,
        answer_analysis_title: read(KEY_AI_JOB_ANSWER_ANALYSIS_TITLE)?,
        answer_analysis_desc: read(KEY_AI_JOB_ANSWER_ANALYSIS_DESC)?,
        answer_analysis_subject: read(KEY_AI_JOB_ANSWER_ANALYSIS_SUBJECT)?,
        chat_page_title: read(KEY_AI_JOB_CHAT_PAGE_TITLE)?,
        chat_page_desc: read(KEY_AI_JOB_CHAT_PAGE_DESC)?,
        chat_page_subject: read(KEY_AI_JOB_CHAT_PAGE_SUBJECT)?,
        theme_scan_title: read(KEY_AI_JOB_THEME_SCAN_TITLE)?,
        theme_scan_desc: read(KEY_AI_JOB_THEME_SCAN_DESC)?,
        theme_scan_subject: read(KEY_AI_JOB_THEME_SCAN_SUBJECT)?,
    })
}

#[cfg(test)]
#[path = "wording_ai_job_rows_tests.rs"]
pub(crate) mod tests;
