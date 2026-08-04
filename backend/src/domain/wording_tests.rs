// Tests for `domain::wording` — task 2.10, the configuration law extended to text.
//
// Two things need proving here, and the second is the one that would rot:
//
//   1. The placeholder machinery does what the refusal promises.
//   2. The test fixture still says what the MIGRATION seeds. A fixture compared
//      only to itself proves nothing — the same argument
//      `the_fixtures_carry_the_values_the_migration_actually_seeds` makes for the
//      seven numbers.

use super::*;
use crate::domain::link_cut::LinkCut;

// ── The placeholder rules ────────────────────────────────────────────────────

#[test]
fn a_template_that_keeps_its_placeholders_is_accepted() {
    assert!(
        missing_placeholders(KEY_SUMMARY_TEMPLATE, "Linked to {allegations} — {cut}").is_empty()
    );
    assert!(missing_placeholders(KEY_PROGRESS_TEMPLATE, "{linked}/{total}").is_empty());
    assert!(missing_placeholders(KEY_SHOW_ALL_LABEL, "All {count} of them").is_empty());
}

#[test]
fn a_template_that_drops_a_placeholder_is_named_as_missing_it() {
    // The defect this exists to stop: a grammatical sentence with the FACT
    // removed. "You linked this to  · they'll use it against us."
    let missing = missing_placeholders(KEY_SUMMARY_TEMPLATE, "You linked this to it · {cut}.");
    assert_eq!(missing, vec!["{allegations}"]);

    let both = missing_placeholders(KEY_SUMMARY_TEMPLATE, "You linked this to something.");
    assert_eq!(both, vec!["{allegations}", "{cut}"]);
}

#[test]
fn a_key_with_no_required_placeholders_never_reports_one() {
    // Most keys are plain sentences. They must not be dragged into the template
    // rules by accident — "Cancel" is a complete label.
    assert!(missing_placeholders(KEY_CANCEL_LABEL, "Cancel").is_empty());
    assert!(missing_placeholders(KEY_PANEL_INTRO, "anything at all").is_empty());
    assert!(missing_placeholders("not_a_key_at_all", "").is_empty());
}

#[test]
fn every_key_with_required_placeholders_is_a_real_key() {
    // Anti-drift: a requirement filed against a renamed key would silently stop
    // guarding anything, because `missing_placeholders` returns empty for a key
    // it does not find.
    for (key, required) in REQUIRED_PLACEHOLDERS {
        assert!(
            WORDING_KEYS.contains(key),
            "{key} has placeholder requirements but is not a stored wording key"
        );
        assert!(!required.is_empty(), "{key} declares an empty requirement");
    }
}

#[test]
fn the_seeded_defaults_satisfy_their_own_placeholder_rules() {
    // If they did not, the migration would seed a store that the write path would
    // refuse — a value nobody could edit back to its own default.
    let values = Wording::for_test_values();
    for (key, _) in REQUIRED_PLACEHOLDERS {
        let seeded = values.get(key).expect("a seeded value for every key");
        assert!(
            missing_placeholders(key, seeded).is_empty(),
            "the seeded default for {key} does not satisfy its own rule: {seeded}"
        );
    }
}

// ── Rendering ────────────────────────────────────────────────────────────────

#[test]
fn rendering_fills_every_placeholder_it_is_given() {
    let out = render(
        "You linked this to {allegations} · {cut}.",
        &[
            ("allegations", "\u{b6}41 — refused to divide the property"),
            ("cut", "they'll use it against us"),
        ],
    );
    assert_eq!(
        out,
        "You linked this to \u{b6}41 — refused to divide the property · they'll use it against us."
    );
}

#[test]
fn a_placeholder_nobody_supplied_stays_visible_rather_than_vanishing() {
    // Deliberate, and the honest end of the chain: a template can only lose a
    // value through an edit the write path refuses, so a `{cut}` reaching a screen
    // means somebody edited the store around the API. Substituting "" would give a
    // confident sentence with a hole in it and nothing to notice.
    let out = render(
        "You linked this to {allegations} · {cut}.",
        &[("allegations", "\u{b6}41")],
    );
    assert!(out.contains("{cut}"), "{out}");
}

#[test]
fn rendering_a_template_with_nothing_to_fill_returns_it_unchanged() {
    assert_eq!(render("Cancel", &[]), "Cancel");
}

#[test]
fn a_value_containing_braces_is_not_re_scanned() {
    // Replacement walks the placeholder list once, so a value that happens to
    // contain `{cut}` cannot be substituted into by a later pass. An accusation's
    // own text is arbitrary human prose and must never be treated as a template.
    let out = render(
        "{allegations} · {cut}",
        &[
            ("allegations", "\u{b6}9 — the {cut} clause"),
            ("cut", "it supports us"),
        ],
    );
    assert_eq!(out, "\u{b6}9 — the {cut} clause · it supports us");
}

// ── The enum meets its two sentence forms ────────────────────────────────────

#[test]
fn each_cut_gets_its_own_stored_sentence_form() {
    let wording = Wording::for_test();
    assert_eq!(wording.cut_phrase(LinkCut::Supports), "it supports us");
    assert_eq!(
        wording.cut_phrase(LinkCut::Against),
        "they'll use it against us"
    );
}

#[test]
fn the_sentence_form_is_a_stored_row_and_not_a_lowercased_label() {
    // The rule stated as a test: nothing in code changes the case of a word a
    // human wrote. If someone deleted the phrase rows and lowercased the labels,
    // this is what would fail.
    let mut wording = Wording::for_test();
    wording.link_cut_supports_phrase = "it helps our side".to_string();
    assert_eq!(wording.cut_phrase(LinkCut::Supports), "it helps our side");
    assert_ne!(
        wording.cut_phrase(LinkCut::Supports),
        wording.link_cut_supports_label.to_lowercase()
    );
}

// ── The fixture is checked against the MIGRATION, not against itself ─────────

/// The migrations that between them decide what the store holds.
///
/// TWO of them now, and the order matters: 2.10 seeds the rows and 2.12
/// CORRECTS one. `link_save_label` was seeded 'Save and next' and is updated to
/// 'Save', so the effective value is the INSERT as amended — a reader that
/// stopped at the INSERT would assert this fixture into disagreeing with the
/// product, which is the exact failure task 2.12 item A calls "a lying button".
const SEED_MIGRATION: &str =
    "pipeline_migrations/20260804132730_create_evidence_allegation_links.sql";
const CORRECTION_MIGRATION: &str =
    "pipeline_migrations/20260804162538_card_and_facts_usability_wording.sql";

/// Pull one key's seeded value out of the migration's INSERT.
///
/// ## Why this handles doubled quotes and the 1.6 version did not
///
/// SQL escapes a literal apostrophe by doubling it, and these are sentences:
/// "isn''t", "They''ll", "can''t". A scan that stopped at the first `'` would
/// report `link_panel_intro` as "This statement isn" and the comparison below
/// would fail on every row that contains an apostrophe — which is most of the
/// interesting ones. So this walks the literal, treating `''` as one character.
fn seeded_value_in(sql: &str, key: &str) -> Option<String> {
    let marker = format!("('{key}',");
    let at = sql.find(&marker)?;
    let rest = &sql[at + marker.len()..];
    // The value literal starts at the first quote after the key.
    let open = rest.find('\'')?;
    let mut out = String::new();
    let mut chars = rest[open + 1..].chars();
    while let Some(c) = chars.next() {
        if c != '\'' {
            out.push(c);
            continue;
        }
        // A doubled quote is one literal apostrophe; a single one ends the value.
        match chars.next() {
            Some('\'') => out.push('\''),
            _ => return Some(out),
        }
    }
    None
}

/// The value a key ends up with: seeded by one migration, possibly corrected by
/// a later one.
///
/// Deliberately crude, like its INSERT sibling: it looks for
/// `SET value = '…'` inside a statement whose WHERE names this key. A migration
/// that corrected a row some other way would not be found — and the count
/// assertion below is what would catch that, by leaving the fixture disagreeing.
fn corrected_value_in(sql: &str, key: &str) -> Option<String> {
    let at = sql.find(&format!("key           = '{key}'"))?;
    // The SET clause precedes the WHERE in an UPDATE, so scan backwards.
    let before = &sql[..at];
    let set = before.rfind("SET value         = '")?;
    let rest = &before[set + "SET value         = '".len()..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

/// The value the store actually ends up holding for one key.
fn effective_value(seed_sql: &str, correction_sql: &str, key: &str) -> Option<String> {
    corrected_value_in(correction_sql, key).or_else(|| seeded_value_in(seed_sql, key))
}

#[test]
fn the_wording_fixture_carries_the_values_the_migration_actually_seeds() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .expect("the wording migration is on disk");
    let correction = std::fs::read_to_string(root.join(CORRECTION_MIGRATION))
        .expect("the correction migration is on disk");

    let fixture = Wording::for_test_values();
    let mut checked = 0usize;

    for key in WORDING_KEYS {
        let seeded = effective_value(&sql, &correction, key)
            .or_else(|| seeded_value_in(&correction, key))
            .unwrap_or_else(|| panic!("{key} is not seeded by either migration"));
        let in_fixture = fixture
            .get(*key)
            .unwrap_or_else(|| panic!("{key} is missing from the test fixture"));

        assert_eq!(
            in_fixture, &seeded,
            "the fixture has {key} = '{in_fixture}' but the migration seeds \
             '{seeded}'. One moved without the other, and every test that asserts \
             on this wording is now describing something the product does not say."
        );
        checked += 1;
    }

    // Anti-vacuity: a parsing change that stopped finding rows would otherwise
    // make this test pass while comparing nothing.
    assert_eq!(
        checked, 28,
        "all twenty-eight stored strings must be compared"
    );
    assert_eq!(WORDING_KEYS.len(), 28);
}

// ── Item A's correction: the one that can ship a lying button ────────────────

/// The save label no longer promises to move the human anywhere.
///
/// THE POINT OF ITEM A, as an assertion on the stored text rather than on the
/// code: saving a link now leaves the selection where it is, so a label reading
/// "and next" would be the button describing behaviour it no longer has. This
/// checks the EFFECTIVE value — the seed as amended by the correction — so it
/// fails if the UPDATE is dropped, mis-keyed, or silently matched nothing.
#[test]
fn the_stored_save_label_does_not_promise_navigation() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let seed = std::fs::read_to_string(root.join(SEED_MIGRATION)).expect("seed on disk");
    let correction =
        std::fs::read_to_string(root.join(CORRECTION_MIGRATION)).expect("correction on disk");

    let effective = effective_value(&seed, &correction, KEY_SAVE_LABEL).expect("a value");
    assert_eq!(effective, "Save");
    assert!(
        !effective.to_lowercase().contains("next"),
        "the button no longer moves anyone: {effective}"
    );
    // And the seed really did hold the old text — otherwise this test would pass
    // by accident on a migration that never needed correcting.
    assert_eq!(
        seeded_value_in(&seed, KEY_SAVE_LABEL).as_deref(),
        Some("Save and next"),
        "the 2.10 seed is what makes the UPDATE necessary"
    );
}

/// The correction fires ONLY on a row still holding the old default.
///
/// A human may have edited this label since 2.10 shipped, and their words are
/// not ours to overwrite. The guard is the VALUE itself rather than
/// `updated_by`: somebody who edited the label and set it back would be wrongly
/// protected by an attribution check, and somebody who changed another column
/// would be wrongly overwritten by one.
#[test]
fn the_save_label_correction_only_fires_on_the_untouched_default() {
    let sql = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CORRECTION_MIGRATION),
    )
    .expect("correction on disk");

    let stmt = sql
        .split_once("UPDATE app_settings")
        .map(|(_, rest)| rest.split_once(';').map(|(s, _)| s).unwrap_or(rest))
        .expect("the correction is an UPDATE");

    assert!(
        stmt.contains("value         = 'Save and next'"),
        "the WHERE must name the old value: {stmt}"
    );
    assert!(
        stmt.contains("default_value = 'Save and next'"),
        "and the old default, so a half-edited row is left alone: {stmt}"
    );
    assert!(
        !stmt.contains("updated_by"),
        "a migration is not a person; the attribution column stays as it was: {stmt}"
    );
}

/// The correction moves the DEFAULT as well as the value.
///
/// Otherwise the Settings page advertises "Default Save and next" for a default
/// that no longer exists, and the stored row falls out of step with
/// `Wording::for_test`.
#[test]
fn the_correction_moves_the_default_as_well_as_the_value() {
    let sql = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CORRECTION_MIGRATION),
    )
    .expect("correction on disk");
    assert!(sql.contains("SET value         = 'Save'"));
    assert!(sql.contains("default_value = 'Save',"));
}

#[test]
fn the_seed_parser_reads_an_escaped_apostrophe_as_one_character() {
    // The parser is the only thing standing between the test above and a false
    // green, so it gets its own test rather than being trusted.
    let sql = "VALUES ('a_key', 'it isn''t over', 'text',";
    assert_eq!(
        seeded_value_in(sql, "a_key").as_deref(),
        Some("it isn't over")
    );
}

#[test]
fn no_stored_string_is_blank() {
    // A blank label is an invisible control. The store refuses one on write; this
    // pins the DEFAULTS, which the write path never sees.
    for (key, value) in Wording::for_test_values() {
        assert!(!value.trim().is_empty(), "{key} seeds a blank string");
    }
}
