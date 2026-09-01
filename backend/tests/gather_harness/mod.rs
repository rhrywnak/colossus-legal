//! Plumbing for the L2b measurement: the connections, the guarded database, and
//! the one function that runs a scenario end to end.
//!
//! Split from `ranked_gather_l2b_measurement.rs` when that file passed 300 code
//! lines (Rule 17). The boundary is a real one and not just a size cut:
//! everything here answers "how do we run a gather against live infrastructure",
//! while the file it came from answers "and did the acceptance bars pass".
//!
//! `tests/gather_harness/mod.rs` rather than a sibling `.rs` so Cargo treats it
//! as a MODULE of the test binary and not as a test binary of its own — a
//! top-level `tests/*.rs` file is compiled and run separately, which would mean
//! this plumbing looked like a test suite with no tests in it.

use colossus_legal_backend::domain::gather_filter::GatherSubjectFilter;
use colossus_legal_backend::repositories::gather_query_repository::allegations_for_query;
use colossus_legal_backend::services::embedding_service::EmbeddingService;
use colossus_legal_backend::services::gather_query::{compose_gather_query, ScenarioQueryInput};
use colossus_legal_backend::services::gather_search::{ranked_gather, GatherInput};
use colossus_legal_backend::services::gather_vector::query_text;
pub use neo4rs::Graph;
use sqlx::{PgPool, Row};

pub type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Databases this file must never be pointed at. It fills a mirror, and rows
/// written here would be indistinguishable from real evidence in the store L2
/// is about to search.
const FORBIDDEN_DATABASES: &[&str] = &["colossus_legal", "colossus_legal_v2"];

/// Open the throwaway mirror database, or refuse by name before connecting.
pub async fn guarded_pool() -> TestResult<PgPool> {
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
pub struct Scenario {
    pub label: String,
    pub subject: String,
    pub theme: Option<String>,
    pub anchors: Vec<String>,
}

/// Read a scenario by its ordinal from the PIPELINE database. READ-ONLY.
pub async fn read_scenario(pipeline: &PgPool, ordinal: i32) -> TestResult<Scenario> {
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
pub async fn measure(
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
            // The shipped default of `gather_probe_max_share`, 1/6. Pinned here
            // so the measurement is reproducible against a named number rather
            // than whatever a store happens to hold.
            probe_max_share: 1.0 / 6.0,
            probe_floor: 3,
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
    println!(
        "  probes extracted      : {}   kept {}   dropped {}",
        gather.probes_extracted,
        gather.probes.len(),
        gather.probes_dropped.len()
    );
    println!("  PROBES KEPT           : {}", gather.probes.join("  "));
    println!(
        "  COLLAPSED             : {}",
        if gather.collapsed.is_empty() {
            "none — every probe found a different set".to_string()
        } else {
            gather
                .collapsed
                .iter()
                .map(|g| format!("{} <- {}", g.representative, g.collapsed.join(", ")))
                .collect::<Vec<_>>()
                .join("   ")
        }
    );
    let mut dropped = gather.probes_dropped.clone();
    dropped.sort_by(|a, b| b.matches.cmp(&a.matches).then(a.probe.cmp(&b.probe)));
    println!(
        "  DROPPED (probe=count) : {}",
        dropped
            .iter()
            .map(|c| format!("{}={}", c.probe, c.matches))
            .collect::<Vec<_>>()
            .join("  ")
    );
    // Sorted by hit count: the noisiest probe first, because that is the one
    // capable of drowning the fused ranking, and a bare total cannot name it.
    let mut by_hits = gather.probe_hits.clone();
    by_hits.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    println!(
        "  probe hits (top 6)    : {}",
        by_hits
            .iter()
            .take(6)
            .map(|(probe, n)| format!("{probe}={n}"))
            .collect::<Vec<_>>()
            .join("  ")
    );
    println!(
        "  trigram returned      : {} rows across {} distinct set(s), from {} probe(s) that hit",
        gather.trigram_hits, gather.trigram_lists, gather.trigram_lists_read
    );
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
pub struct Measured {
    /// The fused ids, best first.
    pub ranked: Vec<String>,
    /// How many of today's cards the ranked list does not contain.
    pub conservation_gap: usize,
    pub subject_only_pool: usize,
}

/// Report conservation LAST, after the acceptance numbers are on screen.
pub fn assert_conservation(m: &Measured) {
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

/// Every connection the measurement needs.
pub async fn wire() -> TestResult<(PgPool, PgPool, Graph, reqwest::Client, String)> {
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
