// Shared fixtures for the two `matrix_order` test files.
//
// Split out when the suite passed 300 lines (Rule 17). The two halves test
// genuinely different things — the ORDER §3 imposes, and the RULES that decide
// what is in the list at all — and both build items the same way, so the builder
// lives here rather than in either.

use super::*;

/// A minimal item: unranked, unrated, undated, unruled, no role.
///
/// Every test starts here and sets only the field it is about, so a test that
/// passes for the wrong reason is visible in its own body.
pub(super) fn item(id: &'static str) -> OrderInput<'static> {
    OrderInput {
        id,
        rank: None,
        role: None,
        confidence: None,
        duplicate_of: None,
        document_date: None,
        ruling: None,
    }
}

/// The visible rows, in order.
pub(super) fn visible_ids(ordered: &[OrderedItem]) -> Vec<&str> {
    ordered
        .iter()
        .filter(|o| o.hidden_reason.is_none())
        .map(|o| o.id.as_str())
        .collect()
}

/// The hidden rows, in order.
pub(super) fn hidden_ids(ordered: &[OrderedItem]) -> Vec<&str> {
    ordered
        .iter()
        .filter(|o| o.hidden_reason.is_some())
        .map(|o| o.id.as_str())
        .collect()
}
