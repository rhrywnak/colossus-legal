//! Compose Admin → Overview's jobs panel from what the store and the folder say
//! (CC_TASK_MODEL_JOBS_PANEL_v1, boards 1, 2 and 4).
//!
//! Pure: [`compose_panel`] takes everything already read — the job rows, their
//! change history, the model list, the instruction files, the server's fixed
//! scan model and the reload price — and returns finished sentences. The I/O
//! lives in `services::ai_jobs`; the choices live here, where plain values test
//! them.

use std::collections::{BTreeMap, HashSet};

use chrono::{DateTime, Utc};

use crate::domain::llm_effort::parse_stored_effort;
use crate::domain::wording_admin::AdminWording;
use crate::domain::wording_templates::render;
use crate::dto::ai_jobs::{
    AiJobControlBody, AiJobControlDto, AiJobDto, AiJobsLinkDto, AiJobsPanelDto,
};
use crate::repositories::pipeline_repository::ai_job_settings::AiJobSettingRow;
use crate::repositories::pipeline_repository::app_settings::AppSettingChangeRecord;
use crate::repositories::pipeline_repository::models::LlmModelRecord;
pub use crate::services::ai_jobs_meta::dollars;
use crate::services::ai_jobs_meta::{job_words, meta_line, warn_model, was_line};
use crate::services::ai_jobs_options::{
    effort_label, effort_options, instruction_options, model_options, ModelRule, JOB_ORDER,
    JOB_THEME_SCAN,
};

// STRUCTURAL: the `ai_role` tokens migration 20260924145150 writes. Code and store
// must spell them identically; renaming one is a migration.
pub const ROLE_MODEL: &str = "model";
// STRUCTURAL: an `ai_role` token, for the reason given on `ROLE_MODEL`.
pub const ROLE_INSTRUCTIONS: &str = "instructions";
// STRUCTURAL: an `ai_role` token, for the reason given on `ROLE_MODEL`.
pub const ROLE_EFFORT: &str = "effort";
// STRUCTURAL: punctuation between two facts of equal weight, the middle dot this
// product uses everywhere (see `practice_clock::local_stamp`). Not wording.
pub(crate) const JOIN: &str = " · ";
// STRUCTURAL: the link targets the page maps to its own routes. Wire tokens.
const LINK_TARGETS: [&str; 3] = ["models", "files", "data"];

/// Everything the panel is composed from, already read.
pub struct PanelInputs<'a> {
    pub rows: &'a [AiJobSettingRow],
    /// Most recent first.
    pub changes: &'a [AppSettingChangeRecord],
    pub models: &'a [LlmModelRecord],
    /// Ids of the models offered for scans.
    pub scan_ids: &'a HashSet<String>,
    /// File names in the instructions folder.
    pub files: &'a [String],
    /// `THEME_SCAN_MODEL`, when the server's configuration fixes the scan's model.
    pub fixed_scan_model: Option<&'a str>,
    /// What reloading the case file would cost now; `None` when not known.
    pub reload_cost: Option<f64>,
    pub wording: &'a AdminWording,
    /// Settings that name a model and belong to no job: (meaning, model id).
    pub loose: &'a [(String, String)],
    /// Formats a moment the way the Chat case file box does.
    pub when: &'a dyn Fn(DateTime<Utc>) -> String,
    /// Turns a login into the name the practice pages use.
    pub who: &'a dyn Fn(&str) -> String,
}

/// A job row the panel cannot show, named.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ComposeError {
    #[error("setting {key} belongs to job {job} with role {role:?}, which this page does not know (expected model, instructions or effort)")]
    UnknownRole {
        key: String,
        job: String,
        role: String,
    },
    #[error(
        "job {job} has two {role} settings ({first} and {second}); a job has at most one of each"
    )]
    DuplicateRole {
        job: String,
        role: String,
        first: String,
        second: String,
    },
}

/// One job's rows, by role.
#[derive(Default)]
pub(crate) struct JobRows<'a> {
    pub(crate) model: Option<&'a AiJobSettingRow>,
    pub(crate) instructions: Option<&'a AiJobSettingRow>,
    pub(crate) effort: Option<&'a AiJobSettingRow>,
}

/// The whole panel.
///
/// # Errors
/// [`ComposeError`] when a job row carries a role this page does not know, or a
/// job has two rows of one role — both store defects a migration would have to
/// make, named rather than drawn wrong.
pub fn compose_panel(inp: &PanelInputs<'_>) -> Result<AiJobsPanelDto, ComposeError> {
    let w = &inp.wording.ai_jobs;
    let jobs = group_rows(inp.rows)?
        .into_iter()
        .map(|(job, rows)| compose_job(inp, &job, &rows))
        .collect();
    let links = LINK_TARGETS
        .iter()
        .zip([
            (&w.link_models_lead, &w.link_models),
            (&w.link_files_lead, &w.link_files),
            (&w.link_data_lead, &w.link_data),
        ])
        .map(|(target, (lead, label))| AiJobsLinkDto {
            target: (*target).to_string(),
            lead: lead.clone(),
            label: label.clone(),
        })
        .collect();
    Ok(AiJobsPanelDto {
        title: w.title.clone(),
        intro: w.intro.clone(),
        save_label: w.save_label.clone(),
        jobs,
        links,
        others_title: w.others_title.clone(),
        others: inp
            .loose
            .iter()
            .map(|(meaning, model)| {
                let name = model_name(inp.models, model);
                render(
                    &w.others_line,
                    &[("setting", meaning.as_str()), ("model", name.as_str())],
                )
            })
            .collect(),
    })
}

/// Rows grouped by job, known jobs in panel order, then any others by name.
///
/// ## Rust Learning: sorting by a key that is itself a tuple
///
/// `sort_by_key(|(job, _)| (position, job))` sorts by the first element and
/// breaks ties by the second. Unknown jobs share the position `usize::MAX`, so
/// they land after every known job, alphabetised among themselves.
fn group_rows(rows: &[AiJobSettingRow]) -> Result<Vec<(String, JobRows<'_>)>, ComposeError> {
    let mut by_job: BTreeMap<String, JobRows<'_>> = BTreeMap::new();
    for row in rows {
        let slot = by_job.entry(row.ai_job.clone()).or_default();
        let place = match row.ai_role.as_str() {
            ROLE_MODEL => &mut slot.model,
            ROLE_INSTRUCTIONS => &mut slot.instructions,
            ROLE_EFFORT => &mut slot.effort,
            other => {
                return Err(ComposeError::UnknownRole {
                    key: row.key.clone(),
                    job: row.ai_job.clone(),
                    role: other.to_string(),
                })
            }
        };
        if let Some(first) = place.replace(row) {
            return Err(ComposeError::DuplicateRole {
                job: row.ai_job.clone(),
                role: row.ai_role.clone(),
                first: first.key.clone(),
                second: row.key.clone(),
            });
        }
    }
    let mut out: Vec<(String, JobRows<'_>)> = by_job.into_iter().collect();
    out.sort_by_key(|(job, _)| {
        let at = JOB_ORDER
            .iter()
            .position(|j| j == job)
            .unwrap_or(usize::MAX);
        (at, job.clone())
    });
    Ok(out)
}

fn compose_job(inp: &PanelInputs<'_>, job: &str, rows: &JobRows<'_>) -> AiJobDto {
    let words = job_words(job, &inp.wording.job_rows);
    let mut warnings = Vec::new();
    let fixed = if job == JOB_THEME_SCAN {
        inp.fixed_scan_model
    } else {
        None
    };
    let first = first_column(inp, job, rows, fixed, &words.subject, &mut warnings);
    let was = was_line(inp, rows, fixed.is_some());
    let instructions =
        instructions_column(inp, rows, fixed.is_some(), &words.subject, &mut warnings);
    AiJobDto {
        job: job.to_string(),
        title: words.title.clone(),
        description: words.description.clone(),
        first,
        instructions,
        meta: meta_line(inp, rows, fixed.is_some(), &words),
        warnings,
        was: was.clone(),
        switch_back: was.map(|_| inp.wording.ai_jobs.switch_back.clone()),
    }
}

/// The first column: the server's fixed model, the model dropdown, the thinking
/// dropdown, or "—" — checking the model in force as it goes.
fn first_column(
    inp: &PanelInputs<'_>,
    job: &str,
    rows: &JobRows<'_>,
    fixed: Option<&str>,
    subject: &str,
    warnings: &mut Vec<String>,
) -> AiJobControlDto {
    let w = &inp.wording.ai_jobs;
    if let Some(model_id) = fixed {
        let name = model_name(inp.models, model_id);
        warn_model(inp, job, model_id, subject, warnings);
        let text = render(&w.fixed_by_server, &[("model", name.as_str())]);
        return control(&w.model_label, AiJobControlBody::Fixed { text });
    }
    if let Some(row) = rows.model {
        warn_model(inp, job, &row.value, subject, warnings);
        return model_control(inp, job, row);
    }
    if let Some(row) = rows.effort {
        return effort_control(inp, row);
    }
    let text = w.empty_control.clone();
    control(&w.model_label, AiJobControlBody::Empty { text })
}

/// The instructions column. A job that picks a model but no instructions keeps
/// them in the program ("Built in"); a job with neither shows "—".
fn instructions_column(
    inp: &PanelInputs<'_>,
    rows: &JobRows<'_>,
    has_model: bool,
    subject: &str,
    warnings: &mut Vec<String>,
) -> AiJobControlDto {
    let w = &inp.wording.ai_jobs;
    match (rows.instructions, has_model || rows.model.is_some()) {
        (Some(row), _) => {
            if !inp.files.contains(&row.value) {
                let values = [("file", row.value.as_str()), ("job", subject)];
                warnings.push(render(&w.warn_file_missing, &values));
            }
            instructions_control(inp, row)
        }
        (None, true) => {
            let text = w.built_in.clone();
            control(&w.instructions_label, AiJobControlBody::Fixed { text })
        }
        (None, false) => {
            let text = w.empty_control.clone();
            control(&w.instructions_label, AiJobControlBody::Empty { text })
        }
    }
}

fn control(label: &str, body: AiJobControlBody) -> AiJobControlDto {
    AiJobControlDto {
        label: label.to_string(),
        body,
    }
}

pub(crate) fn model_name(models: &[LlmModelRecord], id: &str) -> String {
    models
        .iter()
        .find(|m| m.id == id)
        .map_or_else(|| id.to_string(), |m| m.display_name.clone())
}

fn model_control(inp: &PanelInputs<'_>, job: &str, row: &AiJobSettingRow) -> AiJobControlDto {
    let rule = ModelRule::for_job(job);
    let w = &inp.wording.ai_jobs;
    control(
        &w.model_label,
        AiJobControlBody::Choose {
            key: row.key.clone(),
            current: row.value.clone(),
            current_label: model_name(inp.models, &row.value),
            options: model_options(rule, inp.models, inp.scan_ids, &row.value),
            foot: Some(rule.foot(w)),
        },
    )
}

fn effort_control(inp: &PanelInputs<'_>, row: &AiJobSettingRow) -> AiJobControlDto {
    let w = &inp.wording.ai_jobs;
    // An unreadable stored level cannot occur past boot (`effort_of` refuses it);
    // if it did, the raw value is shown rather than a wrong plain word.
    let current_label =
        parse_stored_effort(&row.value).map_or_else(|_| row.value.clone(), |e| effort_label(e, w));
    control(
        &w.effort_label,
        AiJobControlBody::Choose {
            key: row.key.clone(),
            current: row.value.clone(),
            current_label,
            options: effort_options(&row.value, w),
            foot: None,
        },
    )
}

fn instructions_control(inp: &PanelInputs<'_>, row: &AiJobSettingRow) -> AiJobControlDto {
    let w = &inp.wording.ai_jobs;
    let used_before: HashSet<String> = inp
        .changes
        .iter()
        .filter(|c| c.key == row.key)
        .flat_map(|c| [c.old_value.clone(), c.new_value.clone()])
        .collect();
    control(
        &w.instructions_label,
        AiJobControlBody::Choose {
            key: row.key.clone(),
            current: row.value.clone(),
            current_label: row.value.clone(),
            options: instruction_options(&row.value, inp.files, &used_before, w),
            foot: Some(w.foot_files.clone()),
        },
    )
}

#[cfg(test)]
#[path = "ai_jobs_compose_tests.rs"]
pub(crate) mod tests;
