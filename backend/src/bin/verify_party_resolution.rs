//! `verify_party_resolution` — replay every party mention the corpus would
//! ingest, and compare today's resolution against the alias-aware one.
//!
//! **READ-ONLY. No `--apply`, no write path.** It issues two `MATCH` reads
//! against Neo4j and one `SELECT` against Postgres, and writes one report file.
//!
//! ## Why a replay and not a test
//!
//! The question this answers is not "does the matcher work" — unit tests cover
//! that — but "over the 138 real mentions in this corpus, does anything move to
//! a node a human would call wrong?". Only the live data can answer it, and only
//! through the REAL [`resolve_parties`], not a re-implementation of it: the
//! Evidence id arm one branch ago shipped inert for eleven days because the only
//! thing exercising it was a copy living in a fixture.
//!
//! ```bash
//! cd ~/Projects/colossus-legal/backend
//! (set -a; . ./.env.dev-remote; set +a; cargo run --bin verify_party_resolution)
//! ```
//!
//! ## Reading the summary
//!
//! `Now NEW (were attached)` is the regression counter and it must read **0**.
//! `Moved to a DIFFERENT existing node` must also read 0 — a mention silently
//! changing which person it belongs to is the failure this whole design is built
//! to prevent, and it is worse than any number of duplicates.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use colossus_legal_backend::api::pipeline::party_alias::PartyAliasIndex;
use colossus_legal_backend::api::pipeline::{
    fetch_existing_parties, resolve_parties, resolve_parties_baseline,
};
use colossus_legal_backend::models::document_status::PARTY_SUBTYPES;
use colossus_legal_backend::oneshot::cli::{
    connect_graph, connect_pool, emit_report, init_tracing, pipeline_database_url,
};
use colossus_legal_backend::oneshot::exit::{
    help_text, EXIT_BAD_INPUT, EXIT_CONNECTION, EXIT_OK, EXIT_UNIT_ABORTED,
};
use colossus_legal_backend::partyresolve::replay::{classify, count_regressions, render, Replay};
use colossus_legal_backend::repositories::pipeline_repository::ExtractionItemRecord;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(
    name = "verify_party_resolution",
    about = "Replay every party mention against alias-aware resolution. Read-only.",
    after_help = help_text()
)]
struct Args {
    /// Where to write the per-mention list.
    #[arg(long, default_value = "party_resolution_report.txt")]
    out: PathBuf,

    /// Pretend these node ids have already been merged away, and resolve as if
    /// the graph no longer contained them. Repeatable. READ-ONLY — it filters
    /// the in-memory party list and touches nothing.
    ///
    /// This exists to answer one operational question before an operator runs a
    /// merge: "if I collapse these duplicates, will the next ingest re-create
    /// them?" Answering it by actually merging and then looking is a write; this
    /// answers it by arithmetic.
    #[arg(long = "pretend-merged")]
    pretend_merged: Vec<String>,

    /// Postgres URL. Falls back to `PIPELINE_DATABASE_URL`.
    #[arg(long)]
    database_url: Option<String>,
}

#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();
    let _ = dotenvy::dotenv();
    match execute(Args::parse()).await {
        Ok(code) => code,
        Err(code) => code,
    }
}

async fn execute(args: Args) -> Result<ExitCode, ExitCode> {
    let database_url = pipeline_database_url(args.database_url.as_deref())?;
    let pool = connect_pool(&database_url).await?;
    let graph = connect_graph().await?;

    let existing = fetch_existing_parties(&graph).await.map_err(|e| {
        error!(error = ?e, "could not read the existing parties from Neo4j");
        ExitCode::from(EXIT_CONNECTION)
    })?;
    // Applied before anything else reads the list, so the alias index, the
    // baseline and the fixed resolution all see the same hypothetical graph.
    let existing: Vec<colossus_extract::KnownEntity> = existing
        .into_iter()
        .filter(|k| !args.pretend_merged.contains(&k.id))
        .collect();
    if !args.pretend_merged.is_empty() {
        info!(
            pretend_merged = ?args.pretend_merged,
            parties_remaining = existing.len(),
            "resolving against a HYPOTHETICAL graph — nothing was written"
        );
    }

    let items = load_party_items(&pool).await?;
    info!(
        parties_in_graph = existing.len(),
        mentions = items.len(),
        "loaded — nothing was written"
    );

    // The index is rebuilt here only to CLASSIFY each outcome (`via …`). The
    // resolution itself comes from `resolve_parties`, the real function.
    let index = PartyAliasIndex::from_known_entities(&existing);
    let existing_ids: Vec<String> = existing.iter().map(|k| k.id.clone()).collect();

    // Group by document so the report reads in corpus order, and so a per-document
    // resolution is replayed the way ingest actually performs it: one document at
    // a time, against the graph as it stands.
    let mut by_document: BTreeMap<String, Vec<ExtractionItemRecord>> = BTreeMap::new();
    for item in items {
        by_document
            .entry(item.document_id.clone())
            .or_default()
            .push(item);
    }

    let mut replays: Vec<Replay> = Vec::new();
    for (document, doc_items) in &by_document {
        let (map, _) = resolve_parties(doc_items, &existing).await.map_err(|e| {
            error!(error = ?e, document, "resolution failed");
            ExitCode::from(EXIT_UNIT_ABORTED)
        })?;
        let (baseline, _) = resolve_parties_baseline(doc_items, &existing)
            .await
            .map_err(|e| {
                error!(error = ?e, document, "baseline resolution failed");
                ExitCode::from(EXIT_UNIT_ABORTED)
            })?;

        for item in doc_items {
            let props = &item.item_data["properties"];
            let surface = props["party_name"]
                .as_str()
                .or_else(|| props["full_name"].as_str())
                .unwrap_or("unknown")
                .to_string();
            let Some(resolved) = map.get(&surface) else {
                continue;
            };
            let with_fix = resolved.neo4j_id.clone();

            let stored = item
                .neo4j_node_id
                .clone()
                .unwrap_or_else(|| "NEW".to_string());
            let today = baseline
                .get(&surface)
                .map(|r| r.neo4j_id.clone())
                .unwrap_or_else(|| "NEW".to_string());

            let entity_type = if with_fix.starts_with("org-") {
                "Organization"
            } else {
                "Person"
            };
            let via = classify(
                &index,
                entity_type,
                &surface,
                &today,
                &with_fix,
                &existing_ids,
            );

            replays.push(Replay {
                document: document.clone(),
                surface,
                stored,
                today,
                with_fix,
                via,
            });
        }
    }

    let rendered = render(&replays, &index, &existing_ids);
    emit_report(&rendered, &args.out).map_err(|_| ExitCode::from(EXIT_BAD_INPUT))?;

    let regressions = count_regressions(&replays, &existing_ids);
    Ok(ExitCode::from(if regressions == 0 {
        EXIT_OK
    } else {
        EXIT_UNIT_ABORTED
    }))
}

/// Read every party mention in the corpus.
async fn load_party_items(pool: &sqlx::PgPool) -> Result<Vec<ExtractionItemRecord>, ExitCode> {
    const SQL: &str = "SELECT id, run_id, document_id, entity_type, item_data, verbatim_quote, \
                grounding_status, grounded_page, review_status, reviewed_by, reviewed_at, \
                review_notes, graph_status, neo4j_node_id, resolved_entity_type \
         FROM extraction_items WHERE entity_type = ANY($1) ORDER BY document_id, id";

    let subtypes: Vec<String> = PARTY_SUBTYPES.iter().map(|s| s.to_string()).collect();
    sqlx::query_as::<_, ExtractionItemRecord>(SQL)
        .bind(&subtypes)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            error!(error = %e, "could not read extraction_items");
            ExitCode::from(EXIT_CONNECTION)
        })
}
