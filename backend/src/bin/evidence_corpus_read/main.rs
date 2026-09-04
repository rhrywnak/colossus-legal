//! `evidence_corpus_read` — count every defect class in the Evidence corpus.
//!
//! **Read-only, zero cost.** No paid API, no embedding, no LLM. Every Postgres
//! connection runs `SET SESSION CHARACTERISTICS AS TRANSACTION READ ONLY` before
//! it is used, and every Cypher statement is a `MATCH`. This tool cannot change
//! DEV even if it is edited to try.
//!
//! It answers one question — "how many cards fall in each bucket" — so the
//! decision to delete or repair is made on numbers. It recommends nothing; the
//! disposal ruling is the architect's and Roman's.
//!
//! ## Usage
//!
//! ```text
//! EVIDENCE_CORPUS_READ_DATABASE_URL=postgres://…/colossus_legal_v2 \
//! NEO4J_URI=bolt://HOST:7687 NEO4J_USER=neo4j NEO4J_PASSWORD=… \
//! cargo run --bin evidence_corpus_read -- \
//!   --expect-database colossus_legal_v2 --expect-total 1209 \
//!   --out-dir ~/Documents/colossus-legal/AUDITS/EVIDENCE_CORPUS_READ_v1
//! ```
//!
//! Like `gate_fixture`, it does NOT read `.env`: an audit whose corpus depends on
//! whatever a dotfile held is an audit nobody can check.

mod buckets;
mod load;
mod model;
mod norm;
mod output;

use anyhow::{bail, Context, Result};
use clap::Parser;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use colossus_legal_backend::oneshot::cli::connect_graph;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

use buckets::Rules;
use model::Flags;

/// The statement every pooled connection runs before it is used.
///
/// Belt and braces: the code below issues only `SELECT`s, but "the code only
/// issues SELECTs" is a claim about a file somebody may edit. This is a claim the
/// SERVER enforces — a stray `INSERT` fails with `cannot execute INSERT in a
/// read-only transaction` rather than changing DEV.
//
// STRUCTURAL: Postgres wire vocabulary, and the invariant this tool exists to
// hold. There is no environment that would want it relaxed, so it is not config.
const READ_ONLY_SESSION: &str = "SET SESSION CHARACTERISTICS AS TRANSACTION READ ONLY";

#[derive(Parser, Debug)]
#[command(about = "Census of the Evidence corpus. READ-ONLY, zero cost.")]
struct Args {
    /// Postgres URL for the PIPELINE database. Falls back to
    /// `EVIDENCE_CORPUS_READ_DATABASE_URL`; never read from `.env`.
    #[arg(long)]
    database_url: Option<String>,

    /// The database this run must be reading. No default: naming it is how the
    /// operator states which corpus the audit is a claim about.
    #[arg(long)]
    expect_database: String,

    /// STOP 0's card count. The run aborts before reading anything else if the
    /// corpus is not this size.
    #[arg(long)]
    expect_total: i64,

    /// Where `summary.md`, `cards.csv` and `queries.md` are written.
    #[arg(long)]
    out_dir: PathBuf,

    /// Quotes at or under this many characters are surveyed to DERIVE the B1
    /// answer-token set. The instruction names 25.
    #[arg(long, default_value_t = 25)]
    short_quote_chars: usize,

    /// B3's length ratio: the shorter quote must be at least this fraction of
    /// the longer one before containment counts as a near-duplicate.
    #[arg(long, default_value_t = 0.80)]
    near_ratio: f64,
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(error) = run().await {
        eprintln!("\nevidence_corpus_read STOPPED: {error:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

async fn run() -> Result<()> {
    let args = Args::parse();
    std::fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("creating out-dir {}", args.out_dir.display()))?;

    let pool = open_read_only_pool(&database_url(args.database_url.as_deref())?).await?;
    confirm_database(&pool, &args.expect_database).await?;
    let graph = connect_graph()
        .await
        .map_err(|code| anyhow::anyhow!("could not connect to Neo4j (exit {code:?})"))?;

    let queries = stop_zero(&graph, args.expect_total).await?;
    audit(&args, &pool, &graph, queries).await
}

/// STOP 0 — print the shape of the corpus, and refuse to go on if it is not the
/// size the operator declared.
async fn stop_zero(
    graph: &neo4rs::Graph,
    expect_total: i64,
) -> Result<Vec<(&'static str, String, String)>> {
    let total = load::total_evidence(graph).await?;
    println!("STOP 0 — Evidence nodes: {total}");
    if total != expect_total {
        bail!("STOP 0 — expected {expect_total} Evidence nodes, found {total}. Nothing further was read.");
    }

    let keys = load::count_pairs(graph, load::Q_STOP0_KEYS, &["property_key"]).await?;
    println!("\nSTOP 0 — property keys ({} distinct):", keys.len());
    for (k, n) in &keys {
        println!("  {n:>6}  {k}");
    }

    let rels = load::count_pairs(graph, load::Q_STOP0_RELS, &["rel_type", "other_label"]).await?;
    println!(
        "\nSTOP 0 — outgoing relationships ({} type/label pairs):",
        rels.len()
    );
    for (k, n) in &rels {
        println!("  {n:>6}  {k}");
    }
    println!();

    Ok(vec![
        (
            "STOP 0 — total",
            load::Q_STOP0_TOTAL.to_string(),
            format!("{total} Evidence nodes"),
        ),
        (
            "STOP 0 — property keys",
            load::Q_STOP0_KEYS.to_string(),
            render_pairs(&keys),
        ),
        (
            "STOP 0 — relationships",
            load::Q_STOP0_RELS.to_string(),
            render_pairs(&rels),
        ),
    ])
}

fn render_pairs(pairs: &[(String, i64)]) -> String {
    pairs
        .iter()
        .map(|(k, n)| format!("- `{k}` — {n}"))
        .collect::<Vec<String>>()
        .join("\n")
}

/// The census proper.
async fn audit(
    args: &Args,
    pool: &PgPool,
    graph: &neo4rs::Graph,
    mut queries: Vec<(&'static str, String, String)>,
) -> Result<()> {
    let mut cards = load::load_cards(graph).await?;
    let documents = load::load_documents(pool).await?;
    let provenance = load::load_provenance(pool).await?;
    let mirror = load::load_mirror(pool).await?;
    load::widen(&mut cards, &documents, &provenance);

    let survey = short_quote_survey(&cards, args.short_quote_chars);
    let answer_tokens = derive_answer_tokens(&survey);
    let dropped = dropped_statement_types(pool).await?;

    let mirror_ok: Option<HashSet<String>> = mirror;
    let rules = Rules {
        answer_tokens: &answer_tokens,
        dropped_statement_types: &dropped,
        near_duplicate_min_ratio: args.near_ratio,
        mirror_ok_ids: mirror_ok.as_ref(),
    };
    let (flags, duplicates) = buckets::assign(&cards, &rules);

    let b1_with_question = cards
        .iter()
        .zip(flags.iter())
        .filter(|(c, f)| f.0[0] && c.question.as_deref().is_some_and(|q| !q.trim().is_empty()))
        .count();
    let (twins, cross) = duplicates.twin_split(&cards);
    let mirror_note = match mirror_ok.as_ref() {
        None => "the `evidence_search` table does not exist on this database; no card is flagged",
        Some(_) => "the `evidence_search` table exists; cards with no row or a blank probe_text are flagged",
    };

    print_summary(&cards, &flags);

    let ctx = output::SummaryContext {
        total: cards.len(),
        answer_tokens: &answer_tokens,
        short_quote_survey: &survey,
        short_quote_chars: args.short_quote_chars,
        dropped_statement_types: &dropped,
        near_ratio: args.near_ratio,
        mirror_note,
        b1_with_question,
        duplicates: &duplicates,
        twin_count: twins,
        cross_ref_count: cross,
    };
    output::write_summary(&args.out_dir.join("summary.md"), &cards, &flags, &ctx)?;
    let rows = output::write_cards_csv(&args.out_dir.join("cards.csv"), &cards, &flags)?;

    queries.push((
        "Cards (the one consistent read)",
        load::q_cards(),
        format!("{} cards", cards.len()),
    ));
    queries.push((
        "B4 by document",
        load::Q_GROUNDING_BY_DOC.to_string(),
        "see summary.md".to_string(),
    ));
    queries.push((
        "Documents (B9, B10)",
        load::Q_DOCUMENTS.to_string(),
        format!("{} rows", documents.len()),
    ));
    queries.push((
        "Provenance (B11)",
        load::Q_PROVENANCE.to_string(),
        format!("{} node ids", provenance.len()),
    ));
    queries.push((
        "Mirror (B12)",
        load::Q_MIRROR_EXISTS.to_string(),
        match mirror_ok.as_ref() {
            None => "0 — the table does not exist".to_string(),
            Some(ids) => format!("exists; {} ids with non-blank probe_text", ids.len()),
        },
    ));
    output::write_queries(&args.out_dir.join("queries.md"), &queries)?;

    println!(
        "\nwrote cards.csv ({rows} lines), summary.md and queries.md to {}",
        args.out_dir.display()
    );
    Ok(())
}

/// Print the bucket table to the terminal, so the operator sees it before the
/// files are opened.
fn print_summary(cards: &[model::Card], flags: &[Flags]) {
    println!("{:<5} {:>6}  {:>6}  meaning", "code", "cards", "%");
    for (i, (code, meaning)) in model::BUCKETS.iter().enumerate() {
        let n = flags.iter().filter(|f| f.0[i]).count();
        let pct = (n as f64) * 100.0 / (cards.len().max(1) as f64);
        println!("{code:<5} {n:>6}  {pct:>5.1}%  {meaning}");
    }
    let clean = flags.iter().filter(|f| f.clean()).count();
    println!("\nclean by these rules: {clean} of {}", cards.len());
}

/// Every distinct quote at or under `max_chars`, with how many cards carry it.
fn short_quote_survey(cards: &[model::Card], max_chars: usize) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for card in cards {
        let trimmed = card.quote.trim();
        if norm::char_len(trimmed) <= max_chars {
            *counts.entry(trimmed.to_string()).or_insert(0) += 1;
        }
    }
    let mut out: Vec<(String, usize)> = counts.into_iter().collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    out
}

/// The B1 token set, derived: any short quote carried by two or more cards.
///
/// ## Why "two or more" is the rule
///
/// A discovery-response answer token is by nature repeated — `Admitted.` appears
/// once per request admitted. A short quote appearing exactly once is far more
/// likely to be a real, terse finding than a form answer, and condemning it would
/// be the audit inventing its own evidence. Blank quotes are already B1 by the
/// first clause and are excluded here.
fn derive_answer_tokens(survey: &[(String, usize)]) -> Vec<String> {
    survey
        .iter()
        .filter(|(quote, count)| *count >= 2 && !quote.trim().is_empty())
        .map(|(quote, _)| quote.clone())
        .collect()
}

/// The prefilter's stored dropped-kind list, lower-cased as it compares them.
async fn dropped_statement_types(pool: &PgPool) -> Result<Vec<String>> {
    let row = sqlx::query(
        "SELECT value FROM app_settings WHERE key = 'theme_scan_prefilter_statement_types'",
    )
    .fetch_optional(pool)
    .await
    .context("reading theme_scan_prefilter_statement_types from app_settings")?;
    Ok(row
        .map(|r| r.get::<String, _>("value"))
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect())
}

fn database_url(flag: Option<&str>) -> Result<String> {
    if let Some(url) = flag {
        return Ok(url.to_string());
    }
    std::env::var("EVIDENCE_CORPUS_READ_DATABASE_URL").context(
        "no Postgres URL: pass --database-url or set EVIDENCE_CORPUS_READ_DATABASE_URL. \
         This tool does NOT read .env, on purpose — point it at the pipeline database",
    )
}

/// Open the pool with every connection already in a read-only session.
async fn open_read_only_pool(url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(1)
        // DEFAULT: ten seconds. This tool is run by hand from a runbook step
        // with an operator watching, so the value is about that human: a long
        // hang reads as "it's working" when it means "the host is wrong". No
        // override is offered because there is no deployment that wants one —
        // the same operator runs it against DEV and PROD from the same laptop.
        .acquire_timeout(Duration::from_secs(10))
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query(READ_ONLY_SESSION).execute(conn).await?;
                Ok(())
            })
        })
        .connect(url)
        .await
        .with_context(|| {
            format!(
                "connecting to Postgres at {}, or setting the session read-only",
                redact(url)
            )
        })
}

/// The host and database of a Postgres URL, with every credential removed.
///
/// ## Why this exists rather than interpolating the URL
///
/// The operator needs to see WHERE a failed connection was pointed — a wrong
/// host or port is the commonest first-run mistake. But the URL is a
/// `postgres://user:password@host/db` string, and putting it in an error message
/// puts the database password into the terminal, into any log that captures it,
/// and into every report that pastes the failure. Standing Rule 22 forbids
/// plaintext secrets anywhere, and an error path is still anywhere.
///
/// So the userinfo is dropped and only `host/database` survives — which is the
/// whole of what the operator actually needs.
fn redact(url: &str) -> String {
    let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    // Everything before an `@` is userinfo, and there is at most one.
    let host_and_path = after_scheme
        .rsplit_once('@')
        .map_or(after_scheme, |(_, rest)| rest);
    match host_and_path.split_once('?') {
        Some((before_query, _)) => before_query.to_string(),
        None => host_and_path.to_string(),
    }
}

/// Print what was actually reached, and stop unless it is what was asked for.
async fn confirm_database(pool: &PgPool, expected: &str) -> Result<()> {
    let row = sqlx::query(
        "SELECT current_database() AS db, \
                coalesce(host(inet_server_addr()), 'local socket') AS host, \
                current_setting('transaction_read_only') AS ro",
    )
    .fetch_one(pool)
    .await
    .context("reading current_database() — the connection is unusable")?;
    let (db, host, ro): (String, String, String) = (row.get("db"), row.get("host"), row.get("ro"));
    println!("reading database '{db}' on host {host} · transaction_read_only={ro}");
    if db != expected {
        bail!("connected to '{db}', expected '{expected}' — nothing was read");
    }
    Ok(())
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
