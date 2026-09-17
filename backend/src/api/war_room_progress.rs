//! The War Room status card's reads — every scenario of the case, in one request.
//!
//! CC_TASK_WAR_ROOM_v1, ruling Q1 (Option A). Two kinds of read:
//!
//! 1. **Evidence counts** (facts included, candidates to rule, Matrix linked) —
//!    produced by the card queue's own assembly, `scenario_cards_core`, in its
//!    `CountsOnly` mode. Never re-derived.
//! 2. **The four Postgres families** (scan, deck, changed, and the viewer's new
//!    answers) — one statement each over every scenario id
//!    (`pipeline_repository::war_room_status`, `pipeline_repository::review_cursor`).
//!
//! The pure fold that turns both into cards is `services::war_room_progress`.
//!
//! ## Condition 1: the graph gather runs once per SUBJECT, not per scenario
//!
//! The expensive read is the graph gather around a scenario's subject. On PROD
//! the eleven scenarios have two subjects between them, so the gather runs twice
//! and each scenario assembles against a copy of its subject's pool.

use std::collections::HashMap;
use std::time::Instant;

use futures::future::try_join_all;
use uuid::Uuid;

use crate::{
    bias::dto::BiasInstance,
    dto::war_room_progress::ScenarioProgress,
    error::AppError,
    repositories::pipeline_repository::review_cursor::new_answers_for_viewer,
    repositories::pipeline_repository::war_room_status::{changed_counts, deck_counts, last_scans},
    services::war_room_progress::{evidence_counts, fold_progress, EvidenceCounts, FamilyRows},
    state::AppState,
};

use super::scenario_cards_core::{assemble_cards, read_candidate_pool, CardDetail};
use super::scenario_gather::resolve_gather_subject;

/// Every scenario's [`ScenarioProgress`], keyed by id.
///
/// `viewer` is the signed-in user's `username` — the same id `attribution`
/// stamps — or `None` for a request with no user, which makes the viewer's badge
/// `0` everywhere (decided in the query; see `review_cursor::new_answers_for_viewer`).
///
/// ## Rust Learning: `tokio::try_join!`
///
/// The evidence counts and the four family reads do not depend on one another,
/// so they run concurrently: `try_join!` polls all five futures together and
/// returns as soon as they have all finished — or as soon as ONE fails, with that
/// error. Awaiting them one after another would add their latencies; this pays
/// roughly the slowest.
///
/// # Errors
/// A logged 500 naming which read failed.
pub(crate) async fn read_progress(
    state: &AppState,
    scenario_ids: &[Uuid],
    viewer: Option<&str>,
) -> Result<HashMap<Uuid, ScenarioProgress>, AppError> {
    let started = Instant::now();
    let pool = &state.pipeline_pool;
    let (evidence, scans, deck, changed, viewer_new) = tokio::try_join!(
        read_evidence_counts(state, scenario_ids),
        family("last scan", last_scans(pool, scenario_ids)),
        family("deck counts", deck_counts(pool, scenario_ids)),
        family("changed counts", changed_counts(pool, scenario_ids)),
        family(
            "viewer new answers",
            new_answers_for_viewer(pool, scenario_ids, viewer)
        ),
    )?;

    let progress = fold_progress(
        scenario_ids,
        &evidence,
        FamilyRows {
            scans,
            deck,
            changed,
            viewer_new,
        },
    )
    .map_err(|e| {
        tracing::error!(error = %e, "the war room's status reads could not be folded into cards");
        AppError::Internal {
            message: format!("failed to assemble the scenario status cards: {e}"),
        }
    })?;

    tracing::info!(
        scenarios = scenario_ids.len(),
        viewer = viewer.unwrap_or("<none>"),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "read the war room's scenario status"
    );
    Ok(progress)
}

/// Await one family read and give its failure a name.
async fn family<T>(
    name: &'static str,
    read: impl std::future::Future<
        Output = Result<T, crate::repositories::pipeline_repository::PipelineRepoError>,
    >,
) -> Result<T, AppError> {
    read.await.map_err(|e| {
        tracing::error!(error = %e, family = name, "a war room status read failed");
        AppError::Internal {
            message: format!("failed to read the scenarios' {name}"),
        }
    })
}

/// The three evidence numbers for every scenario, from the card assembly.
///
/// A scenario that names no target has no pool — its counts are zero, and the
/// card's Matrix row renders an em dash (`stuck == 0`), exactly as the scenario
/// page renders no progress line for it.
async fn read_evidence_counts(
    state: &AppState,
    scenario_ids: &[Uuid],
) -> Result<HashMap<Uuid, EvidenceCounts>, AppError> {
    let started = Instant::now();
    let mut subjects: Vec<(Uuid, Option<String>)> = Vec::with_capacity(scenario_ids.len());
    for &id in scenario_ids {
        subjects.push((id, resolve_gather_subject(state, id).await?));
    }

    let pools = gather_once_per_subject(state, &subjects).await?;
    let settings = state.settings.current();

    // ## Rust Learning: `try_join_all` over a Vec of futures
    //
    // Unlike `try_join!`, which takes a fixed list written in the source, this
    // takes however many futures the loop built — one per scenario with a
    // subject. The database pool's own connection limit is what bounds how many
    // actually hit Postgres at once, so no second limit is invented here.
    let assembled = try_join_all(subjects.iter().filter_map(|(id, subject)| {
        let subject = subject.as_ref()?;
        let pool = pools.get(subject)?.clone();
        let settings = &settings;
        Some(async move {
            let core =
                assemble_cards(state, *id, subject, pool, settings, CardDetail::CountsOnly).await?;
            Ok::<_, AppError>((*id, evidence_counts(&core.response)))
        })
    }))
    .await?;

    let mut counts: HashMap<Uuid, EvidenceCounts> = assembled.into_iter().collect();
    for (id, subject) in &subjects {
        if subject.is_none() {
            counts.insert(*id, EvidenceCounts::default());
        }
    }
    tracing::info!(
        scenarios = scenario_ids.len(),
        distinct_subjects = pools.len(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "counted the war room's evidence through the card assembly"
    );
    Ok(counts)
}

/// Read each distinct subject's graph pool exactly once (ruling Q1, condition 1).
async fn gather_once_per_subject(
    state: &AppState,
    subjects: &[(Uuid, Option<String>)],
) -> Result<HashMap<String, Vec<BiasInstance>>, AppError> {
    let mut distinct: Vec<&String> = subjects.iter().filter_map(|(_, s)| s.as_ref()).collect();
    distinct.sort();
    distinct.dedup();
    let pools = try_join_all(distinct.into_iter().map(|subject| async move {
        let pool = read_candidate_pool(state, subject).await?;
        Ok::<_, AppError>((subject.clone(), pool))
    }))
    .await?;
    Ok(pools.into_iter().collect())
}
