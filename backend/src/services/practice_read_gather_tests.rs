// The §4 table, where it is a decision rather than a rendering.
//
// `gather_payload` itself needs a database and is exercised on DEV; the two
// decisions inside it that the table actually turns on are pure, and these are
// them. Both are one-line functions whose being wrong costs a whole question kind
// its read — which is exactly the kind of function that never gets a test because
// it looks too small to need one.

use super::*;
use crate::domain::settings::Settings;

/// A NULL tactic is a named absence and the read PROCEEDS.
///
/// **[measured 2026-08-20: 20 of the 30 live deck rows — every question Chuck
/// asks.]** The column's own comment says why NULL is not "none": a Chuck
/// question HAS no tactic; it does not have a tactic called none. Read as a load
/// failure, every direct and every redirect would abstain forever.
#[test]
fn a_question_with_no_tactic_is_absent_by_design() {
    let settings = Settings::for_test();
    assert_eq!(
        tactic_of(&settings, None).expect("a missing tactic is not a failure"),
        Tactic::NoneByDesign
    );
}

/// A card the vocabulary can name is named.
#[test]
fn a_card_the_vocabulary_names_is_sent_by_name() {
    let settings = Settings::for_test();
    // Card 5 of the seeded vocabulary is `compound` — the braid.
    assert_eq!(
        tactic_of(&settings, Some(5)).expect("card 5 is in the vocabulary"),
        Tactic::Named("compound".to_string())
    );
}

/// A card the vocabulary CANNOT name abstains (Roman, A4).
///
/// This is the live defect at HEAD, and the reason the enum exists. Both causes
/// used to produce `Option::None` and both were rendered as "none — this is a
/// direct question" — so a CROSS question whose card number outran a trimmed
/// `practice_tactic_names` row had the model told, in writing, that it carried no
/// tactic. A false statement about the very question being judged.
///
/// The column's CHECK constrains the card to 1–7, so reaching this means the
/// stored vocabulary is shorter than the deck's numbering: a settings row someone
/// edited, not a deck someone mis-seeded.
#[test]
fn a_card_the_vocabulary_cannot_name_abstains_rather_than_claiming_there_is_none() {
    let mut settings = Settings::for_test();
    settings.practice_read.tactic_names.truncate(3);

    let failure = tactic_of(&settings, Some(5)).expect_err("the row cannot name card 5");
    match failure {
        PayloadFailure::TacticUnnamed { card } => assert_eq!(card, 5),
        other => panic!("expected TacticUnnamed, got {other:?}"),
    }
}

/// The two failures a NULL tactic could mean produce DIFFERENT outcomes.
///
/// ANTI-VACUITY, and the whole point of the ruling: a `tactic_of` that abstained
/// on everything, or on nothing, would pass one of the two tests above.
#[test]
fn the_two_tactic_absences_are_not_the_same_outcome() {
    let mut settings = Settings::for_test();
    settings.practice_read.tactic_names.truncate(3);

    assert!(
        tactic_of(&settings, None).is_ok(),
        "no tactic at all is legitimate even when the vocabulary is short"
    );
    assert!(
        tactic_of(&settings, Some(5)).is_err(),
        "a real card the row cannot name is not legitimate"
    );
}

/// `None` and `Some([])` are two different facts and stay two.
#[test]
fn never_opened_and_picked_nothing_are_different_facts() {
    assert_eq!(points_to_of(None), PointsTo::NeverOpened);
    assert_eq!(
        points_to_of(Some(&Vec::new())),
        PointsTo::OpenedAndPickedNothing
    );
    let picked = vec!["your certified letter, 16 Nov 2009".to_string()];
    assert_eq!(
        points_to_of(Some(&picked)),
        PointsTo::Picked(picked.clone())
    );
}

/// Every load failure has BOTH halves of its reason.
///
/// Marie reads `plain_reason`; the operator reads `Display`. An abstain shipping
/// with either empty would be a screen saying nothing, or a log saying nothing,
/// depending which was forgotten — and the `match` is exhaustive, so a variant
/// added later cannot skip this.
#[test]
fn every_payload_failure_says_why_to_both_audiences() {
    let failures = [
        PayloadFailure::Points {
            scenario_id: uuid::Uuid::nil(),
            source: anyhow::anyhow!("connection reset"),
        },
        PayloadFailure::Receipts {
            scenario_id: uuid::Uuid::nil(),
            source: anyhow::anyhow!("connection reset"),
        },
        PayloadFailure::TacticUnnamed { card: 6 },
    ];

    for failure in &failures {
        let operator = failure.to_string();
        let marie = failure.plain_reason();

        assert!(!operator.is_empty());
        assert!(!marie.is_empty());
        assert!(
            !marie.contains("connection reset") && !marie.contains("practice_tactic_names"),
            "Marie's sentence must not carry the operator's diagnostics: {marie}"
        );
    }

    // The operator's half names the thing an operator would act on.
    assert!(failures[0]
        .to_string()
        .contains("her points could not be read"));
    assert!(failures[2].to_string().contains("practice_tactic_names"));
}
