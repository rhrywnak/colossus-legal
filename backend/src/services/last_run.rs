//! Admin → Data → Last document run, composed (CC_TASK_MODEL_JOBS_PANEL_v1,
//! board 5).
//!
//! [`compose_last_run`] is pure: it takes the facts the repository read and the
//! stored words, and returns finished strings. [`last_run`] only gathers them.

use chrono::{DateTime, Utc};

use crate::domain::wording_last_run::LastRunWording;
use crate::domain::wording_templates::render;
use crate::dto::last_run::{LastRunCardDto, LastRunDto, LastRunStepDto};
use crate::repositories::pipeline_repository::last_run::{last_run_facts, LastRunFacts, StepStats};
use crate::repositories::pipeline_repository::PipelineRepoError;
use crate::services::practice_clock::{local_clock, local_date, local_day_month};
use crate::state::AppState;

/// The pipeline's steps in the order a document passes through them.
///
/// ## Domain note: why the order is code and the names are rows
///
/// The ORDER is a fact about the pipeline program — `upload` always precedes
/// `extract_text` — and changes only when the program does. The NAMES are
/// words a person reads, so they are stored rows (`last_run_step_*`). A step the
/// store holds and this list lacks is still shown, after these, by its code
/// name, with a warning saying so (Standing Rule 1).
// STRUCTURAL: the pipeline program's step tokens as `pipeline_steps.step_name`
// stores them, in running order; renaming one is a pipeline change.
pub const STEP_ORDER: &[&str] = &[
    "upload",
    "extract_text",
    "llm_extract_pass1",
    "llm_extract_pass2",
    "verify",
    "auto_approve",
    "ingest",
    "ingest_delta",
    "index",
    "completeness",
];

/// The tab, read now.
///
/// # Errors
/// [`PipelineRepoError`] naming the failed read.
pub async fn last_run(state: &AppState) -> Result<LastRunDto, PipelineRepoError> {
    let facts = last_run_facts(&state.pipeline_pool).await?;
    let settings = state.settings.current();
    Ok(compose_last_run(
        &facts,
        &settings.admin_wording.last_run,
        &settings.practice_read.case_timezone,
    ))
}

/// The tab from its facts.
pub fn compose_last_run(facts: &LastRunFacts, w: &LastRunWording, timezone: &str) -> LastRunDto {
    if facts.counts.total == 0 {
        return LastRunDto {
            empty: Some(w.empty.clone()),
            cards: Vec::new(),
            progress: String::new(),
            columns: Vec::new(),
            steps: Vec::new(),
            summary: String::new(),
            warnings: Vec::new(),
        };
    }
    let (steps, warnings) = step_rows(&facts.steps, w);
    LastRunDto {
        empty: None,
        cards: cards(facts, w, timezone),
        progress: progress(facts, w),
        columns: vec![
            w.col_step.clone(),
            w.col_avg.clone(),
            w.col_runs.clone(),
            w.col_failed.clone(),
        ],
        steps,
        summary: summary(&facts.steps, w, timezone),
        warnings,
    }
}

fn cards(facts: &LastRunFacts, w: &LastRunWording, timezone: &str) -> Vec<LastRunCardDto> {
    let done = facts.counts.finished.to_string();
    let total = facts.counts.total.to_string();
    let quotes = facts
        .quote_pct
        .map_or_else(|| w.quotes_none.clone(), |p| format!("{p:.1}%"));
    let mut out = vec![
        LastRunCardDto {
            value: render(&w.finished_value, &[("done", &done), ("total", &total)]),
            label: w.finished_label.clone(),
        },
        LastRunCardDto {
            value: quotes,
            label: w.quotes_label.clone(),
        },
    ];
    if let Some(at) = facts.last_activity {
        let clock = local_clock(at, timezone);
        out.push(LastRunCardDto {
            value: local_date(at, timezone),
            label: render(&w.when_label, &[("clock", &clock)]),
        });
    }
    out
}

/// The estimate appears only while a step is running (board 5).
fn progress(facts: &LastRunFacts, w: &LastRunWording) -> String {
    if facts.running == 0 {
        return w.idle.clone();
    }
    let left = (facts.counts.total - facts.counts.finished).max(0);
    let count = left.to_string();
    match facts.avg_doc_secs {
        Some(secs) => {
            // `as f64` on a count of documents: exact for any count this store holds.
            let time = duration(secs * left as f64);
            render(&w.running, &[("count", &count), ("time", &time)])
        }
        None => render(&w.running_no_estimate, &[("count", &count)]),
    }
}

/// Known steps in running order, then any others by name, each with a warning.
fn step_rows(stats: &[StepStats], w: &LastRunWording) -> (Vec<LastRunStepDto>, Vec<String>) {
    let mut ordered: Vec<&StepStats> = stats.iter().collect();
    ordered.sort_by_key(|s| {
        let at = STEP_ORDER
            .iter()
            .position(|n| *n == s.step_name)
            .unwrap_or(usize::MAX);
        (at, s.step_name.clone())
    });
    let mut warnings = Vec::new();
    let rows = ordered
        .into_iter()
        .map(|s| {
            let label = step_label(&s.step_name, w).unwrap_or_else(|| {
                warnings.push(render(&w.unknown_step, &[("step", &s.step_name)]));
                s.step_name.clone()
            });
            LastRunStepDto {
                label,
                avg: s.avg_secs.map_or_else(|| w.quotes_none.clone(), duration),
                runs: s.runs.to_string(),
                failed: s.failed.to_string(),
            }
        })
        .collect();
    (rows, warnings)
}

/// The plain name of a step, or `None` for one this page does not know.
pub fn step_label(step: &str, w: &LastRunWording) -> Option<String> {
    let name = match step {
        "upload" => &w.step_upload,
        "extract_text" => &w.step_extract_text,
        "llm_extract_pass1" => &w.step_llm_extract_pass1,
        "llm_extract_pass2" => &w.step_llm_extract_pass2,
        "verify" => &w.step_verify,
        "auto_approve" => &w.step_auto_approve,
        "ingest" => &w.step_ingest,
        "ingest_delta" => &w.step_ingest_delta,
        "index" => &w.step_index,
        "completeness" => &w.step_completeness,
        _ => return None,
    };
    Some(name.clone())
}

fn summary(stats: &[StepStats], w: &LastRunWording, timezone: &str) -> String {
    let runs: i64 = stats.iter().map(|s| s.runs).sum();
    let failed: i64 = stats.iter().map(|s| s.failed).sum();
    let runs = runs.to_string();
    if failed == 0 {
        return render(&w.summary_clean, &[("runs", &runs)]);
    }
    let mut failures: Vec<(&StepStats, DateTime<Utc>)> = stats
        .iter()
        .filter_map(|s| s.last_failed_at.map(|at| (s, at)))
        .collect();
    failures.sort_by_key(|(s, _)| STEP_ORDER.iter().position(|n| *n == s.step_name));
    let detail = failures
        .iter()
        .map(|(s, at)| {
            let step = step_label(&s.step_name, w).unwrap_or_else(|| s.step_name.clone());
            let day = local_day_month(*at, timezone);
            render(&w.failure_detail, &[("step", &step), ("day", &day)])
        })
        .collect::<Vec<_>>()
        .join("; ");
    let failed = failed.to_string();
    render(
        &w.summary,
        &[("runs", &runs), ("failed", &failed), ("detail", &detail)],
    )
}

/// `342 ms`, `15.5 s`, `5 m 24 s`, `1 h 12 m` — board 5's forms.
///
/// The unit letters are notation, not wording, the way a clock's colon is.
pub fn duration(secs: f64) -> String {
    // STRUCTURAL: seconds per minute and per hour.
    const MINUTE: f64 = 60.0;
    // STRUCTURAL: see MINUTE.
    const HOUR: f64 = 3600.0;
    if secs < 1.0 {
        return format!("{:.0} ms", secs * 1000.0);
    }
    if secs < MINUTE {
        return format!("{secs:.1} s");
    }
    // Rounded to whole seconds first so 59.6 s past a minute never prints "60 s".
    let whole = secs.round();
    if whole < HOUR {
        return format!("{} m {} s", (whole / MINUTE).floor(), whole % MINUTE);
    }
    format!(
        "{} h {} m",
        (whole / HOUR).floor(),
        ((whole % HOUR) / MINUTE).floor()
    )
}

#[cfg(test)]
#[path = "last_run_tests.rs"]
mod tests;
