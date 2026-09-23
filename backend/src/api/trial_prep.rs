//! `GET /api/cases/:slug/trial-prep/dashboard` — the War Room dashboard payload.
//!
//! The response is a full `TrialPrepDashboard` assembled by
//! [`ScenarioDashboardAssembler`] from the case's real `scenarios` rows
//! (Postgres pipeline DB) plus each card's live REBUTS count from the graph.
//! With no scenarios authored yet the dashboard is honestly empty (no cards,
//! zeroed metrics, no alerts).
//!
//! Handler pattern (precedent: `claims.rs`): build the assembler from
//! `state.graph.clone()` + `state.pipeline_pool.clone()`, call it with the case
//! slug, return JSON. Observability + the bland 500 body mirror `proof_matrix.rs`.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::dto::scenario_authoring_wording::create_wording;
use crate::dto::trial_prep::{ScenarioDetail, TrialPrepDashboard};
use crate::dto::war_room_wording::WarRoomWordingDto;
use crate::repositories::scenario_repository::ScenarioRepository;
use crate::services::scenario_dashboard::ScenarioDashboardAssembler;
use crate::state::AppState;

use super::war_room_progress::read_progress;

/// Error type for the trial-prep endpoints.
///
/// The dashboard read only ever fails internally (it always has a valid payload
/// otherwise), so it uses `Internal`. The scenario-detail read adds two client
/// errors: a malformed `scenario_id` (`BadRequest`) and an absent scenario
/// (`NotFound` — the legitimate deleted/unknown id). Every 5xx detail is logged
/// for operators and never returned to the client (Standing Rule 1); the 4xx
/// bodies carry only a short, non-sensitive reason.
pub enum TrialPrepEndpointError {
    /// Graph/store error or row-decode failure → HTTP 500 (logged, not returned).
    Internal,
    /// No scenario with the requested id → HTTP 404.
    NotFound,
    /// A malformed path value (e.g. a non-UUID scenario id) → HTTP 400.
    BadRequest { reason: &'static str },
}

/// Error body: a single `error` string. Reused for the 400/404/500 responses; it
/// never carries Cypher, a store cause, or a `details` object (Standing Rule 1 —
/// no leak to the client).
#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
}

impl IntoResponse for TrialPrepEndpointError {
    fn into_response(self) -> Response {
        match self {
            TrialPrepEndpointError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "internal server error",
                }),
            )
                .into_response(),
            TrialPrepEndpointError::NotFound => (
                StatusCode::NOT_FOUND,
                Json(ErrorBody {
                    error: "scenario not found",
                }),
            )
                .into_response(),
            TrialPrepEndpointError::BadRequest { reason } => {
                (StatusCode::BAD_REQUEST, Json(ErrorBody { error: reason })).into_response()
            }
        }
    }
}

/// `GET /api/cases/:slug/trial-prep/dashboard`. No auth required, matching the
/// other case-scoped reads; the user is logged when present.
///
/// `slug` selects which case's scenarios to assemble — it is passed to the
/// assembler, which lists that case's rows from Postgres and computes each card's
/// live count from the graph.
#[instrument(skip(state, user), fields(slug = %slug))]
pub async fn get_trial_prep_dashboard(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<TrialPrepDashboard>, TrialPrepEndpointError> {
    if let Some(u) = &user {
        info!(username = %u.username, "GET /api/cases/{slug}/trial-prep/dashboard");
    }

    // Both handles are cheap clones (Graph and PgPool are each Arc-backed).
    let assembler = ScenarioDashboardAssembler::new(
        ScenarioRepository::new(state.graph.clone()),
        state.pipeline_pool.clone(),
    );

    // One settings snapshot, read here and handed down: the create form's words
    // and everything else on this response then come from the same store state.
    // ONE snapshot for both blocks: the form's words and the page's own words
    // must come from the same read of the store, or a Settings edit landing
    // mid-request could word the heading from one snapshot and the form from
    // another.
    let settings = state.settings.current();
    let create_wording = create_wording(&settings.scenario_authoring_wording);
    // `{reviewer}` on the dashboard names whoever owes the review queue — one
    // name, or the whole bench joined, decided in ONE place so the dashboard and
    // the deck's own bar cannot disagree about who is being waited on.
    let war_room_wording = WarRoomWordingDto::new(
        &settings.war_room_wording,
        &crate::services::war_room_progress::reviewer_display_line(&settings),
    );

    let started = std::time::Instant::now();
    let mut dashboard = assembler
        .assemble(&slug, create_wording, war_room_wording)
        .await
        .map_err(internal("assemble trial-prep dashboard"))?;

    // CC_TASK_WAR_ROOM_v1: each card's status, read for every scenario at once.
    // WHOSE review queue the cards report. Empty for an unidentified caller,
    // which reads as "has seen nothing" — see `read_progress`.
    let viewer = user
        .as_ref()
        .map(|u| crate::services::practice_notes::attribution(u).0)
        .unwrap_or_default();
    attach_progress(&state, &mut dashboard, &viewer).await?;

    // Ruling Q1, condition 2: the whole handler is timed, so the cost of the
    // status card is a number in the log rather than a guess.
    tracing::info!(
        %slug,
        // The cards' review counts are this person's since L2 — see
        // `read_progress`. Empty for an unidentified caller.
        %viewer,
        scenarios = dashboard.scenarios.len(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "served the trial-prep dashboard"
    );
    Ok(Json(dashboard))
}

/// Replace every card's zeroed `progress` with the status reads.
///
/// ## Why a scenario the reads did not cover fails the request
///
/// `record_to_card` starts each card at zero. If this left one untouched, the page
/// would show a scenario with no facts, no deck and "Up to date" — a confident,
/// false card. So a card with no entry, or an id that does not parse, is a logged
/// 500 naming the scenario, never a quiet zero (Standing Rule 1).
async fn attach_progress(
    state: &AppState,
    dashboard: &mut TrialPrepDashboard,
    viewer: &str,
) -> Result<(), TrialPrepEndpointError> {
    let ids = card_ids(dashboard)?;
    let progress = read_progress(state, &ids, viewer).await.map_err(|e| {
        tracing::error!(error = ?e, "the war room's status reads failed");
        TrialPrepEndpointError::Internal
    })?;
    fill_progress(dashboard, &ids, progress)
}

/// Every card's id as a UUID, or a logged 500 naming the one that is not.
fn card_ids(dashboard: &TrialPrepDashboard) -> Result<Vec<Uuid>, TrialPrepEndpointError> {
    dashboard
        .scenarios
        .iter()
        .map(|card| {
            Uuid::parse_str(&card.id).map_err(|e| {
                tracing::error!(error = %e, id = %card.id, "a dashboard card carries an id that is not a UUID");
                TrialPrepEndpointError::Internal
            })
        })
        .collect()
}

/// Put each scenario's progress on its card; a card left without one is a 500.
///
/// `ids` is `card_ids(dashboard)`, in card order — the zip pairs them.
fn fill_progress(
    dashboard: &mut TrialPrepDashboard,
    ids: &[Uuid],
    mut progress: std::collections::HashMap<Uuid, crate::dto::war_room_progress::ScenarioProgress>,
) -> Result<(), TrialPrepEndpointError> {
    for (card, id) in dashboard.scenarios.iter_mut().zip(ids) {
        card.progress = progress.remove(id).ok_or_else(|| {
            tracing::error!(scenario_id = %id, "the status reads returned nothing for this card");
            TrialPrepEndpointError::Internal
        })?;
    }
    Ok(())
}

/// `GET /api/cases/:slug/trial-prep/scenarios/:scenario_id` — one scenario's
/// detail (its record + its anchor allegations' graph evidence as a timeline).
///
/// `slug` selects the case (logged); the scenario is identified by the
/// globally-unique `scenario_id`. A malformed id → 400; an absent scenario → 404
/// (the legitimate deleted/unknown id); a store/graph failure → a logged 500.
#[instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn get_trial_prep_scenario_detail(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<ScenarioDetail>, TrialPrepEndpointError> {
    if let Some(u) = &user {
        info!(username = %u.username, "GET /api/cases/{slug}/trial-prep/scenarios/{scenario_id}");
    }

    // A malformed uuid is a client error (400), not a server fault.
    let id = Uuid::parse_str(&scenario_id).map_err(|_| TrialPrepEndpointError::BadRequest {
        reason: "scenario_id must be a valid UUID",
    })?;

    let assembler = ScenarioDashboardAssembler::new(
        ScenarioRepository::new(state.graph.clone()),
        state.pipeline_pool.clone(),
    );

    // `None` from the assembler ⇒ no such scenario row ⇒ 404 (distinct from a
    // store/graph error, which the `internal` closure collapses to a logged 500).
    assembler
        .assemble_detail(id)
        .await
        .map_err(internal("assemble scenario detail"))?
        .map(Json)
        .ok_or(TrialPrepEndpointError::NotFound)
}

/// Map any displayable error to a logged `Internal` (500). `op` names the failed
/// step for the operator log; the client only ever sees the bland 500 body.
///
/// ## Rust Learning: returning `impl Fn(E) -> _` for `.map_err`
///
/// Mirrors `proof_matrix::internal`: the returned closure captures `op`, logs
/// the underlying error with full context, and collapses every failure into the
/// single opaque `Internal` variant — the place where the `?` chain terminates
/// at a handler that logs (Standing Rule 1).
fn internal<E: std::fmt::Display>(op: &'static str) -> impl Fn(E) -> TrialPrepEndpointError {
    move |e| {
        // `op` (the `operation` field) carries the specific step; the message
        // stays generic so it never contradicts the operation (this closure is
        // shared by the dashboard and scenario-detail handlers).
        error!(error = %e, operation = op, "trial-prep request failed");
        TrialPrepEndpointError::Internal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The 500 body must never carry Cypher, an error message, or a `details`
    /// object — exactly one key, `error`. Guards Standing Rule 1's "no leak to
    /// the client" requirement at the serialization boundary.
    #[test]
    fn internal_server_error_body_does_not_leak_detail() {
        let body = ErrorBody {
            error: "internal server error",
        };
        let value = serde_json::to_value(&body).expect("body serializes");
        assert_eq!(value, json!({"error": "internal server error"}));
        assert_eq!(value.as_object().expect("object body").len(), 1);
    }

    /// `Internal` must map to HTTP 500. The body-shape test above cannot catch a
    /// regression that swapped the status code in the `IntoResponse` match arm.
    #[test]
    fn internal_variant_maps_to_500() {
        let response = TrialPrepEndpointError::Internal.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// `NotFound` (absent scenario) must map to HTTP 404.
    #[test]
    fn not_found_variant_maps_to_404() {
        let response = TrialPrepEndpointError::NotFound.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    /// `BadRequest` (malformed uuid) must map to HTTP 400 and echo only the bland
    /// reason — no internal detail.
    #[test]
    fn bad_request_variant_maps_to_400() {
        let response = TrialPrepEndpointError::BadRequest {
            reason: "scenario_id must be a valid UUID",
        }
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // ── attach_progress's pure halves (CC_TASK_WAR_ROOM_v1) ─────────────────────

    use crate::dto::scenario_authoring_wording::ScenarioCreateWordingDto;
    use crate::dto::trial_prep::{ScenarioStatus, ScenarioSummary, TrialPrepMetrics};
    use crate::dto::war_room_progress::ScenarioProgress;
    use std::collections::HashMap;

    const S1: &str = "00000000-0000-0000-0000-000000000001";
    const S2: &str = "00000000-0000-0000-0000-000000000002";

    fn card(id: &str) -> ScenarioSummary {
        ScenarioSummary {
            id: id.to_string(),
            code: "S-1".to_string(),
            attack: "attack".to_string(),
            status: ScenarioStatus::Draft,
            baseless_repeat_count: None,
            theme_statement: None,
            progress: ScenarioProgress::default(),
        }
    }

    fn dashboard(ids: &[&str]) -> TrialPrepDashboard {
        let word = || "w".to_string();
        TrialPrepDashboard {
            metrics: TrialPrepMetrics {
                scenarios: 0,
                ready: 0,
                drafted_or_review: 0,
            },
            alerts: Vec::new(),
            scenarios: ids.iter().map(|id| card(id)).collect(),
            create_wording: ScenarioCreateWordingDto {
                target_label: word(),
                target_helper: word(),
                target_unset_option: word(),
                accusation_label: word(),
                accusation_helper: word(),
                target_required: word(),
                accusation_required: word(),
            },
            war_room_wording: WarRoomWordingDto::new(
                &crate::domain::wording_war_room::WarRoomWording::for_test(),
                "Chuck",
            ),
        }
    }

    /// A card whose id is not a UUID is a 500, not a skipped card.
    #[test]
    fn a_card_with_a_non_uuid_id_is_an_internal_error() {
        let d = dashboard(&[S1, "marie-obstructive"]);
        assert!(matches!(
            card_ids(&d),
            Err(TrialPrepEndpointError::Internal)
        ));
    }

    /// Well-formed ids come back in card order.
    #[test]
    fn card_ids_are_parsed_in_card_order() {
        let ids = match card_ids(&dashboard(&[S2, S1])) {
            Ok(ids) => ids,
            Err(_) => panic!("both ids are UUIDs"),
        };
        assert_eq!(
            ids.iter().map(Uuid::to_string).collect::<Vec<_>>(),
            [S2, S1]
        );
    }

    /// Each card receives ITS scenario's progress.
    #[test]
    fn fill_progress_puts_each_scenario_on_its_own_card() {
        let mut d = dashboard(&[S1, S2]);
        let ids: Vec<Uuid> = [S1, S2]
            .iter()
            .map(|s| Uuid::parse_str(s).expect("uuid"))
            .collect();
        let mut two = ScenarioProgress::default();
        two.marie_changed = 2;
        let map = HashMap::from([(ids[0], ScenarioProgress::default()), (ids[1], two)]);
        assert!(fill_progress(&mut d, &ids, map).is_ok());
        assert_eq!(d.scenarios[0].progress.marie_changed, 0);
        assert_eq!(d.scenarios[1].progress.marie_changed, 2);
    }

    /// A card the reads returned nothing for is a 500, never a zero card.
    #[test]
    fn a_card_the_reads_missed_is_an_internal_error_not_a_zero_card() {
        let mut d = dashboard(&[S1, S2]);
        let ids: Vec<Uuid> = [S1, S2]
            .iter()
            .map(|s| Uuid::parse_str(s).expect("uuid"))
            .collect();
        let only_first = HashMap::from([(ids[0], ScenarioProgress::default())]);
        assert!(matches!(
            fill_progress(&mut d, &ids, only_first),
            Err(TrialPrepEndpointError::Internal)
        ));
    }
}
