//! Pure mapping helpers for the scenario fact-curation routes.
//!
//! Split out of `scenario_facts` when the anchored-ruling work pushed that module
//! past the 300-line limit. Everything here is PURE — no I/O, no state, no HTTP
//! extraction — which is why it splits cleanly: these are the functions that
//! translate between the module's four vocabularies (the workbench's verbs, the
//! stored state, the ledger's ruling kinds, and HTTP status codes) plus the join
//! that pairs stored references with live graph content.
//!
//! `pub(crate)` rather than `pub`: the handlers next door are the only callers,
//! and these translations are an implementation detail of this route family.

use std::collections::HashMap;

use serde_json::json;

use crate::{
    bias::dto::BiasInstance,
    domain::{fact_status::FactStatus, ruling_anchor::RulingKind},
    dto::{FactAction, ScenarioFactDto},
    error::AppError,
    repositories::pipeline_repository::ScenarioFactRefRecord,
    services::scenario_ruling::RulingError,
};

/// Pair each stored fact reference with its live graph content.
///
/// Pure (no I/O) so it is unit-testable without a database or graph. Builds a
/// `graph_node_id → BiasInstance` index from the content, then walks the
/// references **in their stored order** (oldest tag first, per
/// `list_fact_refs_for_scenario`) producing exactly one DTO per reference.
///
/// ## Domain note: a reference can outlive its node
///
/// A `scenario_fact_refs` row is a pointer into Neo4j; if that Evidence node is
/// later deleted or re-ingested under a new id, the lookup misses. Phase A must
/// not let the saved fact silently disappear (Standing Rule 1), so a miss yields
/// `content: None` *and* a logged warning — the curated count stays honest and
/// the stale reference is visible to both the operator (logs) and the UI (a
/// null-content card carrying the `graph_node_id`).
pub(crate) fn join_facts(
    refs: Vec<ScenarioFactRefRecord>,
    content: Vec<BiasInstance>,
) -> Vec<ScenarioFactDto> {
    let mut by_id: HashMap<String, BiasInstance> = content
        .into_iter()
        .map(|c| (c.evidence_id.clone(), c))
        .collect();

    refs.into_iter()
        .map(|r| {
            // `remove` (not `get`) because each node id appears at most once per
            // scenario (composite PK), so transferring ownership avoids a clone.
            let content = by_id.remove(&r.graph_node_id);
            if content.is_none() {
                tracing::warn!(
                    graph_node_id = %r.graph_node_id,
                    scenario_id = %r.scenario_id,
                    "saved scenario fact references a graph node with no live content; \
                     returning it with null content so the stale reference stays visible"
                );
            }
            // `r.status` is intentionally NOT copied into the DTO here: surfacing
            // the three-state candidate status on the read path is 1a.2/1a.3 work
            // (gated on the workbench UI that consumes it), not this data-layer
            // chunk. See the `ScenarioFactDto` doc — the omission is a decision.
            ScenarioFactDto {
                graph_node_id: r.graph_node_id,
                role: r.role_in_this_scenario,
                note: r.note,
                content,
            }
        })
        .collect()
}

/// Map a [`RulingError`] onto its HTTP status, keeping the service free of any
/// HTTP knowledge.
///
/// ## Why the refusal messages reach the client verbatim
///
/// Most of this codebase returns an opaque message on failure and puts the detail
/// in the operator log, because the detail is usually about the server. These are
/// different: each names something about the ITEM the human is looking at, and the
/// human is the only one who can act on it. A generic "failed to apply fact
/// action" here would leave them clicking include on an unquotable candidate
/// forever with no idea why it will not take.
///
/// `NodeGone` is a 409 rather than a 404: the route and the scenario both exist,
/// and the request was well-formed — it is the world that moved underneath it. The
/// two write failures stay opaque (500) because those ARE about the server.
pub(crate) fn ruling_error_to_app_error(error: RulingError, graph_node_id: &str) -> AppError {
    match error {
        RulingError::NodeGone => AppError::Conflict {
            message: error.to_string(),
            details: json!({ "graph_node_id": graph_node_id, "reason": "node_gone" }),
        },
        RulingError::NoDocument => AppError::BadRequest {
            message: error.to_string(),
            details: json!({ "graph_node_id": graph_node_id, "reason": "no_document" }),
        },
        RulingError::NotCitable => AppError::BadRequest {
            message: error.to_string(),
            details: json!({ "graph_node_id": graph_node_id, "reason": "not_citable" }),
        },
        RulingError::ReasonMismatch { .. } => AppError::BadRequest {
            message: error.to_string(),
            details: json!({ "graph_node_id": graph_node_id, "reason": "defer_reason" }),
        },
        // The graph read and the two writes are server-side failures: log the cause
        // with context, tell the client nothing beyond "it failed".
        RulingError::GraphRead { .. } | RulingError::Write { .. } => {
            tracing::error!(
                error = %error,
                graph_node_id = %graph_node_id,
                "failed to record an anchored ruling"
            );
            AppError::Internal {
                message: "failed to record the ruling".to_string(),
            }
        }
    }
}

/// Map a human [`FactAction`] (imperative verb) to the [`FactStatus`] the ref
/// becomes (state). This is the one translation between the two vocabularies.
///
/// ## Rust Learning: an exhaustive `match` is the compiler-checklist
///
/// This `match` has NO `_ =>` arm, on purpose. An enum `match` with no catch-all
/// is compile-checked for exhaustiveness: if a fourth `FactAction` is ever added,
/// THIS function stops compiling until its arm is written. The compiler becomes
/// the checklist — you cannot ship a new action that some code path silently
/// ignores. Adding a `_ =>` arm would throw that guarantee away (it would swallow
/// the new variant into whatever default it names), so we deliberately omit one.
/// This is the control-flow twin of the data-level guarantee `FactStatus::ALL`
/// gives, and of the closed-enum parse boundary on [`FactAction`] itself.
///
/// ## Domain note: the non-identity mapping
///
/// `Undrop → Undecided` is why `FactAction` and `FactStatus` are separate types:
/// "un-drop" is a verb with no matching state. Un-drop returns a candidate to the
/// pool for reconsideration — deliberately `Undecided`, never `Included` (the
/// human only recovered it, they did not confirm it).
pub(crate) fn action_to_status(action: FactAction) -> FactStatus {
    match action {
        FactAction::Include => FactStatus::Included,
        FactAction::Drop => FactStatus::Dropped,
        FactAction::Undrop => FactStatus::Undecided,
        // Defer leaves the candidate UNDECIDED on purpose — it is not a decision
        // about the fact, it is a decision to postpone. What separates it from an
        // untouched candidate is the `defer_reason` written beside it, and the
        // ledger row recording that a human looked.
        FactAction::Defer => FactStatus::Undecided,
        // Undo returns the candidate to the neutral state, whatever it was in.
        FactAction::Reopen => FactStatus::Undecided,
    }
}

/// Map a human [`FactAction`] onto the [`RulingKind`] the LEDGER records.
///
/// ## Domain note: why this is not the same mapping as `action_to_status`
///
/// `action_to_status` collapses `Undrop` and `Defer` onto the same state
/// (`Undecided`) — correctly, because that IS the state both produce. The ledger
/// must not collapse them: "I recovered this from the dropped tray" and "I looked
/// at this and parked it because the page is missing" are different acts, and the
/// forensic record has to say which happened. Two mappings from one verb, each
/// lossless in its own direction, is exactly why the three vocabularies are
/// separate types.
///
/// `Drop` becomes `Exclude` because the ledger speaks the law's vocabulary
/// (§12.1: include / exclude / defer) while the workbench speaks the UI's.
pub(crate) fn action_to_ruling_kind(action: FactAction) -> RulingKind {
    match action {
        FactAction::Include => RulingKind::Include,
        FactAction::Drop => RulingKind::Exclude,
        FactAction::Undrop => RulingKind::Undrop,
        FactAction::Defer => RulingKind::Defer,
        FactAction::Reopen => RulingKind::Reopen,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    /// A minimal `BiasInstance` carrying just an id + quote — enough to assert
    /// the join pairs the right content with the right reference.
    fn content(evidence_id: &str, quote: &str) -> BiasInstance {
        BiasInstance {
            evidence_id: evidence_id.to_string(),
            title: String::new(),
            verbatim_quote: Some(quote.to_string()),
            question: None,
            page_number: None,
            pattern_tags: Vec::new(),
            stated_by: None,
            about: Vec::new(),
            document: None,
        }
    }

    /// A `scenario_fact_refs` row for one scenario, with the given node id /
    /// role / note.
    fn fact_ref(
        scenario_id: Uuid,
        graph_node_id: &str,
        role: Option<&str>,
        note: Option<&str>,
    ) -> ScenarioFactRefRecord {
        ScenarioFactRefRecord {
            scenario_id,
            graph_node_id: graph_node_id.to_string(),
            role_in_this_scenario: role.map(str::to_string),
            status: FactStatus::Included.code().to_string(),
            note: note.map(str::to_string),
            // These join tests do not exercise confidence (that is gather's read
            // path, tested there) — a human-curated-style None keeps them pure.
            confidence: None,
            // Likewise unexercised here: provenance is read by the run-detail
            // "applied" path, not by the join. None keeps these fixtures
            // human-authored.
            source_run_id: None,
            // A fixed epoch timestamp — the join does not read it, but the
            // struct requires one. Avoids `Utc::now()` so the test is pure.
            tagged_at: chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
            // Not deferred: the join does not read this either, and None is the
            // honest value for a fixture representing a plain include.
            defer_reason: None,
            // Task 2.13's two columns. The join does not read either — it pairs
            // refs with graph content — so the defaults a freshly-included fact
            // carries are the honest fixture: weighed by nobody, placed by nobody.
            tier: crate::domain::fact_tier::FactTier::DEFAULT
                .code()
                .to_string(),
            sort_ordinal: None,
        }
    }

    #[test]
    fn join_facts_with_no_refs_returns_empty() {
        // The list handler short-circuits on `refs.is_empty()` before calling
        // this, but the function's own contract — zero refs in, zero DTOs out —
        // is asserted here directly rather than left implicit.
        let dtos = join_facts(Vec::new(), Vec::new());
        assert!(dtos.is_empty());
    }

    #[test]
    fn join_facts_ignores_orphan_content_with_no_ref() {
        // Content the graph returned for a node that is NOT among the refs must
        // not appear in the output: the refs drive the result, one DTO each.
        let dtos = join_facts(Vec::new(), vec![content("ev-orphan", "unreferenced")]);
        assert!(
            dtos.is_empty(),
            "content without a matching reference is dropped, not invented into a DTO",
        );
    }

    #[test]
    fn join_pairs_role_and_note_with_the_matching_content() {
        let sid = Uuid::new_v4();
        let refs = vec![
            fact_ref(sid, "ev-1", Some("rebuts"), Some("key denial")),
            fact_ref(sid, "ev-2", None, None),
        ];
        // Content arrives in a DIFFERENT order than the refs (the graph reader
        // sorts by speaker/doc); the join must key by id, not by position.
        let content = vec![
            content("ev-2", "second quote"),
            content("ev-1", "first quote"),
        ];

        let dtos = join_facts(refs, content);

        assert_eq!(dtos.len(), 2, "one DTO per reference, in reference order");

        assert_eq!(dtos[0].graph_node_id, "ev-1");
        assert_eq!(dtos[0].role.as_deref(), Some("rebuts"));
        assert_eq!(dtos[0].note.as_deref(), Some("key denial"));
        assert_eq!(
            dtos[0]
                .content
                .as_ref()
                .and_then(|c| c.verbatim_quote.as_deref()),
            Some("first quote"),
            "ev-1's role/note must pair with ev-1's content, not ev-2's",
        );

        assert_eq!(dtos[1].graph_node_id, "ev-2");
        assert!(dtos[1].role.is_none());
        assert_eq!(
            dtos[1]
                .content
                .as_ref()
                .and_then(|c| c.verbatim_quote.as_deref()),
            Some("second quote"),
        );
    }

    #[test]
    fn join_keeps_a_stale_reference_with_null_content() {
        let sid = Uuid::new_v4();
        // Two saved refs, but the graph only returns content for one — the other
        // node was deleted. The stale ref must survive with content = None.
        let refs = vec![
            fact_ref(sid, "ev-live", None, None),
            fact_ref(sid, "ev-deleted", Some("context"), None),
        ];
        let content = vec![content("ev-live", "still here")];

        let dtos = join_facts(refs, content);

        assert_eq!(dtos.len(), 2, "a missing node must NOT drop the reference");
        assert!(dtos[0].content.is_some(), "ev-live keeps its content");
        assert_eq!(dtos[1].graph_node_id, "ev-deleted");
        assert!(
            dtos[1].content.is_none(),
            "the stale ref is surfaced with null content, not silently removed",
        );
        assert_eq!(
            dtos[1].role.as_deref(),
            Some("context"),
            "the stored role/note still travel even when content is gone",
        );
    }

    #[test]
    fn action_to_status_maps_each_verb_to_its_state() {
        // Pin the one non-identity translation the workbench depends on: `undrop`
        // returns a candidate to the pool as `Undecided`, NOT `Included`. If a
        // future edit swapped an arm, this fails here rather than mis-ruling a fact.
        assert_eq!(action_to_status(FactAction::Include), FactStatus::Included);
        assert_eq!(action_to_status(FactAction::Drop), FactStatus::Dropped);
        assert_eq!(action_to_status(FactAction::Undrop), FactStatus::Undecided);
        // Defer is the second non-identity translation and the more dangerous one:
        // if this arm were ever `Dropped`, every deferred candidate would vanish
        // into the dropped tray while the LEDGER correctly recorded a defer. The
        // two records would disagree, and the human would simply lose the card
        // with no error anywhere.
        assert_eq!(action_to_status(FactAction::Defer), FactStatus::Undecided);
        // Undo returns to the neutral state, like defer and undrop.
        assert_eq!(action_to_status(FactAction::Reopen), FactStatus::Undecided);
    }

    /// The ledger's vocabulary is not the workbench's, and the mapping between
    /// them must be exact.
    ///
    /// `Drop → Exclude` is the whole reason this function exists: the UI says
    /// "drop", §12.1 says "exclude". Two of the four arms are identities, which is
    /// precisely what makes a mis-assignment in the other two look plausible on
    /// review — so the full table is pinned rather than the interesting rows only.

    #[test]
    fn action_to_ruling_kind_maps_the_full_table() {
        assert_eq!(
            action_to_ruling_kind(FactAction::Include),
            RulingKind::Include
        );
        assert_eq!(action_to_ruling_kind(FactAction::Drop), RulingKind::Exclude);
        assert_eq!(
            action_to_ruling_kind(FactAction::Undrop),
            RulingKind::Undrop
        );
        assert_eq!(action_to_ruling_kind(FactAction::Defer), RulingKind::Defer);
        // The pair that must NOT collapse: same state, different words.
        assert_eq!(
            action_to_ruling_kind(FactAction::Reopen),
            RulingKind::Reopen
        );
        assert_ne!(
            action_to_ruling_kind(FactAction::Reopen),
            action_to_ruling_kind(FactAction::Undrop)
        );
    }

    /// Each refusal reaches the client as its own status code and its own reason
    /// tag, and the two server-fault variants stay opaque.
    ///
    /// These status codes are deliberate choices, not defaults. `NodeGone` is 409
    /// rather than 404 because the route and the scenario both exist and the
    /// request was well-formed — it is the world that moved. Nothing pinned any of
    /// this before, so a well-meaning simplification to "all refusals are 400"
    /// would have been invisible.
    #[test]
    fn each_refusal_maps_to_its_own_status_and_reason() {
        let cases = [
            (RulingError::NodeGone, "node_gone"),
            (RulingError::NoDocument, "no_document"),
            (RulingError::NotCitable, "not_citable"),
            (
                RulingError::ReasonMismatch {
                    message: "m".to_string(),
                },
                "defer_reason",
            ),
        ];

        for (error, expected_reason) in cases {
            let is_conflict = matches!(error, RulingError::NodeGone);
            let message = error.to_string();
            let app_error = ruling_error_to_app_error(error, "ev-1");

            match app_error {
                AppError::Conflict { details, .. } if is_conflict => {
                    assert_eq!(details["reason"], expected_reason);
                    assert_eq!(details["graph_node_id"], "ev-1");
                }
                AppError::BadRequest {
                    details,
                    message: m,
                } if !is_conflict => {
                    assert_eq!(details["reason"], expected_reason);
                    assert_eq!(details["graph_node_id"], "ev-1");
                    // The human-facing text survives the mapping verbatim — this is
                    // the only feedback the UI has to offer, so a generic
                    // substitution here would strand the user.
                    assert_eq!(m, message);
                }
                other => panic!("{expected_reason} mapped to the wrong variant: {other:?}"),
            }
        }
    }

    #[test]
    fn server_faults_stay_opaque_to_the_client() {
        // The refusal messages are about the ITEM and must reach the user. These
        // two are about the SERVER and must not — they carry a database or graph
        // error whose text is for the operator log only.
        let app_error = ruling_error_to_app_error(
            RulingError::Write {
                source: crate::repositories::pipeline_repository::PipelineRepoError::from(
                    sqlx::Error::PoolClosed,
                ),
            },
            "ev-1",
        );
        match app_error {
            AppError::Internal { message } => {
                assert_eq!(message, "failed to record the ruling");
                assert!(
                    !message.contains("pool"),
                    "the underlying cause must not reach the client: {message}"
                );
            }
            other => panic!("a write failure must be a 500, got {other:?}"),
        }
    }
}
