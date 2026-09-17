// Tests for `domain::wording_war_room_summary`.
//
// Same justification as every sibling wording test file: a key declared to the
// boot loader with no row in the migration makes the backend REFUSE TO START
// (Rule 21, the disk/code consistency pattern).

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The one migration that seeds this block.
const SEED_MIGRATION: &str =
    "pipeline_migrations/20260917104454_simple_counts_reviewer_and_summary_wording.sql";

/// The seeded values, for TESTS ONLY.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_ANSWERED_LABEL, "Questions answered"),
    (KEY_ANSWERED_REST_TEMPLATE, "of {total} · {pct}%"),
    (KEY_UNANSWERED_LABEL, "Unanswered questions"),
    (KEY_REVIEW_LABEL, "Answers requiring review"),
    (KEY_CANDIDATES_LABEL, "Candidates to rule"),
    (KEY_OWNER_MARIE, "Marie"),
    (KEY_OWNER_ROMAN, "Roman"),
    (
        KEY_UNANSWERED_CONTEXT_TEMPLATE,
        "across {n} scenarios · {codes} untouched",
    ),
    (
        KEY_UNANSWERED_CONTEXT_ONE,
        "across {n} scenario · {codes} untouched",
    ),
    (
        KEY_UNANSWERED_CONTEXT_NONE_UNTOUCHED,
        "across {n} scenarios",
    ),
    (
        KEY_UNANSWERED_CONTEXT_NONE_UNTOUCHED_ONE,
        "across {n} scenario",
    ),
    (
        KEY_REVIEW_CONTEXT_TEMPLATE,
        "oldest waiting since {date} · {code} has {n}",
    ),
    (KEY_CANDIDATES_PILE_TEMPLATE, "{codes} {n}"),
    (KEY_LIST_JOINER, "·"),
    (KEY_TIE_JOINER, "&"),
    (KEY_CODE_JOINER, ","),
    (KEY_UNANSWERED_ZERO, "every question answered"),
    (KEY_REVIEW_ZERO, "nothing waiting"),
    (KEY_CANDIDATES_ZERO, "nothing to rule"),
];

impl WarRoomSummaryWording {
    /// The fixture, built through the PRODUCTION builder.
    pub fn for_test() -> Self {
        build_war_room_summary_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in WAR_ROOM_SUMMARY_WORDING_KEYS is in TEST_SEED")
    }

    /// The fixture as a key→value map, in the shape the store reads.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        TEST_SEED
            .iter()
            .map(|(key, value)| (*key, (*value).to_string()))
            .collect()
    }
}

fn echo(key: &str) -> Result<String, std::convert::Infallible> {
    Ok(key.to_string())
}

/// Every field reads the key it claims to.
#[test]
fn every_field_reads_its_own_key() {
    let w = build_war_room_summary_wording(echo).expect("infallible read");
    assert_eq!(w.answered_label, KEY_ANSWERED_LABEL);
    assert_eq!(w.answered_rest_template, KEY_ANSWERED_REST_TEMPLATE);
    assert_eq!(w.unanswered_label, KEY_UNANSWERED_LABEL);
    assert_eq!(w.review_label, KEY_REVIEW_LABEL);
    assert_eq!(w.candidates_label, KEY_CANDIDATES_LABEL);
    assert_eq!(w.owner_marie, KEY_OWNER_MARIE);
    assert_eq!(w.owner_roman, KEY_OWNER_ROMAN);
    assert_eq!(
        w.unanswered_context_template,
        KEY_UNANSWERED_CONTEXT_TEMPLATE
    );
    assert_eq!(w.unanswered_context_one, KEY_UNANSWERED_CONTEXT_ONE);
    assert_eq!(
        w.unanswered_context_none_untouched,
        KEY_UNANSWERED_CONTEXT_NONE_UNTOUCHED
    );
    assert_eq!(
        w.unanswered_context_none_untouched_one,
        KEY_UNANSWERED_CONTEXT_NONE_UNTOUCHED_ONE
    );
    assert_eq!(w.review_context_template, KEY_REVIEW_CONTEXT_TEMPLATE);
    assert_eq!(w.candidates_pile_template, KEY_CANDIDATES_PILE_TEMPLATE);
    assert_eq!(w.list_joiner, KEY_LIST_JOINER);
    assert_eq!(w.tie_joiner, KEY_TIE_JOINER);
    assert_eq!(w.code_joiner, KEY_CODE_JOINER);
    assert_eq!(w.unanswered_zero, KEY_UNANSWERED_ZERO);
    assert_eq!(w.review_zero, KEY_REVIEW_ZERO);
    assert_eq!(w.candidates_zero, KEY_CANDIDATES_ZERO);
}

/// Every declared key is seeded, with the value this build expects; and the
/// fixture holds nothing the boot loader does not read (anti-vacuity).
#[test]
fn every_declared_key_is_seeded_with_the_value_this_build_expects() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .unwrap_or_else(|_| panic!("{SEED_MIGRATION} is on disk"));
    let fixture = WarRoomSummaryWording::for_test_values();
    for key in WAR_ROOM_SUMMARY_WORDING_KEYS {
        let seeded = seeded_value_in(&sql, key)
            .unwrap_or_else(|| panic!("{key} is declared but no migration seeds it"));
        assert_eq!(
            fixture.get(*key),
            Some(&seeded),
            "the migration and the fixture disagree about {key}"
        );
    }
    assert_eq!(TEST_SEED.len(), WAR_ROOM_SUMMARY_WORDING_KEYS.len());
}

/// The keys are distinct, and none collides with the card block's.
#[test]
fn the_keys_are_distinct_and_do_not_collide_with_the_card_block() {
    let mut sorted = WAR_ROOM_SUMMARY_WORDING_KEYS.to_vec();
    sorted.sort_unstable();
    let count = sorted.len();
    sorted.dedup();
    assert_eq!(sorted.len(), count, "two summary wording keys collide");
    for key in WAR_ROOM_SUMMARY_WORDING_KEYS {
        assert!(
            !crate::domain::wording_war_room::WAR_ROOM_WORDING_KEYS.contains(key),
            "{key} is also a war-room card key"
        );
    }
}

/// The templates carry the placeholders the page fills; the plain words carry none.
#[test]
fn templates_carry_their_placeholders_and_labels_carry_none() {
    let w = WarRoomSummaryWording::for_test();
    for (name, value, needed) in [
        (
            "answered_rest_template",
            &w.answered_rest_template,
            &["{total}", "{pct}"][..],
        ),
        (
            "unanswered_context_template",
            &w.unanswered_context_template,
            &["{n}", "{codes}"][..],
        ),
        (
            "unanswered_context_one",
            &w.unanswered_context_one,
            &["{n}", "{codes}"][..],
        ),
        (
            "unanswered_context_none_untouched",
            &w.unanswered_context_none_untouched,
            &["{n}"][..],
        ),
        (
            "unanswered_context_none_untouched_one",
            &w.unanswered_context_none_untouched_one,
            &["{n}"][..],
        ),
        (
            "review_context_template",
            &w.review_context_template,
            &["{date}", "{code}", "{n}"][..],
        ),
        (
            "candidates_pile_template",
            &w.candidates_pile_template,
            &["{codes}", "{n}"][..],
        ),
    ] {
        for placeholder in needed {
            assert!(
                value.contains(placeholder),
                "{name} lost {placeholder}: {value}"
            );
        }
    }
    for (name, value) in [
        ("owner_marie", &w.owner_marie),
        ("owner_roman", &w.owner_roman),
        ("unanswered_zero", &w.unanswered_zero),
        ("review_zero", &w.review_zero),
        ("candidates_zero", &w.candidates_zero),
        ("list_joiner", &w.list_joiner),
        ("tie_joiner", &w.tie_joiner),
        ("code_joiner", &w.code_joiner),
    ] {
        assert!(
            !value.contains('{'),
            "{name} is plain and must carry no placeholder: {value}"
        );
    }
}
