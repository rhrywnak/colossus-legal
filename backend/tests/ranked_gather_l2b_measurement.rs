//! backend/tests/ranked_gather_l2b_measurement.rs
//!
//! ⚑ L2b.5 — AT-1 and AT-2 at the pre-reranker bar.
//!
//! **This is the number the gather cascade exists to produce.** Everything
//! before it — the mirror, the backfill, the composer, the widening, the two
//! reads, the fusion — was built so this file could print whether S-9 and S-11
//! can finally see the evidence they cannot see today.
//!
//! `#[ignore]`d and run by hand, like L1a's and L1b's, because it needs a live
//! graph, a live Qdrant, the model weights, and a throwaway database. The
//! project has no `#[sqlx::test]` fixture tier.
//!
//! ```text
//! EVIDENCE_SEARCH_TEST_DATABASE_URL=postgres://…/colossus_l2b_proof \
//! PIPELINE_DATABASE_URL=postgres://…/colossus_legal_v2 \
//! NEO4J_URI=… NEO4J_USER=… NEO4J_PASSWORD=… \
//! QDRANT_URL=http://…:6333 FASTEMBED_CACHE_PATH=… \
//!   cargo test -p colossus-legal-backend \
//!     --test ranked_gather_l2b_measurement -- --ignored --nocapture --test-threads=1
//! ```
//!
//! ## ⚑ It reports. It does not tune.
//!
//! If a bar is missed this prints the numbers and says which stage lost the
//! card — absent from both reads, or present and ranked low. It does not adjust
//! k, the depth, the weights or the filter to make a bar pass. A gather tuned
//! to pass its own acceptance test proves nothing, and the diagnosis is the
//! architect's move, not this file's.

use neo4rs::Graph;
use sqlx::{PgPool, Row};

use colossus_legal_backend::domain::gather_filter::GatherSubjectFilter;
use colossus_legal_backend::repositories::gather_query_repository::allegations_for_query;
use colossus_legal_backend::services::embedding_service::EmbeddingService;
use colossus_legal_backend::services::gather_query::{compose_gather_query, ScenarioQueryInput};
use colossus_legal_backend::services::gather_search::{ranked_gather, GatherInput};
use colossus_legal_backend::services::gather_vector::query_text;

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Databases this file must never be pointed at. It fills a mirror, and rows
/// written here would be indistinguishable from real evidence in the store L2
/// is about to search.
const FORBIDDEN_DATABASES: &[&str] = &["colossus_legal", "colossus_legal_v2"];

/// AT-1: S-9 must be able to see C-54, within the top 60.
const AT1_BAR_RANK: usize = 60;
/// AT-2: all seven $50,000 admissions in the top 60. The top-20 count is
/// informational here — that bar is L3's, with the reranker.
const AT2_TOP60_BAR: usize = 7;
const AT2_TOP20_INFORMATIONAL: usize = 20;

/// Open the throwaway mirror database, or refuse by name before connecting.
async fn guarded_pool() -> TestResult<PgPool> {
    let url = std::env::var("EVIDENCE_SEARCH_TEST_DATABASE_URL").map_err(|_| {
        "EVIDENCE_SEARCH_TEST_DATABASE_URL is not set — point it at a throwaway database"
    })?;
    let name = url
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .split('?')
        .next()
        .unwrap_or_default();
    if FORBIDDEN_DATABASES.contains(&name) {
        return Err(format!("refusing to run against '{name}'").into());
    }
    Ok(PgPool::connect(&url).await?)
}

/// One scenario, as the measurement needs it.
struct Scenario {
    label: String,
    subject: String,
    theme: Option<String>,
    anchors: Vec<String>,
}

/// Read a scenario by its ordinal from the PIPELINE database. READ-ONLY.
async fn read_scenario(pipeline: &PgPool, ordinal: i32) -> TestResult<Scenario> {
    let row = sqlx::query(
        "SELECT definition->>'target' AS subject, theme_statement, anchor_allegation_ids \
           FROM scenarios WHERE code_ordinal = $1",
    )
    .bind(ordinal)
    .fetch_one(pipeline)
    .await?;

    Ok(Scenario {
        label: format!("S-{ordinal}"),
        subject: row
            .try_get::<Option<String>, _>("subject")?
            .ok_or_else(|| {
                format!("S-{ordinal} has no definition->>'target'; the subject is the party filter")
            })?,
        theme: row.try_get("theme_statement")?,
        anchors: row.try_get("anchor_allegation_ids")?,
    })
}

/// Run one scenario through the whole cascade and print what it produced.
///
/// Returns the fused id list, best-first, so the caller can ask where a
/// specific card landed.
async fn measure(
    scenario: &Scenario,
    pool: &PgPool,
    graph: &Graph,
    client: &reqwest::Client,
    qdrant_url: &str,
) -> TestResult<Measured> {
    let allegations = allegations_for_query(graph, &scenario.anchors).await?;
    let composed = compose_gather_query(
        &ScenarioQueryInput {
            subject: scenario.subject.clone(),
            theme: scenario.theme.clone(),
        },
        &allegations,
        &[],
    );

    // The `search_query:` prefix, then the model. Blocking, on its own thread:
    // `TextEmbedding` is not `Send`.
    let cache = std::env::var("FASTEMBED_CACHE_PATH")
        .map_err(|_| "FASTEMBED_CACHE_PATH is not set — the query cannot be embedded")?;
    let prefixed = query_text(&composed.text);
    let vector = tokio::task::spawn_blocking(move || -> TestResult<Vec<f32>> {
        let mut service = EmbeddingService::new(&cache)?;
        Ok(service.embed_one(&prefixed)?)
    })
    .await??;

    let gather = ranked_gather(
        pool,
        client,
        qdrant_url,
        GatherInput {
            query: &composed.text,
            query_vector: &vector,
            subject: &scenario.subject,
            reachable_parties: &composed.reachable_parties,
            filter_mode: GatherSubjectFilter::Widened,
            // The shipped default of the `gather_read_depth` row. Read from the
            // settings store once L2c wires this behind an endpoint; pinned
            // here so the measurement is reproducible against a named number
            // rather than whatever a store happens to hold.
            read_depth: 200,
        },
    )
    .await?;

    println!(
        "\n─── {} ─────────────────────────────────────────",
        scenario.label
    );
    println!("  subject               : {}", scenario.subject);
    println!(
        "  query basis           : {}",
        composed.query_basis.as_str()
    );
    println!(
        "  query length          : {} chars",
        composed.text.chars().count()
    );
    println!("  allegations linked    : {}", allegations.len());
    println!(
        "  reachable parties     : {}",
        composed.reachable_parties.join(", ")
    );
    println!("  filter mode           : {}", gather.filter_mode);
    println!("  ids admitted by filter: {}", gather.admitted.len());
    println!("  vector read returned  : {}", gather.vector_hits);
    println!("  full-text returned    : {}", gather.full_text_hits);
    println!("  trigram returned      : {}", gather.trigram_hits);
    println!("  read depth (each read): {}", gather.read_depth);
    println!("  RANKED LIST SIZE      : {}", gather.cards.len());
    println!(
        "  subject-only pool     : {}",
        gather.subject_only_pool.len()
    );
    println!("  unreached by reads    : {}", gather.unreached_by_reads);
    println!(
        "  CONSERVATION          : {}",
        if gather.conservation_gap.is_empty() {
            format!(
                "HOLDS — every card in today's pool is present ({} of them carried by the \
                 conservation tail)",
                gather.unreached_by_reads
            )
        } else {
            format!(
                "VIOLATED — {} card(s) dropped: {}",
                gather.conservation_gap.len(),
                gather.conservation_gap.join(", ")
            )
        }
    );
    // Deliberately NOT asserted here. The AT numbers are what this file exists
    // to print, and aborting on conservation before printing them would hide
    // the measurement behind a different failure. The caller asserts last.
    // The AT bars are about RETRIEVAL, so a card sitting in the conservation
    // tail must not be able to satisfy one by accident. `retrieved_ids` is the
    // filter, and it lives on the type where a unit test can reach it.
    let retrieved = gather.retrieved_ids();
    println!("  of which RANKED       : {}", retrieved.len());

    Ok(Measured {
        ranked: retrieved,
        conservation_gap: gather.conservation_gap.len(),
        subject_only_pool: gather.subject_only_pool.len(),
    })
}

/// What one measured scenario produced.
struct Measured {
    /// The fused ids, best first.
    ranked: Vec<String>,
    /// How many of today's cards the ranked list does not contain.
    conservation_gap: usize,
    subject_only_pool: usize,
}

/// Report conservation LAST, after the acceptance numbers are on screen.
fn assert_conservation(m: &Measured) {
    assert_eq!(
        m.conservation_gap,
        0,
        "CONSERVATION VIOLATED: {} of the {} cards in today's subject-only pool are \
         absent from a ranked list of {}. Note the arithmetic before reading this as a \
         ranking defect: a list bounded at the read depth cannot contain a pool larger \
         than itself.",
        m.conservation_gap,
        m.subject_only_pool,
        m.ranked.len()
    );
}

/// Where a card landed, or why it is missing.
fn rank_of(ranked: &[String], id: &str) -> Option<usize> {
    ranked.iter().position(|r| r == id).map(|i| i + 1)
}

/// ⚑ AT-1 — S-9 must be able to see C-54.
#[tokio::test]
#[ignore = "needs a live graph, Qdrant, the model weights and a throwaway database"]
async fn at1_s9_can_see_c54() -> TestResult<()> {
    let (pool, pipeline, graph, client, qdrant) = wire().await?;
    let scenario = read_scenario(&pipeline, 9).await?;
    let measured = measure(&scenario, &pool, &graph, &client, &qdrant).await?;
    let ranked = &measured.ranked;

    let target = std::env::var("AT1_CARD_ID")
        .map_err(|_| "AT1_CARD_ID is not set — it is C-54's evidence id from G0's fixture")?;

    println!("\n  AT-1  S-9  : C-54 ({target})");
    match rank_of(ranked, &target) {
        Some(rank) => {
            println!(
                "        rank = {rank} of {}  (bar: present, <= {AT1_BAR_RANK})",
                ranked.len()
            );
            println!(
                "        {}",
                if rank <= AT1_BAR_RANK {
                    "BAR MET"
                } else {
                    "BAR MISSED — present but ranked low"
                }
            );
            assert_conservation(&measured);
            assert!(
                rank <= AT1_BAR_RANK,
                "C-54 is present at {rank} but the bar is {AT1_BAR_RANK}"
            );
        }
        None => {
            println!("        ABSENT from the ranked list of {}", ranked.len());
            println!("        BAR MISSED — the card never reached the fusion; the stage that");
            println!("        lost it is the read, not the ranking.");
            panic!("AT-1: C-54 is absent from S-9's ranked gather");
        }
    }
    Ok(())
}

/// ⚑ AT-2 — S-11 must be able to see all seven $50,000 admissions.
#[tokio::test]
#[ignore = "needs a live graph, Qdrant, the model weights and a throwaway database"]
async fn at2_s11_can_see_the_seven_admissions() -> TestResult<()> {
    let (pool, pipeline, graph, client, qdrant) = wire().await?;
    let scenario = read_scenario(&pipeline, 11).await?;
    let measured = measure(&scenario, &pool, &graph, &client, &qdrant).await?;
    let ranked = &measured.ranked;

    let raw = std::env::var("AT2_CARD_IDS")
        .map_err(|_| "AT2_CARD_IDS is not set — the seven ids, comma-separated, from G0")?;
    let targets: Vec<&str> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(targets.len(), 7, "AT-2 is about seven admissions");

    println!("\n  AT-2  S-11 : the seven $50,000 admissions, each by rank");
    let mut in_60 = 0usize;
    let mut in_20 = 0usize;
    for id in &targets {
        match rank_of(ranked, id) {
            Some(rank) => {
                if rank <= AT1_BAR_RANK {
                    in_60 += 1;
                }
                if rank <= AT2_TOP20_INFORMATIONAL {
                    in_20 += 1;
                }
                println!("        {id}  rank {rank} of {}", ranked.len());
            }
            None => println!("        {id}  ABSENT — never reached the fusion"),
        }
    }
    println!("\n        in top 60 = {in_60} of 7     (bar: {AT2_TOP60_BAR} of 7)");
    println!("        in top 20 = {in_20} of 7     (informational — that bar is L3's)");

    if in_60 < AT2_TOP60_BAR {
        let absent = targets
            .iter()
            .filter(|id| rank_of(ranked, id).is_none())
            .count();
        println!(
            "\n        BAR MISSED. Of the {} short, {absent} are ABSENT from the list",
            AT2_TOP60_BAR - in_60
        );
        println!("        entirely (lost by the READS) and the rest are present but ranked");
        println!("        below 60 (lost by the RANKING). Reported, not tuned.");
    }
    assert_conservation(&measured);
    assert!(
        in_60 >= AT2_TOP60_BAR,
        "AT-2: {in_60} of 7 admissions in the top 60, bar is {AT2_TOP60_BAR}"
    );
    Ok(())
}

/// Every connection the measurement needs.
async fn wire() -> TestResult<(PgPool, PgPool, Graph, reqwest::Client, String)> {
    let pool = guarded_pool().await?;
    let pipeline =
        PgPool::connect(&std::env::var("PIPELINE_DATABASE_URL").map_err(|_| {
            "PIPELINE_DATABASE_URL is not set — the scenario rows are read from it"
        })?)
        .await?;
    let graph = Graph::new(
        std::env::var("NEO4J_URI")?,
        std::env::var("NEO4J_USER")?,
        std::env::var("NEO4J_PASSWORD")?,
    )
    .await?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(5))
        .build()?;
    let qdrant = std::env::var("QDRANT_URL").map_err(|_| "QDRANT_URL is not set")?;
    Ok((pool, pipeline, graph, client, qdrant))
}
