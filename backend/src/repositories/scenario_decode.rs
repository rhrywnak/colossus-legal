// =============================================================================
// backend/src/repositories/scenario_decode.rs
// =============================================================================
//
// Tightened decode helpers for the scenario repository's Row → DTO mappers.
//
// Why a separate module: these helpers were extracted from
// `scenario_repository.rs` to keep that module under the 300-line ceiling
// (Rule 17). They are a self-contained, repository-private toolkit — the
// `classify`/`require`/`render` decision logic plus the thin `Row`-decoding
// wrappers that use it.
//
// Why the tightened discipline: this deliberately tightens decode against the
// house `.ok()` / `.unwrap_or_default()` convention, which collapses BOTH
// "column null/absent" AND "present but wrong type" into None/empty. The
// scenario surface is what Chuck and Marie read at trial prep, so a malformed
// fact must fail VISIBLY (Standing Rule 1, no silent failure). This is scoped to
// new code only — the older repositories are intentionally left on the `.ok()`
// convention (Roman parked that remediation; see DECISION_LOG 2026-06-25).
//
// The classifier/`require`/`render` decision logic is split out as pure
// functions (no `Row` involved) so the error paths are unit-testable without a
// live graph — matching how the rest of the scenario repository is exercised (no
// repository unit-constructs a `neo4rs::Row`; the async queries are
// integration-tested against a graph fixture). Only the `decode_*` wrappers
// touch a `Row`, and only they are exported (`pub(super)`).
// =============================================================================

use neo4rs::{DeError, Row};

use super::scenario_repository::ScenarioRepositoryError;

/// Decide what a `row.get::<Option<String>>(col)` outcome means under the
/// tightened discipline.
///
/// ## Rust Learning: telling null from type-mismatch via `Option<T>`
///
/// `neo4rs` deserializes a Bolt `Null` into `Option<T>` as `Ok(None)`, a
/// present value of the right type as `Ok(Some(v))`, and a present value of the
/// WRONG type as `Err(DeError::InvalidType { .. })`. An entirely absent column
/// (e.g. a `RETURN` alias typo) is `Err(DeError::NoSuchProperty)`. So decoding
/// into `Option<String>` and then classifying the result is exactly the
/// three-way distinction we want.
fn classify_opt_str(
    column: &str,
    raw: Result<Option<String>, DeError>,
) -> Result<Option<String>, ScenarioRepositoryError> {
    match raw {
        // Present + correct (`Some`) or present + null (`None`) — both fine.
        Ok(value) => Ok(value),
        // Column not in the row at all: degrade to `None` like a null, per the
        // approved discipline (a legitimately-absent column is not an error).
        Err(DeError::NoSuchProperty) => Ok(None),
        // Present but the wrong Bolt type for a String — the case the `.ok()`
        // convention would silently swallow. Surface it, named, with column.
        Err(source) => Err(ScenarioRepositoryError::Decode {
            column: column.to_string(),
            source,
        }),
    }
}

/// Promote a decoded optional into a required value, or a named error.
fn require(column: &str, decoded: Option<String>) -> Result<String, ScenarioRepositoryError> {
    decoded.ok_or_else(|| ScenarioRepositoryError::MissingRequired {
        column: column.to_string(),
    })
}

/// Decode an optional string column (null/absent → `None`; wrong type → error).
pub(super) fn decode_opt_str(
    row: &Row,
    column: &str,
) -> Result<Option<String>, ScenarioRepositoryError> {
    classify_opt_str(column, row.get::<Option<String>>(column))
}

/// Decode a required string column (null/absent → `MissingRequired` error).
pub(super) fn decode_required_str(
    row: &Row,
    column: &str,
) -> Result<String, ScenarioRepositoryError> {
    require(column, decode_opt_str(row, column)?)
}

/// Decide what a `row.get::<Option<i64>>(col)` outcome means under the tightened
/// discipline — the integer analogue of [`classify_opt_str`].
///
/// ## Rust Learning: the same three-way split, a different target type
///
/// `neo4rs` deserializes a Bolt `Null` into `Option<i64>` as `Ok(None)`, a
/// present integer as `Ok(Some(i))`, a present value of the WRONG Bolt type as
/// `Err(DeError::InvalidType { .. })`, and an absent column as
/// `Err(DeError::NoSuchProperty)`. So the classification is identical to the
/// string case (present/null → `Ok`, absent → `None`, wrong-type → named
/// `Decode`); only the decoded Rust type differs.
fn classify_opt_int(
    column: &str,
    raw: Result<Option<i64>, DeError>,
) -> Result<Option<i64>, ScenarioRepositoryError> {
    match raw {
        // Present + correct (`Some`) or present + null (`None`) — both fine.
        Ok(value) => Ok(value),
        // Column not in the row at all: degrade to `None` like a null.
        Err(DeError::NoSuchProperty) => Ok(None),
        // Present but the wrong Bolt type for an i64 — surface it, named.
        Err(source) => Err(ScenarioRepositoryError::Decode {
            column: column.to_string(),
            source,
        }),
    }
}

/// Render a decoded optional integer as the wire's optional string.
///
/// Split out as a pure function (no `Row`) so the int→string rendering is
/// unit-testable on its own, matching the module's `classify`/`require` split.
fn render_opt_int_as_str(decoded: Option<i64>) -> Option<String> {
    decoded.map(|i| i.to_string())
}

/// Decode an optional INTEGER column and render it as an `Option<String>`.
///
/// ## Why an integer column is decoded into a string-typed field
///
/// In the graph, `page_number` is stored as a Bolt INTEGER, but the fact DTO's
/// wire contract for it is `Option<String>` (the frontend treats page numbers as
/// display text, consistent with how the genuinely-string `paragraph_number`
/// serializes). So we decode as `i64` under the tightened discipline, then map
/// `Some(i)` → `Some(i.to_string())` — the int-as-string seam lives HERE, in the
/// repository, so the DTO and the frontend are untouched (`"4"` on the wire).
pub(super) fn decode_opt_int_as_str(
    row: &Row,
    column: &str,
) -> Result<Option<String>, ScenarioRepositoryError> {
    let decoded = classify_opt_int(column, row.get::<Option<i64>>(column))?;
    Ok(render_opt_int_as_str(decoded))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — pure decode-decision logic (error paths included)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_passes_present_value_through() {
        let out = classify_opt_str("topic", Ok(Some("conversion".to_string())));
        assert_eq!(out.expect("ok"), Some("conversion".to_string()));
    }

    #[test]
    fn classify_null_value_is_none() {
        let out = classify_opt_str("topic", Ok(None));
        assert_eq!(out.expect("ok"), None);
    }

    #[test]
    fn classify_absent_column_is_none() {
        // An absent column (alias typo / OPTIONAL MATCH miss) degrades to None.
        let out = classify_opt_str("topic", Err(DeError::NoSuchProperty));
        assert_eq!(out.expect("ok"), None);
    }

    #[test]
    fn classify_type_mismatch_is_named_decode_error() {
        // A present-but-wrong-type value must NOT silently become None — it is a
        // named Decode error carrying the column. (DeError::Other stands in for
        // an InvalidType here; both take the same non-NoSuchProperty branch.)
        let out = classify_opt_str("page_number", Err(DeError::Other("expected string".into())));
        match out {
            Err(ScenarioRepositoryError::Decode { column, .. }) => {
                assert_eq!(column, "page_number")
            }
            other => panic!("expected Decode error, got {other:?}"),
        }
    }

    #[test]
    fn classify_int_passes_present_value_through() {
        let out = classify_opt_int("page_number", Ok(Some(4)));
        assert_eq!(out.expect("ok"), Some(4));
    }

    #[test]
    fn classify_int_null_value_is_none() {
        let out = classify_opt_int("page_number", Ok(None));
        assert_eq!(out.expect("ok"), None);
    }

    #[test]
    fn classify_int_absent_column_is_none() {
        // An absent column (alias typo / OPTIONAL MATCH miss) degrades to None,
        // same as the string classifier.
        let out = classify_opt_int("page_number", Err(DeError::NoSuchProperty));
        assert_eq!(out.expect("ok"), None);
    }

    #[test]
    fn classify_int_type_mismatch_is_named_decode_error() {
        // A present-but-wrong-type value (e.g. a string where an integer is
        // expected) must NOT silently become None — it is a named Decode error
        // carrying the column. (DeError::Other stands in for an InvalidType here;
        // both take the same non-NoSuchProperty branch.)
        let out = classify_opt_int(
            "page_number",
            Err(DeError::Other("expected integer".into())),
        );
        match out {
            Err(ScenarioRepositoryError::Decode { column, .. }) => {
                assert_eq!(column, "page_number")
            }
            other => panic!("expected Decode error, got {other:?}"),
        }
    }

    #[test]
    fn decode_int_renders_as_string() {
        // The DTO wire type stays Option<String>: a graph INTEGER page_number
        // must render as its string form ("4"). This pins the int-as-string
        // contract so a future edit that swaps the mapper back to decode_opt_str
        // fails HERE, not as a 500 in DEV (the bug this fix repaired).
        assert_eq!(render_opt_int_as_str(Some(4)), Some("4".to_string()));
        assert_eq!(render_opt_int_as_str(None), None);
    }

    #[test]
    fn require_returns_present_value() {
        let out = require("evidence_id", Some("evidence-001".to_string()));
        assert_eq!(out.expect("ok"), "evidence-001");
    }

    #[test]
    fn require_missing_is_named_error() {
        match require("evidence_id", None) {
            Err(ScenarioRepositoryError::MissingRequired { column }) => {
                assert_eq!(column, "evidence_id")
            }
            other => panic!("expected MissingRequired error, got {other:?}"),
        }
    }
}
