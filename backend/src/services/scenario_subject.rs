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

use crate::bias::repository::BiasRepository;
use crate::dto::scenario_crud::ScenarioDefinition;
use crate::state::AppState;

/// Failure modes of subject resolution.
///
/// ## Rust Learning: a shared leaf carries its OWN error type
///
/// This resolver does NOT return `ThemeScanError` or `AppError` — it returns its
/// own small `SubjectResolveError`, and each caller `map_err`s it into that
/// caller's domain error. Reusing one caller's error type here would couple both
/// callers to each other through this shared leaf (the API layer would suddenly
/// depend on `ThemeScanError`, or the service on `AppError`). A leaf at a layer
/// boundary must stay ignorant of who calls it — "dependencies point inward"
/// expressed in the type system. The two variants below are the only two ways
/// resolution can fail; everything else is an `Ok(subject_id)`.
#[derive(Debug, thiserror::Error)]
pub enum SubjectResolveError {
    /// The case-default lookup hit the graph and the graph failed (connection,
    /// query, decode). The underlying cause is preserved via `#[source]`.
    #[error("failed to resolve the case-default subject: {source}")]
    DefaultLookupFailed {
        #[source]
        source: crate::bias::repository::BiasRepositoryError,
    },

    /// The scenario names no `target` AND no case-default subject is configured
    /// (`CASE_DEFAULT_SUBJECT_NAME` unset, or it matched no subject in the
    /// graph). A genuine misconfiguration — distinct from "zero candidates" —
    /// so the caller surfaces it loudly rather than returning an empty pool
    /// (Standing Rule 1). The message names the config key that fixes it.
    #[error(
        "no subject: scenario names no target and no case-default subject is \
         configured (CASE_DEFAULT_SUBJECT_NAME)"
    )]
    Unresolvable,
}

/// Resolve the subject a scenario's evidence pool is gathered/scanned over.
///
/// Precedence: the definition's `target` (a party node id chosen from the live
/// vocabulary) if it names one, else the case-default subject
/// (`CASE_DEFAULT_SUBJECT_NAME` → id, via the Bias Explorer's resolver so this
/// and the "About" filter agree on the default). A `target`-present scenario
/// never touches the graph here — the short-circuit avoids the default lookup.
///
/// Takes an already-parsed `&ScenarioDefinition`, so it is agnostic to HOW the
/// caller obtained it. That matters because the two callers treat an
/// *unparseable* definition differently — the Theme Scan errors (it also needs
/// `attack_meaning`), while gather tolerates it and passes a target-less
/// synthetic definition so this falls through to the case default. That
/// per-caller policy lives in the CALLERS; this resolver only ever sees a valid
/// `&ScenarioDefinition` and only reads its `target`.
///
/// # Errors
/// - [`SubjectResolveError::DefaultLookupFailed`] if the case-default lookup
///   fails at the graph layer.
/// - [`SubjectResolveError::Unresolvable`] if there is neither a `target` nor a
///   configured case-default subject.
pub async fn resolve_scenario_subject(
    state: &AppState,
    definition: &ScenarioDefinition,
) -> Result<String, SubjectResolveError> {
    // A definition-named target takes precedence and short-circuits the graph
    // lookup entirely.
    if let Some(subject_id) = target_subject(definition.target.as_deref()) {
        return Ok(subject_id);
    }

    // No target → fall back to the case default, reusing the Bias Explorer's
    // public resolver so the scan, the gather pool, and the "About" filter all
    // agree on the default subject.
    let repo = BiasRepository::new(state.graph.clone());
    let filters = repo
        .available_filters(state.config.case_default_subject_name.as_deref())
        .await
        .map_err(|source| SubjectResolveError::DefaultLookupFailed { source })?;

    filters
        .default_subject_id
        .ok_or(SubjectResolveError::Unresolvable)
}

/// Pure branch-selector: a non-blank `target` is the subject; a blank or absent
/// one means "no target, fall back".
///
/// Extracted as a pure `fn` (no `AppState`, no I/O) so the precedence rule —
/// "trim, and treat an all-whitespace target as absent" — is unit-testable
/// without a live graph. Keeping it separate is also why the async resolver can
/// short-circuit before ever constructing a `BiasRepository`.
fn target_subject(target: Option<&str>) -> Option<String> {
    target
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_subject_uses_a_named_target() {
        assert_eq!(
            target_subject(Some("person-marie-awad")),
            Some("person-marie-awad".to_string())
        );
    }

    #[test]
    fn target_subject_trims_surrounding_whitespace() {
        assert_eq!(
            target_subject(Some("  person-marie-awad  ")),
            Some("person-marie-awad".to_string())
        );
    }

    #[test]
    fn target_subject_treats_blank_as_absent() {
        // An all-whitespace target is NOT a subject — it must fall through to the
        // case default, exactly like `None`. Otherwise a stray-space value would
        // be passed to the graph as a node id and match nothing.
        assert_eq!(target_subject(Some("   ")), None);
        assert_eq!(target_subject(Some("")), None);
        assert_eq!(target_subject(None), None);
    }

    #[test]
    fn unresolvable_display_names_the_config_key() {
        // The operator's fix for an unresolvable subject is to set the env var —
        // so the message must name it (Standing Rule 1: the failure says how to
        // fix it). Mirrors `theme_scan`'s `SubjectUnresolvable` display test.
        let msg = SubjectResolveError::Unresolvable.to_string();
        assert!(
            msg.contains("CASE_DEFAULT_SUBJECT_NAME"),
            "message must name the config key that fixes it: {msg}"
        );
    }

    #[test]
    fn default_lookup_failed_display_surfaces_source() {
        use serde::de::Error as _;
        // The wrapped graph error must reach the operator via `{source}`.
        // Construct a `BiasRepositoryError` via serde's `custom` so the test
        // needs no live Neo4j (same construction as theme_scan's error tests).
        let source = crate::bias::repository::BiasRepositoryError::Deserialize(
            neo4rs::DeError::custom("subjects query failed"),
        );
        let msg = SubjectResolveError::DefaultLookupFailed { source }.to_string();
        assert!(
            msg.contains("case-default subject"),
            "message must describe what failed: {msg}"
        );
        assert!(
            msg.contains("subjects query failed"),
            "the underlying cause must be surfaced: {msg}"
        );
    }
}
