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

mod gather_harness;

use gather_harness::{assert_conservation, measure, read_scenario, wire, TestResult};

/// AT-1: S-9 must be able to see C-54, within the top 60.
const AT1_BAR_RANK: usize = 60;
/// AT-2: all seven $50,000 admissions in the top 60. The top-20 count is
/// informational here — that bar is L3's, with the reranker.
const AT2_TOP60_BAR: usize = 7;
const AT2_TOP20_INFORMATIONAL: usize = 20;

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
