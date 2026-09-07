//! Small text predicates with no domain knowledge in them.
//!
//! Nothing here knows what a card, a scenario or an allegation is. That is the
//! point: the reusability checkpoint (CLAUDE.md rule 11) asks whether another
//! Colossus project could take a helper with zero changes, and these could. They
//! live in `domain` rather than in the two services that first needed them
//! because a second copy of "trim, then treat empty as absent" is a second place
//! for the rule to drift.

/// The trimmed value, or `None` when it is absent or all whitespace.
///
/// ## Rust Learning: `Option<&str>` in, `Option<String>` out
///
/// The input borrows — the caller usually has an `Option<&str>` from a database
/// row it still owns — and the output is owned, because the trimmed slice would
/// otherwise borrow from that row for as long as the result lives. `map` runs the
/// step only on `Some`, `filter` turns a `Some("")` into `None`, and
/// `map(str::to_string)` copies the surviving slice. Three combinators instead of
/// a nested `match`, and each one names what it does.
///
/// ## Why absent and blank collapse
///
/// Every caller so far renders both as nothing at all, and a row that reads the
/// same for either should not branch on the difference. The two stay
/// distinguishable where the difference means something — in the STORE, where a
/// blank string is a human having cleared a field and a NULL is nobody having
/// written one — which is exactly the Rule 1 distinction, kept at the layer that
/// can act on it.
pub fn non_blank(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
