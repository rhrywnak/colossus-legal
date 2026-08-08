// =============================================================================
// backend/src/domain/wording_scan.rs — the words the SCAN surface speaks
// =============================================================================
//
// Three stored strings introduced by task 2.15 Tier 2 (2026-08-08): the
// conservation line under a run's results, and the two controls on a scan-history
// row. They are rows and not literals for the reason every sibling module gives
// (v2 §2b, the configuration law extended from numbers to text) — a sentence a
// human reads is configuration.
//
// ## Why a new module rather than more keys on an existing bundle
//
// The same test the five siblings apply: which SURFACE speaks these, and does its
// vocabulary move independently? These belong to the scan panel — an operator's
// instrument, read while spending money on a model — while `wording` speaks to
// someone curating candidates and `wording_scenario_authoring` to someone
// defining a scenario. The scan's language will move when the scan's mechanics
// move (it just did), and it should be able to move without touching a row the
// curation queue renders.
//
// ## The pre-existing literals in `ThemeScanPanel.tsx` are LEFT ALONE
//
// That component predates the configuration law and carries dozens of them
// ("Relevant findings", "Merge selected", the merge confirm). Converting them is a
// larger change with its own migration; mixing it into this task would put
// untested string plumbing beside the mechanics Roman is waiting on. Recorded,
// not smuggled — the rule this task obeys is that anything NEW arrives as a row.

/// The three stored strings the scan surface renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanWording {
    /// The conservation line under a completed run's counts. Carries `{pool}`,
    /// `{collapsed}`, `{excluded}`, `{judged}` and `{relevant}`.
    ///
    /// Domain note: this sentence is the whole of item 1c. It is the only place a
    /// human can see that a 148-row pool became 124 judged quotes and why — and
    /// it is composed at READ time from the run's frozen counts, so editing the
    /// wording re-words every historical run without rewriting one of them.
    pub conservation_line_template: String,
    /// The per-row control that opens a stored run's results.
    ///
    /// Exists because the row was clickable and did not look it: on 2026-08-07 a
    /// completed run's findings appeared unreachable after a refresh, when in fact
    /// the row had to be guessed at. A visible control is the fix.
    pub history_view_label: String,
    /// The confirmation before a scan run is destroyed. Carries `{run}`.
    ///
    /// Domain note: deleting a run deletes its verdicts, which are the only
    /// support a human has for the rulings that came out of it. One stray click on
    /// a ✕ used to be enough.
    pub history_delete_confirm_template: String,
}

// KEYS: the stable identifiers of the three stored strings. Renaming one is a
// migration, and until it runs the boot loader refuses to start.
pub(crate) const KEY_CONSERVATION_LINE: &str = "scan_conservation_line_template";
pub(crate) const KEY_HISTORY_VIEW_LABEL: &str = "scan_history_view_label";
pub(crate) const KEY_HISTORY_DELETE_CONFIRM: &str = "scan_history_delete_confirm_template";

/// Every scan-wording key this build reads, so a missing one is caught at boot BY
/// NAME rather than as a blank control in front of a human mid-scan.
pub const SCAN_WORDING_KEYS: &[&str] = &[
    KEY_CONSERVATION_LINE,
    KEY_HISTORY_VIEW_LABEL,
    KEY_HISTORY_DELETE_CONFIRM,
];

/// Build a [`ScanWording`] from the stored rows, or say which key is wrong.
///
/// Same generic-closure shape as the five sibling builders — see
/// [`crate::domain::wording_scenario_authoring::build_scenario_authoring_wording`]
/// for why `read` is a closure over a generic error type rather than a database
/// handle.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_scan_wording<E>(read: impl Fn(&str) -> Result<String, E>) -> Result<ScanWording, E> {
    Ok(ScanWording {
        conservation_line_template: read(KEY_CONSERVATION_LINE)?,
        history_view_label: read(KEY_HISTORY_VIEW_LABEL)?,
        history_delete_confirm_template: read(KEY_HISTORY_DELETE_CONFIRM)?,
    })
}

#[cfg(test)]
#[path = "wording_scan_tests.rs"]
pub(crate) mod tests;
