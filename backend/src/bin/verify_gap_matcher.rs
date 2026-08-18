//! `verify_gap_matcher` — offline proof for the verifier's second-chance tiers.
//!
//! Runs the SHIPPED matcher — the same
//! [`find_in_canonical_text_with_policy`] the two verify paths call — against a
//! JSON dump of live quotes and canonical page text, and prints what each item
//! would ground to. Nothing is written anywhere; this tool has no database
//! connection at all.
//!
//! ## Why a dump and not a live connection
//!
//! The proof has to be runnable against a PRODUCTION-shaped corpus without any
//! risk of touching it. A tool that cannot open a socket cannot write a row, and
//! that property is worth more here than the convenience of querying directly.
//!
//! ## Input shape
//!
//! ```json
//! { "documents": [ { "document_id": "...",
//!                    "items": [ {"id": 9402, "quote": "...", "status": "manual", "page": 10} ],
//!                    "pages": [ {"page": 1, "text": "..."} ] } ] }
//! ```
//!
//! ## Usage
//!
//! ```text
//! cargo run --bin verify_gap_matcher -- <dump.json>
//! ```

use std::collections::BTreeMap;
use std::process::ExitCode;

use colossus_legal_backend::api::pipeline::canonical_verifier::{
    find_in_canonical_text_with_policy, is_grounded, CanonicalMatchType,
};
use colossus_legal_backend::domain::quote_gap::GapPolicy;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dump {
    documents: Vec<Doc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Doc {
    document_id: String,
    #[serde(default)]
    items: Vec<Item>,
    #[serde(default)]
    pages: Vec<Page>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Item {
    id: i64,
    quote: String,
    /// `grounding_status` as the row stands today — the baseline to beat.
    status: Option<String>,
    /// `grounded_page` as the row stands today.
    page: Option<i64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Page {
    page: i64,
    text: String,
}

fn tier_name(m: &CanonicalMatchType) -> &'static str {
    match m {
        CanonicalMatchType::Exact => "exact",
        CanonicalMatchType::Normalized => "normalized",
        CanonicalMatchType::NormalizedWithoutNumerals => "no-numerals",
        CanonicalMatchType::NormalizedWithGap { .. } => "with-gap",
        CanonicalMatchType::EmptyAfterStripping => "quote is only numerals",
        CanonicalMatchType::NotFound => "NOT FOUND",
    }
}

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: verify_gap_matcher <dump.json>");
        return ExitCode::FAILURE;
    };

    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) => {
            eprintln!("could not read {path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let dump: Dump = match serde_json::from_str(&raw) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{path} is not the expected shape: {e}");
            return ExitCode::FAILURE;
        }
    };

    // The thresholds come from the caller or from `GapPolicy::default()` — the
    // same one definition production starts from. They are NEVER read from the
    // environment here: a proof whose numbers depend on the shell it was
    // launched from proves nothing repeatable, and every run prints the policy
    // it used so the report can be checked against it.
    let args: Vec<String> = std::env::args().skip(2).collect();
    let policy = match args.len() {
        0 => GapPolicy::default(),
        3 => match (args[0].parse(), args[1].parse(), args[2].parse()) {
            (Ok(g), Ok(f), Ok(w)) => GapPolicy {
                max_gap_chars: g,
                min_half_fraction: f,
                min_half_words: w,
            },
            _ => {
                eprintln!("policy override could not be parsed: {args:?}");
                return ExitCode::FAILURE;
            }
        },
        _ => {
            eprintln!(
                "policy override needs all three: <max_gap_chars> <min_half_fraction> <min_half_words>"
            );
            return ExitCode::FAILURE;
        }
    };
    println!("policy: {policy:?}");

    for doc in &dump.documents {
        let pages: Vec<(u32, String)> = doc
            .pages
            .iter()
            .map(|p| (p.page as u32, p.text.clone()))
            .collect();

        println!("\n=== {} — {} pages ===", doc.document_id, pages.len());

        let mut tiers: BTreeMap<&str, usize> = BTreeMap::new();
        let mut agreed = 0usize;
        let mut disagreed = 0usize;
        let mut recovered = 0usize;
        // The regression guard: an item that already grounded must ground the
        // same way, on the same page, with the second chance switched on. By
        // construction it cannot change — tiers 5 and 6 run only after every
        // contiguous tier has failed — and this counts it rather than trusting
        // the construction.
        let mut disturbed = 0usize;

        for item in &doc.items {
            // The baseline: what the four contiguous tiers say on their own.
            let before = find_in_canonical_text_with_policy(&item.quote, &pages, None);
            let after = find_in_canonical_text_with_policy(&item.quote, &pages, Some(policy));
            *tiers.entry(tier_name(&after.match_type)).or_default() += 1;

            let stored = item.status.as_deref().unwrap_or("(none)");
            let newly_grounded = !is_grounded(&before.match_type) && is_grounded(&after.match_type);
            if newly_grounded {
                recovered += 1;
            } else if is_grounded(&before.match_type)
                && (before.match_type != after.match_type
                    || before.page_number != after.page_number)
            {
                disturbed += 1;
                println!(
                    "  item {:>6}  DISTURBED: was [{}] p{:?}, now [{}] p{:?}",
                    item.id,
                    tier_name(&before.match_type),
                    before.page_number,
                    tier_name(&after.match_type),
                    after.page_number
                );
            }

            // Only the rows that say something: a recovery, or a disagreement
            // with a page a human typed in.
            let human_page = if stored == "manual" { item.page } else { None };
            if let Some(hp) = human_page {
                match after.page_number.map(i64::from) {
                    Some(p) if p == hp => agreed += 1,
                    Some(p) => {
                        disagreed += 1;
                        println!(
                            "  item {:>6}  human p{hp}  matcher p{p}  [{}]  DISAGREES",
                            item.id,
                            tier_name(&after.match_type)
                        );
                    }
                    None => {
                        disagreed += 1;
                        println!(
                            "  item {:>6}  human p{hp}  matcher (none)  DISAGREES",
                            item.id
                        );
                    }
                }
                if after.page_number.map(i64::from) == Some(hp) {
                    let gap = match after.match_type {
                        CanonicalMatchType::NormalizedWithGap { gap_chars } => {
                            format!(" gap={gap_chars}")
                        }
                        _ => String::new(),
                    };
                    println!(
                        "  item {:>6}  human p{hp}  matcher p{hp}  [{}]{gap}  agrees",
                        item.id,
                        tier_name(&after.match_type)
                    );
                }
            } else if newly_grounded {
                let gap = match after.match_type {
                    CanonicalMatchType::NormalizedWithGap { gap_chars } => {
                        format!(" gap={gap_chars}")
                    }
                    _ => String::new(),
                };
                println!(
                    "  item {:>6}  was {stored} (no page)  ->  p{}  [{}]{gap}",
                    item.id,
                    after.page_number.unwrap_or(0),
                    tier_name(&after.match_type)
                );
            }
        }

        println!("  ── {} items with a quote", doc.items.len());
        for (tier, n) in &tiers {
            println!("     {tier:<12} {n}");
        }
        println!("     newly grounded by the second chance: {recovered}");
        println!("     already-grounded items disturbed:    {disturbed}");
        if agreed + disagreed > 0 {
            println!("     vs human-entered pages: {agreed} agree, {disagreed} disagree");
        }
    }

    ExitCode::SUCCESS
}
