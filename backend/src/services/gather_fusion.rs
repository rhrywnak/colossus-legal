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

use std::collections::BTreeMap;

// STRUCTURAL: the k of reciprocal-rank fusion, fixed by the method rather than
// by this deployment. 60 is the value from the original RRF paper (Cormack,
// Clarke & Buettcher 2009) and the one every comparable implementation uses;
// changing it would not tune this project, it would make our fused ranks
// incomparable to every published result. Passed explicitly to `fuse` so the
// tests can vary it without a config surface nobody wants.
pub const RRF_K: f64 = 60.0;

/// One card in the fused list.
#[derive(Debug, Clone, PartialEq)]
pub struct FusedCard {
    pub evidence_id: String,
    /// 1-based position in the fused list.
    pub rank: usize,
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
    // BTreeMap, not HashMap: the accumulation order feeds the sort's input, and
    // a HashMap would make it vary per run even with the tiebreak in place.
    let mut cards: BTreeMap<&str, FusedCard> = BTreeMap::new();

    for (index, id) in vector_ranked.iter().enumerate() {
        let rank = index + 1;
        let card = cards.entry(id.as_str()).or_insert_with(|| blank(id));
        // First occurrence wins: a read that returned the same id twice is a
        // defect in that read, and taking the better rank would hide it while
        // silently double-counting the card's score.
        if card.vector_rank.is_none() {
            card.vector_rank = Some(rank);
            card.fused_score += 1.0 / (k + rank as f64);
        }
    }
    for (index, id) in lexical_ranked.iter().enumerate() {
        let rank = index + 1;
        let card = cards.entry(id.as_str()).or_insert_with(|| blank(id));
        if card.lexical_rank.is_none() {
            card.lexical_rank = Some(rank);
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
        card.rank = index + 1;
    }
    fused
}

/// A card seen in neither read yet.
fn blank(evidence_id: &str) -> FusedCard {
    FusedCard {
        evidence_id: evidence_id.to_string(),
        rank: 0,
        fused_score: 0.0,
        vector_rank: None,
        lexical_rank: None,
    }
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
