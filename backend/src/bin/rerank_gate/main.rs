//! `rerank_gate` — does `BAAI/bge-reranker-v2-m3` rank this corpus well enough
//! to be Stage 2 of the gather cascade?
//!
//! Reads frozen scan fixtures, composes each scenario's query by L2a's rule,
//! scores every candidate on three candidate-text surfaces against a local vLLM
//! reranker, and prints two numbers per surface. Nothing is written to any
//! database; the only outputs are stdout and one CSV per fixture.
//!
//! ## The two gates (design §0.4)
//!
//! - **Gate A** — the reranker's top `--gate-a-k` contains at least
//!   `--gate-a-min` of the cards Opus called relevant.
//! - **Gate B** — the top `--gate-b-k` contains EVERY card Roman Included.
//!
//! Both are read on surface **S2 probe**. S1 and S3 are scored in the same run
//! and printed beside it for information.
//!
//! ## Usage
//!
//! ```text
//! cargo run --bin rerank_gate -- \
//!   --base-url http://HOST:PORT --model BAAI/bge-reranker-v2-m3 \
//!   --fixture GATE/s11_gate_fixture_v1.json --expect 292,44,10,7 \
//!   --fixture GATE/s9_gate_fixture_v1.json  --expect 292,8,2,1 \
//!   --out-dir GATE/
//! ```

mod client;
mod csv_out;
mod fixture;
mod pure;
mod report;

use anyhow::{bail, Context};
use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use client::RerankClient;
use fixture::ExpectedCounts;
use pure::{compose_query, rank_desc, surface_text, would_be_rank, Surface};
use report::{GateBars, SurfaceRun};

/// ## Rust Learning: `clap`'s derive API
///
/// Each field becomes a flag; the doc comment above it becomes that flag's help
/// text, which is why they read as sentences. `Vec<T>` makes a flag repeatable,
/// and a field with no `default_value` and no `Option` is REQUIRED — which is
/// how `--base-url` is given no compiled-in default, as standing Rule 2 demands
/// of anything that varies by environment.
///
/// Every setting is a FLAG, with no environment-variable fallback. clap's
/// `env = "..."` attribute needs its `env` feature, which this workspace does
/// not enable, and turning it on to save four exports would be a dependency
/// change made for a one-off gate. The instruction left the choice open; this is
/// the one that touches no manifest.
#[derive(Parser, Debug)]
#[command(about = "Rerank gate for the gather cascade (design §0.4)")]
struct Args {
    /// Path to a gate fixture JSON. Repeatable; paired by position with --expect.
    #[arg(long = "fixture", required = true)]
    fixtures: Vec<PathBuf>,

    /// Expected counts for the fixture at the same position:
    /// `candidates,relevant,included,outside_pool`. This is STOP condition 3.
    #[arg(long = "expect", required = true)]
    expect: Vec<ExpectedCounts>,

    /// Reranker base URL, e.g. `http://HOST:PORT`. No default — the endpoint is
    /// deployment state, not a code constant.
    #[arg(long)]
    base_url: String,

    /// The model id, which must match what `/v1/models` lists exactly (STOP 2).
    #[arg(long)]
    model: String,

    /// Texts per `/score` call. 64 is the hand-off's stated cap (§3); batches are
    /// sent sequentially, never concurrently.
    #[arg(long, default_value_t = 64)]
    batch: usize,

    /// Per-request HTTP timeout.
    #[arg(long, default_value_t = 120)]
    timeout_secs: u64,

    /// Directory the CSVs are written to. Never inside the repo (law 6).
    #[arg(long)]
    out_dir: PathBuf,

    /// Gate A's cut-off — the top N examined.
    #[arg(long, default_value_t = 60)]
    gate_a_k: usize,

    /// Gate A's floor — at least this many relevant cards inside the cut-off.
    #[arg(long, default_value_t = 40)]
    gate_a_min: usize,

    /// Gate B's cut-off — the top N that must contain every Included card.
    #[arg(long, default_value_t = 20)]
    gate_b_k: usize,
}

/// ## Rust Learning: `ExitCode` from `main`
///
/// Returning `anyhow::Result` from `main` would print the error with `Debug`
/// formatting, which buries the context chain in escaped quotes. Printing it
/// with `{:#}` and returning an explicit `ExitCode` keeps a STOP message
/// readable — this bin's failure output is meant to be pasted into a report.
#[tokio::main]
async fn main() -> ExitCode {
    if let Err(error) = run().await {
        eprintln!("\nrerank_gate STOPPED: {error:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

async fn run() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.fixtures.len() != args.expect.len() {
        bail!(
            "{} --fixture flags but {} --expect flags; they pair by position",
            args.fixtures.len(),
            args.expect.len()
        );
    }
    if args.batch == 0 || args.batch > 64 {
        bail!(
            "--batch must be between 1 and 64 (hand-off §3 cap), got {}",
            args.batch
        );
    }
    std::fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("creating out-dir {}", args.out_dir.display()))?;

    let client = RerankClient::new(&args.base_url, &args.model, args.timeout_secs)?;
    check_readiness(&client, &args.model, &args.base_url).await?;

    let bars = GateBars {
        a_k: args.gate_a_k,
        a_min: args.gate_a_min,
        b_k: args.gate_b_k,
    };
    for (path, expected) in args.fixtures.iter().zip(args.expect.iter()) {
        run_fixture(&client, path, *expected, &args, bars).await?;
    }
    Ok(())
}

/// STOP 1 and STOP 2, run once for the whole invocation.
async fn check_readiness(client: &RerankClient, model: &str, base_url: &str) -> anyhow::Result<()> {
    let ids = client.readiness().await?;
    println!("STOP 1 — GET {base_url}/v1/models answered: {ids:?}");
    if !ids.iter().any(|id| id == model) {
        bail!(
            "STOP 2 — {base_url} is serving {ids:?}, not {model:?}. \
             Another model is loaded; /score is NOT called against it."
        );
    }
    if ids.len() != 1 {
        println!(
            "  note: {} models listed; {model:?} is among them",
            ids.len()
        );
    }
    println!("STOP 2 — model id matches exactly: {model:?}\n");
    Ok(())
}

/// Load one fixture, score all three surfaces, print the block, write the CSV.
async fn run_fixture(
    client: &RerankClient,
    path: &std::path::Path,
    expected: ExpectedCounts,
    args: &Args,
    bars: GateBars,
) -> anyhow::Result<()> {
    let fx = fixture::load(path)?;
    fx.assert_counts(expected)?;
    let relevant = fx.positions_of(&fx.opus_relevant_ids);
    let included = fx.positions_of(&fx.included_ids);

    let query = compose_query(
        &fx.query.theme,
        &fx.query.allegations,
        &fx.query.talking_points,
    );
    if query.trim().is_empty() {
        bail!(
            "{}: the composed query is empty; nothing could be scored",
            fx.scenario
        );
    }

    let mut runs: Vec<SurfaceRun> = Vec::with_capacity(Surface::ALL.len());
    for surface in Surface::ALL {
        runs.push(score_surface(client, &fx, &query, surface, args.batch).await?);
    }

    print!(
        "{}",
        report::header(&fx, path, &query, &args.base_url, &args.model, args.batch)
    );
    for run in &runs {
        print!(
            "{}",
            report::surface_block(&fx, run, &relevant, &included, bars)
        );
    }
    let s2 = runs
        .iter()
        .find(|r| r.surface == Surface::S2Probe)
        .context("the S2 run is missing — the surface list was built wrong")?;
    print!("{}", report::outside_pool_block(&fx, s2, bars));

    write_fixture_csv(&fx, &runs, &relevant, &included, &args.out_dir)?;
    println!("{}\n", "=".repeat(96));
    Ok(())
}

/// Score every candidate and every outside-pool card on one surface.
async fn score_surface(
    client: &RerankClient,
    fx: &fixture::Fixture,
    query: &str,
    surface: Surface,
    batch: usize,
) -> anyhow::Result<SurfaceRun> {
    let candidate_texts: Vec<String> = fx
        .candidates
        .iter()
        .map(|c| surface_text(c, surface))
        .collect();
    let pool_texts: Vec<String> = fx
        .outside_pool
        .iter()
        .map(|c| surface_text(c, surface))
        .collect();

    let started = Instant::now();
    let candidate_scores = score_all(client, query, &candidate_texts, batch).await?;
    let pool_scores = score_all(client, query, &pool_texts, batch).await?;
    let elapsed_secs = started.elapsed().as_secs_f64();

    let candidate_ranks = rank_desc(&candidate_scores);
    let pool_would_be_ranks = pool_scores
        .iter()
        .map(|&s| would_be_rank(s, &candidate_scores))
        .collect();

    Ok(SurfaceRun {
        surface,
        candidate_scores,
        candidate_ranks,
        pool_scores,
        pool_would_be_ranks,
        elapsed_secs,
    })
}

/// Send the texts in sequential batches of at most `batch`.
///
/// Sequential, not concurrent: the hand-off asks for it, and a single GPU serving
/// one model gains nothing from parallel requests but does gain a queue that can
/// time out.
async fn score_all(
    client: &RerankClient,
    query: &str,
    texts: &[String],
    batch: usize,
) -> anyhow::Result<Vec<f64>> {
    let mut scores = Vec::with_capacity(texts.len());
    for chunk in texts.chunks(batch) {
        scores.extend(client.score_batch(query, chunk).await?);
    }
    if scores.len() != texts.len() {
        bail!(
            "STOP 4 — {} texts sent across batches, {} scores collected",
            texts.len(),
            scores.len()
        );
    }
    Ok(scores)
}

/// Write the CSV and say where it went.
fn write_fixture_csv(
    fx: &fixture::Fixture,
    runs: &[SurfaceRun],
    relevant: &[usize],
    included: &[usize],
    out_dir: &std::path::Path,
) -> anyhow::Result<()> {
    let ordered = [
        find_run(runs, Surface::S1Quote)?,
        find_run(runs, Surface::S2Probe)?,
        find_run(runs, Surface::S3Titled)?,
    ];
    let name = format!(
        "{}_rerank_gate_v1.csv",
        fx.scenario.to_lowercase().replace(['-', ' '], "")
    );
    let path = out_dir.join(&name);
    let lines = csv_out::write_csv(&path, fx, &ordered, relevant, included)
        .with_context(|| format!("writing {}", path.display()))?;
    println!("{}", "-".repeat(96));
    println!("  CSV: {} ({lines} lines including header)", path.display());
    Ok(())
}

fn find_run(runs: &[SurfaceRun], surface: Surface) -> anyhow::Result<&SurfaceRun> {
    runs.iter()
        .find(|r| r.surface == surface)
        .with_context(|| format!("no run recorded for surface {}", surface.label()))
}
