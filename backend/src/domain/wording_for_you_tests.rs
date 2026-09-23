// Tests for `domain::wording_for_you`.
//
// The same shape, and the same single justification, as every sibling wording
// test file: a key declared to the boot loader with no row in the migration
// makes the backend REFUSE TO START. Reading the migration off disk is the only
// thing that catches that before a deploy does (Rule 21, the disk/code
// consistency pattern).
//
// This block carries a second reason of its own. Its templates are the only
// thing that tells one KIND of waiting item from another on screen — "on your
// answer of Mon 21 Sep" against "changed the question" — and a row edited to
// lose its placeholder renders a confident sentence with the fact removed.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// Every migration that seeds a row of this block, oldest first.
const SEED_MIGRATIONS: &[&str] = &[
    "pipeline_migrations/20260922165053_for_you_page_wording.sql",
    // L3: the deck row, and a reply as one row of the list.
    "pipeline_migrations/20260922231242_for_you_l3_deck_threshold_replies_and_board_4_marks.sql",
];

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_TITLE, "For you"),
    (KEY_SUBTITLE_WITNESS, "{who}'s notes on your answers. Newest first. A row clears when you open it."),
    (KEY_SUBTITLE_REVIEWER, "{who}'s new answers and her notes. Newest first. A row clears when you open it."),
    (KEY_NOT_YOUR_LIST, "Nothing is waiting for you here. This page carries the answers and notes that pass between the witness and the reviewers."),
    (KEY_TAB_UNREAD_TEMPLATE, "Unread · {count}"),
    (KEY_TAB_EVERYTHING_TEMPLATE, "Everything · {count}"),
    (KEY_GROUP_TODAY, "TODAY"),
    (KEY_GROUP_YESTERDAY, "YESTERDAY"),
    (KEY_GROUP_EARLIER, "EARLIER"),
    (KEY_DECK_LINE_TEMPLATE, "{code} · {deck} — “{question}”"),
    (KEY_DECK_LINE_NO_QUESTION_TEMPLATE, "{code} · {deck}"),
    (KEY_BODY_ANSWER_TEMPLATE, "Answered: “{text}”"),
    (KEY_BODY_CHANGE_TEMPLATE, "Now reads: “{text}”"),
    (KEY_DECK_BODY_TEMPLATE, "{count} answers waiting"),
    (KEY_DECK_BODY_ONE, "{count} answer waiting"),
    (KEY_DECK_BYLINE_TEMPLATE, "oldest {when}"),
    (KEY_BODY_REPLY_TEMPLATE, "Reply to “{parent}”: “{text}”"),
    (KEY_BYLINE_TEMPLATE, "{who} · {what}"),
    (KEY_BYLINE_NOTE_ON_ANSWER_WITNESS, "on your answer of {when}"),
    (KEY_BYLINE_NOTE_ON_ANSWER_REVIEWER, "note on her answer"),
    (KEY_BYLINE_NOTE_ON_QUESTION, "on the question"),
    (KEY_BYLINE_ANSWER, "new answer"),
    (KEY_BYLINE_CHANGE, "changed the question"),
    (KEY_BYLINE_READ_SUFFIX_TEMPLATE, "· read by you {when}"),
    (KEY_EMPTY_TITLE, "Nothing waiting for you."),
    (KEY_EMPTY_HINT_TEMPLATE, "{who}'s notes and answers appear here as they arrive."),
    (KEY_EMPTY_LAST_TEMPLATE, "Last one: {when}."),
    (KEY_WITNESS_NAME, "Marie"),
    (KEY_UNKNOWN_AUTHOR, "Someone")
];

impl ForYouWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_for_you_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| {
                    format!(
                        "{key} is declared to the boot loader but missing from this file's \
                         TEST_SEED fixture — add the value the migration seeds"
                    )
                })
        })
        .expect("every key in FOR_YOU_WORDING_KEYS is in TEST_SEED")
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
/// The equality half is what makes this more than an existence check: a row
/// whose wording someone edited in the migration without editing the fixture
/// would pass a "the key is present" test and ship a screen saying something
/// this build never read.
#[test]
fn every_declared_key_is_seeded_with_the_value_this_build_expects() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sources: Vec<String> = SEED_MIGRATIONS
        .iter()
        .map(|file| {
            std::fs::read_to_string(root.join(file))
                .unwrap_or_else(|cause| panic!("{file} is not on disk: {cause}"))
        })
        .collect();

    for key in FOR_YOU_WORDING_KEYS {
        let seeded = sources
            .iter()
            .find_map(|sql| seeded_value_in(sql, key))
            .unwrap_or_else(|| {
                panic!(
                    "{key} is declared to the boot loader but seeded by no migration \
                     — the backend would refuse to start"
                )
            });
        let expected = TEST_SEED
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| (*v).to_string())
            .unwrap_or_else(|| {
                panic!(
                    "{key} is declared to the boot loader but missing from this file's \
                     TEST_SEED fixture — add the value the migration seeds"
                )
            });
        assert_eq!(
            seeded, expected,
            "the migration and the fixture disagree about {key}"
        );
    }
}

/// The fixture holds nothing the boot loader does not read.
///
/// ANTI-VACUITY: the test above walks `FOR_YOU_WORDING_KEYS`, so a key dropped
/// from that list stops being checked by it and nothing complains. This one
/// fails instead, naming the orphan.
#[test]
fn the_fixture_holds_no_key_the_boot_loader_does_not_read() {
    for (key, _) in TEST_SEED {
        assert!(
            FOR_YOU_WORDING_KEYS.contains(key),
            "{key} is in TEST_SEED but declared to nothing — either the boot \
             loader stopped reading it or the fixture was never cleaned up"
        );
    }
    assert_eq!(
        TEST_SEED.len(),
        FOR_YOU_WORDING_KEYS.len(),
        "the fixture and the declared list must be the same size"
    );
}

/// Every template carries the placeholders its caller fills.
///
/// A byline template that lost `{what}` renders "Chuck ·" with nothing after
/// it: not a crash, not a failure anywhere else, and exactly the small
/// wrongness that makes a reader stop trusting the page. Worse, a row would
/// then no longer say what KIND of thing it is, which is ruling Q1 undone by a
/// Settings edit.
#[test]
fn every_template_carries_its_placeholders() {
    const REQUIRED: &[(&str, &[&str])] = &[
        (KEY_SUBTITLE_WITNESS, &["{who}"]),
        (KEY_SUBTITLE_REVIEWER, &["{who}"]),
        (KEY_TAB_UNREAD_TEMPLATE, &["{count}"]),
        (KEY_TAB_EVERYTHING_TEMPLATE, &["{count}"]),
        (KEY_DECK_LINE_TEMPLATE, &["{code}", "{deck}", "{question}"]),
        (KEY_DECK_LINE_NO_QUESTION_TEMPLATE, &["{code}", "{deck}"]),
        (KEY_BODY_ANSWER_TEMPLATE, &["{text}"]),
        (KEY_BODY_CHANGE_TEMPLATE, &["{text}"]),
        (KEY_DECK_BODY_TEMPLATE, &["{count}"]),
        (KEY_DECK_BODY_ONE, &["{count}"]),
        (KEY_DECK_BYLINE_TEMPLATE, &["{when}"]),
        (KEY_BODY_REPLY_TEMPLATE, &["{parent}", "{text}"]),
        (KEY_BYLINE_TEMPLATE, &["{who}", "{what}"]),
        (KEY_BYLINE_NOTE_ON_ANSWER_WITNESS, &["{when}"]),
        (KEY_BYLINE_READ_SUFFIX_TEMPLATE, &["{when}"]),
        (KEY_EMPTY_HINT_TEMPLATE, &["{who}"]),
        (KEY_EMPTY_LAST_TEMPLATE, &["{when}"]),
    ];
    let seed: HashMap<&str, String> = ForYouWording::for_test_values();
    for (key, placeholders) in REQUIRED {
        let value = seed
            .get(key)
            .unwrap_or_else(|| panic!("{key} is in the fixture"));
        for placeholder in *placeholders {
            assert!(
                value.contains(placeholder),
                "{key} must keep {placeholder}: {value}"
            );
        }
    }
}

/// No stored value carries a leading or trailing space.
///
/// The store TRIMS every value on the way in, so a template written with one
/// would silently lose it and the sentence would come out joined wrong. The
/// renderer supplies the joining space; the row supplies the words. This is why
/// the read suffix begins with a middle dot rather than a space.
#[test]
fn no_stored_value_leans_on_a_space_the_store_would_trim() {
    for (key, value) in TEST_SEED {
        assert_eq!(
            value.trim(),
            *value,
            "{key} carries an edge space the store would trim"
        );
    }
}
