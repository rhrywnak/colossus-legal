//! The three-tier matcher: which new node, if any, is this old node?
//!
//! ## Why there are three tiers and not one comparison
//!
//! Measured on `doc-transcript-post-appeal-restructure-02-28-2012-clean`
//! (2026-09-04): a re-extraction from corrected page text matched **0 of 14**
//! movable nodes, because the only test was byte-for-byte agreement on page +
//! quote + question and the corrected text changed every quote's line breaks.
//! Twenty curated rows went to the human queue for a reason that was purely
//! typographic.
//!
//! Loosening the one comparison to fix that would have loosened it for
//! everything, including the pairs that genuinely must not be matched. Tiers keep
//! the strength of the evidence visible instead: a tier-1 match is a fact, a
//! tier-2 match is a whitespace argument, and a tier-3 match is a measurement
//! with its score printed next to it. The human approving the proposal can see
//! which one they are being asked to trust.
//!
//! ## The order is a preference, not a fallback chain by accident
//!
//! Each tier runs only when the one above it found nothing at all. A node that
//! matches exactly is never also considered for a near match, so a strong match
//! can never be displaced by a weak one.

use std::collections::{BTreeMap, BTreeSet};

use crate::api::pipeline::evidence_key::normalize;

use super::normalize::{loose_normalize, similarity, word_coverage, NearMatchSettings};
use super::plan::{Match, MatchTier, MatchedNode, NewNode, Snapshot, SnapshotNode};

/// The key two nodes must share to be the same statement at tier 1.
///
/// `(page, normalized quote, normalized question)` — the same normalization the
/// stable-id arm uses, so the matcher and the id cannot disagree about what "the
/// same quote" means.
fn exact_key(page: Option<i64>, quote: &str, question: Option<&str>) -> String {
    format!(
        "{}\u{1f}{}\u{1f}{}",
        page.map(|p| p.to_string()).unwrap_or_default(),
        normalize(quote),
        question.map(normalize).unwrap_or_default()
    )
}

/// One new node with its loose forms computed once.
///
/// ## Rust Learning: the lifetime `'a` on a borrowed id
///
/// The id is borrowed from the caller's `&[NewNode]` rather than cloned, so this
/// struct cannot outlive that slice — which is exactly right, since it is a
/// working index built and dropped inside one `match_all` call. The two `String`
/// fields ARE owned, because they are new values (the loose forms) that exist
/// nowhere else.
struct LooseNode<'a> {
    id: &'a str,
    quote: String,
    question: String,
}

/// Everything the per-node decision needs, computed once for the whole document.
struct MatchContext<'a> {
    surviving: BTreeSet<&'a str>,
    new_by_exact_key: BTreeMap<String, Vec<String>>,
    old_exact_key_holders: BTreeMap<String, usize>,
    old_loose_key_holders: BTreeMap<(Option<i64>, String), usize>,
    new_by_page: BTreeMap<Option<i64>, Vec<LooseNode<'a>>>,
    settings: NearMatchSettings,
}

/// Match every node in the snapshot against the graph as it stands now.
pub(super) fn match_all(
    snapshot: &Snapshot,
    new_nodes: &[NewNode],
    settings: NearMatchSettings,
) -> Vec<MatchedNode> {
    let context = build_context(snapshot, new_nodes, settings);
    let mut matched: Vec<MatchedNode> = snapshot
        .nodes
        .iter()
        .map(|old| MatchedNode {
            outcome: decide(old, &context),
            old: old.clone(),
        })
        .collect();
    deconflict(&mut matched);
    matched
}

/// Build the indexes. Every loose form in the document is computed here, once.
fn build_context<'a>(
    snapshot: &Snapshot,
    new_nodes: &'a [NewNode],
    settings: NearMatchSettings,
) -> MatchContext<'a> {
    let mut context = MatchContext {
        surviving: new_nodes.iter().map(|n| n.id.as_str()).collect(),
        new_by_exact_key: BTreeMap::new(),
        old_exact_key_holders: BTreeMap::new(),
        old_loose_key_holders: BTreeMap::new(),
        new_by_page: BTreeMap::new(),
        settings,
    };

    for node in new_nodes {
        let key = exact_key(node.page, &node.verbatim_quote, node.question.as_deref());
        context
            .new_by_exact_key
            .entry(key)
            .or_default()
            .push(node.id.clone());
        context
            .new_by_page
            .entry(node.page)
            .or_default()
            .push(LooseNode {
                id: &node.id,
                quote: loose_normalize(&node.verbatim_quote),
                question: node
                    .question
                    .as_deref()
                    .map(loose_normalize)
                    .unwrap_or_default(),
            });
    }
    // Sorted so an ambiguous candidate list reads the same on every run.
    for ids in context.new_by_exact_key.values_mut() {
        ids.sort();
    }

    for node in &snapshot.nodes {
        let key = exact_key(node.page, &node.verbatim_quote, node.question.as_deref());
        *context.old_exact_key_holders.entry(key).or_insert(0) += 1;
        let loose = (node.page, loose_normalize(&node.verbatim_quote));
        *context.old_loose_key_holders.entry(loose).or_insert(0) += 1;
    }
    context
}

/// Decide one old node's outcome, strongest tier first.
fn decide(old: &SnapshotNode, context: &MatchContext<'_>) -> Match {
    if context.surviving.contains(old.id.as_str()) {
        return Match::Unchanged;
    }
    if let Some(outcome) = tier_one(old, context) {
        return outcome;
    }
    let loose_quote = loose_normalize(&old.verbatim_quote);
    let loose_question = old
        .question
        .as_deref()
        .map(loose_normalize)
        .unwrap_or_default();
    if let Some(outcome) = tier_two(old, &loose_quote, &loose_question, context) {
        return outcome;
    }
    tier_three(old, &loose_quote, &loose_question, context)
}

/// Tier 1 — the key agrees byte for byte after the id-arm normalization.
fn tier_one(old: &SnapshotNode, context: &MatchContext<'_>) -> Option<Match> {
    let key = exact_key(old.page, &old.verbatim_quote, old.question.as_deref());
    let candidates = context.new_by_exact_key.get(&key)?;
    let old_holders = context
        .old_exact_key_holders
        .get(&key)
        .copied()
        .unwrap_or(1);
    Some(one_or_ambiguous(
        candidates.clone(),
        old_holders,
        MatchTier::Exact,
        1.0,
    ))
}

/// Tier 2 — the loose forms agree exactly.
fn tier_two(
    old: &SnapshotNode,
    loose_quote: &str,
    loose_question: &str,
    context: &MatchContext<'_>,
) -> Option<Match> {
    let candidates: Vec<String> = page_nodes(old, context)
        .iter()
        .filter(|n| n.quote == loose_quote && questions_agree(loose_question, &n.question))
        .map(|n| n.id.to_string())
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let old_holders = context
        .old_loose_key_holders
        .get(&(old.page, loose_quote.to_string()))
        .copied()
        .unwrap_or(1);
    Some(one_or_ambiguous(
        candidates,
        old_holders,
        MatchTier::Normalized,
        1.0,
    ))
}

/// Tier 3 — both near-match measures clear their thresholds.
///
/// Domain note: the two measures answer different questions and BOTH must pass.
/// Similarity asks "is this the same text, give or take some drift?"; word
/// coverage asks "does the new quote still contain what the old one said?" A
/// re-extraction that keeps the statement and adds a clause passes the second and
/// strains the first; a re-extraction that keeps the shape and swaps the names
/// passes the first and fails the second. Requiring both is what keeps a near
/// match from becoming a guess.
fn tier_three(
    old: &SnapshotNode,
    loose_quote: &str,
    loose_question: &str,
    context: &MatchContext<'_>,
) -> Match {
    let mut hits: Vec<(String, f64)> = Vec::new();
    for node in page_nodes(old, context) {
        if !questions_agree(loose_question, &node.question) {
            continue;
        }
        let score = similarity(loose_quote, &node.quote);
        if score < context.settings.similarity {
            continue;
        }
        if word_coverage(loose_quote, &node.quote) < context.settings.word_coverage {
            continue;
        }
        hits.push((node.id.to_string(), score));
    }
    match hits.len() {
        0 => Match::Unmatched,
        1 => Match::Unambiguous {
            new_id: hits[0].0.clone(),
            tier: MatchTier::Near,
            score: hits[0].1,
        },
        _ => Match::Ambiguous {
            candidates: hits.into_iter().map(|(id, _)| id).collect(),
            tier: MatchTier::Near,
        },
    }
}

/// The new nodes on this old node's page. Every tier below tier 1 is page-local:
/// the same sentence on a different page is a different statement.
fn page_nodes<'a>(old: &SnapshotNode, context: &'a MatchContext<'_>) -> &'a [LooseNode<'a>] {
    context
        .new_by_page
        .get(&old.page)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// Whether two loose questions may belong to the same statement.
///
/// Compared only when BOTH are non-blank. 287 of 525 live Evidence nodes are
/// documentary and answer nobody, and a re-extraction that starts or stops
/// emitting a question for such a node has not changed what the document says.
/// Two DIFFERENT questions, though, are two different statements — that is the
/// measured reason `question` is in the stable key at all.
fn questions_agree(left: &str, right: &str) -> bool {
    left.is_empty() || right.is_empty() || left == right
}

/// One candidate and one claimant is a match; anything else is a decision.
fn one_or_ambiguous(
    candidates: Vec<String>,
    old_holders: usize,
    tier: MatchTier,
    score: f64,
) -> Match {
    match candidates.as_slice() {
        [only] if old_holders == 1 => Match::Unambiguous {
            new_id: only.clone(),
            tier,
            score,
        },
        _ => Match::Ambiguous { candidates, tier },
    }
}

/// Demote any match that would move two old nodes' rows onto one new node.
///
/// The tier-1 and tier-2 twin guards catch this within a tier, but tier 3 can
/// reach across: two old nodes on one page can each near-match the same new node,
/// and an old node can near-match a new node that is itself another old node's
/// surviving id. Either would silently merge two statements' curated rows.
/// `apply` would refuse the resulting proposal outright with a duplicate-id
/// error; catching it here turns that dead end into a queue entry a human can
/// actually decide.
fn deconflict(nodes: &mut [MatchedNode]) {
    // An old node that kept its id is already the owner of that node, so it
    // counts as the first claim on it. `Unchanged` is the ONLY outcome that
    // establishes such an owner: the other new nodes on the page are unclaimed
    // until something matches them.
    let mut claims: BTreeMap<String, usize> = nodes
        .iter()
        .filter(|n| n.outcome == Match::Unchanged)
        .map(|n| (n.old.id.clone(), 1usize))
        .collect();
    for node in nodes.iter() {
        if let Match::Unambiguous { new_id, .. } = &node.outcome {
            *claims.entry(new_id.clone()).or_insert(0) += 1;
        }
    }

    for node in nodes.iter_mut() {
        let demoted = match &node.outcome {
            Match::Unambiguous { new_id, tier, .. }
                if claims.get(new_id).copied().unwrap_or(0) > 1 =>
            {
                Some(Match::Ambiguous {
                    candidates: vec![new_id.clone()],
                    tier: *tier,
                })
            }
            _ => None,
        };
        if let Some(outcome) = demoted {
            node.outcome = outcome;
        }
    }
}
