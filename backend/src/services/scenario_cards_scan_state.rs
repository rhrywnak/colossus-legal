//! Has anything ever scanned this scenario? (task 2.15, piece 3.)
//!
//! One question, asked by the cards endpoint, answered from `scan_runs`.
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
use crate::repositories::pipeline_repository::count_completed_scan_runs;
use crate::state::AppState;

/// The never-scanned notice, or `None` when at least one scan has completed.
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
    let completed = count_completed_scan_runs(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %scenario_id,
                "failed to read the scan-run history while composing the candidate cards");
            AppError::Internal {
                message: "failed to read the scenario's scan history".to_string(),
            }
        })?;

    let notice = notice_for(
        completed,
        &state
            .settings
            .current()
            .scenario_authoring_wording
            .never_scanned_notice,
    );
    if notice.is_some() {
        tracing::debug!(
            %scenario_id,
            "no completed scan run: serving the candidate pool behind the never-scanned notice"
        );
    }
    Ok(notice)
}

/// The decision itself: notice when nothing has completed, nothing when something
/// has.
///
/// Pure, and split out for that reason — the rule is one comparison, and a rule
/// only reachable through a database, a graph and a settings store is a rule
/// nothing asserts. The caller does the reading; this decides.
fn notice_for(completed_runs: i64, notice: &str) -> Option<String> {
    (completed_runs == 0).then(|| notice.to_string())
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
}
