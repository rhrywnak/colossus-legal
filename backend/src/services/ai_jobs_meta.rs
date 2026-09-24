//! The words around each job on the Overview panel: its name and cost line, the
//! "who changed it" line, and board 4's worst-state sentences
//! (CC_TASK_MODEL_JOBS_PANEL_v1).
//!
//! Split from [`super::ai_jobs_compose`] under Rule 17. Pure, like its parent:
//! everything arrives already read, and every sentence comes from a stored row.

use std::collections::HashSet;

use crate::domain::llm_effort::parse_stored_effort;
use crate::domain::wording_ai_job_rows::AiJobRowsWording;
use crate::domain::wording_ai_jobs::AiJobsWording;
use crate::domain::wording_templates::render;
use crate::repositories::pipeline_repository::ai_job_settings::AiJobSettingRow;
use crate::repositories::pipeline_repository::app_settings::AppSettingChangeRecord;
use crate::repositories::pipeline_repository::models::LlmModelRecord;
use crate::services::ai_jobs_compose::{
    model_name, JobRows, PanelInputs, JOIN, ROLE_EFFORT, ROLE_INSTRUCTIONS, ROLE_MODEL,
};
use crate::services::ai_jobs_options::effort_label;
use crate::services::ai_jobs_options::{
    ModelProblem, ModelRule, JOB_ANSWER_ANALYSIS, JOB_CASE_STORY, JOB_CHAT_PAGE, JOB_DISCUSS_CHAT,
    JOB_DISCUSS_EFFORT, JOB_THEME_SCAN,
};

/// The words one job speaks. An unknown job (added by a later migration without
/// its words yet) is shown by its token, so it is visible rather than hidden.
pub(crate) struct JobWords {
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) subject: String,
    /// The cost line: (with the price, without it).
    pub(crate) note: Option<(String, String)>,
}

pub(crate) fn job_words(job: &str, w: &AiJobRowsWording) -> JobWords {
    let (title, description, subject) = match job {
        JOB_DISCUSS_CHAT => (
            &w.discuss_chat_title,
            &w.discuss_chat_desc,
            &w.discuss_chat_subject,
        ),
        JOB_DISCUSS_EFFORT => (
            &w.discuss_effort_title,
            &w.discuss_effort_desc,
            &w.discuss_effort_subject,
        ),
        JOB_CASE_STORY => (
            &w.case_story_title,
            &w.case_story_desc,
            &w.case_story_subject,
        ),
        JOB_ANSWER_ANALYSIS => (
            &w.answer_analysis_title,
            &w.answer_analysis_desc,
            &w.answer_analysis_subject,
        ),
        JOB_CHAT_PAGE => (&w.chat_page_title, &w.chat_page_desc, &w.chat_page_subject),
        JOB_THEME_SCAN => (
            &w.theme_scan_title,
            &w.theme_scan_desc,
            &w.theme_scan_subject,
        ),
        other => {
            return JobWords {
                title: other.to_string(),
                description: String::new(),
                subject: other.to_string(),
                note: None,
            }
        }
    };
    JobWords {
        title: title.clone(),
        description: description.clone(),
        subject: subject.clone(),
        note: job_note(job, w),
    }
}

/// The job's cost line, with and without a price; `None` for a job that has none.
fn job_note(job: &str, w: &AiJobRowsWording) -> Option<(String, String)> {
    let (priced, unpriced) = match job {
        JOB_DISCUSS_CHAT => (&w.discuss_chat_note, &w.discuss_chat_note_unpriced),
        JOB_DISCUSS_EFFORT => (&w.discuss_effort_note, &w.discuss_effort_note),
        JOB_CASE_STORY => (&w.case_story_note, &w.case_story_note_unpriced),
        _ => return None,
    };
    Some((priced.clone(), unpriced.clone()))
}

/// Add the board-4 sentence for a model that cannot serve its job.
pub(crate) fn warn_model(
    inp: &PanelInputs<'_>,
    job: &str,
    id: &str,
    subject: &str,
    out: &mut Vec<String>,
) {
    if let Some(sentence) = model_problem(
        inp.models,
        inp.scan_ids,
        &inp.wording.ai_jobs,
        job,
        id,
        subject,
    ) {
        out.push(sentence);
    }
}

/// Board 4's sentence for a model that cannot serve `job`, or `None` when it can.
///
/// Shared by the panel's warnings and by Save's refusal, so the page never says
/// one thing on the row and another when the same model is saved.
pub(crate) fn model_problem(
    models: &[LlmModelRecord],
    scan_ids: &HashSet<String>,
    w: &AiJobsWording,
    job: &str,
    id: &str,
    subject: &str,
) -> Option<String> {
    let model = models.iter().find(|m| m.id == id);
    let template = match ModelRule::for_job(job).problem(model, scan_ids)? {
        ModelProblem::Missing => &w.warn_model_missing,
        ModelProblem::Off => &w.warn_model_off,
        ModelProblem::Unquoted => &w.warn_model_unquoted,
        ModelProblem::Local => &w.warn_model_local,
        ModelProblem::NotScan => &w.warn_model_not_scan,
    };
    let name = model_name(models, id);
    Some(render(
        template,
        &[("model", name.as_str()), ("job", subject)],
    ))
}

/// "Model changed by Roman, 3:11 pm · <cost line>".
///
/// "Changed" means a change made in the app: the history (`app_setting_changes`)
/// records every one, and no migration writes to it. A job with no recorded
/// change is "Set by the install, never changed" whatever its `updated_by`.
pub(crate) fn meta_line(
    inp: &PanelInputs<'_>,
    rows: &JobRows<'_>,
    fixed: bool,
    words: &JobWords,
) -> String {
    let w = &inp.wording.ai_jobs;
    if fixed {
        return w.fixed_meta.clone();
    }
    let owned = owned_rows(rows);
    let last = last_change(inp, rows).map(|(c, r)| (c, r.ai_role.as_str()));
    let head = match last {
        None => w.meta_install.clone(),
        Some((change, role)) => {
            let template = match (owned.len() > 1, role) {
                (true, ROLE_MODEL) => &w.meta_model_changed,
                (true, ROLE_INSTRUCTIONS) => &w.meta_instructions_changed,
                _ => &w.meta_changed,
            };
            let who = (inp.who)(&change.actor);
            let when = (inp.when)(change.at);
            render(template, &[("who", who.as_str()), ("when", when.as_str())])
        }
    };
    match &words.note {
        None => head,
        Some((priced, unpriced)) => {
            let note = match inp.reload_cost {
                Some(cost) => render(priced, &[("cost", dollars(cost).as_str())]),
                None => unpriced.clone(),
            };
            format!("{head}{JOIN}{note}")
        }
    }
}

/// `$2.95` — the same two-decimal form the Chat case file box shows.
pub fn dollars(amount: f64) -> String {
    format!("${amount:.2}")
}

/// The job's rows, whichever of the three roles it has.
fn owned_rows<'a>(rows: &JobRows<'a>) -> Vec<&'a AiJobSettingRow> {
    [rows.model, rows.instructions, rows.effort]
        .into_iter()
        .flatten()
        .collect()
}

/// The job's most recent change made in the app, and the row it changed.
///
/// `inp.changes` is most-recent-first, so the first entry naming one of the
/// job's keys is the latest.
pub(crate) fn last_change<'a>(
    inp: &PanelInputs<'a>,
    rows: &JobRows<'a>,
) -> Option<(&'a AppSettingChangeRecord, &'a AiJobSettingRow)> {
    let owned = owned_rows(rows);
    inp.changes
        .iter()
        .find_map(|c| owned.iter().find(|r| r.key == c.key).map(|r| (c, *r)))
}

/// "was Claude Opus 5" — what the job used before its last change, for the one
/// click that restores it (board 3). `None` when nothing was changed in the app,
/// or when the server's configuration fixes the job.
pub(crate) fn was_line(inp: &PanelInputs<'_>, rows: &JobRows<'_>, fixed: bool) -> Option<String> {
    if fixed {
        return None;
    }
    let (change, row) = last_change(inp, rows)?;
    let label = value_label(inp, &row.ai_role, &change.old_value);
    Some(render(
        &inp.wording.ai_jobs.was,
        &[("value", label.as_str())],
    ))
}

/// A stored value as the panel names it: a model by its display name, a
/// thinking level in plain words, a file by its name.
pub(crate) fn value_label(inp: &PanelInputs<'_>, role: &str, value: &str) -> String {
    label_for(inp.models, &inp.wording.ai_jobs, role, value)
}

/// [`value_label`] without the panel's inputs, for Save's confirmation.
pub(crate) fn label_for(
    models: &[LlmModelRecord],
    w: &AiJobsWording,
    role: &str,
    value: &str,
) -> String {
    match role {
        ROLE_MODEL => model_name(models, value),
        ROLE_EFFORT => {
            parse_stored_effort(value).map_or_else(|_| value.to_string(), |e| effort_label(e, w))
        }
        _ => value.to_string(),
    }
}
