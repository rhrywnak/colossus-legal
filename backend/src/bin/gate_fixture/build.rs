//! The reads, and one fixture assembled from them. **Every one is a `SELECT` or
//! a `MATCH`; nothing here writes.**
//!
//! ## Why the pool comes from the repository and not from a query written here
//!
//! The fixture's whole claim is "this is what the scan saw". The scan saw
//! whatever `api::scenario_gather` gathered, and that is
//! [`BiasRepository::all_evidence_about_subject`] — so this calls exactly that
//! function rather than a look-alike query that could drift from it. The two
//! things the repositories genuinely do not offer (an Evidence node's
//! `significance`, and an allegation's text by id) live in
//! `repositories::gate_fixture_repository` beside the rest of the graph layer.
//!
//! ## The one thing this module must never do
//!
//! It must never narrow a read to make a count come out right. A count that
//! differs from the note in the task is reported (see `services::gate_fixture`),
//! never fixed — a tuned query that produces the expected number is a fabricated
//! fixture and destroys the point of the gate.

use std::collections::HashMap;
use std::process::ExitCode;

use chrono::Utc;
use colossus_legal_backend::bias::dto::BiasInstance;
use colossus_legal_backend::bias::repository::BiasRepository;
use colossus_legal_backend::domain::fact_status::FactStatus;
use colossus_legal_backend::domain::scenario_code::{
    allegation_code, candidate_code, scenario_code,
};
use colossus_legal_backend::dto::scenario_crud::ScenarioDefinition;
use colossus_legal_backend::oneshot::exit::{EXIT_BAD_INPUT, EXIT_CONNECTION};
use colossus_legal_backend::repositories::gate_fixture_repository::{
    allegations_by_ids, significance_by_ids, AllegationTextRow,
};
use colossus_legal_backend::repositories::pipeline_repository::{
    list_candidate_ordinals, list_fact_refs_for_scenario, list_relevant_verdicts_for_run,
    list_scan_runs, list_scenarios_for_case, ScenarioRecord,
};
use colossus_legal_backend::services::gate_fixture::{
    AllegationRef, CandidateCard, GateFixture, GatherQuery,
};
use colossus_legal_backend::services::scenario_augmentation::talking_points;
use neo4rs::Graph;
use sqlx::PgPool;
use tracing::{error, info, warn};

use crate::plan::Plan;
use crate::Args;

/// Read one scenario's frozen scan out of DEV.
pub(crate) async fn build_fixture(
    pool: &PgPool,
    graph: &Graph,
    args: &Args,
    plan: &Plan,
    code: &str,
) -> Result<GateFixture, ExitCode> {
    let scenario = find_scenario(pool, &args.case_slug, code).await?;
    let subject = subject_of(&scenario, code)?;
    let (run_id, started_at) = pick_run(pool, &scenario, plan.run_date(code)?, code).await?;

    let query = build_query(pool, graph, &scenario, &subject).await?;
    let ordinals = read(
        list_candidate_ordinals(pool, scenario.scenario_id).await,
        "ordinals",
    )?;
    let significance_of = |instances: &[BiasInstance]| -> Vec<String> {
        instances.iter().map(|i| i.evidence_id.clone()).collect()
    };

    let pool_rows = read_pool(graph, &subject).await?;
    let outside_rows = read(
        BiasRepository::new(graph.clone())
            .evidence_by_ids(plan.outside_ids(code))
            .await
            .map_err(|e| format!("{e:?}")),
        "outside-pool evidence",
    )?;
    warn_missing_outside(plan.outside_ids(code), &outside_rows, code);

    let mut ids = significance_of(&pool_rows);
    ids.extend(significance_of(&outside_rows));
    let significance = read(
        significance_by_ids(graph, &ids)
            .await
            .map_err(|e| e.to_string()),
        "significance",
    )?;

    let card = |i: &BiasInstance| to_card(i, &ordinals, &significance, &args.pinpoint_template);

    Ok(GateFixture {
        scenario: code.to_string(),
        scenario_id: scenario.scenario_id.to_string(),
        run_id: run_id.to_string(),
        run_started_at: started_at,
        extracted_at: Utc::now().format("%Y-%m-%d").to_string(),
        query,
        candidates: pool_rows.iter().map(card).collect(),
        opus_relevant_ids: read_relevant(pool, run_id).await?,
        included_ids: read_included(pool, scenario.scenario_id).await?,
        outside_pool: outside_rows.iter().map(card).collect(),
    })
}

/// The scenario carrying this code, inside this case.
async fn find_scenario(
    pool: &PgPool,
    case_slug: &str,
    code: &str,
) -> Result<ScenarioRecord, ExitCode> {
    let all = read(list_scenarios_for_case(pool, case_slug).await, "scenarios")?;
    all.into_iter()
        .find(|s| {
            colossus_legal_backend::domain::scenario_code::scenario_code(s.code_ordinal) == code
        })
        .ok_or_else(|| {
            error!(scenario = %code, case = %case_slug, "no scenario with this code in this case");
            ExitCode::from(EXIT_BAD_INPUT)
        })
}

/// The scenario's subject — the id its pool is gathered ABOUT.
///
/// Domain note: this is the defect the whole cascade exists to fix. The subject
/// is the ONLY thing gather consults, which is why S-9 and S-11 — two different
/// scenarios that happen to name the same person — see one identical pool.
fn subject_of(scenario: &ScenarioRecord, code: &str) -> Result<String, ExitCode> {
    let definition = ScenarioDefinition::from_value(scenario.definition.clone()).map_err(|e| {
        error!(scenario = %code, error = %e, "the scenario's definition does not parse");
        ExitCode::from(EXIT_BAD_INPUT)
    })?;
    colossus_legal_backend::services::scenario_subject::resolve_scenario_subject(&definition)
        .map_err(|e| {
            error!(scenario = %code, error = %e, "the scenario names no target, so it has no pool");
            ExitCode::from(EXIT_BAD_INPUT)
        })
}

/// The completed run that started on the named date, and only that one.
async fn pick_run(
    pool: &PgPool,
    scenario: &ScenarioRecord,
    date: &str,
    code: &str,
) -> Result<(uuid::Uuid, String), ExitCode> {
    let runs = read(
        list_scan_runs(pool, scenario.scenario_id).await,
        "scan runs",
    )?;
    let matching: Vec<_> = runs
        .iter()
        .filter(|r| r.started_at.format("%Y-%m-%d").to_string() == date)
        .collect();

    match matching.as_slice() {
        [run] => {
            info!(
                scenario = %code, run_id = %run.run_id, status = %run.status,
                candidates_read = run.candidates_read, candidates_judged = run.candidates_judged,
                relevant = run.relevant_count, "froze this run"
            );
            Ok((run.run_id, run.started_at.to_rfc3339()))
        }
        // Zero and many are DIFFERENT failures and say so: "no run that day" is
        // a wrong date, "two runs that day" is an ambiguity only a human can cut.
        [] => {
            error!(scenario = %code, date = %date, runs = runs.len(), "no run started on that date");
            Err(ExitCode::from(EXIT_BAD_INPUT))
        }
        many => {
            error!(scenario = %code, date = %date, found = many.len(), "more than one run that day");
            Err(ExitCode::from(EXIT_BAD_INPUT))
        }
    }
}

/// The Stage-0 query: theme + linked allegation text + talking points.
async fn build_query(
    pool: &PgPool,
    graph: &Graph,
    scenario: &ScenarioRecord,
    subject: &str,
) -> Result<GatherQuery, ExitCode> {
    let anchors = scenario.anchor_allegation_ids.clone().unwrap_or_default();
    let rows = read(
        allegations_by_ids(graph, &anchors)
            .await
            .map_err(|e| e.to_string()),
        "allegations",
    )?;
    warn_missing_allegations(
        &anchors,
        &rows,
        scenario_code(scenario.code_ordinal).as_str(),
    );
    let points = read(
        talking_points(pool, scenario.scenario_id)
            .await
            .map_err(|e| e.to_string()),
        "talking points",
    )?;

    Ok(GatherQuery {
        theme: scenario.theme_statement.clone(),
        allegations: rows
            .iter()
            .map(|r| AllegationRef {
                id: match r.paragraph.as_deref() {
                    Some(p) if !p.trim().is_empty() => allegation_code(p),
                    _ => r.allegation_id.clone(),
                },
                text: r.text.clone().unwrap_or_default(),
            })
            .collect(),
        talking_points: points.into_iter().map(|p| p.text).collect(),
        subject: subject.to_string(),
    })
}

/// The candidate pool, through the SAME read `scenario_gather` uses.
async fn read_pool(graph: &Graph, subject: &str) -> Result<Vec<BiasInstance>, ExitCode> {
    read(
        BiasRepository::new(graph.clone())
            .all_evidence_about_subject(subject)
            .await
            .map_err(|e| format!("{e:?}")),
        "candidate pool",
    )
}

/// The ids Opus called relevant on that run.
async fn read_relevant(pool: &PgPool, run_id: uuid::Uuid) -> Result<Vec<String>, ExitCode> {
    let mut ids: Vec<String> = read(
        list_relevant_verdicts_for_run(pool, run_id).await,
        "verdicts",
    )?
    .into_iter()
    .map(|v| v.graph_node_id)
    .collect();
    // Sorted so two runs of this tool produce byte-identical files.
    ids.sort();
    ids.dedup();
    Ok(ids)
}

/// The ids Roman ruled Included.
async fn read_included(pool: &PgPool, scenario_id: uuid::Uuid) -> Result<Vec<String>, ExitCode> {
    let mut ids: Vec<String> = read(
        list_fact_refs_for_scenario(pool, scenario_id).await,
        "fact refs",
    )?
    .into_iter()
    .filter(|r| r.status == FactStatus::Included.code())
    .map(|r| r.graph_node_id)
    .collect();
    ids.sort();
    Ok(ids)
}

/// One graph instance, as a fixture card.
fn to_card(
    instance: &BiasInstance,
    ordinals: &HashMap<String, i32>,
    significance: &HashMap<String, String>,
    pinpoint_template: &str,
) -> CandidateCard {
    CandidateCard {
        id: instance.evidence_id.clone(),
        c_number: ordinals
            .get(&instance.evidence_id)
            .map(|n| candidate_code(*n)),
        title: instance.title.clone(),
        document: instance.document.as_ref().map(|d| d.title.clone()),
        page: instance.page_number,
        pinpoint: instance
            .page_number
            .map(|p| pinpoint_template.replace("{page}", &p.to_string())),
        quote: instance.verbatim_quote.clone(),
        significance: significance.get(&instance.evidence_id).cloned(),
        about: instance.about.iter().map(|a| a.name.clone()).collect(),
    }
}

/// An anchor allegation id that resolved to no node is named, one warning each.
///
/// ## Why a count is not enough
///
/// This is the stale-pointer defect, and it is one of the things the fixture
/// exists to make visible. "9 anchors, 7 resolved" tells an operator a gap
/// exists and then makes them diff two lists by hand to find it; naming the two
/// dead ids tells them which anchors to re-link. Same discipline as
/// [`warn_missing_outside`] below, which is what this used to differ from.
fn warn_missing_allegations(asked: &[String], got: &[AllegationTextRow], code: &str) {
    for id in asked {
        if !got.iter().any(|r| &r.allegation_id == id) {
            warn!(
                scenario = %code, allegation_id = %id,
                "anchor allegation id resolved to no graph node — it is absent from the fixture's query"
            );
        }
    }
}

/// An outside id that resolved to nothing is a named warning, never a silent gap.
fn warn_missing_outside(asked: &[String], got: &[BiasInstance], code: &str) {
    for id in asked {
        if !got.iter().any(|i| &i.evidence_id == id) {
            warn!(scenario = %code, evidence_id = %id, "outside-pool id matched no Evidence node");
        }
    }
}

/// Turn a repository error into a logged exit code, naming what was being read.
fn read<T, E: std::fmt::Display>(result: Result<T, E>, what: &str) -> Result<T, ExitCode> {
    result.map_err(|e| {
        error!(error = %e, reading = %what, "a read failed — nothing was written");
        ExitCode::from(EXIT_CONNECTION)
    })
}
