//! Has anything ever scanned this scenario? (task 2.15, piece 3.)
//!
//! One question, asked by the cards endpoint, answered from `scan_runs`.
//!
//! ## What the question became on 2026-09-11
//!
//! It was "has any run COMPLETED?" and it is now "is the history EMPTY?". The
//! old form had a false state a human could reach: on PROD S-13 a scenario whose
//! only run was cancelled answered "no completed runs" and was shown "No scan has
//! run yet" directly above a history table listing that run. Both statements came
//! out of the same database and only one of them was true.
//!
//! The fix is not a better sentence, it is one fewer source. The facts header now
//! renders EITHER the newest run's own line or this notice, and the choice is
//! this count — so the pair that contradicted each other cannot both be composed.
//! `count_scan_runs` carries why the old "completed only" reasoning does not
//! survive the change, and why the parentage rule it protected is untouched.
//!
//! ## Why the page cannot answer it for itself
//!
//! Two numbers on the working page LOOK like they answer it and do not:
//!
//! * the queue's "Candidates awaiting ruling — N" is derived from the card pool,
//!   which is every statement about the subject and has nothing to do with scans;
//! * the filter facet "Never scanned (N)" counts cards whose confidence band is
//!   `unscored`, and a card only gains a confidence when a scan verdict is
//!   MERGED onto it. A scenario scanned three times with nothing merged still
//!   counts every card as never scanned.
//!
//! Measured on 2026-08-07: a freshly created scenario led with 148 raw pool cards
//! under "Candidates awaiting ruling — 148 · from all scans", on the same screen
//! that reported all 148 never scanned. Both numbers were arithmetically right and
//! the sentence between them was false. The only truthful source is the run
//! history, which lives in the pipeline database — so the answer is served, not
//! inferred in the browser.
//!
//! ## Why a NOTICE rather than a boolean
//!
//! Exactly the shape of the 2026-08-07 no-target fix beside it: the payload
//! carries the sentence, from a stored row, and the browser renders what it is
//! given. A boolean would put the wording back in the React component and let two
//! surfaces phrase one state two ways.

use uuid::Uuid;

use crate::error::AppError;
use crate::repositories::pipeline_repository::count_scan_runs;
use crate::state::AppState;

/// The never-scanned notice, or `None` when this scenario has ANY run on record.
///
/// # Errors
/// Returns [`AppError::Internal`] when the run history cannot be read. Deliberately
/// NOT degraded to "assume it has been scanned": that guess would suppress the
/// notice on exactly the page the fix exists for, and an unreadable history is a
/// real failure the operator should see (Standing Rule 1).
pub async fn never_scanned_notice(
    state: &AppState,
    scenario_id: Uuid,
) -> Result<Option<String>, AppError> {
    let runs_on_record = count_scan_runs(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %scenario_id,
                "failed to read the scan-run history while composing the candidate cards");
            AppError::Internal {
                message: "failed to read the scenario's scan history".to_string(),
            }
        })?;

    let notice = notice_for(
        runs_on_record,
        &state
            .settings
            .current()
            .scenario_authoring_wording
            .never_scanned_notice,
    );
    if notice.is_some() {
        tracing::debug!(
            %scenario_id,
            "no scan run of any status on record: serving the candidate pool behind \
             the never-scanned notice"
        );
    }
    Ok(notice)
}

/// The decision itself: notice when the history is empty, nothing when it is not.
///
/// Pure, and split out for that reason — the rule is one comparison, and a rule
/// only reachable through a database, a graph and a settings store is a rule
/// nothing asserts. The caller does the reading; this decides.
fn notice_for(runs_on_record: i64, notice: &str) -> Option<String> {
    (runs_on_record == 0).then(|| notice.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTICE: &str = "No scan has run yet. Run a theme scan above, or browse the raw \
                          evidence pool.";

    #[test]
    fn a_scenario_nothing_has_scanned_carries_the_empty_state() {
        // The measured 2026-08-07 screen: 148 cards led the page as "candidates
        // awaiting ruling" on a scenario no scan had ever judged.
        assert_eq!(notice_for(0, NOTICE), Some(NOTICE.to_string()));
    }

    #[test]
    fn one_completed_run_withdraws_the_notice_for_good() {
        // The moment a scan completes the page leads with its results instead
        // (piece 3c), and a notice saying nothing has run would contradict the
        // history table three lines above it.
        assert_eq!(notice_for(1, NOTICE), None);
        assert_eq!(notice_for(9, NOTICE), None);
    }

    #[test]
    fn a_run_that_never_completed_still_withdraws_the_notice() {
        // The PROD S-13 defect, pinned. The count reaching this function now
        // includes cancelled, failed and running rows, so ONE of them is enough:
        // the header has a run to describe, and "No scan has run yet" over a
        // history table with a row in it is the sentence this asserts cannot
        // return.
        //
        // There is no separate case to write per status — widening the count is
        // what made them one case, and that is the point of the change.
        assert_eq!(notice_for(1, NOTICE), None);
    }
}
