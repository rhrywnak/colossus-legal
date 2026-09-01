//! Export one ranked evidence gather per scenario, as Markdown a human reads.
//!
//! ## What this is for
//!
//! Roman drafts seed questions from these lists the night before a meeting. The
//! whole gather cascade — the mirror, the composer, the widening, the two
//! reads, the fusion — exists so this file can be written, and this bin is the
//! only thing that writes it.
//!
//! ## READ-ONLY, against two databases and two search stores
//!
//! It reads the pipeline database for scenario rows and C-numbers, the graph
//! for allegations, the mirror for card bodies and the lexical half, and Qdrant
//! for the vector half. It writes nothing but files, and it writes them OUTSIDE
//! the repo (law 6) — the directory is a required argument with no default, so
//! it cannot quietly write somewhere nobody looks.
//!
//! ```text
//! cargo run --bin ranked_gather_export -- \
//!   --out ~/Documents/colossus-legal/SEEDS --scenarios 12,13,14,9,11
//! ```

mod cards;
mod render;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use neo4rs::Graph;
use sqlx::{PgPool, Row};

use colossus_legal_backend::domain::gather_filter::GatherSubjectFilter;
use colossus_legal_backend::repositories::gather_query_repository::allegations_for_query;
use colossus_legal_backend::services::embedding_service::EmbeddingService;
use colossus_legal_backend::services::gather_fusion::CardPlacement;
use colossus_legal_backend::services::gather_query::{compose_gather_query, ScenarioQueryInput};
use colossus_legal_backend::services::gather_search::{ranked_gather, GatherInput};
use colossus_legal_backend::services::gather_vector::query_text;

type Fallible<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// The settings the gather runs with.
///
/// Passed as flags rather than read from the store because this bin points at a
/// THROWAWAY mirror while the settings live in the real pipeline database; a
/// reader of the exported file needs to know which numbers produced it, and the
/// file records them. The defaults are the shipped rows.
#[derive(Parser, Debug)]
#[command(about = "Export the ranked evidence gather for one or more scenarios as Markdown")]
struct Args {
    /// Directory to write one Markdown file per scenario into. Required and
    /// unguessed: law 6 keeps case output out of the repo.
    #[arg(long)]
    out: PathBuf,
    /// Scenario ordinals, comma-separated — `12,13,14,9,11`.
    #[arg(long)]
    scenarios: String,
    /// `gather_read_depth`.
    // STRUCTURAL: mirrors the shipped settings row and is not read from it —
    // this bin points at a THROWAWAY mirror while the rows live in the real
    // pipeline database, so reading the store would report numbers that did not
    // produce this file. The value is echoed INTO the file for that reason.
    #[arg(long, default_value_t = 200)]
    read_depth: usize,
    /// `gather_probe_max_share`, as a fraction.
    // STRUCTURAL: as above — the shipped row's value, echoed into the file.
    #[arg(long, default_value_t = 1.0 / 3.0)]
    probe_max_share: f64,
    /// `gather_probe_floor`.
    // STRUCTURAL: as above — the shipped row's value, echoed into the file.
    #[arg(long, default_value_t = 3)]
    probe_floor: usize,
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt().with_target(false).init();
    match run(Args::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("ranked gather export failed: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: Args) -> Fallible<()> {
    let wiring = Wiring {
        mirror: PgPool::connect(&env("EVIDENCE_SEARCH_DATABASE_URL")?).await?,
        pipeline: PgPool::connect(&env("PIPELINE_DATABASE_URL")?).await?,
        graph: Graph::new(
            env("NEO4J_URI")?,
            env("NEO4J_USER")?,
            env("NEO4J_PASSWORD")?,
        )
        .await?,
        client: reqwest::Client::builder()
            // DEFAULT: 60s rather than the backend's 30s because this is a
            // one-shot batch, not a request a human is holding a page open for:
            // it embeds five queries in a row and a cold ONNX init on the first
            // is measured near half a minute. Override by editing here — a bin
            // with no config file is the one place a literal is the whole story.
            .timeout(std::time::Duration::from_secs(60))
            // DEFAULT: connect is a TCP handshake to a host on the same LAN;
            // 5s is the backend's own value and there is no reason to differ.
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()?,
        qdrant: env("QDRANT_URL")?,
        cache: env("FASTEMBED_CACHE_PATH")?,
    };

    std::fs::create_dir_all(&args.out)?;

    let ordinals = parse_ordinals(&args.scenarios)?;
    for (index, ordinal) in ordinals.iter().enumerate() {
        let written = export_one(&args, *ordinal, &wiring).await.map_err(|e| {
            // Fail fast, but name what will NOT be written. A human waiting for
            // five files gets two and one error line; without this they have to
            // infer the rest from the count.
            let skipped: Vec<String> = ordinals[index + 1..]
                .iter()
                .map(|o| format!("S-{o}"))
                .collect();
            if skipped.is_empty() {
                e
            } else {
                format!("{e} — not attempted: {}", skipped.join(", ")).into()
            }
        })?;
        // Rule 1: "written" alone cannot tell a full list from an empty one, and
        // an empty list is exactly what the first reader has to catch. The yield
        // goes in the line, not only in the file.
        tracing::info!(
            scenario = format!("S-{ordinal}"),
            ranked = written.ranked,
            tail = written.tail,
            missing_from_mirror = written.missing,
            file = %written.path.display(),
            "written"
        );
        if written.ranked == 0 {
            tracing::warn!(
                scenario = format!("S-{ordinal}"),
                "neither read returned anything; the file is a conservation tail and nothing else"
            );
        }
        if written.missing > 0 {
            tracing::warn!(
                scenario = format!("S-{ordinal}"),
                missing = written.missing,
                "cards in the ranking are absent from the mirror and render without a quote"
            );
        }
    }
    Ok(())
}

/// What one exported file turned out to contain.
///
/// Returned rather than logged inside `export_one` so the caller decides what is
/// worth saying — and so an empty list is a fact the batch loop can act on.
struct Written {
    path: PathBuf,
    ranked: usize,
    tail: usize,
    /// Ranked or tail cards the mirror did not have. They render as a named
    /// placeholder rather than a blank, and this is the count of them.
    missing: usize,
}

/// Everything the export talks to.
///
/// ## Rust Learning: a struct instead of eight parameters
///
/// `export_one` needed eight arguments, four of them borrowed handles that look
/// alike at a call site — two `PgPool`s in particular, where swapping the
/// mirror and the pipeline would compile and read the wrong database. Naming
/// them in a struct makes that mistake unwritable, and keeps the function
/// inside the argument limit clippy enforces for exactly this reason.
struct Wiring {
    mirror: PgPool,
    pipeline: PgPool,
    graph: Graph,
    client: reqwest::Client,
    qdrant: String,
    cache: String,
}

/// One scenario, end to end.
async fn export_one(args: &Args, ordinal: i32, wiring: &Wiring) -> Fallible<Written> {
    let row = sqlx::query(
        "SELECT scenario_id::text AS scenario_id, definition->>'target' AS subject, \
                coalesce(theme_statement, '') AS theme, anchor_allegation_ids \
           FROM scenarios WHERE code_ordinal = $1",
    )
    .bind(ordinal)
    .fetch_one(&wiring.pipeline)
    .await
    .map_err(|e| format!("S-{ordinal} could not be read: {e}"))?;

    let scenario_id: String = row.try_get("scenario_id")?;
    let subject: String = row
        .try_get::<Option<String>, _>("subject")?
        .ok_or_else(|| format!("S-{ordinal} has no definition->>'target'"))?;
    let theme: String = row.try_get("theme")?;
    let anchors: Vec<String> = row.try_get("anchor_allegation_ids")?;

    let allegations = allegations_for_query(&wiring.graph, &anchors).await?;
    let composed = compose_gather_query(
        &ScenarioQueryInput {
            subject: subject.clone(),
            theme: (!theme.trim().is_empty()).then(|| theme.clone()),
        },
        &allegations,
        &[],
    );

    let prefixed = query_text(&composed.text);
    let cache = wiring.cache.clone();
    let vector = tokio::task::spawn_blocking(move || -> Fallible<Vec<f32>> {
        let mut service = EmbeddingService::new(&cache)?;
        Ok(service.embed_one(&prefixed)?)
    })
    .await??;

    let gather = ranked_gather(
        &wiring.mirror,
        &wiring.client,
        &wiring.qdrant,
        GatherInput {
            query: &composed.text,
            query_vector: &vector,
            subject: &subject,
            reachable_parties: &composed.reachable_parties,
            filter_mode: GatherSubjectFilter::Widened,
            probe_max_share: args.probe_max_share,
            probe_floor: args.probe_floor,
            read_depth: args.read_depth,
        },
    )
    .await?;

    let ids: Vec<String> = gather.cards.iter().map(|c| c.evidence_id.clone()).collect();
    let bodies = cards::read_cards(&wiring.mirror, &wiring.pipeline, &scenario_id, &ids).await?;

    let code = format!("S-{ordinal}");
    let page = render::page(
        &render::Scenario {
            code: &code,
            theme: if theme.trim().is_empty() {
                "(no theme written yet)"
            } else {
                theme.trim()
            },
            subject: &subject,
            query: &composed.text,
            basis: composed.query_basis.as_str(),
        },
        &gather,
        &bodies,
        &render::Settings {
            probe_max_share: args.probe_max_share,
            probe_floor: args.probe_floor,
        },
    );

    let path = args.out.join(format!("{code}_ranked_gather.md"));
    std::fs::write(&path, page)?;

    let ranked = gather
        .cards
        .iter()
        .filter(|c| c.placement == CardPlacement::Ranked)
        .count();
    Ok(Written {
        ranked,
        tail: gather.cards.len() - ranked,
        // Counted over the ids the page actually shows, which is where a
        // missing body becomes a card with no quote.
        missing: ids.iter().filter(|id| !bodies.contains_key(*id)).count(),
        path,
    })
}

/// `12,13,14` -> `[12, 13, 14]`, refusing anything else by name.
fn parse_ordinals(raw: &str) -> Fallible<Vec<i32>> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.trim_start_matches("S-")
                .parse::<i32>()
                .map_err(|_| format!("'{s}' is not a scenario ordinal; expected 12 or S-12").into())
        })
        .collect()
}

/// A required environment variable, named when it is missing.
fn env(key: &str) -> Fallible<String> {
    std::env::var(key).map_err(|_| format!("{key} is not set").into())
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
