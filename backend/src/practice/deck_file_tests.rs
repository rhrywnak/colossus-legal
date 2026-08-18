// Tests for `practice::deck_file`.
//
// The subject is a file a human writes by hand and a binary then commits to a
// witness-facing table. Every test below is one way that file can be wrong in a
// way that would otherwise be discovered by Marie, mid-session, on the screen.

use super::*;

/// A deck that is entirely well-formed, for the tests that spoil one thing.
fn good_question() -> DeckQuestion {
    DeckQuestion {
        side: DeckSide::George,
        source_kind: DeckSourceKind::Instance,
        source_index: Some(1),
        tactic: Some(4),
        braid_rows: None,
        text: "Isn't it true that you were at each other's throats?".to_string(),
        receipt: Some("Built from: the hearing, p. 34".to_string()),
        pair_said: Some("they're at each other's throats".to_string()),
        pair_admitted: Some("there was a desire to divide the property".to_string()),
        watch_for: Some("WATCH FOR — the characterization inside the question.".to_string()),
        stronger: Some("That's not true. I asked in writing.".to_string()),
        stronger_lean: Some("leans on point 1".to_string()),
    }
}

fn deck_of(questions: Vec<DeckQuestion>) -> DeckFile {
    DeckFile {
        scenario_code: "S-5".to_string(),
        points: Vec::new(),
        questions,
    }
}

#[test]
fn a_well_formed_deck_passes() {
    assert_eq!(deck_of(vec![good_question()]).validate(), Ok(()));
}

/// An EMPTY deck is refused rather than seeded.
///
/// Without this the tool would write zero rows, print "0 questions written", exit
/// 0, and Roman would read that as success — then Marie would open the page and
/// see "no practice deck yet". A tool must not be able to succeed at nothing.
/// A deck that names no scenario is refused.
///
/// The seed resolves the code to a scenario id; a blank one would parse to no
/// ordinal and come back as "no scenario has the code ''" — an error about the
/// DATABASE for what is a missing line in a file.
#[test]
fn a_deck_naming_no_scenario_is_refused_before_the_database_is_opened() {
    for blank in ["", "   ", "\t"] {
        let deck = DeckFile {
            scenario_code: blank.to_string(),
            points: Vec::new(),
            questions: vec![good_question()],
        };
        assert_eq!(deck.validate(), Err(DeckError::NoScenarioCode), "{blank:?}");
    }
    assert_eq!(
        DeckError::NoScenarioCode.to_string(),
        "the deck names no scenario code"
    );
}

/// The column values are the ones the CHECK constraints permit.
///
/// ## Why two lines of mapping get a test
///
/// These strings are written into every seeded row, and the database would
/// accept a wrong one only by rejecting the whole insert — or, worse, accept
/// `"chuck"` where `"george"` was meant and produce a deck whose every question
/// is attributed to the wrong side. Nothing downstream could tell.
#[test]
fn the_column_values_are_the_ones_the_check_constraints_permit() {
    assert_eq!(DeckSide::George.as_column(), "george");
    assert_eq!(DeckSide::Chuck.as_column(), "chuck");
    assert_eq!(DeckSourceKind::Instance.as_column(), "instance");
    assert_eq!(DeckSourceKind::Point.as_column(), "point");
    assert_eq!(DeckSourceKind::Manual.as_column(), "manual");
}

/// Every refusal names the QUESTION a human has to go and fix.
///
/// The variant is what the code branches on; the SENTENCE is what an operator
/// reads at 7am with a deck file open beside a terminal. Asserting only the
/// variant would let the message lose its position number and nothing would go
/// red.
#[test]
fn every_refusal_names_the_question_and_what_is_wrong_with_it() {
    let mut blank = good_question();
    blank.text = " ".to_string();
    let message = deck_of(vec![good_question(), blank])
        .validate()
        .expect_err("question 2 is blank")
        .to_string();
    assert!(message.contains("question 2"), "{message}");
    assert!(message.contains("blank"), "{message}");

    let mut bad_card = good_question();
    bad_card.tactic = Some(9);
    let message = deck_of(vec![bad_card])
        .validate()
        .expect_err("there are seven cards")
        .to_string();
    assert!(message.contains("question 1"), "{message}");
    assert!(message.contains("9"), "{message}");
    assert!(message.contains("seven cards"), "{message}");

    let mut unsourced = good_question();
    unsourced.source_index = None;
    let message = deck_of(vec![unsourced])
        .validate()
        .expect_err("an instance question needs an index")
        .to_string();
    assert!(message.contains("question 1"), "{message}");
    assert!(message.contains("instance"), "{message}");
    assert!(message.contains("source_index"), "{message}");

    assert!(DeckError::NoQuestions
        .to_string()
        .contains("report success"));
}

#[test]
fn an_empty_deck_is_refused_rather_than_seeded_as_nothing() {
    assert_eq!(deck_of(vec![]).validate(), Err(DeckError::NoQuestions));
}

#[test]
fn a_blank_question_is_refused_by_its_position_in_the_file() {
    let mut q = good_question();
    q.text = "   ".to_string();
    assert_eq!(
        deck_of(vec![good_question(), q]).validate(),
        Err(DeckError::BlankText { position: 2 })
    );
}

/// A tactic outside the seven cards is refused BEFORE the database sees it.
///
/// The column has its own CHECK, so this is belt and braces — deliberately. The
/// CHECK's message names a constraint; this one names the line of the file a
/// human has open, which is the difference between a two-minute fix and a hunt.
#[test]
fn a_tactic_outside_the_seven_cards_is_refused_here_not_by_the_column() {
    let mut q = good_question();
    q.tactic = Some(8);
    assert_eq!(
        deck_of(vec![q]).validate(),
        Err(DeckError::UnknownTactic {
            position: 1,
            tactic: 8
        })
    );

    // The boundaries themselves are legal — an off-by-one in the range check
    // would silently cost the deck card 1 (broad generalization) or card 7
    // (echo), and nothing else in the system would notice.
    for card in 1..=7 {
        let mut q = good_question();
        q.tactic = Some(card);
        assert_eq!(deck_of(vec![q]).validate(), Ok(()), "card {card}");
    }
}

/// The source rules, both directions.
///
/// These are the two halves of the same law the `practice_questions_ref_matches_kind`
/// constraint states: a sourced question has a source, and a manual one does not
/// pretend to. Catching it here means the seed never begins a transaction it will
/// have to abort.
#[test]
fn a_sourced_question_needs_an_index_and_a_manual_one_must_not_have_it() {
    let mut q = good_question();
    q.source_index = None;
    assert_eq!(
        deck_of(vec![q]).validate(),
        Err(DeckError::MissingSourceIndex {
            position: 1,
            kind: "instance"
        })
    );

    let mut q = good_question();
    q.source_kind = DeckSourceKind::Manual;
    assert_eq!(
        deck_of(vec![q]).validate(),
        Err(DeckError::ManualWithSourceIndex {
            position: 1,
            index: 1
        })
    );

    let mut q = good_question();
    q.source_kind = DeckSourceKind::Manual;
    q.source_index = None;
    q.receipt = None;
    assert_eq!(deck_of(vec![q]).validate(), Ok(()));
}

/// Position 0 is refused, because the file counts from one.
///
/// A `source_index: 0` would otherwise underflow into "the last instance" or
/// panic on the subtraction the resolver does. It is refused where a human can
/// read why.
#[test]
fn a_zero_source_index_is_refused_because_the_file_counts_from_one() {
    let mut q = good_question();
    q.source_index = Some(0);
    assert_eq!(
        deck_of(vec![q]).validate(),
        Err(DeckError::ZeroSourceIndex {
            position: 1,
            index: 0
        })
    );
}

/// Half a pair is refused.
///
/// Screen S2 renders the pair as two labelled columns. One half present would
/// render a heading, a quote, and an empty box under "What they admitted under
/// oath" — which reads to a witness as "they admitted nothing", the opposite of
/// what an absent pair means.
#[test]
fn half_a_pair_is_refused_because_the_screen_would_read_as_an_admission_of_nothing() {
    let mut q = good_question();
    q.pair_admitted = None;
    assert_eq!(
        deck_of(vec![q]).validate(),
        Err(DeckError::HalfAPair {
            position: 1,
            half: "pair_admitted"
        })
    );

    let mut q = good_question();
    q.pair_said = None;
    assert_eq!(
        deck_of(vec![q]).validate(),
        Err(DeckError::HalfAPair {
            position: 1,
            half: "pair_said"
        })
    );

    // Neither half is legal: a Chuck question has no pair at all.
    let mut q = good_question();
    q.pair_said = None;
    q.pair_admitted = None;
    assert_eq!(deck_of(vec![q]).validate(), Ok(()));
}

/// The point receipts are refused the same three ways a question is.
#[test]
fn a_malformed_point_receipt_is_refused_by_its_place_in_the_file() {
    let with = |points: Vec<DeckPoint>| DeckFile {
        scenario_code: "S-5".to_string(),
        points,
        questions: vec![good_question()],
    };

    assert_eq!(
        with(vec![DeckPoint {
            position: 1,
            text: "  ".to_string()
        }])
        .validate(),
        Err(DeckError::BlankPointReceipt { ordinal: 1 })
    );

    // Position 0 would underflow the printed numbering the screen uses.
    assert_eq!(
        with(vec![DeckPoint {
            position: 0,
            text: "a receipt".to_string()
        }])
        .validate(),
        Err(DeckError::ZeroPointPosition {
            ordinal: 1,
            position: 0
        })
    );

    // Two receipts for one point. The column's UNIQUE would catch it too — but
    // only mid-transaction, as a constraint name, AFTER the questions were
    // written. Here it is a sentence naming the point, before anything opens.
    assert_eq!(
        with(vec![
            DeckPoint {
                position: 2,
                text: "one".to_string()
            },
            DeckPoint {
                position: 2,
                text: "two".to_string()
            },
        ])
        .validate(),
        Err(DeckError::DuplicatePointPosition { position: 2 })
    );

    // A deck with NO receipts is legitimate — every point then shows the stored
    // named-absence line, which is what every scenario but S-5 does today.
    assert_eq!(with(Vec::new()).validate(), Ok(()));
}

/// THE SHIPPED DECK PARSES, AND IS THE TEN QUESTIONS OF THE MOCKUP.
///
/// ## Why this test reads a file off disk (Rule 21)
///
/// The deck is data, and data in this repo has one failure mode nothing else
/// catches: it parses in the author's head and not in serde's. A typo in a key
/// name, one wrong indent under a folded scalar, a `tactic: 8` — every one of
/// them is invisible until an operator runs the binary against the live
/// database, which on this timetable means Tuesday morning.
///
/// The counts are asserted because they are the CLAIM: five George questions from
/// five ruled instances, five Chuck questions from three points, ten in all.
#[test]
fn the_shipped_s5_deck_parses_and_holds_the_mockups_ten_questions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read_to_string(root.join("practice_decks/S-5.yaml"))
        .expect("the S-5 deck is on disk");
    let deck: DeckFile = serde_yaml::from_str(&raw).expect("the S-5 deck parses");

    deck.validate().expect("the S-5 deck is valid");
    assert_eq!(deck.scenario_code, "S-5");
    assert_eq!(deck.questions.len(), 10);

    let george: Vec<_> = deck
        .questions
        .iter()
        .filter(|q| q.side == DeckSide::George)
        .collect();
    let chuck: Vec<_> = deck
        .questions
        .iter()
        .filter(|q| q.side == DeckSide::Chuck)
        .collect();
    assert_eq!(george.len(), 5, "one cross question per ruled instance");
    assert_eq!(chuck.len(), 5, "Chuck's direct");

    // Every George question is tagged, sourced to an instance, and carries the
    // pair its reveal renders. Those three together are what makes the cross side
    // TRACEABLE — the property design §1 states as "nothing is invented".
    for q in &george {
        assert!(q.tactic.is_some(), "a cross question carries its tactic");
        assert_eq!(q.source_kind, DeckSourceKind::Instance);
        assert!(q.pair_said.is_some() && q.pair_admitted.is_some());
        assert!(
            q.receipt.is_some(),
            "and the line saying where it came from"
        );
    }

    // Chuck's carry none of that and must not pretend to: no tactic (there is no
    // trap in a friendly question), no pair (nobody is being impeached).
    for q in &chuck {
        assert!(q.tactic.is_none());
        assert_eq!(q.source_kind, DeckSourceKind::Point);
        assert!(q.pair_said.is_none() && q.pair_admitted.is_none());
    }

    // The braid is the fifth George question and the only one naming barrage rows.
    let braids: Vec<_> = deck
        .questions
        .iter()
        .filter(|q| q.braid_rows.is_some())
        .collect();
    assert_eq!(braids.len(), 1);
    assert_eq!(braids[0].tactic, Some(5), "a braid is card 5, compound");

    // Roman's ruling of 2026-08-17: one receipt per talking point, seeded with
    // the deck, and they carry NO "Backed by:" — that word is wording and the
    // renderer joins the two. A receipt shipped with the prefix baked in would
    // print "Backed by: Backed by: …" on the reveal.
    assert_eq!(deck.points.len(), 3, "one receipt per talking point");
    for (i, point) in deck.points.iter().enumerate() {
        assert_eq!(point.position, i + 1, "the three are positions 1, 2, 3");
        assert!(
            !point.text.contains("Backed by"),
            "the stored prefix must not be baked into the receipt: {}",
            point.text
        );
    }
    assert_eq!(deck.points[0].text, "your certified letter, 16 Nov 2009");
    assert_eq!(deck.points[1].text, "CFS Interrogatory Response, p. 10");
    assert_eq!(deck.points[2].text, "CFS Interrogatory Response, p. 14");
}
