//! Tests for the ranked gather's pure parts: the lexical nesting, the filter's
//! effect on the party set, and the read depth.
//!
//! The live behaviour — real rows, real vectors, the AT-1 and AT-2 numbers — is
//! measured by `tests/ranked_gather_l2b_measurement.rs`, which is `#[ignore]`d
//! and needs a database, a Qdrant and the model weights.

use super::*;
use crate::services::gather_fusion::append_conservation_tail;

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_string()).collect()
}

/// ⚑ The two lexical halves are fused into ONE opinion before the vector read
/// joins, so the lexical side cannot outvote the vector side two-to-one.
///
/// This is the load-bearing structural choice in the fusion. If the three lists
/// were fused flat, a card both lexical halves found would beat a card the
/// vector read alone found — not because two retrieval methods agreed, but
/// because the lexical side happens to be implemented as two queries. The
/// nesting is the weighting.
#[test]
fn the_lexical_halves_are_one_vote_not_two() {
    // `both-lexical` is rank 1 in BOTH lexical halves and absent from vector.
    // `vector-top` is rank 1 in vector and absent from lexical.
    let lexical = fuse_lexical(
        &ids(&["both-lexical"]),
        &[("p".to_string(), ids(&["both-lexical"]))],
    );
    let nested = fuse(&ids(&["vector-top"]), &lexical, RRF_K);

    let scores = |list: &[FusedCard], id: &str| {
        list.iter()
            .find(|c| c.evidence_id == id)
            .map(|c| c.fused_score)
            .unwrap_or_else(|| panic!("{id} missing"))
    };
    assert!(
        (scores(&nested, "both-lexical") - scores(&nested, "vector-top")).abs() < 1e-12,
        "one lexical vote must weigh the same as one vector vote"
    );

    // The counterfactual. Fused FLAT — three lists, each contributing once —
    // the lexical card would score 2/61 against the vector card's 1/61 and win
    // by a factor of two, having been found by ONE retrieval method that
    // happens to run two queries. The nesting is what makes it 1/61 each.
    //
    // Only the comparison against the real nested score is asserted: that
    // `2/61 > 1/61` is arithmetic on a constant and could never fail, so
    // asserting it would read as a guard while guarding nothing.
    let flat_lexical = 2.0 / (RRF_K + 1.0);
    assert!(
        (scores(&nested, "both-lexical") - flat_lexical).abs() > 1e-6,
        "and the nested score must NOT be the flat one"
    );
}

/// Fusing the halves keeps every card either half found.
#[test]
fn the_lexical_fusion_keeps_every_card_either_half_found() {
    let merged = fuse_lexical(&ids(&["a", "b"]), &[("p".to_string(), ids(&["c", "b"]))]);

    assert_eq!(merged.len(), 3, "a, b and c all survive");
    assert_eq!(merged[0], "b", "the card both halves found leads");
    assert!(merged.contains(&"a".to_string()) && merged.contains(&"c".to_string()));
}

/// One empty half leaves the other half's order intact — the state a query
/// with no trigram match produces, which is most queries.
#[test]
fn a_lexical_half_returning_nothing_is_not_an_error() {
    assert_eq!(
        fuse_lexical(&ids(&["a", "b", "c"]), &[]),
        vec!["a", "b", "c"]
    );
    assert!(fuse_lexical(&[], &[]).is_empty());
}

/// ⚑ `strict` admits the subject's parties only, which is what makes it the
/// conservation baseline, and `widened` admits Emil Awad.
#[test]
fn the_filter_mode_decides_which_parties_reach_the_reads() {
    let reachable = ids(&[
        "org-catholic-family-services",
        "person-emil-awad",
        "person-george-phillips",
    ]);
    let subject = "person-george-phillips";

    assert_eq!(
        GatherSubjectFilter::Strict.parties(subject, &reachable),
        Some(vec![subject]),
        "strict is today's read: the subject and nothing else"
    );
    let widened = GatherSubjectFilter::Widened
        .parties(subject, &reachable)
        .expect("widened filters");
    assert!(
        widened.contains(&"person-emil-awad"),
        "the four admissions filed about Emil Awad alone need him in the set"
    );
    assert_eq!(GatherSubjectFilter::Off.parties(subject, &reachable), None);
}

/// ⚑ The read depth is CONFIGURATION now, not a constant.
///
/// It shipped as a compiled 200 and was flagged: a retrieval limit is a
/// per-deployment threshold by Rule 13's own list, and no structural claim
/// about it would have been true — L3 exists partly to find out whether 200 is
/// the right number. It is a settings row, and it reaches both reads.
#[test]
fn the_read_depth_reaches_both_reads_from_one_number() {
    // The lexical LIMIT is derived from the same usize the vector read is given,
    // so the two sides can never be asked to different depths — a vector read of
    // 200 fused against a lexical read of 50 would look like the lexical side
    // having no opinion about 150 cards when it was simply never asked.
    assert_eq!(depth_as_limit(200), 200_i64);
    assert_eq!(depth_as_limit(20), 20_i64);
    assert_eq!(
        depth_as_limit(usize::MAX),
        i64::MAX,
        "a stored value too wide for the LIMIT saturates rather than wrapping to a \
         NEGATIVE limit, which Postgres would refuse with an error naming nothing"
    );
}

/// The two stages fail with different names, because they send an operator to
/// different machines.
#[test]
fn the_two_stages_fail_by_name() {
    let lexical = GatherSearchError::Lexical(LexicalReadError::Query {
        operation: "lexical_trigram",
        source: sqlx::Error::RowNotFound,
    })
    .to_string();
    assert!(lexical.contains("lexical stage"), "{lexical}");
    assert!(
        lexical.contains("lexical_trigram"),
        "the cause survives: {lexical}"
    );

    let vector = GatherSearchError::Vector(QdrantError::Api {
        status: 404,
        body: "collection not found".to_string(),
    })
    .to_string();
    assert!(vector.contains("vector stage"), "{vector}");
    assert!(vector.contains("collection not found"), "{vector}");
}

// ---------------------------------------------------------------------------
// Which stage lost the card
// ---------------------------------------------------------------------------

fn gather_with(admitted: &[&str], card_ids: &[&str]) -> RankedGather {
    RankedGather {
        cards: fuse(&ids(card_ids), &[], RRF_K),
        admitted: ids(admitted),
        subject_only_pool: Vec::new(),
        conservation_gap: Vec::new(),
        unreached_by_reads: 0,
        probes: Vec::new(),
        trigram_lists: 0,
        probe_hits: Vec::new(),
        probes_extracted: 0,
        probes_dropped: Vec::new(),
        filter_mode: GatherSubjectFilter::Widened,
        read_depth: 200,
        vector_hits: card_ids.len(),
        full_text_hits: 0,
        trigram_hits: 0,
    }
}

/// ⚑ The question every thin gather raises, answered.
///
/// "Which stage lost the card?" has two answers with two different fixes, and
/// before the admitted ID SET was kept they were indistinguishable: a count
/// cannot tell "the filter never let it through" from "the reads missed it".
/// The first is fixed by widening; the second by reading deeper or querying
/// better. Sending an operator to the wrong one wastes the afternoon.
#[test]
fn a_missing_card_names_the_stage_that_lost_it() {
    let gather = gather_with(
        &["admitted-and-found", "admitted-not-found"],
        &["admitted-and-found"],
    );

    assert_eq!(
        gather.why_missing("admitted-and-found"),
        None,
        "a card that is present is not missing at all"
    );
    assert_eq!(
        gather.why_missing("admitted-not-found"),
        Some(MissingStage::NotRetrieved),
        "the filter let it through and neither read returned it — a READ problem"
    );
    assert_eq!(
        gather.why_missing("never-admitted"),
        Some(MissingStage::NotAdmitted),
        "the filter never let it through — a FILTER problem, fixed by widening"
    );
}

/// ⚑ The seven admissions, as the measurement actually found them.
///
/// Under `strict` not one of the seven is admitted (none is filed ABOUT the
/// subject — measured, `under_strict = false` for all seven). Under `widened`
/// all seven are admitted and all seven were retrieved. So the stage that fell
/// short is neither the filter nor the reads: it is the RANKING, and this pins
/// that the diagnostic says so rather than blaming the widening.
#[test]
fn the_seven_admissions_were_lost_by_the_ranking_not_the_reads() {
    let seven = [
        "0898037c", "41068bce", "70e7c1f6", "7bf6759b", "949e69ac", "c511ed8d", "568d6db0",
    ];

    let strict = gather_with(&[], &[]);
    for id in seven {
        assert_eq!(
            strict.why_missing(id),
            Some(MissingStage::NotAdmitted),
            "under strict the filter is what loses {id}"
        );
    }

    let widened = gather_with(&seven, &seven);
    for id in seven {
        assert_eq!(
            widened.why_missing(id),
            None,
            "under widened every one of the seven is retrieved — the shortfall is rank"
        );
    }
}

/// The conservation log names a few ids and counts the rest.
///
/// Measured, the gap runs to ~200 ids — about 7,800 characters in one log
/// field, past where collectors truncate, which would drop the very ids the
/// line exists to name. The full list stays on the struct.
#[test]
fn the_conservation_log_is_bounded_and_says_how_many_it_left_out() {
    let many: Vec<String> = (1..=204).map(|n| format!("doc:evidence:{n:04}")).collect();
    let line = id_excerpt(&many);

    assert!(line.contains("doc:evidence:0001"));
    assert!(line.ends_with("+194 more"), "{line}");
    assert!(
        line.len() < 400,
        "the line must stay loggable: {} chars",
        line.len()
    );

    assert_eq!(id_excerpt(&[]), "");
    assert_eq!(
        id_excerpt(&ids(&["a", "b"])),
        "a, b",
        "a short gap is named in full"
    );
}

/// ⚑ A conservation-tail card can never satisfy an acceptance bar.
///
/// The bars measure RETRIEVAL. A tail card is present because it is in the
/// subject-only pool, not because anything found it, so counting one would
/// report a ranking as working when it had found nothing. The filter used to
/// live inside the `#[ignore]`d measurement harness, where no fast test could
/// reach it — removing it would have left every unit test green.
#[test]
fn a_conservation_tail_card_is_not_a_retrieved_card() {
    let mut gather = gather_with(&["found", "unreached"], &["found"]);
    gather.cards = append_conservation_tail(gather.cards, &ids(&["found", "unreached"]));

    assert_eq!(gather.cards.len(), 2, "the list carries both");
    assert_eq!(
        gather.retrieved_ids(),
        vec!["found"],
        "but only the retrieved one may be offered to an acceptance bar"
    );
    assert!(
        !gather.retrieved_ids().contains(&"unreached".to_string()),
        "a card nothing found cannot prove the search found it"
    );
}

/// With no tail, every card is retrieved — the filter does not over-reach.
#[test]
fn without_a_tail_every_card_is_retrieved() {
    let gather = gather_with(&["a", "b"], &["a", "b"]);
    assert_eq!(gather.retrieved_ids(), vec!["a", "b"]);
}
