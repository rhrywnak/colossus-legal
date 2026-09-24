//! The wire shape of Admin → Data → Last document run
//! (CC_TASK_MODEL_JOBS_PANEL_v1, board 5). Every string is finished; the page
//! only draws it (Standing Rule 12).

use serde::Serialize;

/// The whole tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LastRunDto {
    /// Set, and everything below left empty, when no document exists yet.
    pub empty: Option<String>,
    /// The three cards: big value, small label under it.
    pub cards: Vec<LastRunCardDto>,
    /// Whether a run is in progress, and the estimate when one is.
    pub progress: String,
    /// The table's four column headings.
    pub columns: Vec<String>,
    pub steps: Vec<LastRunStepDto>,
    /// "211 step runs, 1 failed (…)".
    pub summary: String,
    /// A step this page cannot name, said out loud.
    pub warnings: Vec<String>,
}

/// One summary card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LastRunCardDto {
    pub value: String,
    pub label: String,
}

/// One row of the step table, in running order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LastRunStepDto {
    pub label: String,
    pub avg: String,
    pub runs: String,
    pub failed: String,
}
