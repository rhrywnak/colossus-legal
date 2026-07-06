//! Scenario fact-curation HTTP routes (task 1.x — Phase A).
//!
//! Three routes over the existing storage and graph readers — no new tables,
//! no migration, no LLM, no retrieval. A human picks Evidence (from the Bias
//! Explorer's pre-tagged candidates) and saves it onto a scenario; these routes
//! persist, remove, and list those saved references with their live content.
//!
//! - `POST   /cases/:slug/scenarios/:scenario_id/facts`            → 201
//! - `DELETE /cases/:slug/scenarios/:scenario_id/facts/:graph_node_id` → 204
//! - `GET    /cases/:slug/scenarios/:scenario_id/facts`            → 200 `[…]`
//!
//! ## CRITICAL — the pipeline pool
//!
//! `scenarios` and `scenario_fact_refs` live in the **pipeline** database
//! (`colossus_legal_v2`), so every Postgres call uses `&state.pipeline_pool`,
//! NOT `state.pg_pool`. The graph content is read from `state.graph`.
//!
//! ## Layering — why the existence check, why the join
//!
//! Each handler holds the same two invariants as `api::scenarios`:
//! 1. A `scenario_id` must parse as a UUID (else 400).
//! 2. The scenario must exist *in the case named by the URL* (else 404) — the
//!    "path-as-fence" rule, so a scenario can't be reached through another
//!    case's path. Checking it before the write also turns "no such scenario"
//!    into a clean 404 instead of an opaque foreign-key 500 (Phase A Q2).
//!
//! The list route joins each stored reference onto its live graph content; a
//! reference whose node has since disappeared is returned with `content: null`
//! rather than dropped, so a stale reference stays observable (Standing Rule 1).

use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{require_edit, AuthUser},
    bias::{dto::BiasInstance, repository::BiasRepository},
    dto::{AddFactRequest, ScenarioFactDto},
    error::AppError,
    repositories::pipeline_repository::{
        delete_fact_ref, get_scenario, list_fact_refs_for_scenario, upsert_fact_ref,
        ScenarioFactRefRecord,
    },
    state::AppState,
};

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Parse the path `scenario_id` as a UUID, mapping a malformed value to a 400.
///
/// A bad UUID is a client error, not a server fault — mirrors
/// `api::scenarios::get_scenario_by_id`.
fn parse_scenario_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::BadRequest {
        message: "scenario_id must be a valid UUID".to_string(),
        details: json!({ "field": "scenario_id" }),
    })
}

/// Confirm the scenario exists AND belongs to the URL's case, or return a 404.
///
/// ## Why this runs before every operation
///
/// - On the write paths it is the Phase-A Q2 decision: do the existence check
///   here (the `get_scenario_by_id` house pattern) so "no such scenario" is a
///   404, rather than letting `scenario_fact_refs`'s foreign key reject the
///   insert as a 500.
/// - On the read path it keeps two states distinct (Standing Rule 1): a missing
///   scenario is a 404, while an existing scenario with no curated facts yet is
///   a 200 with `[]`. Without this check both would look like an empty list.
/// - The `case_slug` comparison is the path-as-fence: `scenario_id` is globally
///   unique, so reaching a real scenario through the wrong case path returns
///   `NotFound` (not `Forbidden`) — the response never confirms the row exists
///   under some other case.
async fn ensure_scenario_in_case(
    state: &AppState,
    scenario_id: Uuid,
    slug: &str,
) -> Result<(), AppError> {
    let record = get_scenario(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to look up scenario");
            AppError::Internal {
                message: "failed to look up scenario".to_string(),
            }
        })?
        .ok_or_else(|| {
            tracing::debug!("scenario not found");
            AppError::NotFound {
                message: "scenario not found".to_string(),
            }
        })?;

    if record.case_slug != slug {
        tracing::warn!(actual_case = %record.case_slug, "scenario requested through the wrong case path");
        return Err(AppError::NotFound {
            message: "scenario not found".to_string(),
        });
    }
    Ok(())
}

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
fn join_facts(
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
            ScenarioFactDto {
                graph_node_id: r.graph_node_id,
                role: r.role_in_this_scenario,
                note: r.note,
                content,
            }
        })
        .collect()
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// `POST /cases/:slug/scenarios/:scenario_id/facts` — save a fact onto a scenario.
///
/// The reference is stored `confirmed = true`: a human deliberately picked it.
/// Re-posting the same `graph_node_id` is an in-place update (the store upserts
/// on the composite key), so the route is idempotent on the pair.
#[tracing::instrument(skip(state, user, payload), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn add_scenario_fact(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
    Json(payload): Json<AddFactRequest>,
) -> Result<StatusCode, AppError> {
    require_edit(&user)?;
    tracing::info!(
        "{} POST /cases/{}/scenarios/{}/facts",
        user.username,
        slug,
        scenario_id
    );

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    // confirmed = true: a human curated it. role / note are accepted but not yet
    // surfaced by any UI (Phase A) — the columns round-trip, ready for a later
    // phase, without forcing the client to send a value.
    //
    // confidence = None: this is the HUMAN path. A hand-curated fact has no model
    // confidence, so NULL is the correct, permanent value here — not a stand-in.
    // Only the Theme Scan (D2b) writes a Some(_) confidence.
    upsert_fact_ref(
        &state.pipeline_pool,
        id,
        &payload.graph_node_id,
        payload.role.as_deref(),
        true,
        payload.note.as_deref(),
        None,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, graph_node_id = %payload.graph_node_id, "failed to add scenario fact ref");
        AppError::Internal {
            message: "failed to add scenario fact".to_string(),
        }
    })?;

    Ok(StatusCode::CREATED)
}

/// `DELETE /cases/:slug/scenarios/:scenario_id/facts/:graph_node_id` — un-save a fact.
///
/// A delete that removes nothing (`rows_affected == 0`) is a 404: the pair was
/// not on this scenario. A successful removal is a 204. Those stay distinct
/// observables rather than collapsing into one "OK" (Standing Rule 1).
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id, graph_node_id = %graph_node_id))]
pub async fn remove_scenario_fact(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id, graph_node_id)): Path<(String, String, String)>,
) -> Result<StatusCode, AppError> {
    require_edit(&user)?;
    tracing::info!(
        "{} DELETE /cases/{}/scenarios/{}/facts/{}",
        user.username,
        slug,
        scenario_id,
        graph_node_id
    );

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    let removed = delete_fact_ref(&state.pipeline_pool, id, &graph_node_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, graph_node_id = %graph_node_id, "failed to remove scenario fact ref");
            AppError::Internal {
                message: "failed to remove scenario fact".to_string(),
            }
        })?;

    if removed == 0 {
        tracing::debug!("no fact reference to remove");
        return Err(AppError::NotFound {
            message: "fact reference not found on this scenario".to_string(),
        });
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /cases/:slug/scenarios/:scenario_id/facts` — list saved facts with content.
///
/// Reads the stored references, then reads their live graph content by id and
/// joins the two. An existing scenario with no facts yet is a valid `200 []`
/// (distinct from the `404` for a missing scenario).
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn list_scenario_facts(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<Vec<ScenarioFactDto>>, AppError> {
    if let Some(ref u) = user {
        tracing::info!(
            "{} GET /cases/{}/scenarios/{}/facts",
            u.username,
            slug,
            scenario_id
        );
    }

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    let refs = list_fact_refs_for_scenario(&state.pipeline_pool, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to list scenario fact refs");
            AppError::Internal {
                message: "failed to list scenario facts".to_string(),
            }
        })?;

    // No references → no graph round-trip. The scenario exists (checked above),
    // so this is a genuine empty curated set, returned as 200 [].
    if refs.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let ids: Vec<String> = refs.iter().map(|r| r.graph_node_id.clone()).collect();
    let content = BiasRepository::new(state.graph.clone())
        .evidence_by_ids(&ids)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to read scenario fact content from graph");
            AppError::Internal {
                message: "failed to read scenario fact content".to_string(),
            }
        })?;

    Ok(Json(join_facts(refs, content)))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal `BiasInstance` carrying just an id + quote — enough to assert
    /// the join pairs the right content with the right reference.
    fn content(evidence_id: &str, quote: &str) -> BiasInstance {
        BiasInstance {
            evidence_id: evidence_id.to_string(),
            title: String::new(),
            verbatim_quote: Some(quote.to_string()),
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
            confirmed: true,
            note: note.map(str::to_string),
            // A fixed epoch timestamp — the join does not read it, but the
            // struct requires one. Avoids `Utc::now()` so the test is pure.
            tagged_at: chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
        }
    }

    #[test]
    fn parse_scenario_id_rejects_non_uuid() {
        let result = parse_scenario_id("not-a-uuid");
        assert!(matches!(result, Err(AppError::BadRequest { .. })));
    }

    #[test]
    fn parse_scenario_id_accepts_a_uuid() {
        let id = Uuid::new_v4();
        let parsed = parse_scenario_id(&id.to_string()).expect("a valid uuid string parses");
        assert_eq!(parsed, id);
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
}
