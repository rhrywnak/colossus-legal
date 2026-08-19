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
    domain::{scenario_code::scenario_code, wording_scenario_authoring::ScenarioAuthoringWording},
    dto::{
        scenario_crud::CURRENT_SCHEMA_V, ScenarioCreateRequest, ScenarioDefinition, ScenarioDto,
        ScenarioUpdateRequest,
    },
    error::AppError,
    repositories::pipeline_repository::{
        delete_scenario as delete_scenario_row, get_scenario, insert_scenario,
        list_scenarios_for_case, update_scenario as update_scenario_row, PipelineRepoError,
        ScenarioRecord,
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
/// The statuses a scenario may be CREATED in (task R1 Piece 4, ruled 2026-08-10).
///
/// ## Domain note: narrower than the column, deliberately
///
/// This is the one vocabulary here that is NOT a mirror of its CHECK constraint.
/// The column still permits `needs_evidence`; this list does not, and the gap is
/// the point.
///
/// `needs_evidence` was dead vocabulary that only the create form could produce:
/// measured at zero rows in task 1.5 and again on DEV on 2026-08-10, while the
/// form's Status `<select>` went on offering it. A scenario created that way then
/// rendered as **Draft** on the scenario page — `ScenarioStatusControl` has two
/// segments, and its `status === "ready"` test folds every other value into the
/// Draft one — while the dashboard card showed "Needs evidence". Two surfaces
/// disagreeing about one scenario, with no control able to move it out of the
/// state (clicking Draft on a scenario already rendering as Draft is a no-op).
///
/// So the write path closes. Ruling 6 of CC_TASK_R1_RULINGS_v1 deliberately
/// leaves the READ path alone — the CHECK constraint stays, and `statusMeta`
/// keeps its third arm — so a hand-written row still renders honestly instead of
/// rendering as nothing. Retiring the column vocabulary is a constraint swap and
/// stays filed as task 3.6's remainder.
///
/// `ready` stays here because this list also feeds nothing else: creation cannot
/// reach it through the UI (the form no longer asks), but an API caller declaring
/// a scenario ready at birth is a legitimate act the readiness ledger records
/// elsewhere, and refusing it here would be a new rule rather than a repair.
const ALLOWED_STATUSES: &[&str] = &["draft", "ready"];
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

/// `status` must be one of the statuses a scenario may be CREATED in.
///
/// See [`ALLOWED_STATUSES`] for why that is narrower than the column's CHECK.
/// The refusal names the two that are accepted rather than the three the column
/// stores, because the caller's question is "what may I send", not "what exists".
fn validate_status(status: &str) -> Result<(), AppError> {
    if !ALLOWED_STATUSES.contains(&status) {
        return Err(AppError::BadRequest {
            message: "status must be one of: draft, ready".to_string(),
            details: json!({ "field": "status" }),
        });
    }
    Ok(())
}

/// Compose the stored `definition` for a NEW scenario from the two fields the
/// create form collects, or refuse by name if either is blank.
///
/// ## Domain note: ONE attack box (Roman's ruling, 2026-08-10 — supersedes 08-07)
///
/// A v2 definition carries two texts about the attack:
///
///   `attack_text`    — what the other side claims, in their words
///   `attack_meaning` — a plain-language gloss of what that asserts
///
/// The 2026-08-07 ruling had this function seed BOTH from the one answer the
/// create form collects, so `attack_text` (required by the parse contract) was
/// never blank. It worked, and it produced a scenario that shipped with two
/// identical texts, only one of which any read surface rendered — while the Theme
/// Scan judged against the copy. Asking a human the same question twice, in two
/// boxes with different labels, is how the two drift apart later.
///
/// So there is one box now. `attack_text` is seeded; `attack_meaning` is left
/// absent, and the identity modal no longer offers to write one. The column stays
/// and no stored gloss is destroyed — `theme_scan_validate` still falls back to it
/// when a legacy row has one and no attack text, which is what keeps scenarios
/// authored before today scanning against the words their author wrote.
///
/// ## Why the refusals come from stored rows
///
/// These two sentences are the ones a human meets when the form stops them, so
/// they are configuration like every other user-facing string (v2 §2b). They
/// also each name a CONSEQUENCE rather than a constraint — see the seed
/// migration for why.
fn authored_definition(
    target: &str,
    accusation: &str,
    wording: &ScenarioAuthoringWording,
) -> Result<serde_json::Value, AppError> {
    let target = target.trim();
    if target.is_empty() {
        return Err(AppError::BadRequest {
            message: wording.create_target_required_refusal.clone(),
            details: json!({ "field": "target" }),
        });
    }

    let accusation = accusation.trim();
    if accusation.is_empty() {
        return Err(AppError::BadRequest {
            message: wording.create_accusation_required_refusal.clone(),
            details: json!({ "field": "accusation" }),
        });
    }

    let definition = ScenarioDefinition {
        attack_text: accusation.to_string(),
        // NOT seeded (2026-08-10). One question, one box, one stored answer.
        attack_meaning: None,
        target: Some(target.to_string()),
        // Nobody is named as wielding the attack yet — that is authored later in
        // the identity modal. An empty list is the honest "not yet said", and it
        // is what `ScenarioDefinition`'s `#[serde(default)]` would produce anyway.
        wielders: Vec::new(),
        schema_v: CURRENT_SCHEMA_V,
    };

    definition.to_value().map_err(|e| {
        // All-scalar shape; this cannot fail in practice. Surfaced rather than
        // unwrapped so that if it ever did, it would be an observable 500 and not
        // a panic (Standing Rule 1).
        tracing::error!(error = %e, "failed to serialize the new scenario's definition");
        AppError::Internal {
            message: "failed to serialize scenario definition".to_string(),
        }
    })
}

/// The generic update route may not change `status` (task 1.5, ruled 2026-08-01).
///
/// ## Why this is a refusal and not a validation
///
/// Every sibling here validates a value the route WILL write. This one refuses a
/// field the route must never write at all: `ready` is what puts a scenario in
/// front of a witness in rehearsal mode, and v2 §5/§6 make both directions human
/// acts with an actor recorded. This route records no actor, so allowing it would
/// let a rename carry a readiness change with nobody's name against the decision.
///
/// Refused rather than silently dropped: ignoring the field would answer "200 OK"
/// for a change that never happened (Standing Rule 1). The message names the
/// route that does the job, so the caller has somewhere to go.
///
/// Extracted as its own function — like `validate_name` and `validate_status` —
/// so the refusal is unit-testable without a database or a running router.
fn refuse_status_edit(status: Option<&str>) -> Result<(), AppError> {
    if status.is_some() {
        return Err(AppError::BadRequest {
            message: "status is not editable here — declaring a scenario ready (or \
                      taking it back out of rehearsal) is a recorded human act. \
                      Use POST /cases/:slug/scenarios/:id/ready."
                .to_string(),
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
        theme_statement: record.theme_statement,
        motivation: record.motivation,
        code: scenario_code(record.code_ordinal),
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
///
/// ## Every scenario this route creates is fully defined (2026-08-07)
///
/// It used to accept an optional `definition` and default it to `{}` — which is
/// what every UI-created scenario got, because the form could not author one.
/// The result was a scenario with no target, which silently gathered evidence
/// over the case-default subject and rendered as a copy of whichever scenario
/// had named that subject on purpose (see
/// `CC-REPORTS/CC_REPORT_SCENARIO_COPY_DIAGNOSTIC.md`).
///
/// Now the target and the accusation are required fields, and
/// [`authored_definition`] composes a definition that PARSES from the moment the
/// row exists. The un-authored state is still reachable — legacy rows carry it,
/// and the read paths render it by name — but it can no longer be created.
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

    // The definition, composed and validated before anything is written: a
    // refusal here means no row is created at all, so a half-defined scenario
    // never reaches the database (and never reaches a candidate queue).
    let definition = authored_definition(
        &payload.target,
        &payload.accusation,
        &state.settings.current().scenario_authoring_wording,
    )?;

    // Creation ALSO allocates the scenario's `S-n` code, atomically in the same
    // statement — see `INSERT_SCENARIO_SQL`. Both come back so the response can
    // name the code without a read-back.
    let (scenario_id, code_ordinal) = insert_scenario(
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
        // A freshly created scenario has a definition but no ANSWER yet: the
        // theme statement is how WE reply to the attack, and it is written later
        // on the working view. `None` is the honest answer, not an empty string.
        theme_statement: None,
        motivation: None,
        code: scenario_code(code_ordinal),
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

/// Map a delete's `rows_affected` count onto the HTTP outcome.
///
/// ## Rust Learning: a pure mapper split out from the fallible IO
///
/// The delete itself is `async` and can fail (a store fault → `Result::Err`,
/// handled in the caller). But the DECISION "did anything get deleted?" is pure:
/// given the row count, `0` is a 404 and `1` a 204 — no IO, no `async`. Splitting
/// that decision into this small `fn` (rather than inlining it in the handler)
/// makes it unit-testable WITHOUT a database — a plain `assert` over inputs, the
/// same way `map_update_error` is tested. It mirrors the "no silent success" rule:
/// a delete that matched no row is a loud 404, never a 204 pretending it worked.
///
/// A count above 1 cannot occur — `scenario_id` is the primary key, so the
/// `(scenario_id, case_slug)` fence matches at most one row — but we treat "≥ 1"
/// as success rather than asserting `== 1`, so an impossible count is still a
/// clean 204 rather than a panic.
fn delete_rows_to_status(rows_affected: u64) -> Result<StatusCode, AppError> {
    if rows_affected == 0 {
        // Valid uuid, no matching row in this case — a real miss OR a cross-case
        // mismatch (the fence makes them indistinguishable by design, so the
        // response never confirms the row exists under another case).
        tracing::debug!("no scenario deleted (unknown id or wrong case)");
        return Err(AppError::NotFound {
            message: "scenario not found".to_string(),
        });
    }
    Ok(StatusCode::NO_CONTENT)
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
    // `status` is REFUSED here rather than validated (task 1.5, ruled
    // 2026-08-01). `ready` is the gate that puts a scenario in front of a witness
    // in rehearsal mode, and v2 §5/§6 make both directions HUMAN ACTS with a
    // recorded actor. This route records no actor, so allowing it here would let
    // a rename carry a readiness change as a side effect, with nobody's name
    // against the decision.
    //
    // Refused, not silently ignored: dropping the field would tell the caller
    // "200 OK" for a change that never happened — a silent failure. The message
    // names the route that does the job.
    refuse_status_edit(payload.status.as_deref())?;

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
        // Always `None` — see the refusal above; readiness travels its own route.
        None,
        payload.feeds_count_id.as_deref(),
        payload.anchor_allegation_ids.as_deref(),
        definition.as_ref(),
        payload.theme_statement.as_deref(),
        payload.motivation.as_deref(),
        // Recorded against `theme_statement` ONLY, and only when it is actually
        // being written — the rehearsal page attributes that sentence to a human
        // by name, and a rename must not re-date it (task 2.11 C, ruling C2).
        &user.username,
    )
    .await
    .map_err(|e| map_update_error(e, &slug))?;

    // The merged row read back by RETURNING is the source of truth (a partial
    // update leaves non-provided fields at their prior DB values). 200, symmetric
    // with create's 201.
    Ok((StatusCode::OK, Json(to_dto(record))))
}

#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id))]

/// `DELETE /cases/:slug/scenarios/:scenario_id` — hard-delete one scenario.
///
/// Scenarios are disposable prep artifacts (the evidence lives in the graph, not
/// here), so this is a HARD delete: the row is removed and the `ON DELETE CASCADE`
/// chain takes its curated facts and responses with it. Success is `204 No
/// Content` (no body). A valid uuid that names no row in this case is `404`; a
/// malformed uuid is `400`; a store fault is `500`.
///
/// ## Rust Learning: `Path((slug, scenario_id))` and `Result<StatusCode, AppError>`
///
/// The two path segments are destructured straight out of the `Path` extractor
/// tuple — `Path((slug, scenario_id)): Path<(String, String)>` binds both in one
/// pattern. The return type `Result<StatusCode, AppError>` is the whole HTTP
/// contract in one line: the `Ok` arm carries the status to send, and every `?`
/// below turns an error into an `AppError` that Axum renders as the right status —
/// so the error path never has to hand-build a response.
///
/// The `#[instrument]` span carries `slug` + `scenario_id` (mirroring the peer
/// handlers), so every event below — including the store-fault `tracing::error!`
/// — inherits WHICH scenario failed without re-stating it.
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn delete_scenario(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    require_edit(&user)?;
    tracing::info!(
        "{} DELETE /cases/{}/scenarios/{}",
        user.username,
        slug,
        scenario_id
    );

    // A malformed uuid is a client error (400), not a server fault (500) — same as
    // get_scenario_by_id / update_scenario.
    let id = Uuid::parse_str(&scenario_id).map_err(|_| AppError::BadRequest {
        message: "scenario_id must be a valid UUID".to_string(),
        details: json!({ "field": "scenario_id" }),
    })?;

    crate::api::practice_fences::refuse_if_practised(&state, id).await?;

    // The store delete returns the row count (a no-such-row is Ok(0), NOT an
    // error). A genuine store fault (Err) is a 500 we log with context; the `?`
    // threads that PipelineRepoError into the HTTP mapping.
    let rows_affected = delete_scenario_row(&state.pipeline_pool, id, &slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, case_slug = %slug, "failed to delete scenario");
            AppError::Internal {
                message: "failed to delete scenario".to_string(),
            }
        })?;

    // 0 rows → 404, ≥ 1 → 204. The count-to-status decision is pure and tested.
    delete_rows_to_status(rows_affected)
}

#[cfg(test)]
#[path = "scenarios_tests.rs"]
mod tests;
