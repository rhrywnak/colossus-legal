//! Tests for the timeline READ handlers' pure helpers and their logging.
//!
//! The handlers themselves need an `AppState` (two pools, a graph, a registry),
//! which this project has no test tier for — the same gap the chronology's own
//! validation guard names.
//!
//! ## What moved out in Phase C
//!
//! `checkable_target_ids` and its three tests moved to
//! `api::timeline_write_support`, with the function: the read handler and the
//! write handlers now resolve link targets by ONE piece of code, and a copied
//! test would have been a second assertion about a function this file no longer
//! contains. What stays here is the read side's own contract — the case slug on
//! every span and every failure log, ruled 2026-08-25.

// ─── the case slug on the logs (2026-08-25 ruling) ───────────────────────────

/// This module's own source, with `//` comments removed.
///
/// ⚑ Required before any scan of this repository's source: this codebase
/// documents its rules next to its rules, so a scanner hunting for a token finds
/// the DOCUMENTATION first. The rule is stated once in `domain::wording_tests`.
fn source_without_comments() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/api/timeline.rs");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} is not readable: {e}", path.display()));
    raw.lines()
        .map(|line| match line.find("//") {
            Some(at) => &line[..at],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Both handlers declare `case_slug` on their span, and both record it.
///
/// ## Why a source scan and not an assertion on a captured log
///
/// A span field is only observable through a subscriber, and this project has no
/// tracing test tier — installing one to prove four lines would be a harness
/// nobody asked for. What CAN be checked on disk is that the field is declared
/// and that something records it, which is exactly the pair that goes wrong: a
/// field declared `Empty` and never recorded logs a blank, and a `record` call
/// for a field the attribute never declared is silently dropped by tracing.
/// `api::practice_answers::tests` reads its own handler the same way.
#[test]
fn both_timeline_handlers_carry_the_case_slug_on_their_span() {
    let source = source_without_comments();

    assert_eq!(
        source.matches("case_slug = tracing::field::Empty").count(),
        2,
        "both handlers must DECLARE the field; a recorded field the attribute \
         never declared is dropped without a word"
    );
    // Matched on the CALL and not on `record("case_slug"` as one string: rustfmt
    // wraps the longer of the two calls, putting the field name on its own line,
    // and a matcher that only works before the formatter runs is a matcher that
    // fails the next time anyone touches the file.
    assert_eq!(
        source.matches("Span::current().record(").count(),
        2,
        "both handlers must RECORD it; a declared field nobody records logs blank"
    );
    assert_eq!(
        source.matches("\"case_slug\"").count(),
        2,
        "each record call names the field, and names the one the span declared"
    );
}

/// Every failure this module reports names a case, or says it is not scoped to one.
#[test]
fn the_failure_log_carries_a_case_field_at_every_call_site() {
    let source = source_without_comments();

    assert!(
        source.contains("case_slug = case.unwrap_or(NOT_CASE_SCOPED)"),
        "the failure log must emit the field even when there is no case, so a \
         reader can tell 'no case' from 'nobody logged it'"
    );
    // ⚑ The span and its error events must agree. Recording the field only when
    // a case is configured left the SPAN Empty while every error event on the
    // same trace carried the sentinel — one field name with two values, which a
    // structured processor joins into a contradiction. Both sides read the same
    // named constant, and neither may go back to a literal.
    assert_eq!(
        source.matches("NOT_CASE_SCOPED").count(),
        3,
        "the sentinel must be ONE constant, declared once and read by both the \
         span record and the failure log"
    );
    assert!(
        !source.contains("if let Some(case) = state.config.case_slug"),
        "the event handler must record the field unconditionally, or its span \
         disagrees with its own error events"
    );
    // A vacuity guard, not a census: the count is a FLOOR because later phases
    // add reads and each adds a call. What matters is that the scan found the
    // call sites at all — an assertion over zero of them would pass forever.
    let calls = source.matches("read_failure(e,").count();
    assert!(
        calls >= 8,
        "the scan found only {calls} failure call sites; it has stopped seeing them"
    );
    assert_eq!(
        source.matches("read_failure(e, \"").count(),
        calls,
        "every call site names the operation as a literal"
    );
}
