//! The Settings page's "Admin" area: the words the Admin pages themselves speak.
//!
//! Split from [`super::settings_map`] because that file sits a few lines under
//! Rule 17's 300-line limit, and a new area is about twenty lines. The bundling
//! rule is unchanged: every block points at an existing `*_KEYS` const and
//! re-types nothing. `settings_map_tests` scans this file together with
//! `settings_map.rs`, so a list declared but bundled nowhere still fails.

use super::settings_map::{Area, Block};
use crate::domain::wording_ai_job_rows::AI_JOB_ROWS_WORDING_KEYS;
use crate::domain::wording_ai_jobs::AI_JOBS_WORDING_KEYS;
use crate::domain::wording_case_file::CASE_FILE_WORDING_KEYS;
use crate::domain::wording_last_run::LAST_RUN_WORDING_KEYS;

/// The Admin area (CC_TASK_MODEL_JOBS_PANEL_v1).
pub const ADMIN_AREA: Area = Area {
    id: "admin",
    label: "Admin",
    blocks: &[
        Block {
            id: "ai_jobs",
            label: "Admin → Overview: the jobs panel's words",
            keys: AI_JOBS_WORDING_KEYS,
        },
        Block {
            id: "ai_job_rows",
            label: "Admin → Overview: each job's name and cost line",
            keys: AI_JOB_ROWS_WORDING_KEYS,
        },
        Block {
            id: "last_run",
            label: "Admin → Data: the last document run",
            keys: LAST_RUN_WORDING_KEYS,
        },
        Block {
            id: "case_file",
            label: "Admin → Overview: the Chat case file box's automatic line",
            keys: CASE_FILE_WORDING_KEYS,
        },
    ],
};
