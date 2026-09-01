//! Reciprocal-rank fusion: two ranked lists in, one ranked list out.
//!
//! **Pure. No database, no search, no embedding.** Everything it needs is in
//! its arguments, which is what lets the ties, the one-sided cards and the
//! empty-side cases be pinned without a corpus.
//!
//! ## Why reciprocal rank rather than score
//!
//! The two reads produce numbers that cannot be compared. Qdrant returns a
//! cosine similarity in roughly `[0, 1]`; Postgres returns a `ts_rank` whose
//! magnitude depends on the document, the weights and the query's term count.
//! Normalising them against each other would need a calibration nobody has
//! measured, and would move whenever the corpus grew.
//!
//! Reciprocal rank throws the magnitudes away and keeps only the ORDER, which
//! is the part both reads agree on the meaning of. A card at rank 1 contributes
//! `1/(k+1)` whether it got there with a cosine of 0.91 or a `ts_rank` of 0.07.
//!
//! ## Domain note: k = 60 and what it buys
//!
//! `k` flattens the top of the curve. At k=60 the gap between rank 1 and rank 2
//! is about 1.6% of the score, so a card both reads rank highly beats a card
//! either read ranks first alone. That is the behaviour this cascade wants:
//! **agreement between the two reads is the signal**, because the lexical read
//! knows about `$50,000` and the vector read knows about "the money he
//! deposited", and a card both find is more likely the one a human meant.

use std::collections::{BTreeMap, BTreeSet};

// STRUCTURAL: the k of reciprocal-rank fusion, fixed by the method rather than
// by this deployment. 60 is the value from the original RRF paper (Cormack,
// Clarke & Buettcher 2009) and the one every comparable implementation uses;
// changing it would not tune this project, it would make our fused ranks
// incomparable to every published result. Passed explicitly to `fuse` so the
// tests can vary it without a config surface nobody wants.
pub const RRF_K: f64 = 60.0;

/// Whether a card was RETRIEVED or is only present because conservation
/// requires it.
///
/// ## ⚑ The distinction a reader must be able to make
///
/// "Ranked 120th" and "neither read reached this, but it was in yesterday's
/// pool" are entirely different statements about a card, and a list that
/// renders them identically would be lying by omission. L2c's DTO carries this
/// as a field so the page can separate them — a rule below the last ranked card
/// and a different treatment for what follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardPlacement {
    /// At least one read returned it; [`FusedCard::fused_score`] is meaningful.
    Ranked,
    /// Neither read reached it. It is here only because it is in the
    /// subject-only pool and conservation says nothing visible yesterday may be
    /// invisible today. Score is zero and both ranks are `None`.
    ConservationTail,
}

/// One card in the fused list.
#[derive(Debug, Clone, PartialEq)]
pub struct FusedCard {
    pub evidence_id: String,
    /// 1-based rank among the RETRIEVED cards, or `None` for a card the reads
    /// never returned.
    ///
    /// ## Rust Learning: `Option` instead of a field that means two things
    ///
    /// This was a bare `usize`, and a conservation-tail card was given its
    /// position in the list. That made `rank` mean "how well it scored" for one
    /// kind of card and "where it happens to sit" for another, with only
    /// [`Self::placement`] to tell a caller which — a precondition the compiler
    /// could not check and a reader had to remember. `None` carries "not
    /// ranked" in the type, so the ambiguity cannot be written.
    pub rank: Option<usize>,
    /// Whether this card was retrieved or appended.
    pub placement: CardPlacement,
    pub fused_score: f64,
    /// 1-based rank in the vector read, `None` if that read did not return it.
    pub vector_rank: Option<usize>,
    /// 1-based rank in the lexical read, `None` if that read did not return it.
    pub lexical_rank: Option<usize>,
}

/// Fuse two ranked id lists into one.
///
/// Both inputs are in rank order, best first. An id appearing in both
/// contributes from both; an id in one contributes from one, which is how a
/// card only the trigram index can find still reaches the list.
///
/// ## The order is total, so the output is reproducible
///
/// Ties are broken by `evidence_id` ascending. Without that, two cards with the
/// same score would come out in whatever order the map iterated, the gather
/// would differ between runs, and a measurement taken today could not be
/// compared with the same measurement taken tomorrow. `sort_by` is stable, but
/// stability alone does not help when the input order is itself arbitrary — the
/// tiebreak has to be on the data.
pub fn fuse(vector_ranked: &[String], lexical_ranked: &[String], k: f64) -> Vec<FusedCard> {
    let mut fused = fuse_many(&[vector_ranked, lexical_ranked], k);
    // The named ranks are filled HERE, not inside `fuse_many`, because they are
    // a property of THESE two lists being the vector read and the lexical read.
    // Setting them inside the general function — on the strength of it having
    // received exactly two lists — would mean its output shape depended on a
    // count the caller chose, and a future two-list caller would silently get
    // fields it never asked for.
    for card in &mut fused {
        card.vector_rank = position_of(vector_ranked, &card.evidence_id);
        card.lexical_rank = position_of(lexical_ranked, &card.evidence_id);
    }
    fused
}

/// 1-based position of an id in a ranked list, or `None` if absent.
fn position_of(list: &[String], evidence_id: &str) -> Option<usize> {
    list.iter().position(|id| id == evidence_id).map(|i| i + 1)
}

/// Fuse ANY number of ranked lists.
///
/// The trigram half runs one query per probe, so the number of lists to fuse is
/// however many probes the query yielded — not two. A card matching three
/// probes outranks one matching a single probe, which is the behaviour wanted:
/// probes are independent evidence about the same card, and agreement between
/// them means the same thing agreement between the two reads does.
///
/// Purely aggregative: it accumulates `fused_score` and never touches
/// `vector_rank` or `lexical_rank`, so its output shape does not depend on how
/// many lists it was given. [`fuse`] adds the named ranks afterwards, because
/// those mean something only when the two lists ARE the vector and lexical
/// reads.
pub fn fuse_many(lists: &[&[String]], k: f64) -> Vec<FusedCard> {
    // BTreeMap, not HashMap: the accumulation order feeds the sort's input, and
    // a HashMap would make it vary per run even with the tiebreak in place.
    let mut cards: BTreeMap<&str, FusedCard> = BTreeMap::new();

    for list in lists {
        // Which ids this list has already contributed. First occurrence wins:
        // a read that returned the same id twice is a defect in that read, and
        // taking the better rank would hide it while double-counting the score.
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for (index, id) in list.iter().enumerate() {
            if !seen.insert(id.as_str()) {
                continue;
            }
            let rank = index + 1;
            let card = cards.entry(id.as_str()).or_insert_with(|| blank(id));
            card.fused_score += 1.0 / (k + rank as f64);
        }
    }

    let mut fused: Vec<FusedCard> = cards.into_values().collect();
    fused.sort_by(|a, b| {
        b.fused_score
            .partial_cmp(&a.fused_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.evidence_id.cmp(&b.evidence_id))
    });
    for (index, card) in fused.iter_mut().enumerate() {
        card.rank = Some(index + 1);
    }
    fused
}

/// A card seen in neither read yet.
fn blank(evidence_id: &str) -> FusedCard {
    FusedCard {
        evidence_id: evidence_id.to_string(),
        rank: None,
        placement: CardPlacement::Ranked,
        fused_score: 0.0,
        vector_rank: None,
        lexical_rank: None,
    }
}

/// Append every baseline card the reads did not reach, in id order.
///
/// ## ⚑ Conservation by CONSTRUCTION, not by luck
///
/// The identity — every card visible yesterday is still visible today — cannot
/// be met by a relevance-bounded read at any depth: the two reads return the
/// most RELEVANT cards, while the subject-only pool is defined by MEMBERSHIP,
/// and no top-K of the first is guaranteed to contain the second. Measured
/// before this existed: 204 of S-9's 292 and 197 of S-11's 292 were absent.
///
/// So they are appended rather than retrieved. The ranked cards keep their
/// order and their scores; the unreached ones follow, in id order, marked
/// [`CardPlacement::ConservationTail`] so nothing can mistake them for results.
/// In a case file, "a card you could see yesterday is gone" outranks a tidy
/// list — which is the whole reason this is a guarantee and not a metric.
pub fn append_conservation_tail(mut fused: Vec<FusedCard>, baseline: &[String]) -> Vec<FusedCard> {
    let mut missing = conservation_gap(baseline, &fused);
    missing.sort();
    missing.dedup();

    for evidence_id in missing {
        fused.push(FusedCard {
            evidence_id,
            // No rank: neither read returned it, so there is no position among
            // the retrieved cards to give it.
            rank: None,
            placement: CardPlacement::ConservationTail,
            fused_score: 0.0,
            vector_rank: None,
            lexical_rank: None,
        });
    }
    fused
}

/// The ids in `baseline` that the fused list does not contain.
///
/// ## ⚑ The conservation identity
///
/// Every card in today's subject-only pool must still appear. The ranked gather
/// ADDS and RE-ORDERS; it never drops. A card a human could see yesterday and
/// cannot see today is a defect, not a ranking — and it is the one failure mode
/// that would make this whole cascade a net loss rather than a gain.
///
/// Returned as the missing ids rather than a bool so a violation names the
/// cards, which is the difference between "conservation failed" and something
/// an operator can go look at.
pub fn conservation_gap(baseline: &[String], fused: &[FusedCard]) -> Vec<String> {
    let present: std::collections::BTreeSet<&str> =
        fused.iter().map(|c| c.evidence_id.as_str()).collect();
    baseline
        .iter()
        .filter(|id| !present.contains(id.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
#[path = "gather_fusion_tests.rs"]
mod tests;
