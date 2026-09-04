//! Turning one ranked gather into a Markdown page a human reads.
//!
//! ## Who this is for
//!
//! Roman, drafting seed questions from it the night before a meeting. Not a
//! machine, and not a diff. So: the query first, so he can see what was
//! searched for; the probes, so he can see what the search actually asked; then
//! the cards with their quotes IN FULL, because a truncated quote is a card he
//! has to go and look up somewhere else.

use std::collections::BTreeMap;

use colossus_legal_backend::services::gather_fusion::CardPlacement;
use colossus_legal_backend::services::gather_search::RankedGather;

use crate::cards::{card_label, Card};

// STRUCTURAL: the acceptance bar's own depth. The list shows the sixty the bar
// is defined over, not a number chosen for the page — a shorter page would hide
// cards the bar counts and a longer one would imply the bar reaches further.
const RANKED_SHOWN: usize = 60;

/// What the basis MEANT for this gather, beside the token itself.
///
/// The token says what the query was built from; this says what that produced,
/// which is the part a reader drafting questions actually needs. A scenario
/// whose subject has no evidence of its own reads very differently from one
/// with a full pool, and the token alone cannot tell them apart.
fn basis_in_plain_words(gather: &RankedGather) -> String {
    let ranked = gather
        .cards
        .iter()
        .filter(|c| c.placement == CardPlacement::Ranked)
        .count();
    match (ranked, gather.subject_only_pool.len()) {
        (0, 0) => "nothing found, and the subject has no evidence of its own".to_string(),
        (0, pool) => format!("neither read returned anything; {pool} cards carried by the tail"),
        (n, 0) => format!("{n} cards found; the subject has no evidence of its own"),
        (n, pool) => format!("{n} cards found, against a subject-only pool of {pool}"),
    }
}

/// The thresholds that produced this file.
///
/// Echoed into the page because two runs of this bin at different shares
/// produce files that look alike and are not comparable, and an auditor reading
/// one months later has no other way to tell which is which.
pub struct Settings {
    pub probe_max_share: f64,
    pub probe_floor: usize,
}

/// The scenario's own words, as the page opens with them.
///
/// ## Rust Learning: grouping parameters that travel together
///
/// `page` and `header` each took eight arguments, five of them `&str` — and a
/// call site that swapped `theme` and `subject`, or `query` and `basis`, would
/// compile and produce a page that reads as nonsense. Grouping the ones that
/// belong to the scenario makes those swaps unwritable and brings both
/// functions back inside the argument limit clippy enforces for this reason.
pub struct Scenario<'a> {
    pub code: &'a str,
    pub theme: &'a str,
    pub subject: &'a str,
    /// The composed query, verbatim.
    pub query: &'a str,
    /// The basis token, as `QueryBasis::as_str` spells it.
    pub basis: &'a str,
}

/// Render one scenario's page.
pub fn page(
    scenario: &Scenario<'_>,
    gather: &RankedGather,
    cards: &BTreeMap<String, Card>,
    settings: &Settings,
) -> String {
    let mut out = String::new();
    header(&mut out, scenario, gather, settings);
    probes(&mut out, gather);
    ranked(&mut out, gather, cards);
    tail(&mut out, gather, cards);
    out
}

fn header(out: &mut String, scenario: &Scenario<'_>, gather: &RankedGather, settings: &Settings) {
    let Scenario {
        code,
        theme,
        subject,
        query,
        basis,
    } = scenario;
    let ranked_count = gather
        .cards
        .iter()
        .filter(|c| c.placement == CardPlacement::Ranked)
        .count();
    out.push_str(&format!("# {code} — ranked evidence gather\n\n"));
    out.push_str(&format!("**Theme.** {theme}\n\n"));
    out.push_str(&format!("**Subject.** `{subject}`\n\n"));
    out.push_str(&format!(
        "**Query basis.** {basis} — {}\n\n",
        basis_in_plain_words(gather)
    ));
    out.push_str("## What was searched for\n\n```\n");
    out.push_str(query.trim());
    out.push_str("\n```\n\n");
    out.push_str("## The list\n\n");
    out.push_str(&format!(
        "- **{ranked_count}** cards reached by the two reads, ranked.\n\
         - **{}** cards in today's subject-only pool.\n\
         - **{}** of those the reads did not reach — carried by the conservation tail below.\n\
         - Read depth {} per read; party filter `{}`, admitting {} cards.\n\n",
        gather.subject_only_pool.len(),
        gather.unreached_by_reads,
        gather.read_depth,
        gather.filter_mode,
        gather.admitted.len(),
    ));
    out.push_str(&format!(
        "- Probe share {:.4}, probe floor {} — the thresholds this file was produced at.\n\n",
        settings.probe_max_share, settings.probe_floor
    ));
}

fn probes(out: &mut String, gather: &RankedGather) {
    out.push_str("## The probes\n\n");
    out.push_str(&format!(
        "**Kept ({}).** {}\n\n",
        gather.probes.len(),
        if gather.probes.is_empty() {
            "none — the query yielded no figures or names".to_string()
        } else {
            gather.probes.join(" · ")
        }
    ));
    if !gather.probes_dropped.is_empty() {
        let mut dropped = gather.probes_dropped.clone();
        dropped.sort_by(|a, b| b.matches.cmp(&a.matches).then(a.probe.cmp(&b.probe)));
        out.push_str(&format!(
            "**Dropped ({}), for matching too much of the pool to distinguish anything.** {}\n\n",
            dropped.len(),
            dropped
                .iter()
                .map(|c| format!("{} ({})", c.probe, c.matches))
                .collect::<Vec<_>>()
                .join(" · ")
        ));
    }
    if !gather.collapsed.is_empty() {
        out.push_str(
            "**Collapsed** — these matched exactly the same cards, so they vote once:\n\n",
        );
        for group in &gather.collapsed {
            out.push_str(&format!(
                "- `{}` absorbed {}\n",
                group.representative,
                group
                    .collapsed
                    .iter()
                    .map(|p| format!("`{p}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        out.push('\n');
    }
}

fn ranked(out: &mut String, gather: &RankedGather, cards: &BTreeMap<String, Card>) {
    out.push_str(&format!("## The top {RANKED_SHOWN}\n\n"));
    let shown: Vec<_> = gather
        .cards
        .iter()
        .filter(|c| c.placement == CardPlacement::Ranked)
        .take(RANKED_SHOWN)
        .collect();

    if shown.is_empty() {
        out.push_str("_Neither read returned anything. Everything below is the tail._\n\n");
        return;
    }
    for fused in shown {
        let Some(card) = cards.get(&fused.evidence_id) else {
            // The mirror did not have it. Named rather than skipped: a card in
            // the ranking that cannot be shown is a mirror gap, not a blank.
            out.push_str(&format!(
                "### {}. `{}` — NOT IN THE MIRROR\n\n",
                fused.rank.unwrap_or(0),
                fused.evidence_id
            ));
            continue;
        };
        out.push_str(&format!(
            "### {}. {}\n\n",
            fused.rank.unwrap_or(0),
            card_label(card)
        ));
        out.push_str(&format!(
            "**{}**\n\n",
            if card.title.is_empty() {
                "(untitled)"
            } else {
                &card.title
            }
        ));
        out.push_str("> ");
        out.push_str(&card.quote.replace('\n', "\n> "));
        out.push_str("\n\n");
        if !card.significance.is_empty() {
            out.push_str(&format!("*Why it matters.* {}\n\n", card.significance));
        }
        out.push_str(&format!(
            "`{}` · page {} · about: {} · vector {} · lexical {}\n\n---\n\n",
            card.document_id,
            card.page.map_or_else(|| "—".to_string(), |p| p.to_string()),
            if card.about.is_empty() {
                "—".to_string()
            } else {
                card.about.join(", ")
            },
            fused
                .vector_rank
                .map_or_else(|| "—".to_string(), |r| r.to_string()),
            fused
                .lexical_rank
                .map_or_else(|| "—".to_string(), |r| r.to_string()),
        ));
    }
}

fn tail(out: &mut String, gather: &RankedGather, cards: &BTreeMap<String, Card>) {
    let tail: Vec<_> = gather
        .cards
        .iter()
        .filter(|c| c.placement == CardPlacement::ConservationTail)
        .collect();
    out.push_str(&format!(
        "## In today's subject-only pool, not reached by either read ({})\n\n",
        tail.len()
    ));
    if tail.is_empty() {
        out.push_str("_None — the reads reached every card in the pool._\n");
        return;
    }
    out.push_str(
        "These are visible in the scenario today. The ranked search did not surface them, and \
         they are listed so nothing that was visible yesterday is invisible tonight.\n\n",
    );
    for fused in tail {
        let label = cards.get(&fused.evidence_id).map_or_else(
            || "(not in the mirror)".to_string(),
            |c| {
                if c.title.is_empty() {
                    "(untitled)".to_string()
                } else {
                    c.title.clone()
                }
            },
        );
        out.push_str(&format!("- `{}` — {label}\n", fused.evidence_id));
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
