// =============================================================================
// backend/src/domain/wording_admin.rs — the Admin area's own words, in one field
// =============================================================================
//
// CC_TASK_MODEL_JOBS_PANEL_v1 (Stage A). Three stored-string blocks speak on the
// Admin pages: the jobs panel on Overview, each job's name and cost line, and
// Data → Last document run.
//
// ## Why one nested struct and not three fields on `Settings`
//
// `domain/settings.rs` sits a handful of lines under Rule 17's 300-line limit.
// The practice blocks solved the same problem the same way: `Settings` gains ONE
// field, and the blocks hang off it (see `wording_practice`'s header).

use super::wording_ai_job_rows::{build_ai_job_rows_wording, AiJobRowsWording};
use super::wording_ai_jobs::{build_ai_jobs_wording, AiJobsWording};
use super::wording_last_run::{build_last_run_wording, LastRunWording};

/// The Admin area's stored words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminWording {
    /// The jobs panel on Admin → Overview.
    pub ai_jobs: AiJobsWording,
    /// Each AI job's name, description, warning subject and cost line.
    pub job_rows: AiJobRowsWording,
    /// Admin → Data → Last document run.
    pub last_run: LastRunWording,
}

/// Build all three blocks with one reader, or name the first key that is wrong.
///
/// ## Rust Learning: passing the same closure three times
///
/// `read` is taken by `&impl Fn`, so each builder borrows it rather than taking
/// ownership. A plain `impl Fn` parameter would be MOVED into the first call and
/// the second call would not compile.
///
/// # Errors
/// Whatever `read` returns for the first key that is missing, mis-kinded or blank.
pub fn build_admin_wording<E>(
    read: &impl Fn(&str) -> Result<String, E>,
) -> Result<AdminWording, E> {
    Ok(AdminWording {
        ai_jobs: build_ai_jobs_wording(read)?,
        job_rows: build_ai_job_rows_wording(read)?,
        last_run: build_last_run_wording(read)?,
    })
}

#[cfg(test)]
impl AdminWording {
    /// The fixture, built from each block's own fixture.
    pub fn for_test() -> Self {
        AdminWording {
            ai_jobs: AiJobsWording::for_test(),
            job_rows: AiJobRowsWording::for_test(),
            last_run: LastRunWording::for_test(),
        }
    }
}
