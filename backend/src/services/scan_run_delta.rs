//! The "+N since the previous scan" delta (task 1.7C, ruling R2).
//!
//! ## The question this module refuses to answer dishonestly
//!
//! Design §2.3 asks the scan history for a **New** column and the last-run line
//! for an "N new" count. Phase A measured what that could possibly mean, and the
//! answer was uncomfortable: **a scan does not create candidates.**
//! `theme_scan::read_candidates` calls `BiasRepository::all_evidence_about_subject`
//! — it READS the graph pool and judges it. The pool grows only when documents are
//! processed, which the embargo forbids. So "candidates this run added" is
//! structurally zero, forever, and rendering "3 new" would be the page inventing
//! a quantity.
//!
//! Ruling R2 (Roman, 2026-08-03) settled the honest reading: the delta is how much
//! bigger the pool this run read was than the pool the previous run read. On the
//! live DEV history that is `+54 / 0 / — / —`, which is true and useful.
//!
//! ## The two rules that keep it honest
//!
//! 1. **A run with no recorded pool read is not a baseline.** `candidates_read`
//!    is `NOT NULL`, so a run that failed before reading the pool stores `0` — and
//!    `0` there means "read nothing", not "read a pool of zero". Skipping it means
//!    the delta compares two real measurements. Dry runs are NOT skipped: a dry
//!    run's pool read is a real measurement of the pool (R2, explicitly).
//! 2. **The first measurable run has no delta at all** — `None`, rendered as an em
//!    dash in the table and omitted from the meta line. Never `0`: "the pool did
//!    not change" and "there is nothing to compare against" are different facts,
//!    and only one of them is reassuring.
//!
//! The delta is SIGNED. A negative value is not a bug to clamp away: after task
//! 2.5's re-anchoring the pool can legitimately shrink, and a `-12` that says so is
//! worth more than a `0` that hides it.

use crate::dto::ScanRunHeader;

/// One history row's delta against the previous measurable run.
///
/// `None` means there is no earlier run that recorded a pool read — the first
/// measurable scan in the scenario's history.
pub type PoolDelta = Option<i32>;

/// Whether this run recorded a real pool read, and can therefore serve as a
/// baseline for the run after it.
///
/// `candidates_read == 0` is the "never got there" case (see the module doc).
fn recorded_a_pool_read(run: &ScanRunHeader) -> bool {
    run.candidates_read > 0
}

/// Compute the pool delta for every run in a history list.
///
/// `runs` arrives **newest first** — the order `list_scan_runs` returns and the
/// order the history table renders. The returned vector is parallel to it: entry
/// `i` is the delta for `runs[i]`.
///
/// ## Rust Learning: `enumerate()` + a tail slice instead of a `windows(2)` pair
///
/// `windows(2)` would give each run its immediate neighbour, which is wrong here:
/// the baseline is the next run that recorded a pool read, and there may be
/// several unmeasurable ones in between. Indexing lets the search scan forward
/// from `index + 1` — and `runs[index + 1..]` on the last element is the empty
/// slice rather than a panic, so the oldest run needs no special case.
fn pool_deltas(runs: &[ScanRunHeader]) -> Vec<PoolDelta> {
    runs.iter()
        .enumerate()
        .map(|(index, run)| {
            if !recorded_a_pool_read(run) {
                // A run that never read the pool has no delta of its own to show.
                return None;
            }
            // The baseline is the next OLDER run that recorded a read. `runs` is
            // newest-first, so "older" is further along the slice.
            runs[index + 1..]
                .iter()
                .find(|previous| recorded_a_pool_read(previous))
                .map(|previous| run.candidates_read - previous.candidates_read)
        })
        .collect()
}

/// Fill in each run's `pool_delta` before the history goes on the wire.
///
/// ## Why the backend computes this and not the browser
///
/// Standing Rule 12 — no business logic in the frontend. "How much did the pool
/// grow" is a derivation over the whole history with two non-obvious rules in it
/// (skip runs that never read the pool; the first measurable run gets no delta at
/// all rather than zero). A browser reimplementing those rules is a browser that
/// will eventually disagree with the backend about what the pool did, and the
/// human has no way to tell which one is lying.
///
/// The delta is a property of a run's POSITION in the history, not of the run
/// itself — delete an older scan and the deltas change. That is why it is
/// computed here, at list assembly, and why nothing that constructs a
/// `ScanRunHeader` in isolation is expected to know it.
pub fn with_pool_deltas(mut runs: Vec<ScanRunHeader>) -> Vec<ScanRunHeader> {
    let deltas = pool_deltas(&runs);
    for (run, delta) in runs.iter_mut().zip(deltas) {
        run.pool_delta = delta;
    }
    runs
}

#[cfg(test)]
#[path = "scan_run_delta_tests.rs"]
mod tests;
