//! Validating a coupled group's submitted entries, and encoding them for the store.
//!
//! Pure: no pool, no I/O. It takes what the editor submitted and either names
//! what is wrong with it or hands back the rows to write.
//!
//! ## ⚑ The length mismatch is not validated here — it is unrepresentable
//!
//! The defect this fixes was two lists that had to be the same length and were
//! saved one at a time. The obvious fix is to check the lengths on the way in.
//! The better one is the shape: an entry carries ONE cell per column, so
//! "logins has two and names has one" cannot be submitted, cannot be spelled,
//! and needs no check. `entries: Vec<Vec<String>>` with a fixed arity per row is
//! the whole of it.
//!
//! What remains below are the rules the SHAPE cannot enforce, and every one of
//! them exists because of something `parse_verbatim_list` does silently.
//!
//! ## Why each refusal is here rather than left to the reader
//!
//! `domain::settings::parse_verbatim_list` — the function that reads these rows
//! back — drops blank entries, drops duplicates, and treats the lone word `none`
//! as an empty list. Every one of those is right for a reader and wrong for a
//! writer:
//!
//! | Submitted | The reader would | The operator would see |
//! |---|---|---|
//! | a blank cell | drop it | one list silently shorter — a boot refusal later, or the wrong name beside a queue |
//! | the same login twice | drop the second | the OTHER column keeps both, and the lengths no longer match |
//! | a cell containing `,` | split it into two entries | the columns drift apart by one, from a name like `Smith, Jr.` |
//! | one entry reading `none` | return an EMPTY list | a bench with nobody on it, silently |
//!
//! Each is refused at the door, naming the row and the column, because by the
//! time the reader has normalised them the operator's mistake is invisible.

use crate::services::settings_groups::CoupledGroup;

/// The separator the store encodes these lists with.
// STRUCTURAL: the stored wire format of a list row, fixed by
// `domain::settings::parse_verbatim_list`, which splits on this byte with no
// escape mechanism. Not a deployment value — changing it would re-encode every
// list row in the store.
const LIST_SEPARATOR: char = ',';

/// The stored word that means "this list is deliberately empty".
// STRUCTURAL: mirrors `domain::settings::LIST_NONE_TOKEN`, refused as an ENTRY
// here for the reason the table above gives.
const NONE_TOKEN: &str = "none";

/// Why a submitted group was refused.
///
/// Every variant names the group's editor, because the operator is looking at
/// that editor when it happens and "400" tells them nothing about which of the
/// rows in front of them is wrong.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PairError {
    #[error(
        "{group} needs at least one {noun}: with nobody listed, no one can press \
         Done reviewing and the review queue can never be cleared. Add a {noun}, \
         or leave the ones already there."
    )]
    Empty {
        group: &'static str,
        noun: &'static str,
    },

    #[error(
        "{group}: {column} is empty on {noun} {position}. Every column needs a \
         value — a blank one is dropped when the row is read back, which leaves \
         the lists a different length and the wrong name beside the queue."
    )]
    BlankCell {
        group: &'static str,
        column: &'static str,
        noun: &'static str,
        position: usize,
    },

    #[error(
        "{group}: {column} reads “{value}” on more than one {noun}. Repeats are \
         dropped when the row is read back, so the columns would stop lining up."
    )]
    Duplicate {
        group: &'static str,
        column: &'static str,
        noun: &'static str,
        value: String,
    },

    #[error(
        "{group}: {column} on {noun} {position} contains a comma (“{value}”). \
         These rows are stored as comma-separated lists with no way to escape \
         one, so the value would be read back as two."
    )]
    SeparatorInValue {
        group: &'static str,
        column: &'static str,
        noun: &'static str,
        position: usize,
        value: String,
    },

    #[error(
        "{group}: {column} on {noun} {position} reads “none”, which the store \
         uses to mean an EMPTY list. Pick another value, or remove the {noun}."
    )]
    NoneTokenAsValue {
        group: &'static str,
        column: &'static str,
        noun: &'static str,
        position: usize,
    },

    /// The only variant that signals a VERSION SKEW rather than a bad value, so
    /// its remedy is the only one that is not "fix what you typed".
    #[error(
        "{group}: {noun} {position} carries {found} value(s) but this group has \
         {expected} column(s). The editor and this build disagree about the \
         shape of a {noun} — reload the Settings page to pick up the current \
         columns, then try again."
    )]
    WrongArity {
        group: &'static str,
        noun: &'static str,
        position: usize,
        found: usize,
        expected: usize,
    },
}

/// One row of the store, ready to write: the key and its encoded list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedRow {
    pub key: &'static str,
    pub value: String,
}

/// Check the submitted entries and encode them one row per column.
///
/// `entries` is one inner vector per {noun} — a reviewer — holding one cell per
/// column, in the group's column order.
///
/// # Errors
/// [`PairError`] naming the group, the column and the position of the first
/// problem. One at a time and in order, because an operator fixes them one at a
/// time and a list of six is harder to act on than the first.
pub fn encode_entries(
    group: &CoupledGroup,
    entries: &[Vec<String>],
) -> Result<Vec<EncodedRow>, PairError> {
    if entries.is_empty() {
        return Err(PairError::Empty {
            group: group.label,
            noun: group.entry_noun,
        });
    }

    for (index, entry) in entries.iter().enumerate() {
        // `position` is 1-based: the operator is counting rows on a screen, not
        // indexing an array.
        let position = index + 1;
        if entry.len() != group.columns.len() {
            return Err(PairError::WrongArity {
                group: group.label,
                noun: group.entry_noun,
                position,
                found: entry.len(),
                expected: group.columns.len(),
            });
        }
        for (column, cell) in group.columns.iter().zip(entry) {
            check_cell(group, column.label, position, cell.trim())?;
        }
    }

    check_no_duplicates(group, entries)?;

    Ok(group
        .columns
        .iter()
        .enumerate()
        .map(|(at, column)| EncodedRow {
            key: column.key,
            value: entries
                .iter()
                .map(|entry| entry[at].trim())
                .collect::<Vec<_>>()
                .join(&LIST_SEPARATOR.to_string()),
        })
        .collect())
}

/// The four rules one cell must satisfy.
fn check_cell(
    group: &CoupledGroup,
    column: &'static str,
    position: usize,
    cell: &str,
) -> Result<(), PairError> {
    if cell.is_empty() {
        return Err(PairError::BlankCell {
            group: group.label,
            column,
            noun: group.entry_noun,
            position,
        });
    }
    if cell.contains(LIST_SEPARATOR) {
        return Err(PairError::SeparatorInValue {
            group: group.label,
            column,
            noun: group.entry_noun,
            position,
            value: cell.to_string(),
        });
    }
    if cell.eq_ignore_ascii_case(NONE_TOKEN) {
        return Err(PairError::NoneTokenAsValue {
            group: group.label,
            column,
            noun: group.entry_noun,
            position,
        });
    }
    Ok(())
}

/// No column may repeat a value.
///
/// ## Why EVERY column and not only the identity one
///
/// The obvious rule is "logins must be unique". The reader is stricter than
/// that: `parse_verbatim_list` de-duplicates whatever list it is given, so two
/// reviewers who happen to share a display name would collapse that column to
/// one entry while the logins column kept both — and the store would then refuse
/// to boot, naming a length mismatch nobody typed.
fn check_no_duplicates(group: &CoupledGroup, entries: &[Vec<String>]) -> Result<(), PairError> {
    for (at, column) in group.columns.iter().enumerate() {
        let mut seen: Vec<&str> = Vec::new();
        for entry in entries {
            let cell = entry[at].trim();
            if seen.contains(&cell) {
                return Err(PairError::Duplicate {
                    group: group.label,
                    column: column.label,
                    noun: group.entry_noun,
                    value: cell.to_string(),
                });
            }
            seen.push(cell);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "settings_pair_tests.rs"]
mod tests;
