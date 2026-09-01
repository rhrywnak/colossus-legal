//! The ranked gather: two reads, one fused list, and the conservation check.
//!
//! ## What this replaces
//!
//! Today a scenario's gather is one graph read — every Evidence node filed
//! ABOUT the subject — with no query and no ranking. That is why S-9 and S-11,
//! two scenarios about different things that happen to name the same person,
//! receive byte-identical pools of 292 cards in arbitrary order.
//!
//! ## The shape
//!
//! ```text
//!   composed query (L2a)
//!        │
//!        ├── vector  : Qdrant, bounded by the allowed id set      → ranked ids
//!        └── lexical : evidence_search, full text ⊕ trigram       → ranked ids
//!                          │
//!                     RRF (k=60)  ← the two lexical halves first
//!                          │
//!                     RRF (k=60)  ← then lexical against vector
//!                          │
//!                    one ranked list + the conservation check
//! ```
//!
//! ## ⚑ Why the two lexical halves are fused BEFORE the vector read joins
//!
//! Full text and trigram answer different questions about the same store, and
//! neither is a whole retrieval on its own: full text cannot tell `$50,000`
//! from `$15,000` (L1b measured 16 real false positives on that one figure),
//! and trigram has no idea what a word means. Fusing them first makes "the
//! lexical read" one opinion, so the final fusion weighs **one lexical vote
//! against one vector vote** rather than letting the lexical side outvote the
//! vector side two-to-one purely because it happens to be implemented as two
//! queries. The nesting is the weighting, and it is deliberate.
//!
//! ## This module does not embed
//!
//! `TextEmbedding` is not `Send`, so embedding has to happen inside a
//! `spawn_blocking` the caller owns. The query VECTOR arrives as an argument
//! and [`crate::services::gather_vector::query_text`] supplies the
//! `search_query:` prefix, so the asymmetric-prefix pairing stays in one tested
//! place rather than being retyped at each call site.

use sqlx::PgPool;

use crate::domain::gather_filter::GatherSubjectFilter;
use crate::repositories::evidence_search_repository::lexical::{
    lexical_search, party_membership, LexicalReadError,
};
use crate::services::gather_fusion::{
    append_conservation_tail, conservation_gap, fuse, fuse_many, CardPlacement, FusedCard, RRF_K,
};
use crate::services::gather_probes::probes_of;
use crate::services::gather_vector::vector_search;
use crate::services::qdrant_service::QdrantError;

/// One ranked gather, with everything needed to explain it.
///
/// The counts are not decoration. When a gather comes back thin the question is
/// always "which stage lost the card?", and that is answerable only if the size
/// of each read is carried out beside the result.
#[derive(Debug, Clone)]
pub struct RankedGather {
    /// The fused list, best first.
    pub cards: Vec<FusedCard>,
    /// Today's pool: every card filed ABOUT the subject alone. The baseline the
    /// conservation identity is measured against.
    pub subject_only_pool: Vec<String>,
    /// Cards in [`Self::subject_only_pool`] that the returned list does NOT
    /// contain. **Empty by construction** since the conservation tail was
    /// added — non-empty means the append itself is broken.
    pub conservation_gap: Vec<String>,
    /// How many of today's pool NEITHER read reached, measured before the tail
    /// was appended.
    ///
    /// This is the number the tail exists for, and the only place it survives:
    /// after the append the gap is zero, so without this a reader could not
    /// tell a gather where the reads found everything from one where they found
    /// almost nothing and the tail carried the pool.
    pub unreached_by_reads: usize,
    /// The mode in force, so a thin result can be attributed to it.
    pub filter_mode: GatherSubjectFilter,
    /// Every id the party filter admitted, before either read ran.
    ///
    /// Kept as the ID SET, not a count. When a card is missing the first
    /// question is which stage lost it, and only this answers the first branch:
    /// a card absent from here was never admitted (a FILTER problem), while a
    /// card present here and absent from [`Self::cards`] was admitted and not
    /// retrieved (a READ problem). A count cannot tell them apart, and the two
    /// send an operator to different places.
    pub admitted: Vec<String>,
    /// How many ids the vector read returned.
    pub vector_hits: usize,
    /// How many the full-text half returned.
    pub full_text_hits: usize,
    /// How many rows the trigram half returned, summed across every probe.
    pub trigram_hits: usize,
    /// How many probes returned at least one row.
    ///
    /// Distinct from `probes.len()`: "we searched for nine things" and "three of
    /// them were found" are different facts, and a trigram half that is finding
    /// nothing looks exactly like one that had nothing to look for unless both
    /// are reported.
    pub trigram_lists: usize,
    /// Each probe that matched, with how many rows it matched.
    ///
    /// The row count alone cannot tell a useful probe from a useless one:
    /// measured on S-11, `Court` matched 534 of 1030 admitted cards while
    /// `$50,000` matched 73. Without the pairing, an operator looking at a
    /// badly-ranked gather cannot find the probe that drowned it.
    pub probe_hits: Vec<(String, usize)>,
    /// The probes the trigram half searched for.
    ///
    /// Carried out because it is the most reviewable artefact of the whole
    /// lexical half: a human can look at this list and see at once whether the
    /// extractor found the figures and names the scenario turns on. No ranking
    /// number would tell them that.
    pub probes: Vec<String>,
    /// How deep each read went, from the `gather_read_depth` settings row. A
    /// thin result cannot be judged without it: 40 cards off a depth of 200 and
    /// 40 off a depth of 40 are different findings.
    pub read_depth: usize,
}

impl RankedGather {
    /// The retrieved cards' ids, best first — the conservation tail excluded.
    ///
    /// ## ⚑ Why this is a method and not a filter at each call site
    ///
    /// The acceptance bars measure RETRIEVAL. A conservation-tail card was not
    /// retrieved; it is present because it is in the subject-only pool, and
    /// letting one satisfy a bar would report a ranking as working when it had
    /// found nothing. That filter was written once inside the measurement
    /// harness, where no fast test could reach it — so removing it would have
    /// left every unit test green. It lives here now, and is tested.
    pub fn retrieved_ids(&self) -> Vec<String> {
        self.cards
            .iter()
            .filter(|card| card.placement == CardPlacement::Ranked)
            .map(|card| card.evidence_id.clone())
            .collect()
    }

    /// Which stage lost a card — the question every thin gather raises.
    ///
    /// Answerable only because [`Self::admitted`] keeps the id set rather than
    /// a count. Returns `None` when the card is present, so a caller can ask
    /// about any id and get an answer rather than having to know first.
    pub fn why_missing(&self, evidence_id: &str) -> Option<MissingStage> {
        if self.cards.iter().any(|c| c.evidence_id == evidence_id) {
            return None;
        }
        if self.admitted.iter().any(|id| id == evidence_id) {
            Some(MissingStage::NotRetrieved)
        } else {
            Some(MissingStage::NotAdmitted)
        }
    }
}

/// Why a card is not in a ranked gather.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingStage {
    /// The party filter never let it through — a FILTER problem. Widening the
    /// mode, or the scenario's allegation links, is what would reach it.
    NotAdmitted,
    /// It was admitted and neither read returned it within the depth — a READ
    /// problem. A deeper read, or a better query, is what would reach it.
    NotRetrieved,
}

/// Errors a ranked gather can raise, naming the stage that raised them.
///
/// Split by STAGE rather than by store: "the lexical read failed" and "the
/// vector read failed" send an operator to different machines, and that is the
/// distinction worth carrying.
#[derive(Debug, thiserror::Error)]
pub enum GatherSearchError {
    #[error("the ranked gather's lexical stage failed: {0}")]
    Lexical(#[from] LexicalReadError),

    #[error("the ranked gather's vector stage failed: {0}")]
    Vector(#[from] QdrantError),
}

/// Run one ranked gather.
///
/// `query` is the composed text from L2a — unprefixed, because the lexical read
/// wants the words and only the vector side wants the model's prefix.
/// `query_vector` is that same text, prefixed and embedded by the caller.
///
/// # Errors
/// Returns [`GatherSearchError`] naming which stage failed.
pub async fn ranked_gather(
    pool: &PgPool,
    client: &reqwest::Client,
    qdrant_url: &str,
    input: GatherInput<'_>,
) -> Result<RankedGather, GatherSearchError> {
    let parties = input
        .filter_mode
        .parties(input.subject, input.reachable_parties);
    let parties_slice = parties.as_deref();

    // The filter is resolved ONCE, in Postgres, and the id set it yields bounds
    // both reads. Qdrant's payload carries node properties and `ABOUT` is an
    // edge, so there is no party field to filter on there — and doing it this
    // way means both reads see exactly the same universe, which is what makes
    // their ranks comparable and the conservation identity checkable.
    let admitted = party_membership(pool, parties_slice).await?;
    let subject_only_pool = party_membership(pool, Some(&[input.subject])).await?;

    let vector_ranked = vector_search(
        client,
        qdrant_url,
        input.query_vector,
        Some(&admitted),
        input.read_depth,
    )
    .await?;

    // The probes the trigram half matches, extracted from the composed query.
    // Derived here rather than passed in so a caller cannot hand the two halves
    // text that disagrees — the full-text half reads the query, the trigram
    // half reads what this pulled out of that same query.
    let probes = probes_of(input.query);

    let lexical = lexical_search(
        pool,
        input.query,
        &probes,
        parties_slice,
        depth_as_limit(input.read_depth),
    )
    .await?;

    let lexical_ranked = fuse_lexical(&lexical.full_text, &lexical.trigram);
    let trigram_hits: usize = lexical.trigram.iter().map(|(_, hits)| hits.len()).sum();
    let probe_hits: Vec<(String, usize)> = lexical
        .trigram
        .iter()
        .map(|(probe, hits)| (probe.clone(), hits.len()))
        .collect();
    let retrieved = fuse(&vector_ranked, &lexical_ranked, RRF_K);

    // How many of today's pool the reads actually reached, measured BEFORE the
    // tail is appended. After the append the gap is zero by construction, so
    // this is the only moment the number exists — and it is the number that
    // says how much work the tail is doing.
    let unreached = conservation_gap(&subject_only_pool, &retrieved).len();
    let cards = append_conservation_tail(retrieved, &subject_only_pool);

    // Now an assertion that can hold, and holds: the tail was just appended
    // from this very list, so a non-empty gap here means the append is broken.
    let gap = conservation_gap(&subject_only_pool, &cards);

    if !gap.is_empty() {
        // Rule 1: unreachable by construction now — the tail is appended from
        // the same baseline this compares against — so reaching here means
        // `append_conservation_tail` is broken, which is worse than the
        // original defect and must be just as loud.
        tracing::error!(
            missing = gap.len(),
            ids_excerpt = %id_excerpt(&gap),
            filter = %input.filter_mode,
            "CONSERVATION VIOLATION: append_conservation_tail returned a list still \
             missing cards from the subject-only pool. Since the tail is appended from \
             the same baseline this compares against, this is a DEFECT IN THAT FUNCTION \
             — not a ranking result and not a data problem. The gather is serving a \
             short pool; examine append_conservation_tail before trusting any list."
        );
    }

    // Rule 1: the two extreme gathers — the reads found everything, and the
    // reads found almost nothing and the tail carried the pool — are entirely
    // different states and were indistinguishable from the logs. The struct
    // always carried the numbers; nothing emitted them.
    tracing::info!(
        filter = %input.filter_mode,
        admitted = admitted.len(),
        read_depth = input.read_depth,
        vector_hits = vector_ranked.len(),
        full_text_hits = lexical.full_text.len(),
        trigram_hits,
        trigram_lists = lexical.trigram.len(),
        probe_count = probes.len(),
        probes = ?probes,
        probe_hits = ?probe_hits,
        subject_only_pool = subject_only_pool.len(),
        unreached_by_reads = unreached,
        ranked = cards.len() - unreached,
        total = cards.len(),
        "ranked gather complete"
    );

    Ok(RankedGather {
        vector_hits: vector_ranked.len(),
        full_text_hits: lexical.full_text.len(),
        trigram_hits,
        trigram_lists: lexical.trigram.len(),
        probe_hits,
        probes,
        read_depth: input.read_depth,
        unreached_by_reads: unreached,
        cards,
        admitted,
        subject_only_pool,
        conservation_gap: gap,
        filter_mode: input.filter_mode,
    })
}

/// What one gather needs, gathered into a struct.
///
/// ## Rust Learning: a parameter struct instead of eight arguments
///
/// `ranked_gather` would otherwise take eight parameters, four of them `&str`,
/// and any two of those could be swapped at a call site without the compiler
/// noticing — `subject` and `query` are both strings. Naming them at the call
/// site makes the mistake unwritable, and keeps the function under Rule 18.
pub struct GatherInput<'a> {
    /// The composed query text, unprefixed.
    pub query: &'a str,
    /// That text, prefixed with `search_query:` and embedded.
    pub query_vector: &'a [f32],
    /// The scenario's subject party id.
    pub subject: &'a str,
    /// The subject plus every party the linked allegations name (L2a).
    pub reachable_parties: &'a [String],
    pub filter_mode: GatherSubjectFilter,
    /// How deep each read goes, from the `gather_read_depth` settings row.
    /// Passed in rather than read here so this function stays testable without
    /// a settings store, and so both reads provably get the SAME number.
    pub read_depth: usize,
}

// STRUCTURAL: how many ids the conservation log names before it stops. A
// log-line format bound, the same family as
// `anthropic_stream::MALFORMED_PREVIEW_CHARS`. Measured on the real corpus the
// gap runs to ~200 ids, roughly 7,800 characters in one field — past the point
// where collectors truncate, which would lose the very ids the line exists to
// name. The FULL list is on `RankedGather::conservation_gap`; the log is for
// alerting, the struct is for forensics.
const CONSERVATION_LOG_IDS: usize = 10;

/// The first few ids, with a count of the rest.
fn id_excerpt(ids: &[String]) -> String {
    let head = ids
        .iter()
        .take(CONSERVATION_LOG_IDS)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    match ids.len().checked_sub(CONSERVATION_LOG_IDS) {
        Some(rest) if rest > 0 => format!("{head}, +{rest} more"),
        _ => head,
    }
}

/// The read depth as the `LIMIT` the lexical statements take.
///
/// ## Rust Learning: `try_from` instead of `as`, on a bound that comes from data
///
/// `read_depth` is a `usize` read from a settings ROW, so its value is data a
/// human can edit, not a literal the compiler saw. Written `depth as i64` a
/// stored value above `i64::MAX` would wrap to a negative LIMIT and Postgres
/// would refuse the statement with a syntax-looking error that named nothing.
/// The row's own `max_value` of 2000 makes that unreachable today; it is
/// handled anyway because the bound is data and this cast is code.
fn depth_as_limit(depth: usize) -> i64 {
    i64::try_from(depth).unwrap_or(i64::MAX)
}

/// Fuse the lexical halves into one lexical opinion.
///
/// The trigram half is now one ranked list PER PROBE, so this fuses
/// `1 + probes` lists, not two. A card matching three probes outranks one
/// matching a single probe — probes are independent evidence about the same
/// card, and agreement between them means what agreement between the two reads
/// means.
///
/// Returned as bare ids because the caller fuses again: only the ORDER carries
/// forward, and keeping the intermediate scores would invite someone to add
/// them to the vector scores, which is exactly the incomparable-magnitudes
/// mistake reciprocal rank exists to avoid.
fn fuse_lexical(full_text: &[String], trigram: &[(String, Vec<String>)]) -> Vec<String> {
    let mut lists: Vec<&[String]> = Vec::with_capacity(trigram.len() + 1);
    lists.push(full_text);
    lists.extend(trigram.iter().map(|(_, hits)| hits.as_slice()));

    fuse_many(&lists, RRF_K)
        .into_iter()
        .map(|card| card.evidence_id)
        .collect()
}

#[cfg(test)]
#[path = "gather_search_tests.rs"]
mod tests;
