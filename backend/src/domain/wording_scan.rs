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

    // ── Scan → ruling (2026-08-08) ───────────────────────────────────────────
    /// The one line a COLLAPSED scan card shows. Carries `{when}`, `{model}` and
    /// `{count}`.
    ///
    /// Domain note: the scan card collapses once a run exists because the scan is
    /// no longer where the work happens — the queue below it is. This sentence has
    /// to carry enough that a human never expands the card just to find out
    /// whether it is worth expanding.
    pub card_collapsed_summary_template: String,
    /// The line under the scan report's heading, saying the report needs no click.
    ///
    /// Domain note: this is the epitaph of select-twice. The findings list used to
    /// be a work surface with checkboxes and a Merge button; it is now a receipt,
    /// and the sentence says so before anybody goes looking for the controls that
    /// used to be there.
    pub report_advisory_note: String,
    /// The report's live proposed line. Carries `{count}`.
    ///
    /// Domain note: kept OUT of the conservation sentence deliberately (architect
    /// ruling R5). Conservation is composed from the run's FROZEN counts and
    /// describes what that run did; "proposed" falls every time the human rules.
    /// Splicing a live number into a frozen record would make the record appear to
    /// move, so the two sentences stay separate and this one says it is live.
    pub report_proposed_line_template: String,
    /// The five report tile captions, in the order the tiles are shown.
    pub report_tile_gathered: String,
    pub report_tile_folded: String,
    pub report_tile_set_aside: String,
    pub report_tile_judged: String,
    pub report_tile_proposed: String,
}

// KEYS: the stable identifiers of the three stored strings. Renaming one is a
// migration, and until it runs the boot loader refuses to start.
pub(crate) const KEY_CONSERVATION_LINE: &str = "scan_conservation_line_template";
pub(crate) const KEY_HISTORY_VIEW_LABEL: &str = "scan_history_view_label";
pub(crate) const KEY_HISTORY_DELETE_CONFIRM: &str = "scan_history_delete_confirm_template";
pub(crate) const KEY_CARD_COLLAPSED_SUMMARY: &str = "scan_card_collapsed_summary_template";
pub(crate) const KEY_REPORT_ADVISORY_NOTE: &str = "scan_report_advisory_note";
pub(crate) const KEY_REPORT_PROPOSED_LINE: &str = "scan_report_proposed_line_template";
pub(crate) const KEY_REPORT_TILE_GATHERED: &str = "scan_report_tile_gathered";
pub(crate) const KEY_REPORT_TILE_FOLDED: &str = "scan_report_tile_folded";
pub(crate) const KEY_REPORT_TILE_SET_ASIDE: &str = "scan_report_tile_set_aside";
pub(crate) const KEY_REPORT_TILE_JUDGED: &str = "scan_report_tile_judged";
pub(crate) const KEY_REPORT_TILE_PROPOSED: &str = "scan_report_tile_proposed";

/// Every scan-wording key this build reads, so a missing one is caught at boot BY
/// NAME rather than as a blank control in front of a human mid-scan.
pub const SCAN_WORDING_KEYS: &[&str] = &[
    KEY_CONSERVATION_LINE,
    KEY_HISTORY_VIEW_LABEL,
    KEY_HISTORY_DELETE_CONFIRM,
    KEY_CARD_COLLAPSED_SUMMARY,
    KEY_REPORT_ADVISORY_NOTE,
    KEY_REPORT_PROPOSED_LINE,
    KEY_REPORT_TILE_GATHERED,
    KEY_REPORT_TILE_FOLDED,
    KEY_REPORT_TILE_SET_ASIDE,
    KEY_REPORT_TILE_JUDGED,
    KEY_REPORT_TILE_PROPOSED,
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
        card_collapsed_summary_template: read(KEY_CARD_COLLAPSED_SUMMARY)?,
        report_advisory_note: read(KEY_REPORT_ADVISORY_NOTE)?,
        report_proposed_line_template: read(KEY_REPORT_PROPOSED_LINE)?,
        report_tile_gathered: read(KEY_REPORT_TILE_GATHERED)?,
        report_tile_folded: read(KEY_REPORT_TILE_FOLDED)?,
        report_tile_set_aside: read(KEY_REPORT_TILE_SET_ASIDE)?,
        report_tile_judged: read(KEY_REPORT_TILE_JUDGED)?,
        report_tile_proposed: read(KEY_REPORT_TILE_PROPOSED)?,
    })
}

#[cfg(test)]
#[path = "wording_scan_tests.rs"]
pub(crate) mod tests;
