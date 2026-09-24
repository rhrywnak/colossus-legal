//! The wire shape of Admin → Overview's jobs panel (CC_TASK_MODEL_JOBS_PANEL_v1).
//!
//! Every string here is a finished sentence or a finished label. The browser
//! draws them and decides nothing (Standing Rule 12): which models a job may
//! use, which file is "in use", what a warning says — all of it is worked out by
//! `services::ai_jobs_compose` before it leaves the server.

use serde::Serialize;

/// The whole panel.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AiJobsPanelDto {
    pub title: String,
    pub intro: String,
    pub save_label: String,
    /// One per job, in the order the panel shows them.
    pub jobs: Vec<AiJobDto>,
    /// The line of links under the panel.
    pub links: Vec<AiJobsLinkDto>,
    /// Settings that name a model but belong to no job yet — listed so that a
    /// future one cannot hide in Settings (Law 22). Empty on a tidy store.
    pub others_title: String,
    pub others: Vec<String>,
}

/// One AI job: a row of the panel.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AiJobDto {
    /// The stable job token, for the page's keys and tests. Never shown.
    pub job: String,
    pub title: String,
    pub description: String,
    /// The first control column: the model, or the thinking level.
    pub first: AiJobControlDto,
    /// The second control column: the instructions file.
    pub instructions: AiJobControlDto,
    /// Who changed it and when, plus the job's cost line.
    pub meta: String,
    /// What is broken and what to do, one sentence each. Empty when healthy.
    pub warnings: Vec<String>,
    /// "was Claude Opus 5" — set when the app's history holds a change to undo.
    pub was: Option<String>,
    /// The Switch back control's label, beside `was`; `None` exactly when `was` is.
    pub switch_back: Option<String>,
}

/// One control column of a job row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AiJobControlDto {
    /// The small label over the control ("Model", "Instructions", "Thinking").
    pub label: String,
    #[serde(flatten)]
    pub body: AiJobControlBody,
}

/// What sits under the label.
///
/// ## Rust Learning: an internally tagged enum on the wire
///
/// `#[serde(tag = "kind")]` writes each variant as an object with a `"kind"`
/// field naming it — `{"kind":"choose", …}` or `{"kind":"fixed", "text": …}`. The
/// TypeScript side mirrors it as a discriminated union and `switch`es on `kind`,
/// so the compiler checks both ends handle every variant.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AiJobControlBody {
    /// A dropdown.
    Choose {
        /// The settings key this control writes (Stage B's Save).
        key: String,
        /// The stored value in force now.
        current: String,
        /// What the closed dropdown shows.
        current_label: String,
        options: Vec<AiJobOptionDto>,
        /// The dropdown's last line: why anything is missing.
        foot: Option<String>,
    },
    /// No dropdown: the words say why ("Fixed by the server: …", "Built in, …").
    Fixed { text: String },
    /// Nothing to choose for this job ("—").
    Empty { text: String },
}

/// One entry of a dropdown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AiJobOptionDto {
    /// The value Save would write.
    pub value: String,
    /// What the entry says.
    pub label: String,
    /// "in use", "used before", "new, never used", "earlier version" — or none.
    pub marker: Option<String>,
    /// True for the value in force now.
    pub current: bool,
}

/// One link under the panel. `target` is a stable token the page maps to a
/// route, so no address is composed on the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AiJobsLinkDto {
    pub target: String,
    pub lead: String,
    pub label: String,
}

/// `PUT /api/admin/ai-jobs/:job` — the controls whose values Save changes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiJobSaveRequest {
    pub changes: Vec<AiJobChangeDto>,
}

/// One setting of the job, and the value to write.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiJobChangeDto {
    pub key: String,
    pub value: String,
}

/// The answer to Save or Switch back: board 3's confirmation, and the panel as
/// it now stands.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AiJobSavedDto {
    /// The job that changed (for the confirmation's Switch back).
    pub job: String,
    /// "Discuss chat now uses …, from the next use. The next question reloads…"
    pub confirmation: String,
    /// The Switch back label shown beside the confirmation.
    pub switch_back: String,
    pub panel: AiJobsPanelDto,
}
