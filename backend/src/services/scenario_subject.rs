//! Shared scenario-subject resolution.
//!
//! A scenario's evidence work — the Theme Scan (D2b) and the candidate-workbench
//! gather (1a.2) — both operate over "every Evidence node ABOUT the scenario's
//! subject". They MUST agree on who that subject is: the scan writes `undecided`
//! fact-refs keyed to the subject's candidate pool, and gather reads that same
//! pool. If the two resolved the subject differently, a ref the scan wrote could
//! point at a node that is NOT in gather's pool, and it would silently vanish
//! from the workbench (the pool drives the output). That is the exact
//! silent-state-divergence Standing Rule 1 forbids.
//!
//! So subject resolution lives HERE, once, and both callers call it — they read
//! the same subject *by construction*, not by two implementations that happen to
//! match today.
//!
//! ## The case-default fallback was REMOVED on 2026-08-07
//!
//! Until this date, a scenario naming no `target` fell back to the case-default
//! subject (`CASE_DEFAULT_SUBJECT_NAME`), logged at `debug` and invisible to the
//! caller. The measured consequence: a scenario created that morning with an
//! empty definition gathered 148 candidates over `person-marie-awad` — byte for
//! byte the pool of the scenario beside it, which named that target explicitly —
//! and rendered as though two weeks of curation had been copied into it. The
//! diagnostic is `CC-REPORTS/CC_REPORT_SCENARIO_COPY_DIAGNOSTIC.md`.
//!
//! Two operationally distinct states — "the pool I chose" and "the pool a
//! default chose for me" — produced one observable. That is precisely what
//! Standing Rule 1 forbids, so the fallback is gone: a definition with no target
//! now resolves to nothing, loudly, and every caller renders that state by name.
//!
//! `CASE_DEFAULT_SUBJECT_NAME` still exists and is still used — by the Bias
//! Explorer's "About" filter, where a default is a *starting view* the human can
//! see and change, not a silent substitution inside a stored definition.
//!
//! ## Why this is a `services/` module, not a helper inside either caller
//!
//! The Theme Scan lives in `services::theme_scan`; the gather handler lives in
//! `api::scenario_gather`. Those are PEERS — an API handler must not import a
//! service's internals, nor the reverse. Shared logic therefore sinks to a level
//! BOTH callers already depend on (dependencies point inward): a `services/`
//! leaf that knows about neither caller. This is the same "push the shared thing
//! down to where both sides can see it" discipline as the `impl PgExecutor<'_>`
//! seam in `scenario_store.rs`, which lets one function serve both a `&PgPool`
//! caller and a transaction caller.

use crate::dto::scenario_crud::ScenarioDefinition;

/// The one way subject resolution can fail: the scenario names nobody.
///
/// ## Rust Learning: a shared leaf carries its OWN error type
///
/// This resolver does NOT return `ThemeScanError` or `AppError` — it returns its
/// own small `SubjectResolveError`, and each caller `map_err`s it into that
/// caller's domain error. Reusing one caller's error type here would couple both
/// callers to each other through this shared leaf (the API layer would suddenly
/// depend on `ThemeScanError`, or the service on `AppError`). A leaf at a layer
/// boundary must stay ignorant of who calls it — "dependencies point inward"
/// expressed in the type system.
///
/// ## Rust Learning: why an `enum` with ONE variant, and not `Option`
///
/// This enum carried two variants until 2026-08-07; removing the case-default
/// fallback (see the module header) removed the graph lookup and with it both
/// old variants. What is left could be modelled as `Option<String>` — but an
/// `Option`'s `None` carries no message, and every caller here has to TELL A
/// HUMAN what went wrong. A `thiserror` variant carries its own sentence,
/// including the fix, and `?` still works at every call site. The enum shape
/// also means adding a second failure mode later is an additive change rather
/// than a signature change through four callers.
#[derive(Debug, thiserror::Error)]
pub enum SubjectResolveError {
    /// The scenario's definition names no `target`.
    ///
    /// Before 2026-08-07 this silently became the case-default subject. It is
    /// now a real outcome with a real surface: the gather and card endpoints
    /// answer 200 with an EMPTY pool and a stored notice, and the Theme Scan
    /// refuses to start. Never an empty pool with no explanation, and never
    /// somebody else's pool.
    #[error(
        "the scenario names no target — nothing can be gathered until one is \
         chosen (edit the scenario's identity and name who it is about)"
    )]
    NoTarget,
}

/// Resolve the subject a scenario's evidence pool is gathered/scanned over.
///
/// The scenario's own `target` — a party node id chosen from the live vocabulary
/// — and nothing else. A definition that names none resolves to
/// [`SubjectResolveError::NoTarget`]; it does NOT borrow a default (see the
/// module header for the defect that rule exists to prevent).
///
/// Takes an already-parsed `&ScenarioDefinition`, so it is agnostic to HOW the
/// caller obtained it. That matters because the callers treat an *unparseable*
/// definition differently — the Theme Scan errors on it (it also needs
/// `attack_meaning`), while gather tolerates it and passes a target-less
/// synthetic definition, which now lands on `NoTarget` instead of on a borrowed
/// pool. That per-caller policy lives in the CALLERS; this resolver only ever
/// sees a valid `&ScenarioDefinition` and only reads its `target`.
///
/// ## Rust Learning: this function stopped being `async`
///
/// It used to take `&AppState` and `.await` a graph query for the case default.
/// With the fallback gone there is no I/O left — it reads one `Option<String>`
/// off a struct — so it is a plain synchronous `fn` over borrowed data. That is
/// worth noticing rather than leaving `async` for symmetry: an `async fn` tells
/// every reader "this may block, this needs a runtime, this must be `.await`ed",
/// and saying so when it is not true is a lie the compiler will not catch. It
/// also makes the whole resolver unit-testable with no database and no graph.
///
/// # Errors
/// [`SubjectResolveError::NoTarget`] if the definition names no target.
pub fn resolve_scenario_subject(
    definition: &ScenarioDefinition,
) -> Result<String, SubjectResolveError> {
    target_subject(definition.target.as_deref()).ok_or(SubjectResolveError::NoTarget)
}

/// Pure branch-selector: a non-blank `target` is the subject; a blank or absent
/// one means "no target".
///
/// Kept as its own function even now that the resolver is a one-liner around it,
/// because the normalisation rule it carries — "trim, and treat an
/// all-whitespace target as absent" — is the load-bearing half. A target of
/// `"   "` passed through to the graph as a node id would match nothing and
/// return an empty pool, which is the silent-empty state this whole change
/// exists to eliminate.
fn target_subject(target: Option<&str>) -> Option<String> {
    target
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::scenario_crud::CURRENT_SCHEMA_V;

    /// A definition carrying the given target, and nothing else that matters
    /// here.
    fn definition_with_target(target: Option<&str>) -> ScenarioDefinition {
        ScenarioDefinition {
            attack_text: "they say she refused to divide the property".to_string(),
            attack_meaning: None,
            target: target.map(str::to_string),
            wielders: Vec::new(),
            schema_v: CURRENT_SCHEMA_V,
        }
    }

    #[test]
    fn a_named_target_is_the_subject() {
        let definition = definition_with_target(Some("person-marie-awad"));
        assert_eq!(
            resolve_scenario_subject(&definition).expect("a named target resolves"),
            "person-marie-awad"
        );
    }

    #[test]
    fn a_surrounding_space_does_not_make_a_different_subject() {
        // A pasted id with a stray space would otherwise be sent to the graph
        // verbatim, match no node, and return an empty pool with no explanation.
        let definition = definition_with_target(Some("  person-marie-awad  "));
        assert_eq!(
            resolve_scenario_subject(&definition).expect("a padded target resolves"),
            "person-marie-awad"
        );
    }

    /// THE regression test for 2026-08-07.
    ///
    /// Before the fix, each of these three definitions resolved to
    /// `person-marie-awad` — the case default — and gathered 148 candidates
    /// belonging to a scenario the human never named. Any of them resolving to
    /// a subject again means the fallback is back.
    #[test]
    fn no_target_resolves_to_nothing_rather_than_to_the_case_default() {
        for absent in [None, Some(""), Some("   ")] {
            let definition = definition_with_target(absent);
            let error = resolve_scenario_subject(&definition)
                .expect_err("a target-less scenario must not resolve to any subject");
            assert!(
                matches!(error, SubjectResolveError::NoTarget),
                "target {absent:?} produced {error:?} instead of NoTarget"
            );
        }
    }

    #[test]
    fn the_refusal_tells_the_human_what_to_do_about_it() {
        // Standing Rule 1: the message a human meets must carry its own fix. The
        // operator action here is not a config key — it is authoring a target on
        // the scenario, which is the sentence this must name.
        let msg = SubjectResolveError::NoTarget.to_string();
        assert!(
            msg.contains("target"),
            "the refusal must name what is missing: {msg}"
        );
        assert!(
            msg.contains("identity"),
            "the refusal must name the control that fixes it: {msg}"
        );
    }
}
