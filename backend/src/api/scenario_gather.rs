//! Scenario candidate-workbench gather (Phase 1a.2).
//!
//! One route that assembles a scenario's candidate pool: every Evidence node
//! ABOUT the scenario's subject, each tagged with its derived workbench status
//! and its persisted candidate identifier for THIS scenario. No LLM.
//!
//! - `GET /cases/:slug/scenarios/:scenario_id/facts/gather` → 200
//!   `{ pool: [...], dropped: [...] }`
//!
//! ## DERIVE-ON-READ (ratified) — for candidate STATE
//!
//! Candidate **state** is never persisted here. The endpoint reads the graph pool,
//! reads the persisted fact-refs, and computes each candidate's status in memory.
//! A pool node with NO ref row is `Undecided` — persisted nowhere. There is no
//! `upsert_fact_ref` and no fact-ref `INSERT` on this path; only 1a.3's
//! include/drop ever writes a ruling.
//!
//! That contract is load-bearing beyond this module: `scenario_facts::join_facts`
//! reads a fact-ref lookup MISS as "this ref points at a dead graph node", so
//! materializing a row for every pool member would corrupt that meaning.
//!
//! ## The ONE deliberate exception: identity memoization
//!
//! Gather DOES write `scenario_candidate_ordinals` — see
//! [`ensure_candidate_ordinals`]. This is not a breach of the contract above,
//! because the contract protects candidate **state** (status, score, note) and an
//! ordinal is **identity**:
//!
//! | | identity (`C-14`) | state (included / 0.85 / "key denial") |
//! |---|---|---|
//! | when it exists | the moment the candidate first appears | only once someone decides or scores something |
//! | who authors it | the system, once, mechanically | the human, repeatedly |
//! | may it change | never | that is the whole point |
//!
//! An identifier that only existed after a ruling would be useless — the human
//! needs to say "look at C-14" precisely about candidates nobody has touched yet.
//! And deriving it per-request instead would make it depend on the pool's current
//! contents and ordering, which is exactly the instability the persisted design
//! exists to prevent. So it is memoized on first sight: idempotent, append-only,
//! never touching an existing row, and carrying no user decision (hence no
//! edit-gate change).
//!
//! ## Why a separate module from `scenario_facts`
//!
//! This lives beside `api::scenario_facts` (whose `parse_scenario_id` and
//! `ensure_scenario_in_case` it reuses) rather than inside it, because
//! `scenario_facts` was already at the module-size limit (Rule 17). Same split
//! discipline as `theme_scan` / `theme_scan_judge` / `theme_scan_parse`.

use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    api::scenario_facts::{ensure_scenario_in_case, parse_scenario_id},
    auth::AuthUser,
    bias::{dto::BiasInstance, repository::BiasRepository},
    domain::fact_status::FactStatus,
    dto::{
        scenario_crud::{ScenarioDefinition, CURRENT_SCHEMA_V},
        CandidateDto, GatherCandidatesResponse,
    },
    error::AppError,
    repositories::pipeline_repository::{
        assign_candidate_ordinals, get_scenario, list_candidate_ordinals,
        list_fact_refs_for_scenario, ScenarioFactRefRecord,
    },
    services::scenario_subject::{resolve_scenario_subject, SubjectResolveError},
    state::AppState,
};

/// The workbench data reconcile carries from a fact-ref onto a candidate: the
/// decoded [`FactStatus`] plus the role/note/confidence recorded on the ref.
///
/// ## Rust Learning: a `type` alias to name a compound type
///
/// A `HashMap<String, (FactStatus, Option<String>, Option<String>, Option<f32>)>`
/// as a function signature trips `clippy::type_complexity` — and it is genuinely
/// hard to read. A `type` alias gives the tuple one name used everywhere, so the
/// index type reads as `HashMap<String, RefEntry>`. An alias is a compile-time
/// synonym (zero runtime cost, not a new type), so it stays positionally
/// destructurable as `(status, role, note, confidence)` at the use site.
type RefEntry = (FactStatus, Option<String>, Option<String>, Option<f32>);

// ── Pure reconcile (the inverted join) ─────────────────────────────────────────

/// Pair every LIVE pool node with its derived workbench status for this scenario.
///
/// Pure (no I/O) so it is unit-testable without a database or graph.
///
/// ## Domain note: this is `join_facts` INVERTED
///
/// `api::scenario_facts::join_facts` is driven by the saved REFS: it walks the
/// refs and a lookup MISS means the ref points at a dead graph node → `content:
/// null`. `reconcile_candidates` is driven by the graph POOL: it walks the pool
/// and a lookup MISS means a live node no human has ruled on → `Undecided`. Same
/// HashMap-index technique, opposite driver, opposite meaning-of-a-miss:
///
/// - build the `graph_node_id → (status, role, note)` index ONCE, then do O(1)
///   lookups while mapping the pool — NOT a nested O(n×m) scan of refs per node;
/// - the index is read with `.get()` (not `.remove()` as `join_facts` uses):
///   a ref's data is `Clone`-cheap, so we don't need the ownership-transfer
///   optimization — clarity over a micro-save.
///
/// The function returns `Result` SOLELY because of the status decode step (see
/// below); the pool walk and the partition themselves cannot fail.
///
/// ## The decode is a loud boundary (Standing Rule 1)
///
/// Each ref's raw `status` `String` is decoded to a typed [`FactStatus`] via
/// `try_from`. A token this build does not understand is a data-integrity fault
/// — it is logged with its ids and the offending token and returns
/// `AppError::Internal`. It is NEVER collapsed to `Undecided`: silently
/// bucketing an unknown status as undecided would feed a fact back to the Theme
/// Scan for re-judgment as though no human had touched it.
fn reconcile_candidates(
    pool: Vec<BiasInstance>,
    refs: Vec<ScenarioFactRefRecord>,
    ordinals: &HashMap<String, i32>,
) -> Result<GatherCandidatesResponse, AppError> {
    // The one fallible step (the status decode) lives in `build_ref_index`;
    // everything below is an infallible pool walk + partition.
    let index = build_ref_index(refs)?;

    // The POOL drives the output: exactly one CandidateDto per live pool node.
    let mut working: Vec<CandidateDto> = Vec::new();
    let mut dropped: Vec<CandidateDto> = Vec::new();

    for content in pool {
        // Miss = a live node no human has ruled on = Undecided (persisted nowhere).
        // A miss also has no confidence — an undecided candidate was never scored
        // *for this scenario*, so `None` ("unscored") is the correct absence, not 0.
        let (status, role, note, confidence) = match index.get(&content.evidence_id) {
            Some((status, role, note, confidence)) => {
                (*status, role.clone(), note.clone(), *confidence)
            }
            None => (FactStatus::Undecided, None, None, None),
        };

        // A miss here means this node has no ordinal YET (assignment runs just
        // before this walk, so in practice only a node that arrived in the same
        // instant, or one whose assignment failed, lands here). `None` is carried
        // honestly rather than substituted with a positional index — a made-up
        // number would be indistinguishable from a real one (Standing Rule 1).
        let ordinal = ordinals.get(&content.evidence_id).copied();

        let candidate = CandidateDto {
            content,
            status,
            role,
            note,
            confidence,
            ordinal,
        };

        // Dropped goes in its own list; undecided + included form the working pool.
        match status {
            FactStatus::Dropped => dropped.push(candidate),
            FactStatus::Undecided | FactStatus::Included => working.push(candidate),
        }
    }

    sort_by_ordinal(&mut working);
    sort_by_ordinal(&mut dropped);

    Ok(GatherCandidatesResponse {
        pool: working,
        dropped,
    })
}

/// Order candidates by ascending ordinal — the workbench's one display order.
///
/// ## Domain note: why ordering is decided HERE, not in the browser
///
/// The order is part of the workbench's contract, not a presentation whim: the
/// list must be stable across visits and must NEVER move a card because it was
/// scanned, merged, scored, Included, or Dropped. A client-side sort (the frontend
/// previously sorted by confidence) reintroduces exactly what the model rejects —
/// a list that reshuffles under curation, and "the score knows better than you"
/// ordering. Sorting once at the boundary keeps both listings agreeing that C-14
/// sits where C-14 always sits.
///
/// ## Rust Learning: `sort_by_key` with a tuple makes the tie-break explicit
///
/// The key is `(ordinal.is_none(), ordinal, evidence_id)`. Because `false < true`,
/// the leading bool floats numbered cards ABOVE un-numbered ones without needing a
/// custom comparator. `Option<i32>` then orders naturally among the numbered
/// entries, and `evidence_id` is a total tie-break so the result is deterministic
/// even for the (transient) un-numbered ones — an unstable tail would make the
/// list flicker between reloads.
fn sort_by_ordinal(candidates: &mut [CandidateDto]) {
    candidates.sort_by_key(|c| {
        (
            c.ordinal.is_none(),
            c.ordinal,
            c.content.evidence_id.clone(),
        )
    });
}

/// Decode the persisted fact-refs into a `graph_node_id → (status, role, note)`
/// index — the O(1) lookup table [`reconcile_candidates`] maps the pool against.
///
/// Extracted from `reconcile_candidates` to isolate the single fallible step (the
/// status decode) and keep each function within the size limit.
///
/// ## The decode is a loud boundary (Standing Rule 1)
///
/// Each ref's raw `status` `String` is decoded to a typed [`FactStatus`] via
/// `try_from`. A token this build does not understand is a data-integrity fault:
/// it is logged with its ids and the offending token, then returns
/// `AppError::Internal`. It is NEVER collapsed to `Undecided` — silently
/// bucketing an unknown status as undecided would feed a fact back to the Theme
/// Scan for re-judgment as though no human had touched it.
fn build_ref_index(
    refs: Vec<ScenarioFactRefRecord>,
) -> Result<HashMap<String, RefEntry>, AppError> {
    let mut index = HashMap::with_capacity(refs.len());
    for r in refs {
        // Decode at the boundary. `try_from` borrows the raw token; on failure we
        // log the ids and the bad token, then fail loudly (never default).
        let status = match FactStatus::try_from(r.status.as_str()) {
            Ok(status) => status,
            Err(e) => {
                tracing::error!(
                    graph_node_id = %r.graph_node_id,
                    scenario_id = %r.scenario_id,
                    token = %e.token,
                    "scenario fact ref carries a status token this build cannot interpret; \
                     refusing to mis-bucket it as undecided"
                );
                return Err(AppError::Internal {
                    message: "scenario fact reference has an unrecognized status".to_string(),
                });
            }
        };
        index.insert(
            r.graph_node_id,
            (status, r.role_in_this_scenario, r.note, r.confidence),
        );
    }
    Ok(index)
}

/// A target-less, minimally-valid definition used ONLY as the gather fallback.
///
/// ## Why gather synthesizes one instead of erroring
///
/// The shared subject resolver takes a parsed `&ScenarioDefinition`, but a
/// half-authored scenario's stored `definition` is `{}` (or a retired v1 shape),
/// which `ScenarioDefinition::from_value` deliberately REJECTS as "not yet
/// authored". The Theme Scan treats that as a hard error — it also needs
/// `attack_meaning`. Gather does NOT: viewing a draft scenario's candidate pool
/// is legitimate, so gather feeds the resolver a `target: None` definition and
/// lets it fall through to the case-default subject. Same resolver, two caller
/// policies on an unparseable definition — the divergence is deliberate
/// per-caller policy, not resolver behavior. Only `target` is read by the
/// resolver, so the other fields are inert placeholders.
fn fallback_definition() -> ScenarioDefinition {
    ScenarioDefinition {
        attack_text: String::new(),
        attack_meaning: None,
        target: None,
        wielders: Vec::new(),
        schema_v: CURRENT_SCHEMA_V,
    }
}

// ── Handler ────────────────────────────────────────────────────────────────────

/// `GET /cases/:slug/scenarios/:scenario_id/facts/gather` — the candidate pool.
///
/// Follows `list_scenario_facts` exactly for the front matter (`Option<AuthUser>`
/// audit log, `parse_scenario_id`, `ensure_scenario_in_case`), then resolves the
/// subject, reads the pool + refs, and reconciles them in memory.
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn gather_scenario_candidates(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<GatherCandidatesResponse>, AppError> {
    if let Some(ref u) = user {
        tracing::info!(
            "{} GET /cases/{}/scenarios/{}/facts/gather",
            u.username,
            slug,
            scenario_id
        );
    }

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    let subject_id = resolve_gather_subject(&state, id).await?;

    // The pool: every Evidence node ABOUT the subject (ungated by pattern_tags).
    let pool = BiasRepository::new(state.graph.clone())
        .all_evidence_about_subject(&subject_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, subject_id = %subject_id, "failed to read candidate pool from graph");
            AppError::Internal {
                message: "failed to read candidate pool".to_string(),
            }
        })?;

    // The refs: this scenario's persisted include/drop/undecided rulings.
    let refs = list_fact_refs_for_scenario(&state.pipeline_pool, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to list scenario fact refs for gather");
            AppError::Internal {
                message: "failed to list scenario fact refs".to_string(),
            }
        })?;

    // Identity memoization — the ONE deliberate write on this read path (see the
    // module doc's derive-on-read note).
    let ordinals = ensure_candidate_ordinals(&state.pipeline_pool, id, &pool).await?;

    let response = reconcile_candidates(pool, refs, &ordinals)?;
    Ok(Json(response))
}

/// Assign ordinals to any new pool members, then read the scenario's full
/// `graph_node_id → ordinal` index.
///
/// ## Why a READ endpoint is allowed to write here
///
/// Gather is derive-on-read by ratified decision, and that contract is intact:
/// it protects candidate **state** — status, score, note — none of which this
/// touches. An ordinal is **identity**, not state. It must exist for every pool
/// member the moment it first appears (an un-numbered card cannot be referred to
/// out loud), and it must never change afterwards, so it is memoized on first
/// sight rather than derived per request. Deriving it instead would make the id
/// depend on the pool's current contents and ordering — the very instability the
/// persisted design exists to prevent.
///
/// The write is idempotent, so a page refresh is free; it never touches an
/// existing row; and it carries no user decision, so no edit-gate is warranted.
///
/// Assignment order is the pool's own order, which is deterministic (see
/// `BiasRepository::all_evidence_about_subject` and the aggregation's sort key).
/// It is consulted only for candidates being numbered for the FIRST time — once
/// persisted, an ordinal is immune to any later change in pool ordering.
async fn ensure_candidate_ordinals(
    pool_db: &sqlx::PgPool,
    scenario_id: Uuid,
    pool: &[BiasInstance],
) -> Result<HashMap<String, i32>, AppError> {
    let node_ids: Vec<String> = pool.iter().map(|c| c.evidence_id.clone()).collect();

    let minted = assign_candidate_ordinals(pool_db, scenario_id, &node_ids, Utc::now())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %scenario_id,
                "failed to assign candidate ordinals during gather");
            AppError::Internal {
                message: "failed to assign candidate identifiers".to_string(),
            }
        })?;

    // Worth a log line: this is the only place new candidates are announced, and
    // "N new candidates since last review" is a question the human actually asks.
    if minted > 0 {
        tracing::info!(%scenario_id, minted, "assigned ordinals to new candidates");
    }

    list_candidate_ordinals(pool_db, scenario_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %scenario_id,
                "failed to read candidate ordinals during gather");
            AppError::Internal {
                message: "failed to read candidate identifiers".to_string(),
            }
        })
}

/// Load the scenario row and resolve the subject its pool is gathered over.
///
/// Extracted from the handler to keep it within the size limit; runs inside the
/// handler's `#[tracing::instrument]` span, so its logs still carry `slug` /
/// `scenario_id`.
///
/// ## Gather's tolerated-fallback policy (vs the Theme Scan's)
///
/// A half-authored scenario's stored `definition` is `{}` / a retired v1 shape,
/// which `ScenarioDefinition::from_value` rejects. Gather must still show that
/// scenario's pool, so a parse failure here is NOT an error — it is logged at
/// debug and falls back to a target-less [`fallback_definition`], letting the
/// shared resolver use the case default. (Contrast the Theme Scan, which errors
/// on an unparseable definition because it also needs `attack_meaning`.) The
/// divergence is a deliberate per-caller policy, not resolver behavior.
async fn resolve_gather_subject(state: &AppState, id: Uuid) -> Result<String, AppError> {
    // Re-read the row for its definition (the existence/case fence in the caller
    // does not hand it back). A `None` here is a race — the scenario was deleted
    // between the fence check and this read — so it is a 404, not a 500.
    let record = get_scenario(&state.pipeline_pool, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to load scenario for candidate gather");
            AppError::Internal {
                message: "failed to load scenario".to_string(),
            }
        })?
        .ok_or_else(|| {
            tracing::warn!("scenario vanished between the existence check and the gather load");
            AppError::NotFound {
                message: "scenario not found".to_string(),
            }
        })?;

    // Parse failure = "not yet authored" (a valid state) → fall back to a
    // target-less definition. Observable at debug, not a swallowed error.
    let definition = match ScenarioDefinition::from_value(record.definition) {
        Ok(def) => def,
        Err(e) => {
            tracing::debug!(
                scenario_id = %id,
                parse_error = %e,
                "scenario definition not authored/parseable; gather falls back to the case-default subject"
            );
            fallback_definition()
        }
    };

    resolve_scenario_subject(state, &definition)
        .await
        .map_err(|e| map_subject_error(id, e))
}

/// Map the shared [`SubjectResolveError`] into this handler's `AppError`.
///
/// - `Unresolvable` → **503** `ServiceUnavailable`: no target and no configured
///   case-default subject is a MISCONFIGURATION the operator fixes by setting
///   `CASE_DEFAULT_SUBJECT_NAME` — distinct from "zero candidates" (a 200 with an
///   empty pool). Standing Rule 1 keeps the two observables apart.
/// - `DefaultLookupFailed` → **500** `Internal`: a graph fault inside the server,
///   with the underlying cause logged.
fn map_subject_error(scenario_id: Uuid, err: SubjectResolveError) -> AppError {
    match err {
        SubjectResolveError::Unresolvable => {
            tracing::error!(
                %scenario_id,
                "cannot gather candidates: scenario names no target and no case-default \
                 subject is configured (CASE_DEFAULT_SUBJECT_NAME)"
            );
            AppError::ServiceUnavailable {
                message: "no subject configured for this case (CASE_DEFAULT_SUBJECT_NAME); \
                          cannot gather candidates"
                    .to_string(),
            }
        }
        SubjectResolveError::DefaultLookupFailed { source } => {
            tracing::error!(%scenario_id, error = %source, "failed to resolve case-default subject for gather");
            AppError::Internal {
                message: "failed to resolve scenario subject".to_string(),
            }
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "scenario_gather_tests.rs"]
mod tests;
