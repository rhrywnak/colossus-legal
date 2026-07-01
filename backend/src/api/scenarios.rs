//! Scenario CRUD HTTP routes (authored-state store, task 1.1 — Chunk 1).
//!
//! Three read+create routes over the existing Postgres `scenario_store` free
//! functions. Follows the `api/claims.rs` CRUD precedent: `State` + `Path`/`Json`
//! extractors, `Option<AuthUser>` for reads / `AuthUser` + `require_edit` for the
//! write, `AppError` mapping, one `tracing::info!` line per handler.
//!
//! ## CRITICAL — the pipeline pool
//!
//! The `scenarios` table lives in the **pipeline database** (`colossus_legal_v2`).
//! Every store call here passes `&state.pipeline_pool`, NOT `state.pg_pool`
//! (a different database — using it would yield "relation scenarios does not
//! exist").

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{require_edit, AuthUser},
    dto::{ScenarioCreateRequest, ScenarioDto, ScenarioUpdateRequest},
    error::AppError,
    repositories::pipeline_repository::{
        get_scenario, insert_scenario, list_scenarios_for_case,
        update_scenario as update_scenario_row, PipelineRepoError, ScenarioRecord,
    },
    state::AppState,
};

// CONST: the DB CHECK-constraint vocabularies for `scenarios.direction` /
// `scenarios.status`. These mirror the table's CHECK constraints exactly so a
// bad value is rejected as a 400 here BEFORE the insert (rather than surfacing as
// a 500 from the constraint). They are a schema-coupling invariant, not a
// deployment knob — changing them requires a matching migration (Standing Rule 2
// does not apply; same rationale as the store's column projections).
const ALLOWED_DIRECTIONS: &[&str] = &["offense", "defense"];
const ALLOWED_STATUSES: &[&str] = &["draft", "needs_evidence", "ready"];
/// The status applied when the create request omits one (mirrors the column's
/// `'draft'` default so the Rust path and the DB backstop agree).
const DEFAULT_STATUS: &str = "draft";

// ── Validation (pure, unit-tested without a DB) ──────────────────────────────

/// `name` must carry non-whitespace content (the column is NOT NULL, and a blank
/// name is a useless scenario label).
fn validate_name(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::BadRequest {
            message: "name must not be empty".to_string(),
            details: json!({ "field": "name" }),
        });
    }
    Ok(())
}

/// `direction` must be one of the table's CHECK values.
fn validate_direction(direction: &str) -> Result<(), AppError> {
    if !ALLOWED_DIRECTIONS.contains(&direction) {
        return Err(AppError::BadRequest {
            message: "direction must be one of: offense, defense".to_string(),
            details: json!({ "field": "direction" }),
        });
    }
    Ok(())
}

/// `status` must be one of the table's CHECK values.
fn validate_status(status: &str) -> Result<(), AppError> {
    if !ALLOWED_STATUSES.contains(&status) {
        return Err(AppError::BadRequest {
            message: "status must be one of: draft, needs_evidence, ready".to_string(),
            details: json!({ "field": "status" }),
        });
    }
    Ok(())
}

/// Map a stored `ScenarioRecord` onto the wire DTO.
///
/// Two adaptations: the `Uuid` is rendered as a string, and
/// `anchor_allegation_ids: None` flattens to `[]` (the wire never sees null for a
/// list the client only iterates). Timestamps are dropped for this chunk.
fn to_dto(record: ScenarioRecord) -> ScenarioDto {
    ScenarioDto {
        scenario_id: record.scenario_id.to_string(),
        name: record.name,
        direction: record.direction,
        status: record.status,
        case_slug: record.case_slug,
        feeds_count_id: record.feeds_count_id,
        anchor_allegation_ids: record.anchor_allegation_ids.unwrap_or_default(),
        definition: record.definition,
    }
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// `GET /cases/:slug/scenarios` — list a case's scenarios (newest first).
#[tracing::instrument(skip(state, user), fields(slug = %slug))]
pub async fn list_scenarios(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Vec<ScenarioDto>>, AppError> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /cases/{}/scenarios", u.username, slug);
    }

    let records = list_scenarios_for_case(&state.pipeline_pool, &slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, case_slug = %slug, "failed to list scenarios");
            AppError::Internal {
                message: "failed to list scenarios".to_string(),
            }
        })?;

    let dtos = records.into_iter().map(to_dto).collect();
    Ok(Json(dtos))
}

/// `GET /cases/:slug/scenarios/:scenario_id` — read one scenario.
///
/// The lookup is by globally-unique `scenario_id`, but the returned row's
/// `case_slug` is then checked against the URL `slug`: a mismatch yields
/// `NotFound`, so a scenario cannot be read through a different case's path. This
/// is the read-side of the same path-as-fence invariant `create_scenario` holds
/// on the write side.
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn get_scenario_by_id(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<ScenarioDto>, AppError> {
    if let Some(ref u) = user {
        tracing::info!(
            "{} GET /cases/{}/scenarios/{}",
            u.username,
            slug,
            scenario_id
        );
    }

    // A malformed uuid is a client error (400), not a server fault (500).
    let id = Uuid::parse_str(&scenario_id).map_err(|_| AppError::BadRequest {
        message: "scenario_id must be a valid UUID".to_string(),
        details: json!({ "field": "scenario_id" }),
    })?;

    // The span (above) carries `slug` + `scenario_id`, so these events inherit
    // those fields without re-stating them.
    let record = get_scenario(&state.pipeline_pool, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch scenario");
            AppError::Internal {
                message: "failed to fetch scenario".to_string(),
            }
        })?
        .ok_or_else(|| {
            // Valid uuid, no such row — distinct from a malformed uuid (400) and
            // from a store error (500). Logged so the miss is observable.
            tracing::debug!("scenario not found");
            AppError::NotFound {
                message: "scenario not found".to_string(),
            }
        })?;

    // Case-isolation fence: the row must belong to the case named in the URL.
    // `scenario_id` is globally unique, so a mismatch means the caller reached a
    // real scenario through the wrong case path. Return NotFound (not Forbidden)
    // so the response does not confirm the row exists under another case.
    if record.case_slug != slug {
        tracing::warn!(actual_case = %record.case_slug, "scenario requested through the wrong case path");
        return Err(AppError::NotFound {
            message: "scenario not found".to_string(),
        });
    }

    Ok(Json(to_dto(record)))
}

/// `POST /cases/:slug/scenarios` — create a scenario in the URL's case.
///
/// `case_slug` is sourced from the PATH, never the body, so a request cannot
/// write a scenario into a different case than its URL names.
#[tracing::instrument(skip(state, user, payload), fields(slug = %slug))]
pub async fn create_scenario(
    user: AuthUser,
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(payload): Json<ScenarioCreateRequest>,
) -> Result<(StatusCode, Json<ScenarioDto>), AppError> {
    require_edit(&user)?;
    tracing::info!("{} POST /cases/{}/scenarios", user.username, slug);

    // Validate against the table's CHECK vocabularies BEFORE insert so a bad
    // value is a named 400, not a 500 from the constraint.
    let name = payload.name.trim().to_string();
    validate_name(&name)?;
    validate_direction(&payload.direction)?;
    let status = payload.status.unwrap_or_else(|| DEFAULT_STATUS.to_string());
    validate_status(&status)?;

    // The column is NOT NULL — an omitted definition becomes `{}`, never SQL null.
    let definition = payload.definition.unwrap_or_else(|| json!({}));

    let scenario_id = insert_scenario(
        &state.pipeline_pool,
        &name,
        &payload.direction,
        &status,
        &slug,
        payload.feeds_count_id.as_deref(),
        payload.anchor_allegation_ids.as_deref(),
        &definition,
    )
    .await
    .map_err(|e| {
        // A CHECK violation that slips past validation lands here, logged with
        // its cause — surfaced as a 500, never a silent success (Standing Rule 1).
        tracing::error!(error = %e, case_slug = %slug, "failed to create scenario");
        AppError::Internal {
            message: "failed to create scenario".to_string(),
        }
    })?;

    // Construct the response from the validated request + the DB-minted id. This
    // is the path with NO second failure mode (the instruction's preference): the
    // values inserted are exactly the values returned, so no read-back is needed.
    let dto = ScenarioDto {
        scenario_id: scenario_id.to_string(),
        name,
        direction: payload.direction,
        status,
        case_slug: slug,
        feeds_count_id: payload.feeds_count_id,
        anchor_allegation_ids: payload.anchor_allegation_ids.unwrap_or_default(),
        definition,
    };

    Ok((StatusCode::CREATED, Json(dto)))
}

/// Map an `update_scenario` store error onto the HTTP surface.
///
/// `NotFound` (no row for the `(scenario_id, case_slug)` pair — a missing id OR a
/// cross-case mismatch) becomes a `404`, so the response never confirms the row
/// exists under a different case (the write-side of the read fence). Anything
/// else is an unexpected server fault (`500`), logged with its cause so the
/// failure is observable (Standing Rule 1). Extracted from the handler to keep it
/// under the function-length limit.
fn map_update_error(error: PipelineRepoError, slug: &str) -> AppError {
    match error {
        PipelineRepoError::NotFound(_) => {
            tracing::debug!("scenario not found for update");
            AppError::NotFound {
                message: "scenario not found".to_string(),
            }
        }
        other => {
            tracing::error!(error = %other, case_slug = %slug, "failed to update scenario");
            AppError::Internal {
                message: "failed to update scenario".to_string(),
            }
        }
    }
}

/// `PUT /cases/:slug/scenarios/:scenario_id` — partially update a scenario.
///
/// Mirrors [`create_scenario`]'s auth / extractor / pool / error shape. The
/// differences are the whole point of B1: every body field is optional (absent =
/// leave unchanged), `direction` is not updatable, and — unlike create — the
/// response is built from the row the store reads back via `RETURNING`, because a
/// partial update leaves non-provided fields at DB values the handler does not
/// hold.
///
/// The cross-case fence lives in the store's `WHERE ... AND case_slug = $`: an
/// update reached through the wrong `:slug` matches zero rows and surfaces as a
/// `404` (same as the read fence), never confirming the row exists under another
/// case.
#[tracing::instrument(skip(state, user, payload), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn update_scenario(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
    Json(payload): Json<ScenarioUpdateRequest>,
) -> Result<(StatusCode, Json<ScenarioDto>), AppError> {
    require_edit(&user)?;
    tracing::info!(
        "{} PUT /cases/{}/scenarios/{}",
        user.username,
        slug,
        scenario_id
    );

    // A malformed uuid is a client error (400), not a server fault (500) — same
    // as `get_scenario_by_id`.
    let id = Uuid::parse_str(&scenario_id).map_err(|_| AppError::BadRequest {
        message: "scenario_id must be a valid UUID".to_string(),
        details: json!({ "field": "scenario_id" }),
    })?;

    // Validate ONLY the fields being changed; an absent field is left untouched,
    // so there is nothing to validate. A bad `status` would otherwise surface as
    // a 500 from the CHECK constraint instead of a named 400.
    if let Some(ref name) = payload.name {
        validate_name(name)?;
    }
    if let Some(ref status) = payload.status {
        validate_status(status)?;
    }

    // Typed definition → opaque jsonb for the store (symmetric with create). A
    // MALFORMED definition body was already rejected as a 400 by the JSON
    // extractor before this handler ran (the loud boundary). `to_value` failing
    // here is a serialization fault we surface rather than unwrap (Standing
    // Rule 1); `.transpose()` turns `Option<Result<_>>` into `Result<Option<_>>`.
    let definition = payload
        .definition
        .as_ref()
        .map(|d| d.to_value())
        .transpose()
        .map_err(|e| {
            tracing::error!(error = %e, case_slug = %slug, "failed to serialize scenario definition");
            AppError::Internal {
                message: "failed to serialize scenario definition".to_string(),
            }
        })?;

    // Trim a provided name to match create's normalization; owned so the `&str`
    // bind below borrows from a live value.
    let name = payload.name.as_ref().map(|n| n.trim().to_string());

    let record = update_scenario_row(
        &state.pipeline_pool,
        id,
        &slug,
        name.as_deref(),
        payload.status.as_deref(),
        payload.feeds_count_id.as_deref(),
        payload.anchor_allegation_ids.as_deref(),
        definition.as_ref(),
    )
    .await
    .map_err(|e| map_update_error(e, &slug))?;

    // The merged row read back by RETURNING is the source of truth (a partial
    // update leaves non-provided fields at their prior DB values). 200, symmetric
    // with create's 201.
    Ok((StatusCode::OK, Json(to_dto(record))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record(anchor: Option<Vec<String>>) -> ScenarioRecord {
        // A fixed epoch timestamp keeps the record deterministic (and avoids the
        // chrono `clock` feature); `to_dto` drops timestamps anyway.
        let ts = chrono::DateTime::from_timestamp(0, 0).expect("epoch is valid");
        ScenarioRecord {
            scenario_id: Uuid::nil(),
            name: "Marie is obstructive".to_string(),
            direction: "defense".to_string(),
            status: "draft".to_string(),
            case_slug: "awad_v_catholic_family_service".to_string(),
            feeds_count_id: None,
            anchor_allegation_ids: anchor,
            definition: json!({}),
            created_at: ts,
            updated_at: ts,
        }
    }

    #[test]
    fn validate_name_rejects_blank() {
        // The error must be a BadRequest naming the field (the response contract),
        // not merely "an error" — matching the direction/status validator tests.
        match validate_name("   ") {
            Err(AppError::BadRequest { details, .. }) => {
                assert_eq!(details, json!({ "field": "name" }));
            }
            other => panic!("expected BadRequest naming name, got {other:?}"),
        }
        // An empty string is rejected the same way.
        assert!(validate_name("").is_err());
    }

    #[test]
    fn validate_name_accepts_nonempty() {
        assert!(validate_name("Marie is obstructive").is_ok());
    }

    #[test]
    fn validate_direction_accepts_both_valid() {
        assert!(validate_direction("offense").is_ok());
        assert!(validate_direction("defense").is_ok());
    }

    #[test]
    fn validate_direction_rejects_unknown() {
        match validate_direction("sideways") {
            Err(AppError::BadRequest { details, .. }) => {
                assert_eq!(details, json!({ "field": "direction" }));
            }
            other => panic!("expected BadRequest naming direction, got {other:?}"),
        }
    }

    #[test]
    fn validate_status_accepts_all_three_valid() {
        for s in ["draft", "needs_evidence", "ready"] {
            assert!(validate_status(s).is_ok(), "status {s} should be valid");
        }
    }

    #[test]
    fn validate_status_rejects_unknown() {
        match validate_status("archived") {
            Err(AppError::BadRequest { details, .. }) => {
                assert_eq!(details, json!({ "field": "status" }));
            }
            other => panic!("expected BadRequest naming status, got {other:?}"),
        }
    }

    #[test]
    fn to_dto_flattens_none_anchor_to_empty_vec() {
        let dto = to_dto(sample_record(None));
        assert_eq!(dto.anchor_allegation_ids, Vec::<String>::new());
        // The Uuid renders as its canonical string form.
        assert_eq!(dto.scenario_id, "00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn to_dto_preserves_populated_anchor() {
        let ids = vec![
            "doc-awad-v-catholic-family-complaint-11-1-13:allegation:cd24fccb".to_string(),
            "doc-x:allegation:def".to_string(),
        ];
        let dto = to_dto(sample_record(Some(ids.clone())));
        assert_eq!(dto.anchor_allegation_ids, ids);
    }

    #[test]
    fn map_update_error_not_found_becomes_404() {
        // A store `NotFound` (missing id OR cross-case mismatch) must surface as a
        // 404, so the response never confirms the row exists under another case.
        match map_update_error(
            PipelineRepoError::NotFound("some-uuid".to_string()),
            "awad_v_cfs",
        ) {
            AppError::NotFound { message } => assert!(message.contains("not found")),
            other => panic!("expected NotFound → 404, got {other:?}"),
        }
    }

    #[test]
    fn map_update_error_other_becomes_500() {
        // Any non-NotFound store error is an unexpected server fault → 500, never a
        // silent success (Standing Rule 1).
        match map_update_error(
            PipelineRepoError::Database("conn refused".to_string()),
            "awad_v_cfs",
        ) {
            AppError::Internal { message } => assert!(message.contains("update scenario")),
            other => panic!("expected Internal → 500, got {other:?}"),
        }
    }
}
