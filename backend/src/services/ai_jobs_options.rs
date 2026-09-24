//! What each AI job's dropdowns may offer, and what is wrong with the value in
//! force (CC_TASK_MODEL_JOBS_PANEL_v1, board 2 and board 4).
//!
//! Pure: every function takes what the store and the template folder already
//! said, so each rule is tested with plain values and no database.
//!
//! ## Domain note: each list is the set of models that job's CODE can call
//!
//! A dropdown that offers a model the job cannot use is the trap this panel
//! exists to remove. So each rule below is copied from the code path that makes
//! the call, not from a preference:
//!
//! - Discuss chat: active AND "Quotes checked" (`grounded`) — the check
//!   `chat_model_check::unusable_reason` runs on save and at boot.
//! - Answer analysis: any active model — `practice_model_call::resolve_model`
//!   builds anthropic and vllm alike through `provider_for_model`.
//! - Chat page: active AND Anthropic — `main::build_chat_providers` skips every
//!   other provider.
//! - Theme scan: active AND offered for scans — `list_scan_eligible_models`.

use std::collections::HashSet;

use crate::domain::llm_effort::{Effort, ABSENT};
use crate::domain::wording_ai_jobs::AiJobsWording;
use crate::dto::ai_jobs::AiJobOptionDto;
use crate::repositories::pipeline_repository::models::LlmModelRecord;

// STRUCTURAL: the job tokens migration 20260924145150 writes into
// `app_settings.ai_job`. The code and the store must spell them identically;
// renaming one is a migration, never a deployment choice.
pub const JOB_DISCUSS_CHAT: &str = "discuss_chat";
// STRUCTURAL: a job token, for the reason given on `JOB_DISCUSS_CHAT`.
pub const JOB_DISCUSS_EFFORT: &str = "discuss_effort";
// STRUCTURAL: a job token, for the reason given on `JOB_DISCUSS_CHAT`.
pub const JOB_CASE_STORY: &str = "case_story";
// STRUCTURAL: a job token, for the reason given on `JOB_DISCUSS_CHAT`.
pub const JOB_ANSWER_ANALYSIS: &str = "answer_analysis";
// STRUCTURAL: a job token, for the reason given on `JOB_DISCUSS_CHAT`.
pub const JOB_CHAT_PAGE: &str = "chat_page";
// STRUCTURAL: a job token, for the reason given on `JOB_DISCUSS_CHAT`.
pub const JOB_THEME_SCAN: &str = "theme_scan";

/// The order the panel shows the jobs it knows (board 1, top to bottom). A job
/// the store names and this list does not is shown after them — it is never
/// hidden.
// STRUCTURAL: the approved board's row sequence over the job tokens above; it
// changes only with the design and those tokens, never per deployment.
pub const JOB_ORDER: &[&str] = &[
    JOB_DISCUSS_CHAT,
    JOB_DISCUSS_EFFORT,
    JOB_CASE_STORY,
    JOB_ANSWER_ANALYSIS,
    JOB_CHAT_PAGE,
    JOB_THEME_SCAN,
];

// STRUCTURAL: the `llm_models.provider` token chat dispatch accepts
// (`main::build_chat_providers`). A wire vocabulary, not a tunable.
const ANTHROPIC_PROVIDER: &str = "anthropic";

/// Which models a job's code can call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelRule {
    /// Active and "Quotes checked".
    Grounded,
    /// Active, any provider.
    Active,
    /// Active and Anthropic.
    Anthropic,
    /// Active and offered for scans.
    Scan,
}

/// Why the model in force cannot serve its job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelProblem {
    Missing,
    Off,
    Unquoted,
    Local,
    NotScan,
}

impl ModelRule {
    /// The rule for a job token. An unknown job gets the loosest honest rule:
    /// any active model.
    pub fn for_job(job: &str) -> Self {
        match job {
            JOB_DISCUSS_CHAT => Self::Grounded,
            JOB_CHAT_PAGE => Self::Anthropic,
            JOB_THEME_SCAN => Self::Scan,
            _ => Self::Active,
        }
    }

    /// What is wrong with `model` for this job, or `None` when it can serve.
    pub fn problem(
        self,
        model: Option<&LlmModelRecord>,
        scan_ids: &HashSet<String>,
    ) -> Option<ModelProblem> {
        let Some(m) = model else {
            return Some(ModelProblem::Missing);
        };
        if !m.is_active {
            return Some(ModelProblem::Off);
        }
        match self {
            Self::Grounded if !m.grounded => Some(ModelProblem::Unquoted),
            Self::Anthropic if m.provider != ANTHROPIC_PROVIDER => Some(ModelProblem::Local),
            Self::Scan if !scan_ids.contains(&m.id) => Some(ModelProblem::NotScan),
            _ => None,
        }
    }

    /// The dropdown's last line for this rule.
    pub fn foot(self, w: &AiJobsWording) -> String {
        match self {
            Self::Grounded => w.foot_grounded.clone(),
            Self::Active => w.foot_active.clone(),
            Self::Anthropic => w.foot_anthropic.clone(),
            Self::Scan => w.foot_scan.clone(),
        }
    }
}

/// The models a job may pick, with the one in force marked.
///
/// The value in force is ALWAYS listed, even when the rule would exclude it: a
/// dropdown that silently dropped it would show some other model as chosen. The
/// warning under the row says what is wrong with it.
pub fn model_options(
    rule: ModelRule,
    models: &[LlmModelRecord],
    scan_ids: &HashSet<String>,
    current: &str,
) -> Vec<AiJobOptionDto> {
    let mut out: Vec<AiJobOptionDto> = models
        .iter()
        .filter(|m| m.id == current || rule.problem(Some(m), scan_ids).is_none())
        .map(|m| AiJobOptionDto {
            value: m.id.clone(),
            label: m.display_name.clone(),
            marker: None,
            current: m.id == current,
        })
        .collect();
    if !out.iter().any(|o| o.current) {
        out.push(AiJobOptionDto {
            value: current.to_string(),
            label: current.to_string(),
            marker: None,
            current: true,
        });
    }
    out
}

/// `question_chat_prompt_v12.md` → `("question_chat_prompt_v", 12)`.
///
/// Hand-parsed because production code in this crate cannot use `regex` (it is a
/// dev-dependency only). `None` for any name not shaped `<stem>_v<digits>.md`.
pub fn split_version(file: &str) -> Option<(&str, u32)> {
    let stem = file.strip_suffix(".md")?;
    let at = stem.rfind("_v")?;
    let digits = &stem[at + 2..];
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    // best-effort: only a version too long for a u32 fails here; such a name is
    // treated as unversioned (listed alone), which is what `None` means.
    Some((&stem[..at + 2], digits.parse().ok()?))
}

/// The versions of the file in force that sit in the instructions folder, each
/// with its marker (STOP 5's rule, accepted 2026-09-24):
///
/// - the file in force → "in use";
/// - a file the change history shows this setting held → "used before";
/// - a higher version the history has never recorded → "new, never used";
/// - a lower one the history has not recorded → "earlier version" (a migration
///   may have used it; the history records only changes made in the app).
pub fn instruction_options(
    current: &str,
    files: &[String],
    used_before: &HashSet<String>,
    w: &AiJobsWording,
) -> Vec<AiJobOptionDto> {
    let Some((stem, in_force)) = split_version(current) else {
        return vec![in_use(current, w)];
    };
    let mut family: Vec<(u32, &String)> = files
        .iter()
        .filter_map(|f| match split_version(f) {
            Some((s, n)) if s == stem => Some((n, f)),
            _ => None,
        })
        .collect();
    family.sort();
    let mut out: Vec<AiJobOptionDto> = family
        .into_iter()
        .map(|(n, f)| {
            if f == current {
                return in_use(current, w);
            }
            let marker = if used_before.contains(f) {
                &w.marker_used_before
            } else if n > in_force {
                &w.marker_new
            } else {
                &w.marker_earlier
            };
            AiJobOptionDto {
                value: f.clone(),
                label: f.clone(),
                marker: Some(marker.clone()),
                current: false,
            }
        })
        .collect();
    if !out.iter().any(|o| o.current) {
        out.push(in_use(current, w));
    }
    out
}

fn in_use(file: &str, w: &AiJobsWording) -> AiJobOptionDto {
    AiJobOptionDto {
        value: file.to_string(),
        label: file.to_string(),
        marker: Some(w.marker_in_use.clone()),
        current: true,
    }
}

/// The thinking levels the store accepts (`parse_stored_effort`), in plain
/// words. "absent" is offered only when it is the value in force — it is how an
/// install says "send none", not a level anyone picks (ruling Q5: all five).
pub fn effort_options(current: &str, w: &AiJobsWording) -> Vec<AiJobOptionDto> {
    let mut out: Vec<AiJobOptionDto> = Effort::ALL
        .iter()
        .map(|e| AiJobOptionDto {
            value: e.as_wire().to_string(),
            label: effort_label(Some(*e), w),
            marker: None,
            current: e.as_wire() == current,
        })
        .collect();
    if current.eq_ignore_ascii_case(ABSENT) {
        out.push(AiJobOptionDto {
            value: ABSENT.to_string(),
            label: effort_label(None, w),
            marker: None,
            current: true,
        });
    }
    out
}

/// The plain name of one thinking level; `None` is "the model's own default".
pub fn effort_label(effort: Option<Effort>, w: &AiJobsWording) -> String {
    match effort {
        None => w.effort_unset.clone(),
        Some(Effort::Low) => w.effort_low.clone(),
        Some(Effort::Medium) => w.effort_medium.clone(),
        Some(Effort::High) => w.effort_high.clone(),
        Some(Effort::XHigh) => w.effort_xhigh.clone(),
        Some(Effort::Max) => w.effort_max.clone(),
    }
}

#[cfg(test)]
#[path = "ai_jobs_options_tests.rs"]
pub(crate) mod tests;
