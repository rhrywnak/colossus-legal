//! Rows that must be edited TOGETHER, and why the page cannot know which.
//!
//! Some settings rows are not independent. `practice_reviewer_usernames` and
//! `practice_reviewer_display_names` are two comma-separated lists read
//! index-aligned: the third login's name is the third name. Either one alone is
//! a valid row; the PAIR is what has to hold.
//!
//! ## The defect this module exists to end (Law 23, 2026-09-20)
//!
//! The invariant was enforced on every single-row save, so growing the bench
//! from one reviewer to two was impossible in either order — save the logins
//! first and the names list is one short; save the names first and the logins
//! are. Both refused. v2.1.14 shipped with 5,540 green tests and the feature's
//! own purpose could not be performed, because every test checked a STATE and
//! none walked the TASK.
//!
//! Law 23(b): *"coupled values are edited as ONE unit or the design is wrong."*
//! This is the declaration of which values those are.
//!
//! ## Where the invariant is enforced, stated once (Law 23(b))
//!
//! | Stage | What checks it | What happens |
//! |---|---|---|
//! | BOOT | `settings_practice::reviewer_bench`, via `build_settings` | the process refuses to start |
//! | WRITE | `settings_write::trial_snapshot`, which runs that same `build_settings` before committing | the save is refused and NOTHING is written |
//! | READ | nothing — by then the lists are already aligned | — |
//!
//! There is only ever ONE implementation of the rule. The write path does not
//! re-state it; it runs the boot check against a trial store. **How a legitimate
//! edit crosses it:** both rows are submitted together and substituted into the
//! trial together, so the store the check sees is the store the operator meant.
//!
//! ## Why the grouping is declared on the SERVER
//!
//! The Settings page names nothing specific to this application — a disk test
//! (`settingsImportRule.test.ts`) fails the build if it starts to. A page that
//! special-cased two reviewer keys would defeat that, and would have to be
//! edited again for the next coupled pair. The server declares the coupling the
//! same way it already declares the grouping, by reference to the `KEY_*` consts
//! that exist, and the browser renders whatever it is handed.

use std::collections::HashMap;

use crate::domain::practice_params::{
    KEY_PRACTICE_REVIEWER_DISPLAY_NAMES, KEY_PRACTICE_REVIEWER_USERNAMES,
};
use crate::domain::settings::parse_verbatim_list;
use crate::repositories::pipeline_repository::AppSettingRecord;

/// One column of a coupled group — a single settings row holding one list.
#[derive(Debug, Clone, Copy)]
pub struct CoupledColumn {
    /// The stored key. Always an existing `KEY_*` const, never a fresh literal.
    pub key: &'static str,
    /// The column heading in the editor.
    pub label: &'static str,
    /// What to show when the cell is empty, so a half-filled row reads as one.
    pub placeholder: &'static str,
}

/// A set of rows edited as ONE unit.
///
/// ## Why the columns are a list and not a pair
///
/// The reviewer bench is two rows, and it is the only coupled group today. It is
/// declared as N columns because the shape costs nothing and because the second
/// group — whenever it arrives — is the moment a hard-coded pair would have to
/// be rewritten. Nothing below assumes two.
#[derive(Debug, Clone, Copy)]
pub struct CoupledGroup {
    /// Stable id — what the pair route's path names and what the page routes on.
    pub id: &'static str,
    /// The heading over the editor.
    pub label: &'static str,
    /// One line saying what the group IS, shown above the rows.
    pub note: &'static str,
    /// What one entry is called, for the Add control ("Add a reviewer").
    pub entry_noun: &'static str,
    /// The columns, in the order the editor shows them.
    pub columns: &'static [CoupledColumn],
}

/// Every coupled group this build declares.
// STRUCTURAL: which rows are read index-aligned is a property of the DATA MODEL,
// not of a deployment — `settings_practice::reviewer_bench` reads these two
// together in every environment, and a store where they were uncoupled would
// refuse to boot. The `id` is an API routing contract (`PUT
// /settings/group/:group_id`), and the column `key`s are the existing `KEY_*`
// consts rather than fresh literals. What remains are the LABELS, and they are
// here for the same reason `settings_map::AREAS`' labels are: everything
// user-visible on this page is composed server-side, and the browser holds no
// copy of any of it.
pub const COUPLED_GROUPS: &[CoupledGroup] = &[CoupledGroup {
    id: "reviewer_bench",
    label: "Reviewers shown on the war room",
    note: "Each reviewer is a sign-in name and the name screens print for them. \
           They are stored as two lists read in step, so they are edited together \
           — adding one without the other is what used to be impossible. \
           These names are DISPLAY: they are who the war room and the deck bar \
           name, and whose own answers and notes do not wait for review. An \
           administrator may press “Done reviewing” whether or not they are \
           listed here.",
    entry_noun: "reviewer",
    columns: &[
        CoupledColumn {
            key: KEY_PRACTICE_REVIEWER_USERNAMES,
            label: "Sign-in name",
            placeholder: "cpenzien",
        },
        CoupledColumn {
            key: KEY_PRACTICE_REVIEWER_DISPLAY_NAMES,
            label: "Name shown on screen",
            placeholder: "Chuck",
        },
    ],
}];

/// The group that claims `key`, if any.
pub fn group_of(key: &str) -> Option<&'static CoupledGroup> {
    COUPLED_GROUPS
        .iter()
        .find(|group| group.columns.iter().any(|column| column.key == key))
}

/// The group with this id, if any. `None` is a 404, not a panic.
pub fn group_by_id(id: &str) -> Option<&'static CoupledGroup> {
    COUPLED_GROUPS.iter().find(|group| group.id == id)
}

/// Is this row edited as part of a group rather than on its own?
pub fn is_coupled(key: &str) -> bool {
    group_of(key).is_some()
}

/// The group's stored rows, decoded into the entries the editor shows.
///
/// One inner vector per entry — a reviewer — holding one cell per column, in the
/// group's column order. The transpose of how the store holds it.
///
/// ## Why the browser is handed entries and not the raw rows
///
/// The rows are comma-separated lists. Splitting them is the STORE's encoding,
/// with rules the page has no business knowing: the separator, the `none` token,
/// the de-duplication. A page that split on `,` itself would be a second
/// implementation of `parse_verbatim_list` in the one place that cannot be
/// tested against it — and it would get `Smith, Jr.` wrong on its first day.
///
/// ## ⚑ A misaligned store renders, padded, rather than refusing
///
/// Columns of different lengths cannot normally exist: the boot check refuses
/// that store, so the process would not be running. If one ever does — a hand-
/// edited database, a restore — the shorter columns are padded with blanks so
/// the editor DRAWS the misalignment instead of hiding it. This control is the
/// tool that repairs such a store, and it can only repair what it can show
/// (Law 23(c): the worst state and the edit path, not only the display states).
/// The submitted entries are then validated as normal, so the blanks cannot be
/// saved as they are.
pub fn entries_of(
    group: &CoupledGroup,
    rows: &HashMap<String, AppSettingRecord>,
) -> Vec<Vec<String>> {
    let columns: Vec<Vec<String>> = group
        .columns
        .iter()
        .map(|column| {
            rows.get(column.key)
                .map(|record| {
                    // A row this build cannot read is shown as empty rather than
                    // dropped: an empty editor over a present row is visible and
                    // fixable, a missing one is neither.
                    parse_verbatim_list(&record.key, &record.value).unwrap_or_default()
                })
                .unwrap_or_default()
        })
        .collect();

    let depth = columns.iter().map(Vec::len).max().unwrap_or(0);
    (0..depth)
        .map(|at| {
            columns
                .iter()
                .map(|column| column.get(at).cloned().unwrap_or_default())
                .collect()
        })
        .collect()
}

#[cfg(test)]
#[path = "settings_groups_tests.rs"]
mod tests;
