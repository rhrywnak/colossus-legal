//! Scenario candidate-workbench gather (Phase 1a.2).
//!
//! One **read-only** route that assembles a scenario's candidate pool: every
//! Evidence node ABOUT the scenario's subject, each tagged with its derived
//! workbench status for THIS scenario. No writes, no migration, no LLM.
//!
//! - `GET /cases/:slug/scenarios/:scenario_id/facts/gather` → 200
//!   `{ pool: [...], dropped: [...] }`
//!
//! ## DERIVE-ON-READ (ratified)
//!
//! The endpoint NEVER writes. It reads the graph pool, reads the persisted
//! fact-refs, and computes each candidate's status in memory. A pool node with
//! NO ref row is `Undecided` — persisted nowhere. There is no `INSERT`, no
//! `upsert_fact_ref`, no `ON CONFLICT` here; only 1a.3's include/drop ever
//! writes.
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
        get_scenario, list_fact_refs_for_scenario, ScenarioFactRefRecord,
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

        let candidate = CandidateDto {
            content,
            status,
            role,
            note,
            confidence,
        };

        // Dropped goes in its own list; undecided + included form the working pool.
        match status {
            FactStatus::Dropped => dropped.push(candidate),
            FactStatus::Undecided | FactStatus::Included => working.push(candidate),
        }
    }

    Ok(GatherCandidatesResponse {
        pool: working,
        dropped,
    })
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

    let response = reconcile_candidates(pool, refs)?;
    Ok(Json(response))
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
mod tests {
    use super::*;

    /// A minimal `BiasInstance` carrying just an id — enough to drive reconcile.
    fn content(evidence_id: &str) -> BiasInstance {
        BiasInstance {
            evidence_id: evidence_id.to_string(),
            title: String::new(),
            verbatim_quote: None,
            question: None,
            page_number: None,
            pattern_tags: Vec::new(),
            stated_by: None,
            about: Vec::new(),
            document: None,
        }
    }

    /// A `scenario_fact_refs` row with the given node id, raw status token, and
    /// optional role/note. Confidence defaults to `None` (an unscored / human-
    /// curated ref); tests that exercise the confidence path use [`scored_ref`].
    fn fact_ref(
        node: &str,
        status: &str,
        role: Option<&str>,
        note: Option<&str>,
    ) -> ScenarioFactRefRecord {
        ScenarioFactRefRecord {
            scenario_id: Uuid::nil(),
            graph_node_id: node.to_string(),
            role_in_this_scenario: role.map(str::to_string),
            status: status.to_string(),
            note: note.map(str::to_string),
            confidence: None,
            tagged_at: chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
        }
    }

    /// A merged/scanned ref: an `undecided` row carrying a model role + confidence
    /// (exactly what the set-as-basis merge writes). Kept separate from [`fact_ref`]
    /// so the common human-curated case stays terse while the scored case is loud
    /// about what it is testing.
    fn scored_ref(node: &str, role: &str, confidence: f32) -> ScenarioFactRefRecord {
        ScenarioFactRefRecord {
            scenario_id: Uuid::nil(),
            graph_node_id: node.to_string(),
            role_in_this_scenario: Some(role.to_string()),
            status: "undecided".to_string(),
            note: None,
            confidence: Some(confidence),
            tagged_at: chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
        }
    }

    #[test]
    fn miss_is_undecided_and_lands_in_pool() {
        // A live pool node with NO ref row is derived Undecided, in the working
        // pool, WITHOUT any persistence (this is a pure fn — there is nothing to
        // persist to, which is the point of derive-on-read).
        let response =
            reconcile_candidates(vec![content("ev-1")], Vec::new()).expect("no decode to fail");

        assert_eq!(response.pool.len(), 1);
        assert!(response.dropped.is_empty());
        assert_eq!(response.pool[0].content.evidence_id, "ev-1");
        assert_eq!(response.pool[0].status, FactStatus::Undecided);
        assert!(response.pool[0].role.is_none());
        assert!(response.pool[0].note.is_none());
        // A miss was never scored for this scenario → confidence is None ("unscored"),
        // NOT Some(0.0). The card must be able to tell the two apart (Standing Rule 1).
        assert!(
            response.pool[0].confidence.is_none(),
            "an undecided miss has no model confidence — None, never Some(0.0)"
        );
    }

    #[test]
    fn scored_undecided_ref_carries_role_and_confidence_into_the_pool() {
        // The set-as-basis merge writes undecided rows with a model role + confidence.
        // Both must survive reconcile onto the CandidateDto so the workbench can
        // render "corroborates · 85%" (the whole point of this chunk).
        let refs = vec![scored_ref("ev-1", "corroborates", 0.85)];
        let response = reconcile_candidates(vec![content("ev-1")], refs).expect("known token");

        assert_eq!(
            response.pool.len(),
            1,
            "an undecided scored pick stays in the pool"
        );
        assert_eq!(response.pool[0].status, FactStatus::Undecided);
        assert_eq!(response.pool[0].role.as_deref(), Some("corroborates"));
        assert_eq!(response.pool[0].confidence, Some(0.85));
    }

    #[test]
    fn human_curated_ref_has_no_confidence() {
        // A human include with a NULL confidence column must reconcile to None, not
        // 0.0 — a hand-curated fact carries no *model* score and reads "unscored".
        let refs = vec![fact_ref("ev-1", "included", Some("rebuts"), None)];
        let response = reconcile_candidates(vec![content("ev-1")], refs).expect("known token");

        assert_eq!(response.pool[0].status, FactStatus::Included);
        assert_eq!(response.pool[0].role.as_deref(), Some("rebuts"));
        assert!(
            response.pool[0].confidence.is_none(),
            "a human-curated ref (NULL confidence) must be None, never Some(0.0)"
        );
    }

    #[test]
    fn included_ref_lands_in_pool_with_role_and_note() {
        let refs = vec![fact_ref(
            "ev-1",
            "included",
            Some("rebuts"),
            Some("key denial"),
        )];
        let response = reconcile_candidates(vec![content("ev-1")], refs).expect("known token");

        assert_eq!(
            response.pool.len(),
            1,
            "included belongs in the working pool"
        );
        assert!(response.dropped.is_empty());
        assert_eq!(response.pool[0].status, FactStatus::Included);
        assert_eq!(response.pool[0].role.as_deref(), Some("rebuts"));
        assert_eq!(response.pool[0].note.as_deref(), Some("key denial"));
    }

    #[test]
    fn dropped_ref_lands_in_its_own_list() {
        let refs = vec![fact_ref("ev-1", "dropped", None, None)];
        let response = reconcile_candidates(vec![content("ev-1")], refs).expect("known token");

        assert!(
            response.pool.is_empty(),
            "a dropped candidate must NOT appear in the working pool"
        );
        assert_eq!(response.dropped.len(), 1, "dropped goes in its own list");
        assert_eq!(response.dropped[0].status, FactStatus::Dropped);
        assert_eq!(response.dropped[0].content.evidence_id, "ev-1");
    }

    #[test]
    fn undecided_and_included_share_the_pool_dropped_is_split_out() {
        // Three nodes, three fates: one undecided (no ref), one included, one
        // dropped. The pool holds the first two; dropped holds the third.
        let pool = vec![
            content("ev-undecided"),
            content("ev-included"),
            content("ev-dropped"),
        ];
        let refs = vec![
            fact_ref("ev-included", "included", None, None),
            fact_ref("ev-dropped", "dropped", None, None),
        ];
        let response = reconcile_candidates(pool, refs).expect("known tokens");

        assert_eq!(response.pool.len(), 2);
        assert_eq!(response.dropped.len(), 1);
        assert_eq!(response.dropped[0].content.evidence_id, "ev-dropped");
    }

    #[test]
    fn a_ref_with_no_matching_pool_node_is_simply_absent() {
        // The pool drives output: a ref pointing at a node NOT in the pool (e.g.
        // its Evidence was re-ingested under a new id) contributes no candidate.
        // It is not invented into the output, and — being neither dropped-in-pool
        // nor pool — it simply does not appear. (1a.3's un-drop tray, not gather,
        // is where such a ref would resurface.)
        let refs = vec![fact_ref("ev-orphan-ref", "included", None, None)];
        let response = reconcile_candidates(vec![content("ev-1")], refs).expect("known token");

        assert_eq!(response.pool.len(), 1);
        assert_eq!(response.pool[0].content.evidence_id, "ev-1");
        assert_eq!(response.pool[0].status, FactStatus::Undecided);
    }

    #[test]
    fn unknown_status_token_errs_loudly_not_bucketed() {
        // Standing Rule 1: a persisted status this build cannot interpret is a
        // data-integrity fault — a loud Err, NEVER silently bucketed as undecided.
        let refs = vec![fact_ref("ev-1", "archived", None, None)];
        let result = reconcile_candidates(vec![content("ev-1")], refs);

        assert!(
            matches!(result, Err(AppError::Internal { .. })),
            "an unrecognized status token must fail loudly, not default to undecided"
        );
    }

    #[test]
    fn fallback_definition_has_no_target() {
        // The gather fallback must have `target: None` so the shared resolver
        // falls through to the case default — the whole reason it exists.
        assert!(fallback_definition().target.is_none());
    }

    #[test]
    fn map_subject_error_unresolvable_is_503_naming_config_key() {
        // An unresolvable subject is a MISCONFIGURATION → 503, and the message
        // must name the env var that fixes it (distinct from a 200 empty pool).
        let err = map_subject_error(Uuid::nil(), SubjectResolveError::Unresolvable);
        match err {
            AppError::ServiceUnavailable { message } => assert!(
                message.contains("CASE_DEFAULT_SUBJECT_NAME"),
                "503 message must name the config key: {message}"
            ),
            other => panic!("expected 503 ServiceUnavailable, got {other:?}"),
        }
    }

    #[test]
    fn map_subject_error_lookup_failed_is_internal_500() {
        use serde::de::Error as _;
        // A graph fault while resolving the default subject is a server-side 500,
        // not a config problem. Construct the wrapped BiasRepositoryError via
        // serde's `custom` so no live Neo4j is needed (mirrors theme_scan's tests).
        let source = crate::bias::repository::BiasRepositoryError::Deserialize(
            neo4rs::DeError::custom("subjects query failed"),
        );
        let err = map_subject_error(
            Uuid::nil(),
            SubjectResolveError::DefaultLookupFailed { source },
        );
        assert!(
            matches!(err, AppError::Internal { .. }),
            "a graph-layer lookup fault must map to 500 Internal"
        );
    }
}
