// Tests for `services::matrix_strength`.
//
// Split into their own file under the `#[path]` convention this codebase uses
// for every module whose tests would push it past Rule 17's 300-line limit
// (`wording_*_tests.rs`, `settings_store_tests.rs`). The seam is purely size:
// nothing here is a different concern from the module it tests.

use super::*;

fn map() -> EvidenceTierMap {
    let strong = vec![
        "admission+sworn_party_admission".to_string(),
        "court_finding+court_finding".to_string(),
    ];
    let hedged = vec!["partial_admission+sworn_party_admission".to_string()];
    let other = vec!["factual_assertion+sworn_testimony".to_string()];
    EvidenceTierMap::from_entries(&[
        (EvidenceTier::Strong, "matrix_tier_strong_pairs", &strong),
        (EvidenceTier::Hedged, "matrix_tier_hedged_pairs", &hedged),
        (EvidenceTier::Other, "matrix_tier_other_pairs", &other),
    ])
    .expect("test map parses")
}

/// A Q/A admission: speaker + question + answer.
fn qa(id: &str, speaker: &str, question: &str, answer: &str) -> CorroboratingItem {
    CorroboratingItem {
        id: id.to_string(),
        statement_type: Some("admission".to_string()),
        evidence_strength: Some("sworn_party_admission".to_string()),
        speaker: Some(speaker.to_string()),
        question: Some(question.to_string()),
        quote: Some(answer.to_string()),
    }
}

/// Documentary evidence: a speaker and a quote, no question.
fn documentary(id: &str, speaker: &str, quote: &str) -> CorroboratingItem {
    CorroboratingItem {
        id: id.to_string(),
        statement_type: Some("court_finding".to_string()),
        evidence_strength: Some("court_finding".to_string()),
        speaker: Some(speaker.to_string()),
        question: None,
        quote: Some(quote.to_string()),
    }
}

/// THE MEASURED TRAP, as a test. George Phillips answered "Yes." to three
/// different interrogatories on DEV. Under the originally-drafted key (speaker
/// + quote) those three sworn admissions collapse to one row reading "×3" and
/// the Element's strong count drops by two. Under the ruled amendment they
/// stay three rows.
#[test]
fn three_distinct_yes_admissions_do_not_collapse() {
    let items = vec![
        qa(
            "e-1",
            "George Phillips",
            "Did you receive the letter?",
            "Yes.",
        ),
        qa("e-2", "George Phillips", "Was the auction held?", "Yes."),
        qa(
            "e-3",
            "George Phillips",
            "Did you sign the accounting?",
            "Yes.",
        ),
    ];
    let groups = collapse_and_rank(&items, &map());
    assert_eq!(groups.len(), 3, "distinct questions must not merge");
    for group in &groups {
        assert_eq!(group.occurrences, 1);
    }
    assert_eq!(
        tally(&groups),
        StrengthTally {
            strong: 3,
            approved: 3
        }
    );
}

/// The genuine C-9≡C-11 class: the SAME statement recorded twice — same
/// speaker, same question, same answer. One row, "×2".
#[test]
fn the_same_statement_recorded_twice_collapses_with_a_count() {
    let items = vec![
        qa(
            "e-9",
            "George Phillips",
            "Was correspondence received?",
            "Correspondence was received from Marie Awad.",
        ),
        qa(
            "e-11",
            "George Phillips",
            "Was correspondence received?",
            "Correspondence was received from Marie Awad.",
        ),
    ];
    let groups = collapse_and_rank(&items, &map());
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].occurrences, 2);
    assert_eq!(groups[0].lead.id, "e-9", "the first item seen leads");
    assert_eq!(groups[0].merged_ids, vec!["e-11".to_string()]);
    assert_eq!(
        tally(&groups),
        StrengthTally {
            strong: 1,
            approved: 1
        },
        "a collapsed pair counts ONCE in both numbers",
    );
}

/// Case and whitespace differences are not real differences.
#[test]
fn collapse_ignores_case_and_whitespace() {
    let items = vec![
        qa(
            "e-1",
            "George Phillips",
            "Was it received?",
            "It   was\n received.",
        ),
        qa(
            "e-2",
            "george phillips",
            "was it received?",
            "IT WAS RECEIVED.",
        ),
    ];
    let groups = collapse_and_rank(&items, &map());
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].occurrences, 2);
}

/// Punctuation is NOT normalized away. "yes" and "yes." stay two rows: the
/// module widens its net only as far as the ruling went, because a wrongly
/// merged pair is proof that silently stopped existing.
#[test]
fn punctuation_is_not_stripped() {
    let items = vec![
        qa("e-1", "George Phillips", "Well?", "Yes"),
        qa("e-2", "George Phillips", "Well?", "Yes."),
    ];
    assert_eq!(collapse_and_rank(&items, &map()).len(), 2);
}

/// Two speakers saying the same words are two pieces of proof, not one.
#[test]
fn different_speakers_do_not_collapse() {
    let items = vec![
        qa("e-1", "George Phillips", "Was it received?", "Yes."),
        qa(
            "e-2",
            "Catholic Family Services",
            "Was it received?",
            "Yes.",
        ),
    ];
    assert_eq!(collapse_and_rank(&items, &map()).len(), 2);
}

/// Documentary evidence answers nobody, so it keys on speaker + quote — the
/// rule as originally written, which is correct for this class.
#[test]
fn documentary_evidence_collapses_on_speaker_and_quote() {
    let items = vec![
        documentary("e-1", "Judge Tighe", "The property fight drove the fees."),
        documentary("e-2", "Judge Tighe", "The property fight drove the fees."),
        documentary("e-3", "Judge Tighe", "Marie was treated differently."),
    ];
    let groups = collapse_and_rank(&items, &map());
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].occurrences, 2);
}

/// A Q/A item and a documentary item with the same words are different
/// claims: one is an answer under oath, the other a finding. The question
/// component keeps them apart without a special case.
#[test]
fn a_qa_item_never_merges_with_a_documentary_one() {
    let items = vec![
        qa(
            "e-1",
            "George Phillips",
            "Was it received?",
            "It was received.",
        ),
        documentary("e-2", "George Phillips", "It was received."),
    ];
    assert_eq!(collapse_and_rank(&items, &map()).len(), 2);
}

/// The same node arriving twice from a fan-out query is ONE item, and must not
/// inflate the "×N" — that marker means "this statement was made twice", not
/// "this row was joined twice".
#[test]
fn a_repeated_id_is_dropped_rather_than_counted_as_a_duplicate() {
    let item = qa("e-1", "George Phillips", "Was it received?", "Yes.");
    let groups = collapse_and_rank(&[item.clone(), item], &map());
    assert_eq!(groups.len(), 1);
    assert_eq!(
        groups[0].occurrences, 1,
        "the same node twice is one statement, not two",
    );
    assert!(groups[0].merged_ids.is_empty());
}

/// Strongest first, unmapped last — and nothing is dropped on the way.
#[test]
fn ranking_is_strong_then_hedged_then_other_then_unmapped() {
    let mut unmapped = qa("e-unmapped", "Counsel", "n/a", "Argument, not proof.");
    unmapped.statement_type = Some("attorney_argument".to_string());
    unmapped.evidence_strength = Some("attorney_assertion".to_string());

    let mut hedged = qa(
        "e-hedged",
        "George Phillips",
        "Did you review it?",
        "To the best of my recollection.",
    );
    hedged.statement_type = Some("partial_admission".to_string());

    let mut other = qa("e-other", "Marie Awad", "n/a", "I asked in writing.");
    other.statement_type = Some("factual_assertion".to_string());
    other.evidence_strength = Some("sworn_testimony".to_string());

    let strong = qa("e-strong", "George Phillips", "Was it received?", "Yes.");

    // Deliberately supplied in the WRONG order.
    let groups = collapse_and_rank(&[unmapped, other, hedged, strong], &map());
    let ids: Vec<&str> = groups.iter().map(|g| g.lead.id.as_str()).collect();
    assert_eq!(ids, vec!["e-strong", "e-hedged", "e-other", "e-unmapped"]);
    assert_eq!(groups[3].tier, None, "an unmapped item carries no tier");
    assert_eq!(
        tally(&groups),
        StrengthTally {
            strong: 1,
            approved: 4
        },
        "the unmapped item is counted as approved but never as strong",
    );
}

/// Two items of the same tier keep the order the caller supplied — the sort is
/// stable, so the page cannot reshuffle equally-ranked rows between renders.
#[test]
fn equal_tiers_keep_their_input_order() {
    let items = vec![
        qa("e-b", "George Phillips", "Second question?", "Yes."),
        qa("e-a", "George Phillips", "First question?", "Yes."),
    ];
    let groups = collapse_and_rank(&items, &map());
    let ids: Vec<&str> = groups.iter().map(|g| g.lead.id.as_str()).collect();
    assert_eq!(ids, vec!["e-b", "e-a"]);
}

/// An item with no quote cannot be near-identical to anything, so each one
/// stands alone rather than every quoteless item sharing one row.
#[test]
fn quoteless_items_never_merge_with_each_other() {
    let mut a = qa("e-1", "George Phillips", "Was it received?", "");
    a.quote = None;
    let mut b = qa("e-2", "George Phillips", "Was it received?", "");
    b.quote = Some("   ".to_string());
    let groups = collapse_and_rank(&[a, b], &map());
    assert_eq!(groups.len(), 2);
    assert_eq!(
        tally(&groups).approved,
        2,
        "an item with no words is still approved evidence and is still counted",
    );
}

/// The empty case: no evidence is zero of both, not a panic and not a gap the
/// caller has to special-case.
#[test]
fn an_empty_list_tallies_to_zero() {
    let groups = collapse_and_rank(&[], &map());
    assert!(groups.is_empty());
    assert_eq!(
        tally(&groups),
        StrengthTally {
            strong: 0,
            approved: 0
        }
    );
}
