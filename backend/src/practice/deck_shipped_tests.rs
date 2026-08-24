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

// ─── The RULED order, pinned on disk (build .403 bundle, §C) ────────────────
//
// ## Why the order is a test and not a convention
//
// `sort_order` is written from the position of each question in the YAML, so
// the file IS the order — and until now nothing asserted what that order was.
// The decks could be re-sequenced by an editor tidying a file, and the only
// symptom would be Marie being asked the conclusion before the facts it rests
// on, which is precisely the sequence the 08-19 ruling exists to prevent.
//
// ## Domain note: why this order and not deck-key order
//
// Roman's ruling of 2026-08-19 evening: the defense's cross leads with the FACTS
// it can prove, then the conclusion it wants drawn from them, then the braid
// that ties three rows together. A witness who meets the conclusion first has
// nothing to answer it with. The keys (`g1`…`g5`) are the order the questions
// were WRITTEN in; they are stable handles, deliberately not re-numbered, and
// they no longer describe the order they are asked in.
//
// Chuck's direct questions come next (foundation, then her three points), and
// the redirects last — each still bound to its defense question by
// `follows_key`, which is what lets Mixed re-pair them.

/// The order a deck's keys appear in, which is the order `sort_order` gets.
fn shipped_key_order(file: &str) -> Vec<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read_to_string(root.join(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
    let deck: DeckFile = serde_yaml::from_str(&raw).expect("the deck parses");
    deck.questions
        .iter()
        .map(|q| q.key.clone().unwrap_or_default())
        .collect()
}

/// S-6: facts first, the conclusion, the braid — then Chuck, then the redirects.
#[test]
fn the_shipped_s6_deck_is_in_the_ruled_order() {
    assert_eq!(
        shipped_key_order("practice_decks/S-6.yaml"),
        [
            "g2", "g4", "g3", "g1", "g5", // the defense: facts → conclusion → braid
            "c1", "c2", "c3", "c4", "c5", // Chuck: foundation → her three points
            "r1", "r2", "r3", "r4", "r5", // Chuck again, after each defense question
        ]
    );
}

/// S-5: the same shape, its own facts-first sequence.
///
/// g3 (never came in, and it was stipulated) · g4 (multiple contacts) · g2 (he
/// was right about that) · g1 (at each other's throats — the conclusion) · g5
/// (the braid).
#[test]
fn the_shipped_s5_deck_is_in_the_ruled_order() {
    assert_eq!(
        shipped_key_order("practice_decks/S-5.yaml"),
        [
            "g3", "g4", "g2", "g1", "g5", //
            "c1", "c2", "c3", "c4", "c5", //
            "r1", "r2", "r3", "r4", "r5", //
        ]
    );
}

/// Every redirect still names a defense question that EXISTS, after the re-order.
///
/// The pairing is by `follows_key`, not by position, so re-ordering cannot break
/// it — but that is the claim, and this is what makes it a checked one. A
/// redirect whose target was renamed would fall out of Mixed's pairs silently
/// and be dealt at the end instead, which reads as a deck bug rather than a
/// broken link.
#[test]
fn every_redirect_still_follows_a_question_the_deck_holds() {
    for file in ["practice_decks/S-5.yaml", "practice_decks/S-6.yaml"] {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let raw = std::fs::read_to_string(root.join(file)).expect("the deck is on disk");
        let deck: DeckFile = serde_yaml::from_str(&raw).expect("the deck parses");
        let keys: Vec<String> = deck
            .questions
            .iter()
            .filter_map(|q| q.key.clone())
            .collect();
        for q in deck.questions.iter().filter(|q| q.follows.is_some()) {
            let target = q.follows.clone().unwrap_or_default();
            assert!(
                keys.contains(&target),
                "{file}: a redirect follows `{target}`, which the deck does not hold"
            );
        }
    }
}

// -----------------------------------------------------------------------------
// ⚑ NO RECEIPT NAMES A DECK KEY — the guard on the SOURCE
// -----------------------------------------------------------------------------
//
// "Chuck's redirect after G3 — point 1." printed a question CODE on Chuck's
// paper. Codes left the screen and the printed sheets on 2026-08-23; this one
// survived inside AUTHORED PROSE, where no code change reached it. It pointed at
// something appearing nowhere else on paper or on screen.
//
// ## What this guards, and what it does NOT
//
// It does not guard the fix — the fix is in the migration and in the YAML, and
// both are done. It guards the SOURCE against the defect coming back: the YAML
// is alive, Chuck will write receipts again, and this is where a code would
// re-enter.
//
// ## ⚑ SELF-DERIVING, deliberately
//
// The forbidden strings come from the DECK'S OWN KEYS, not from a hardcoded
// `g1`–`g5` / `c1`–`c5` / `r1`–`r5` pattern. A hardcoded pattern silently stops
// guarding the day a deck gains a key shape nobody updated the test for; a
// self-deriving one cannot.
//
// ## ⚑ CASE-INSENSITIVE, deliberately
//
// What printed was `G3`, uppercase. The keys in the file are lowercase. A
// case-sensitive match would have found nothing and reported the deck clean.
//
// ## Domain note: "after the half-truth" is NOT this defect
//
// S-6's five redirects read "after the generalization", "after the half-truth",
// "after the authority borrow", "after the echo", "after the braid". Those name
// TACTICS and point at something a reader can use. The ruling was never about
// the word "after" — it was about pointing at a code that appears nowhere else.
// This test looks for KEYS, so it passes those and would fail `after g3`.

/// Every shipped deck, by the path it lives at.
// STRUCTURAL: a repo-internal fixture registry, not deployment configuration.
// These deck files are committed to this codebase and read off disk by the test
// itself (Rule 21) — they cannot vary between DEV and PROD, and a path that
// moved would fail this test rather than mis-serve a request. The sibling
// migration-path constants in the wording fixtures carry the same annotation for
// the same reason.
const SHIPPED_DECKS: &[&str] = &[
    "practice_decks/S-5.yaml",
    "practice_decks/S-6.yaml",
    "practice_decks/S-7.yaml",
];

/// Is `key` present in `text` as a WORD, case-insensitively?
///
/// Bounded on both sides so a key never matches inside another word — `g1` must
/// not fire on "g10", and `r1` must not fire on "Interrogatory R1esponse". The
/// bounds are "not alphanumeric", which is what a deck key is made of.
fn names_key(text: &str, key: &str) -> bool {
    let haystack = text.to_lowercase();
    let needle = key.to_lowercase();
    let bytes = haystack.as_bytes();
    let mut from = 0;
    while let Some(at) = haystack[from..].find(&needle) {
        let start = from + at;
        let end = start + needle.len();
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let after_ok = end == bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        from = end;
    }
    false
}

#[test]
fn no_shipped_receipt_names_a_deck_key() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

    let mut receipts_checked = 0_usize;
    let mut keys_checked = 0_usize;

    for file in SHIPPED_DECKS {
        let raw = std::fs::read_to_string(root.join(file))
            .unwrap_or_else(|cause| panic!("{file} is on disk: {cause}"));
        let deck: DeckFile =
            serde_yaml::from_str(&raw).unwrap_or_else(|cause| panic!("{file} parses: {cause}"));

        // The keys THIS deck declares — derived, never a pattern.
        let keys: Vec<String> = deck
            .questions
            .iter()
            .filter_map(|q| q.key.clone())
            .collect();
        keys_checked += keys.len();

        for question in &deck.questions {
            let Some(receipt) = question.receipt.as_ref() else {
                continue;
            };
            receipts_checked += 1;
            for key in &keys {
                assert!(
                    !names_key(receipt, key),
                    "{file}: a receipt names the deck key {key:?}, which appears \
                     nowhere else on paper or on screen — so it points at nothing. \
                     Receipt: {receipt:?}"
                );
            }
        }
    }

    // ANTI-VACUITY. A parse that yielded no keys, or no receipts, would pass
    // every assertion above forever. Both must be non-trivial.
    assert!(
        keys_checked > 0,
        "no deck declared a key — the guard read nothing"
    );
    assert!(
        receipts_checked > 0,
        "no deck carried a receipt — the guard read nothing"
    );
}

/// The matcher is case-insensitive and word-bounded.
///
/// Pinned directly because the whole guard rests on it: what printed was `G3`,
/// uppercase, against lowercase keys in the file.
#[test]
fn the_key_matcher_ignores_case_and_respects_word_bounds() {
    assert!(names_key("Chuck's redirect after G3 — point 1.", "g3"));
    assert!(names_key("after g3 —", "g3"));

    // Not inside another token: `g1` must not fire on `g10`.
    assert!(!names_key("Chuck's redirect after g10 —", "g1"));
    // Not inside a word.
    assert!(!names_key("Interrogatory R1esponse", "r1"));
    // A tactic name is not a key, which is why S-6 passes.
    assert!(!names_key(
        "Chuck's redirect after the half-truth — point 1.",
        "g3"
    ));
}
