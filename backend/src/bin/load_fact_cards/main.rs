//! `load_fact_cards` — put Job B's and Job D's drafts into the scenario fact
//! cards (FACT_CARD_v2 §1).
//!
//! **A one-off data load, not a pipeline step.** Zero cost: no LLM, no embedding,
//! no paid API. It reads audit files off disk and writes rows a human then edits.
//!
//! Three modes, one per input shape the instruction names:
//!
//! | flag | file | what it writes |
//! |---|---|---|
//! | *(default)* | `B_S*.jsonl`, `D_B_S-*.jsonl` | the five card fields |
//! | `--talking-points` | `D_talking_points.jsonl` | `response_items` |
//! | `--candidates` | `D_S-*_candidates.jsonl` | included facts, in pick order |
//!
//! ## Usage
//!
//! ```text
//! NEO4J_URI=bolt://HOST:7687 NEO4J_USER=neo4j NEO4J_PASSWORD=… \
//! PIPELINE_DATABASE_URL=postgres://…/colossus_legal_v2 \
//! cargo run --bin load_fact_cards -- \
//!   --input ~/Documents/colossus-legal/AUDITS/LINKING_RUN_v1/B_S11.jsonl \
//!   --scenario S-11 --expect-count 10 [--apply]
//! ```
//!
//! Without `--apply` it is a DRY RUN: it reads everything, resolves everything,
//! prints exactly what would be written, and stops before the first write. The
//! plan the dry run prints is the same value the apply consumes — see `plan.rs`
//! for why that matters.
//!
//! ## Why the graph is read at all
//!
//! Job B names accusations by their SHORT id — the graph id's hash suffix. The
//! card stores the FULL node id, like everything else in this system, so the
//! loader resolves them. An id it cannot resolve is NAMED in the output and its
//! card is loaded without it (see `plan_cards`).

mod model;
mod plan;
mod write;

use anyhow::{bail, Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;

use colossus_legal_backend::oneshot::cli::{
    connect_graph, connect_pool, init_tracing, pipeline_database_url,
};
use colossus_legal_backend::oneshot::exit::{EXIT_EXECUTION_FAILED, EXIT_OK, EXIT_UNSAFE_PLAN};

use model::{CandidatesFile, DraftedCard, TalkingPointsFile};

/// How many words a MACHINE-drafted title may run to (§1).
//
// STRUCTURAL: the limit Job B was given, applied to Job B's output. Not a
// deployment value — a human editing a title past it is making an editorial
// choice on their own surface, and the database deliberately does not argue.
const TITLE_WORD_LIMIT: usize = 14;

#[derive(Parser, Debug)]
#[command(
    about = "Load Job B / Job D drafts into the scenario fact cards",
    long_about = None
)]
struct Args {
    /// The JSONL file to read.
    #[arg(long)]
    input: PathBuf,
    /// The scenario these rows belong to, as its code — `S-11`.
    ///
    /// Required for the card mode, whose files carry no scenario id of their own.
    /// The other two modes read the id from the file and REFUSE if this disagrees
    /// with it.
    #[arg(long)]
    scenario: Option<String>,
    /// How many records the file must hold. A file that has grown or shrunk since
    /// the runbook step was written is a file nobody has re-read.
    #[arg(long)]
    expect_count: usize,
    /// Load `D_talking_points.jsonl` into `response_items` instead.
    #[arg(long)]
    talking_points: bool,
    /// Load `D_S-*_candidates.jsonl`: the picks become included facts in display
    /// order.
    #[arg(long)]
    candidates: bool,
    /// Write. Without it, everything is read, resolved and printed, and nothing
    /// is stored.
    #[arg(long)]
    apply: bool,
    /// Override `PIPELINE_DATABASE_URL`.
    #[arg(long)]
    database_url: Option<String>,
}

#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();
    let args = Args::parse();

    if args.talking_points && args.candidates {
        tracing::error!("--talking-points and --candidates load different files; pass one");
        return ExitCode::from(EXIT_UNSAFE_PLAN);
    }

    match run(&args).await {
        Ok(()) => ExitCode::from(EXIT_OK),
        Err(e) => {
            // `{e:#}` walks anyhow's context chain, so the file and the line
            // number a parse failed on reach the operator rather than only the
            // outermost sentence.
            tracing::error!(error = format!("{e:#}"), "load_fact_cards failed");
            ExitCode::from(EXIT_EXECUTION_FAILED)
        }
    }
}

/// Read, plan, print, and — only with `--apply` — write.
async fn run(args: &Args) -> Result<()> {
    let url = pipeline_database_url(args.database_url.as_deref())
        .map_err(|_| anyhow::anyhow!("no pipeline database url"))?;
    let pool = connect_pool(&url)
        .await
        .map_err(|_| anyhow::anyhow!("could not connect to the pipeline database"))?;

    if args.talking_points {
        // The talking-point path goes through the augmentation SERVICE, which
        // checks the stored cap — so the loader needs the settings snapshot the
        // service reads that cap from. The other two modes do not.
        let settings = write::load_settings(&pool).await?;
        return load_talking_points(args, &pool, &settings).await;
    }
    if args.candidates {
        return load_candidates(args, &pool).await;
    }
    load_cards(args, &pool).await
}

/// The default mode: Job B's five sentences per card.
async fn load_cards(args: &Args, pool: &sqlx::PgPool) -> Result<()> {
    let Some(code) = args.scenario.as_deref() else {
        bail!("--scenario is required when loading cards: the B files carry no scenario id");
    };
    let cards: Vec<DraftedCard> = model::read_jsonl(&args.input)?;
    expect(cards.len(), args.expect_count, "cards")?;

    let graph = connect_graph()
        .await
        .map_err(|_| anyhow::anyhow!("could not connect to Neo4j"))?;
    let allegations = write::short_id_index(&graph)
        .await
        .context("reading the accusation ids Job B abbreviates")?;

    let plan = plan::plan_cards(&cards, &allegations, TITLE_WORD_LIMIT)?;
    let scenario_id = write::scenario_id_for_code(pool, code).await?;

    println!(
        "{code} ({scenario_id}) — {} cards, {} fields to write",
        plan.cards,
        plan.fields.len()
    );
    for field in &plan.fields {
        println!(
            "  {} {:<15} {}",
            short(&field.graph_node_id),
            field.field.code(),
            preview(field.value.as_deref())
        );
    }
    report_unresolved(&plan.unresolved_accusations);

    if !args.apply {
        println!("DRY RUN — nothing written. Re-run with --apply.");
        return Ok(());
    }
    let written = write::apply_cards(pool, scenario_id, &plan).await?;
    println!(
        "APPLIED — {written} fields written as {}",
        plan::LOADER_AUTHOR
    );
    Ok(())
}

/// `--talking-points`: `D_talking_points.jsonl` into `response_items`.
async fn load_talking_points(
    args: &Args,
    pool: &sqlx::PgPool,
    settings: &colossus_legal_backend::domain::settings::Settings,
) -> Result<()> {
    let files: Vec<TalkingPointsFile> = model::read_jsonl(&args.input)?;
    expect(files.len(), args.expect_count, "scenarios")?;
    let planned = plan::plan_points(&files)?;

    for scenario in &planned {
        println!(
            "{} ({}) — {} talking points",
            scenario.scenario_code,
            scenario.scenario_id,
            scenario.items.len()
        );
        for (index, text) in &scenario.items {
            println!("  {}. {}", index + 1, preview(Some(text)));
        }
    }

    if !args.apply {
        println!("DRY RUN — nothing written. Re-run with --apply.");
        return Ok(());
    }
    let written = write::apply_points(pool, &planned, settings).await?;
    println!(
        "APPLIED — {written} talking points written as {}",
        plan::LOADER_AUTHOR
    );
    Ok(())
}

/// `--candidates`: the ten picks become included facts, in display order.
async fn load_candidates(args: &Args, pool: &sqlx::PgPool) -> Result<()> {
    let files: Vec<CandidatesFile> = model::read_jsonl(&args.input)?;
    expect(files.len(), args.expect_count, "scenarios")?;
    let planned = plan::plan_picks(&files)?;

    for scenario in &planned {
        println!(
            "{} ({}) — {} picks",
            scenario.scenario_code,
            scenario.scenario_id,
            scenario.picks.len()
        );
        for pick in &scenario.picks {
            println!(
                "  {:>6}  {}  {}",
                pick.sort_ordinal,
                short(&pick.graph_node_id),
                preview(Some(&pick.title))
            );
        }
        if scenario.stored_reasons > 0 {
            // Said before the apply says it: integration ruling R2 gave the
            // ranker's reasons a home on the include EVENT, not on the card.
            // Naming the destination in the dry run is what lets a reader object
            // to it before anything is written. See `plan::PlannedPick`.
            println!(
                "  {} pick reasons will be stored as the include event's note \
                 (scenario_fact_card_events), not as a card field",
                scenario.stored_reasons
            );
        }
    }

    if !args.apply {
        println!("DRY RUN — nothing written. Re-run with --apply.");
        return Ok(());
    }
    let written = write::apply_picks(pool, &planned).await?;
    println!("APPLIED — {written} facts included with their titles");
    Ok(())
}

/// The count guard.
///
/// ## Why a count and not a checksum
///
/// The instruction's own numbers (S-5 9, S-6 8, S-7 10 …) are what a runbook step
/// carries, and the failure this catches is a file regenerated since that step was
/// written. A checksum would also catch an edited SENTENCE, which is a change
/// nobody needs to be stopped for.
fn expect(found: usize, expected: usize, unit: &str) -> Result<()> {
    if found != expected {
        bail!("expected {expected} {unit} and the file holds {found}");
    }
    Ok(())
}

/// A graph id's hash suffix, for a readable plan.
fn short(id: &str) -> &str {
    id.rsplit(':').next().unwrap_or(id)
}

/// A value, cut to one line so a 59-card plan stays readable.
fn preview(value: Option<&str>) -> String {
    let Some(value) = value else {
        return "(cleared)".to_string();
    };
    let flat: String = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= 72 {
        return flat;
    }
    let head: String = flat.chars().take(69).collect();
    format!("{head}...")
}

/// Print the accusations nothing in the graph matched.
///
/// A separate function so the message is one sentence in one place: this is the
/// stale-pointer class, and the loader is the cheapest place in the system to see
/// it.
fn report_unresolved(unresolved: &[String]) {
    if unresolved.is_empty() {
        return;
    }
    println!(
        "  {} accusation(s) named by an id nothing in the graph matches — those \
         cards load WITHOUT them:",
        unresolved.len()
    );
    for entry in unresolved {
        println!("    {entry}");
    }
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
