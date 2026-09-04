//! Tests for the sentence at the top of every page.
//!
//! `basis_in_plain_words` is what a reader sees before anything else, and every
//! one of its arms describes a materially different gather. Getting it wrong
//! says "nothing found" over a full list, or quotes a pool size that is not the
//! pool — and neither looks like a bug on the page.

use super::*;
use colossus_legal_backend::services::gather_fusion::FusedCard;

fn gather(ranked: usize, tail: usize, pool: usize) -> RankedGather {
    let card = |placement| FusedCard {
        evidence_id: "e".to_string(),
        rank: None,
        placement,
        fused_score: 0.0,
        vector_rank: None,
        lexical_rank: None,
    };
    RankedGather {
        cards: std::iter::repeat_with(|| card(CardPlacement::Ranked))
            .take(ranked)
            .chain(std::iter::repeat_with(|| card(CardPlacement::ConservationTail)).take(tail))
            .collect(),
        subject_only_pool: vec!["p".to_string(); pool],
        admitted: Vec::new(),
        conservation_gap: Vec::new(),
        unreached_by_reads: tail,
        filter_mode: colossus_legal_backend::domain::gather_filter::GatherSubjectFilter::Widened,
        read_depth: 200,
        vector_hits: 0,
        full_text_hits: 0,
        trigram_hits: 0,
        trigram_lists: 0,
        trigram_lists_read: 0,
        probe_hits: Vec::new(),
        collapsed: Vec::new(),
        probes_extracted: 0,
        probes_dropped: Vec::new(),
        probes: Vec::new(),
    }
}

/// The ordinary case names both numbers a reader needs.
#[test]
fn a_full_gather_reports_what_was_found_against_the_pool() {
    let words = basis_in_plain_words(&gather(652, 68, 292));
    assert!(words.contains("652"), "{words}");
    assert!(words.contains("292"), "{words}");
}

/// ⚑ Nothing found is said in those words, and the tail is not mistaken for it.
///
/// A page whose ranked section is empty but whose tail carries 292 cards is not
/// an empty gather — it is a gather the reads missed entirely, and the reader
/// has to know which so they do not conclude the corpus is bare.
#[test]
fn an_empty_ranked_list_is_distinguished_from_an_empty_corpus() {
    let neither = basis_in_plain_words(&gather(0, 0, 0));
    assert!(neither.contains("nothing found"), "{neither}");

    let tail_only = basis_in_plain_words(&gather(0, 292, 292));
    assert!(tail_only.contains("neither read"), "{tail_only}");
    assert!(
        tail_only.contains("292"),
        "the tail's size is the story: {tail_only}"
    );
    assert_ne!(neither, tail_only, "these are different gathers");
}

/// A subject with no evidence of its own is called out, because every count on
/// the page is then measured against nothing.
#[test]
fn a_subject_with_no_pool_of_its_own_is_named() {
    let words = basis_in_plain_words(&gather(40, 0, 0));
    assert!(words.contains("no evidence of its own"), "{words}");
    assert!(words.contains("40"), "{words}");
}
