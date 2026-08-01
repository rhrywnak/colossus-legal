//! Scenario fact-curation HTTP routes.
//!
//! A human picks Evidence (from the Bias Explorer's pre-tagged candidates) and
//! saves it onto a scenario; these routes persist, remove, and list those saved
//! references with their live content.
//!
//! - `POST   /cases/:slug/scenarios/:scenario_id/facts`            → 201
//! - `DELETE /cases/:slug/scenarios/:scenario_id/facts/:graph_node_id` → 204
//! - `GET    /cases/:slug/scenarios/:scenario_id/facts`            → 200 `[…]`
//! - `POST   /cases/:slug/scenarios/:scenario_id/facts/:graph_node_id/action` → 200
//!   — the workbench ruling (`include` / `drop` / `undrop` / `defer`).
//!
//! ## Every ruling is anchored (§12.1, task 1.1)
//!
//! All three write routes go through `services::scenario_ruling`, which reads the
//! item from the graph, captures its anchor (document, page, verbatim + normalized
//! quote, speaker), and writes the state row and the ledger row in one
//! transaction. No route writes a fact-ref directly — `upsert_fact_ref` and
//! `delete_fact_ref` are withheld from the repository's re-export precisely so
//! that stays true.
//!
//! `DELETE` is included: §12.1's list of rulings is non-exhaustive (ratified
//! 2026-08-01), and un-saving a reference changes a human decision, so it goes
//! through `record_removal`.
//!
//! The visible consequence: a ruling that cannot be anchored is REFUSED (400/409)
//! where it previously succeeded. The one exception is a removal, which is never
//! refused for missing content — a vanished node is often WHY the reference is
//! being discarded, so it records an empty anchor rather than failing.
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

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{require_edit, AuthUser},
    bias::repository::BiasRepository,
    domain::{fact_status::FactStatus, ruling_anchor::RulingKind},
    dto::{AddFactRequest, FactActionRequest, ScenarioFactDto},
    error::AppError,
    repositories::pipeline_repository::{get_scenario, list_fact_refs_for_scenario},
    services::scenario_ruling::{record_removal, record_ruling, RulingRequest},
    state::AppState,
};

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Parse the path `scenario_id` as a UUID, mapping a malformed value to a 400.
///
/// A bad UUID is a client error, not a server fault — mirrors
/// `api::scenarios::get_scenario_by_id`.
///
/// `pub(crate)` so the sibling `api::scenario_gather` handler reuses the exact
/// same 400 discipline rather than re-deriving it (the gather feature was split
/// into its own module for the 300-line rule, but shares this fence).
pub(crate) fn parse_scenario_id(raw: &str) -> Result<Uuid, AppError> {
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
///
/// `pub(crate)` so the sibling `api::scenario_gather` handler reuses the same
/// existence-and-case fence (see `parse_scenario_id`).
pub(crate) async fn ensure_scenario_in_case(
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

// The pure mapping helpers (`join_facts`, the two verb translations, and the
// error→HTTP mapping) live in the sibling `scenario_facts_mapping` module — they
// are pure, and moving them kept this module inside the 300-line limit.
use super::scenario_facts_mapping::{
    action_to_ruling_kind, action_to_status, join_facts, ruling_error_to_app_error,
};

// ── Handlers ──────────────────────────────────────────────────────────────────

/// `POST /cases/:slug/scenarios/:scenario_id/facts` — save a fact onto a scenario.
///
/// The reference is stored `status = FactStatus::Included`: a human deliberately
/// picked it. Re-posting the same `graph_node_id` is an in-place update (the
/// store upserts on the composite key), so the route is idempotent on the pair.
///
/// ## This is an INCLUDE ruling, so it anchors (§12.1)
///
/// Saving a fact onto a scenario IS a ruling, not a bookkeeping operation, so it
/// goes through the same [`record_ruling`] choke point as the workbench's action
/// route. The consequence is visible: an item with no verbatim quote can no longer
/// be saved here, and the 400 says why and names defer as the alternative. That is
/// the citability law applying to every include, not only to the ones made from
/// the workbench.
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

    // status = Included: a human curated it. role / note are accepted but not yet
    // surfaced by any UI (Phase A) — the columns round-trip, ready for a later
    // phase, without forcing the client to send a value.
    let anchor_id = record_ruling(
        &state.pipeline_pool,
        &BiasRepository::new(state.graph.clone()),
        &RulingRequest {
            scenario_id: id,
            graph_node_id: &payload.graph_node_id,
            kind: RulingKind::Include,
            ruled_by: &user.username,
            role: payload.role.as_deref(),
            note: payload.note.as_deref(),
            // A save is never a defer, so no reason may ride along.
            defer_reason: None,
        },
        FactStatus::Included,
        Utc::now(),
    )
    .await
    .map_err(|e| ruling_error_to_app_error(e, &payload.graph_node_id))?;

    tracing::info!(
        anchor_id = %anchor_id,
        graph_node_id = %payload.graph_node_id,
        "recorded an anchored include"
    );

    Ok(StatusCode::CREATED)
}

/// `DELETE /cases/:slug/scenarios/:scenario_id/facts/:graph_node_id` — un-save a fact.
///
/// A delete that removes nothing (`rows_affected == 0`) is a 404: the pair was
/// not on this scenario. A successful removal is a 204. Those stay distinct
/// observables rather than collapsing into one "OK" (Standing Rule 1).
///
/// ## Un-saving is a RULING, so it anchors too (ratified 2026-08-01)
///
/// §12.1's list of rulings is non-exhaustive: any verb that changes a candidate's
/// ruling state writes a ledger row. Discarding an include is such a change, and
/// until this route went through [`record_removal`] it was the last traceless
/// path in the system — the state row simply vanished, leaving its include anchor
/// with nothing to say it had been withdrawn.
///
/// Unlike every other ruling, a removal is never refused for missing content. A
/// vanished graph node is often exactly WHY the human is discarding the
/// reference; refusing would leave them holding something they can neither use
/// nor delete. The removal is recorded with an empty anchor instead, which states
/// precisely that.
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

    let anchor_id = record_removal(
        &state.pipeline_pool,
        &BiasRepository::new(state.graph.clone()),
        &RulingRequest {
            scenario_id: id,
            graph_node_id: &graph_node_id,
            kind: RulingKind::Remove,
            ruled_by: &user.username,
            // A removal discards the row wholesale, so none of the workbench
            // fields apply — and a reason belongs only to a defer.
            role: None,
            note: None,
            defer_reason: None,
        },
        Utc::now(),
    )
    .await
    .map_err(|e| ruling_error_to_app_error(e, &graph_node_id))?;

    // `None` = the pair was not on this scenario, so nothing was removed and
    // nothing was recorded. Distinct from a real removal (Standing Rule 1).
    let Some(anchor_id) = anchor_id else {
        tracing::debug!("no fact reference to remove");
        return Err(AppError::NotFound {
            message: "fact reference not found on this scenario".to_string(),
        });
    };

    // A confirmed removal gets its own line, matching the two ruling handlers'
    // post-write logs. Without it the only trace of a successful delete is the
    // ABSENCE of an error after the request line — and "nothing was logged" is
    // exactly how the 2026-07-24 loss looked from the outside.
    tracing::info!(
        anchor_id = %anchor_id,
        graph_node_id = %graph_node_id,
        "recorded an anchored removal"
    );

    Ok(StatusCode::NO_CONTENT)
}

/// `POST /cases/:slug/scenarios/:scenario_id/facts/:graph_node_id/action` — a
/// human ruling (include / drop / un-drop) on one candidate.
///
/// Phase 1a.3: the write side of the candidate workbench. This is the first
/// producer of `FactStatus::Dropped`. All three actions write through the single
/// always-overwrite `scenario_store::upsert_fact_ref` — there is no second writer and no
/// status-only path (ratified: a human ruling always overwrites the current
/// status). Reload stays derive-on-read (`api::scenario_gather`), which this
/// route does not touch.
///
/// ## Why role / note / confidence are all `None`
///
/// `upsert_fact_ref` overwrites role, note, AND confidence — not status alone —
/// so a ruling here nulls any Theme-Scan-proposed role/confidence on the row.
/// That is correct, not lossy: NONE of these three actions is the scan-acceptance
/// path. Drop and un-drop don't need a proposed role (a dropped/undecided
/// candidate's role is meaningless), and `action=include` here is a human picking
/// a RAW candidate, not accepting a scan suggestion. Under all three rulings there
/// is no scan judgment that *should* survive, so nulling it is the right outcome.
///
/// ## SEAM for 1a.4 (scan-acceptance) — do NOT fold it into this route
///
/// Accepting a scan suggestion while KEEPING its proposed role is 1a.4, and it
/// cannot reuse this route: `confidence` (and the proposed role) are effectively
/// write-only — `confidence` is absent from `SCENARIO_FACT_REF_COLUMNS` /
/// `ScenarioFactRefRecord`, so no reader can fetch a proposed role/confidence back
/// to re-supply it on accept. 1a.4's accept path must therefore CARRY the proposed
/// role in its request (from the suggestion the UI already holds), not try to
/// preserve it in the DB. Named here so 1a.4 does not inherit a silent role-loss.
///
/// A status-flip is an idempotent ruling, not a creation, so it returns `200 OK`
/// (contrast `add_scenario_fact`'s `201` for a genuine save).
#[tracing::instrument(
    skip(state, user, payload),
    fields(slug = %slug, scenario_id = %scenario_id, graph_node_id = %graph_node_id, action = ?payload.action)
)]
pub async fn apply_fact_action(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id, graph_node_id)): Path<(String, String, String)>,
    Json(payload): Json<FactActionRequest>,
) -> Result<StatusCode, AppError> {
    require_edit(&user)?;
    tracing::info!(
        "{} POST /cases/{}/scenarios/{}/facts/{}/action ({:?})",
        user.username,
        slug,
        scenario_id,
        graph_node_id,
        payload.action
    );

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    let status = action_to_status(payload.action);
    let kind = action_to_ruling_kind(payload.action);

    // role / note = None on every action — see the doc comment above: no scan
    // judgment should survive an include/drop/undrop ruling. The defer reason is
    // the one caller-supplied field, and `record_ruling` refuses it on any verb
    // but defer (and refuses a defer without one).
    let anchor_id = record_ruling(
        &state.pipeline_pool,
        &BiasRepository::new(state.graph.clone()),
        &RulingRequest {
            scenario_id: id,
            graph_node_id: &graph_node_id,
            kind,
            ruled_by: &user.username,
            role: None,
            note: None,
            defer_reason: payload.reason.as_deref(),
        },
        status,
        Utc::now(),
    )
    .await
    .map_err(|e| ruling_error_to_app_error(e, &graph_node_id))?;

    tracing::info!(
        anchor_id = %anchor_id,
        graph_node_id = %graph_node_id,
        ruling = %kind.code(),
        "recorded an anchored ruling"
    );

    Ok(StatusCode::OK)
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
//
// The join and the vocabulary translations moved to `scenario_facts_mapping`
// along with their tests; what remains here is the path-parsing fence, the only
// pure helper this module still owns.

#[cfg(test)]
mod tests {
    use super::*;

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
}
