// Tests for `domain::wording_practice_print`.
//
// Same law as every sibling wording test file: a key declared to the boot loader
// with no row in the migration makes the backend REFUSE TO START, and reading the
// migration off disk is the only thing that catches it before a deploy takes DEV
// down (Rule 21).
//
// This block carries two reasons of its own.
//
// FIRST: these strings go on PAPER, and paper leaves the building. A screen with
// a wrong sentence is corrected by a redeploy and the old one is gone; a sheet
// Chuck has taken to a meeting is not. Everything here is therefore pinned to the
// migration character for character rather than merely checked for existence.
//
// SECOND: two of these rows exist to say what is ABSENT — the missing-kinds line
// and the hidden-questions line. If either goes blank the sheet does not look
// broken, it looks COMPLETE, and Chuck rewrites a question the deck already has.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The migration that seeds every row this module reads.
const SEED_MIGRATION: &str =
    "pipeline_migrations/20260822154321_practice_print_questions_wording.sql";

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to the
/// migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_QUESTIONS_LABEL, "🖨 Print questions"),
    (KEY_QUESTIONS_EMPTY_HINT, "No questions in this deck yet."),
    (KEY_NOW_LABEL, "Print"),
    (KEY_BACK_LABEL, "◂ Back to the deck"),
    (KEY_PAGE_TITLE, "Questions — {code}"),
    (KEY_SHEET_CROSS_TITLE, "The defense asks"),
    (KEY_SHEET_DIRECT_TITLE, "Chuck asks"),
    (KEY_SHEET_REDIRECT_TITLE, "Chuck, after the defense"),
    (KEY_SHEET_SUBTITLE_TEMPLATE, "{code} · “{title}”"),
    (
        KEY_SHEET_REDIRECT_SUBTITLE,
        "the redirects — each follows one of the defense's questions",
    ),
    (KEY_PRINTED_TEMPLATE, "printed {when}"),
    (
        KEY_DECK_AS_OF_TEMPLATE,
        "deck as of {date} · {n} of {m} questions",
    ),
    (
        KEY_HOWTO_CROSS,
        "In the order the defense would ask them at trial — the facts first, the conclusion last. Mark anything up. To enter your changes: Trial Prep → {code} → Practice → Edit the deck. The code in the blue box is the question's permanent name; it does not change when the deck is re-ordered.",
    ),
    (
        KEY_HOWTO_DIRECT,
        "Your direct — foundation first, then her three points. Each says which point it rests on.",
    ),
    (
        KEY_HOWTO_REDIRECT,
        "Each one repairs one defense question, so the question it follows is printed above it — a redirect read on its own means nothing.",
    ),
    (
        KEY_HOWTO_REDIRECT_DRAFTS,
        "These are drafts, written for you to rewrite.",
    ),
    (KEY_AFTER_TEMPLATE, "After the defense asks {key}: {question}"),
    (
        KEY_AFTER_MISSING,
        "The defense question this one repairs is no longer in the deck.",
    ),
    (KEY_FOOTER_TEMPLATE, "{code} · {sheet} · {n} questions"),
    (KEY_SHEET_NUMBER_TEMPLATE, "sheet {n} of {m}"),
    (KEY_MISSING_PREFIX, "This deck has"),
    (KEY_MISSING_CROSS, "no questions from the defense yet"),
    (KEY_MISSING_DIRECT, "no questions from Chuck yet"),
    (KEY_MISSING_REDIRECT, "no redirects"),
    (KEY_MISSING_JOINER, ", and"),
    (KEY_HIDDEN_TEMPLATE, "{n} questions are hidden and are not shown."),
];

impl PracticePrintWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_practice_print_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in PRACTICE_PRINT_WORDING_KEYS is in TEST_SEED")
    }

    /// The fixture as a key→value map, in the shape the store reads.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        TEST_SEED
            .iter()
            .map(|(key, value)| (*key, (*value).to_string()))
            .collect()
    }
}

/// Every declared key is seeded by the migration, with the value this build
/// expects.
///
/// The equality half is what makes this more than an existence check: a row whose
/// wording someone edited in the migration without editing the fixture would pass
/// a "the key is present" test and put a sentence on Chuck's paper that this build
/// never read.
#[test]
fn every_declared_key_is_seeded_with_the_value_this_build_expects() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .expect("the print wording migration is on disk");

    for key in PRACTICE_PRINT_WORDING_KEYS {
        let seeded = seeded_value_in(&sql, key).unwrap_or_else(|| {
            panic!(
                "{key} is declared to the boot loader but seeded by no migration \
                 — the backend would refuse to start"
            )
        });
        let expected = TEST_SEED
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| (*v).to_string())
            .unwrap_or_else(|| panic!("{key} is missing from TEST_SEED"));
        assert_eq!(
            seeded, expected,
            "the migration and the fixture disagree about {key}"
        );
    }
}

/// The fixture holds nothing the boot loader does not read.
///
/// ANTI-VACUITY, and not a formality: the test above walks
/// `PRACTICE_PRINT_WORDING_KEYS`, so a fixture entry for a key nobody declares
/// would never be visited. Without this, a key removed from the struct but left in
/// the fixture would look tested forever.
#[test]
fn the_fixture_declares_no_key_the_build_does_not_read() {
    for (key, _) in TEST_SEED {
        assert!(
            PRACTICE_PRINT_WORDING_KEYS.contains(key),
            "{key} is in the fixture but no field reads it"
        );
    }
    assert_eq!(
        TEST_SEED.len(),
        PRACTICE_PRINT_WORDING_KEYS.len(),
        "the fixture and the declared key list are different lengths"
    );
}

/// The joiner carries NO trailing space, because the store would eat it.
///
/// ## Why this is worth a test of its own
///
/// `app_settings` trims every value on the way in. A joiner seeded as `", and "`
/// arrives as `", and"`, and a renderer written to trust the stored space prints
/// *"no redirects, andno questions from Chuck yet"* on a sheet going to a meeting.
/// The rule is: the STORE holds the words, the RENDERER supplies the space — and
/// this is the assertion that keeps a future edit from quietly reintroducing one.
#[test]
fn the_missing_joiner_holds_no_edge_whitespace() {
    let wording = PracticePrintWording::for_test();
    assert_eq!(
        wording.missing_joiner,
        wording.missing_joiner.trim(),
        "the joiner must not carry its own spacing — the renderer supplies it"
    );
}

/// The sheet number says SHEET, and no row anywhere says "page N of".
///
/// ## The defect this pins, named
///
/// The mockup footed each sheet `page N of M`. That counts SHEETS while calling
/// them PAGES, and the two differ the moment a sheet runs long: S-7 has eight
/// direct questions, which is more than one piece of paper, and both halves would
/// have read "page 2 of 3". Physical pagination belongs to the browser; this
/// number belongs to the document. Roman's correction, 2026-08-22.
#[test]
fn the_document_counts_sheets_and_never_claims_to_count_pages() {
    let wording = PracticePrintWording::for_test();
    assert!(
        wording.sheet_number_template.contains("sheet"),
        "the template must name what it counts: {}",
        wording.sheet_number_template
    );
    for (key, value) in TEST_SEED {
        assert!(
            !value.to_lowercase().contains("page {n}"),
            "{key} counts pages, which this document cannot know: {value}"
        );
    }
}

/// No string on the paper carries a literal NUMBER.
///
/// ## What this can and cannot judge, stated honestly
///
/// The mockup's redirect instruction read "All five are drafts written for you to
/// rewrite" — a count written against S-5's 5/5/5, on a deck (S-7) that has two.
/// The sentence was wrong in the STORE, before any code read it.
///
/// A scan for number WORDS cannot catch that without also failing correct English:
/// this block legitimately says "each follows ONE of the defense's questions" and
/// "each ONE repairs ONE defense question". A first version of this test failed
/// both, which is how a guard gets weakened until it means nothing.
///
/// So this asserts the narrow, decidable half — **no digit appears in any printed
/// string** — which catches "All 5 are drafts" and "page 2 of 3". The word half is
/// enforced where it is actually decidable: the S-7 fixture (6 cross, 8 direct, 2
/// redirect) in the print helpers' own tests, where a rendered sheet either says a
/// wrong number or does not.
#[test]
fn no_printed_string_carries_a_literal_number() {
    for (key, value) in TEST_SEED {
        assert!(
            !value.chars().any(|c| c.is_ascii_digit()),
            "{key} carries a literal number, which differs per deck \
             (S-5 is 5/5/5, S-7 is 6/8/2): {value}"
        );
    }
}

/// A row the store cannot supply refuses the whole block, naming the key.
///
/// ## Why the ERROR path is worth a test on a builder this mechanical
///
/// Because the alternative to refusing is a sheet with a blank heading on it, and
/// paper leaves the building. There is no literal to fall back to (the wording
/// law, v2 §2b): a missing row means the store and this build disagree, and the
/// honest response is a boot refusal that names the key — not a printout with a
/// hole where "The defense asks" should be.
///
/// The generic `E` is what makes this testable at all: production hands the
/// builder a reader whose error refuses boot, and this hands it one that returns
/// a `String`, through the very same code path.
#[test]
fn a_missing_row_refuses_the_block_and_names_the_key() {
    let refused = build_practice_print_wording::<String>(|key| Err(format!("no row for {key}")));
    let reason = refused.expect_err("a reader that supplies nothing cannot build the block");
    assert!(
        reason.starts_with("no row for practice_print_"),
        "the refusal must name the key an operator has to add: {reason}"
    );

    // ANTI-VACUITY: the builder must not refuse a reader that CAN supply rows —
    // a guard that refuses everything is a wall, not a check.
    assert!(build_practice_print_wording::<String>(|key| {
        TEST_SEED
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| (*v).to_string())
            .ok_or_else(|| format!("{key} missing"))
    })
    .is_ok());
}
