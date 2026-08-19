// The SHIPPED deck, read off disk (Rule 21).
//
// Its own file, split from `deck_file_tests` on 2026-08-19 when the five
// redirect rows carried that file past Rule 17's limit. The seam is real as well
// as arithmetical: every test in the sibling spoils a fixture on purpose and
// asserts the refusal, while this one asserts that the deck Marie will actually
// be asked from is the deck the task describes.
//
// ## Why a test reads a YAML file at all
//
// The deck is DATA, and data in this repo has one failure mode nothing else
// catches: it parses in the author's head and not in serde's. A typo in a key
// name, one wrong indent under a folded scalar, a `tactic: 8`, a `follows`
// pointing at a key nobody kept — every one of them is invisible until an
// operator runs the binary against the live database, which on this timetable
// means Thursday morning, in front of Chuck.

use super::*;

/// THE SHIPPED DECK PARSES, AND IS THE FIFTEEN QUESTIONS OF THE V1 DECK.
///
/// ## Why this test reads a file off disk (Rule 21)
///
/// The deck is data, and data in this repo has one failure mode nothing else
/// catches: it parses in the author's head and not in serde's. A typo in a key
/// name, one wrong indent under a folded scalar, a `tactic: 8` — every one of
/// them is invisible until an operator runs the binary against the live
/// database, which on this timetable means Tuesday morning.
///
/// The counts are asserted because they are the CLAIM: five George questions
/// from five ruled instances, five Chuck direct questions from three points, and
/// — since 2026-08-19 — five redirects, one per George trap. Fifteen in all.
#[test]
fn the_shipped_s5_deck_parses_and_holds_the_v1_decks_fifteen_questions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read_to_string(root.join("practice_decks/S-5.yaml"))
        .expect("the S-5 deck is on disk");
    let deck: DeckFile = serde_yaml::from_str(&raw).expect("the S-5 deck parses");

    deck.validate().expect("the S-5 deck is valid");
    assert_eq!(deck.scenario_code, "S-5");
    assert_eq!(deck.questions.len(), 15);

    let george: Vec<_> = deck
        .questions
        .iter()
        .filter(|q| q.side == DeckSide::George)
        .collect();
    let chuck: Vec<_> = deck
        .questions
        .iter()
        .filter(|q| q.resolved_kind() == DeckKind::Direct)
        .collect();
    let redirects: Vec<_> = deck
        .questions
        .iter()
        .filter(|q| q.resolved_kind() == DeckKind::Redirect)
        .collect();
    assert_eq!(george.len(), 5, "one cross question per ruled instance");
    assert_eq!(chuck.len(), 5, "Chuck's direct");
    assert_eq!(redirects.len(), 5, "one redirect per George trap");

    // Every question carries a key, and the fifteen keys are distinct. This is
    // what `--update` matches on: without it, reconciling Chuck's edits with this
    // file falls back to matching TEXT, which stops working the first time he
    // re-words a question and inserts a duplicate instead of updating a row.
    let keys: Vec<&str> = deck
        .questions
        .iter()
        .map(|q| q.key.as_deref().expect("every v1 question carries a key"))
        .collect();
    let mut unique = keys.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(
        unique.len(),
        keys.len(),
        "the keys must be distinct: {keys:?}"
    );

    // Every redirect follows a CROSS question that is in this file, wears Chuck's
    // side, and carries NO stronger answer — the drawer shows the stored
    // "Tell it — this is Chuck's time." line instead, because on a redirect the
    // right answer is length and the honest-gap sentence would be exactly wrong.
    for q in &redirects {
        assert_eq!(q.side, DeckSide::Chuck, "a redirect wears Chuck's pill");
        let follows = q
            .follows
            .as_deref()
            .expect("a redirect names what it follows");
        assert!(
            keys.contains(&follows),
            "{follows} names no question in this deck"
        );
        assert!(
            q.stronger.is_none(),
            "a redirect carries no stronger example"
        );
        assert_eq!(
            q.draft_by.as_deref(),
            Some("architect"),
            "the five drafts are marked unreviewed until Chuck edits them"
        );
    }

    // The five documents the picker offers, and nothing else: only a question
    // that stands on a document of its own carries a source line. Chuck's ten
    // stand on her POINTS, which the picker already lists from the seeded
    // receipts — a source line on one of them would list the same exhibit twice.
    let sourced: Vec<&str> = deck
        .questions
        .iter()
        .filter_map(|q| q.source_line.as_deref())
        .collect();
    assert_eq!(sourced.len(), 5, "one per George question: {sourced:?}");
    for q in george.iter() {
        assert!(
            q.source_line.is_some(),
            "every cross question names the exhibit it stands on"
        );
    }

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
    for q in chuck.iter().chain(redirects.iter()) {
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

/// THE SHIPPED S-6 DECK PARSES, AND IS THE FIFTEEN QUESTIONS THE ARCHITECT WROTE.
///
/// S-6 arrived on 2026-08-19 written in the architect's own shorthand and was
/// transcribed into the loader's schema by hand. A transcription is exactly the
/// operation this test exists to catch: every string is correct in the source
/// file and one field name is wrong in the copy, which serde reports as a
/// missing field at SEED time — against the live database, on the morning Chuck
/// reads the deck.
#[test]
fn the_shipped_s6_deck_parses_and_holds_the_architects_fifteen_questions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read_to_string(root.join("practice_decks/S-6.yaml"))
        .expect("the S-6 deck is on disk");
    let deck: DeckFile = serde_yaml::from_str(&raw).expect("the S-6 deck parses");

    deck.validate().expect("the S-6 deck is valid");
    assert_eq!(deck.scenario_code, "S-6");
    assert_eq!(deck.questions.len(), 15);
    assert_eq!(deck.points.len(), 3, "one receipt per talking point");

    let of = |kind: DeckKind| {
        deck.questions
            .iter()
            .filter(|q| q.resolved_kind() == kind)
            .count()
    };
    assert_eq!(of(DeckKind::Cross), 5, "five George traps");
    assert_eq!(of(DeckKind::Redirect), 5, "one redirect per trap");
    assert_eq!(of(DeckKind::Direct), 5, "Chuck's direct");

    // The five instance bindings the architect's header names, and no index the
    // scenario cannot have: S-6 carries four ruled instances, so an index of 5
    // would be a refusal at seed time against DEV rather than here.
    for q in deck
        .questions
        .iter()
        .filter(|q| q.resolved_kind() == DeckKind::Cross)
    {
        assert_eq!(q.source_kind, DeckSourceKind::Instance);
        let index = q
            .source_index
            .expect("a cross question binds to an instance");
        assert!(
            (1..=4).contains(&index),
            "S-6 has four ruled instances; {} names instance {index}",
            q.key.as_deref().unwrap_or("?")
        );
        assert!(q.pair_said.is_some() && q.pair_admitted.is_some());
        assert!(
            q.source_line.is_some(),
            "the picker needs a handle per exhibit"
        );
    }

    // Every redirect names a trap that is in this file, and carries no stored
    // example — the drawer shows the redirect line instead.
    for q in deck
        .questions
        .iter()
        .filter(|q| q.resolved_kind() == DeckKind::Redirect)
    {
        assert!(q.stronger.is_none());
        assert_eq!(q.draft_by.as_deref(), Some("architect"));
    }

    // The braid is g5, it is the LAST George question (R10: general → specific,
    // the braid last), and it is the only one naming barrage rows.
    let braids: Vec<_> = deck
        .questions
        .iter()
        .filter(|q| q.braid_rows.is_some())
        .collect();
    assert_eq!(braids.len(), 1);
    assert_eq!(braids[0].key.as_deref(), Some("g5"));
    assert_eq!(braids[0].tactic, Some(5), "a braid is card 5, compound");
}
