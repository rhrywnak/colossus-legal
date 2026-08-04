//! Tests for `scenario_link_options` — task 2.10.
//!
//! The short list is where the design's research lands ("limiting the options
//! produces a faster AND more accurate review"), so the ordering rules are the
//! thing worth testing: the anchor leads, the most-touched follow, the cap is the
//! stored one, and nothing appears twice.

use super::*;
use crate::domain::settings::Settings;

fn settings() -> Settings {
    Settings::for_test()
}

/// One accusation as the graph returns it.
fn row(id: &str, paragraph: &str, pool_items: i64) -> AllegationOptionRow {
    AllegationOptionRow {
        allegation_id: id.to_string(),
        summary: Some(format!("summary of {id}")),
        title: Some(format!("title of {id}")),
        paragraph: Some(paragraph.to_string()),
        count_number: Some(2),
        count_name: Some("Negligence".to_string()),
        pool_items,
    }
}

fn ids(options: &[AllegationOptionDto]) -> Vec<&str> {
    options.iter().map(|o| o.allegation_id.as_str()).collect()
}

// ─── The label ───────────────────────────────────────────────────────────────

#[test]
fn an_accusation_reads_in_complaint_language_with_its_paragraph() {
    let out = build_options(
        vec![row("a-41", "41", 0)],
        &["a-41".to_string()],
        &settings(),
    );
    assert_eq!(out.serving[0].label, "¶41 — summary of a-41");
}

/// The label rule is the SAME rule the card's bears-on line uses.
///
/// The two cannot be one function (different inputs — see `accusation_label`'s
/// doc), so they are pinned to each other here: summary preferred over title,
/// title over the bare id, paragraph prefixed when present.
#[test]
fn the_label_rule_matches_the_cards_own_accusation_rule() {
    // Summary wins over title.
    let mut r = row("a-41", "41", 0);
    assert_eq!(accusation_label(&r), "¶41 — summary of a-41");

    // Title is the fallback.
    r.summary = None;
    assert_eq!(accusation_label(&r), "¶41 — title of a-41");

    // The bare id is the last resort — poor, but honest, and actionable.
    r.title = None;
    assert_eq!(accusation_label(&r), "¶41 — a-41");

    // No paragraph, no prefix. A "¶ — " with nothing after the pilcrow would be
    // punctuation pretending to be a citation.
    r.paragraph = None;
    assert_eq!(accusation_label(&r), "a-41");
    r.paragraph = Some("   ".to_string());
    assert_eq!(accusation_label(&r), "a-41");
}

#[test]
fn the_count_line_reads_as_the_card_writes_it() {
    let mut r = row("a-41", "41", 0);
    assert_eq!(count_label(&r).as_deref(), Some("Count 2 — Negligence"));

    r.count_name = None;
    assert_eq!(count_label(&r).as_deref(), Some("Count 2"));

    r.count_number = None;
    r.count_name = Some("Negligence".to_string());
    assert_eq!(count_label(&r).as_deref(), Some("Negligence"));

    r.count_name = None;
    assert!(count_label(&r).is_none(), "no count, no chip");
}

#[test]
fn the_filter_haystack_covers_the_label_and_the_count_and_is_lowercased() {
    let out = build_options(vec![row("a-41", "41", 1)], &[], &settings());
    let option = &out.serving[0];
    assert!(option.filter_text.contains("summary of a-41"));
    assert!(
        option.filter_text.contains("negligence"),
        "{}",
        option.filter_text
    );
    assert_eq!(option.filter_text, option.filter_text.to_lowercase());
}

// ─── The short list ──────────────────────────────────────────────────────────

/// The scenario's own anchor leads, whatever its pool count.
///
/// Measured on DEV: S-2's anchor is ¶41 and its pool touches 22 accusations. On
/// the day a scenario is created its anchor has no evidence at all, and it must
/// still be the first thing offered — it is the accusation the scenario exists to
/// answer.
#[test]
fn the_anchor_leads_the_short_list_even_with_no_evidence_behind_it() {
    let rows = vec![
        row("a-41", "41", 0),
        row("a-55", "55", 11),
        row("a-61", "61", 6),
    ];
    let out = build_options(rows, &["a-41".to_string()], &settings());
    assert_eq!(ids(&out.serving), vec!["a-41", "a-55", "a-61"]);
}

#[test]
fn the_rest_of_the_short_list_is_ordered_by_how_much_of_the_pool_touches_it() {
    let rows = vec![
        row("a-32", "32", 3),
        row("a-55", "55", 11),
        row("a-61", "61", 6),
    ];
    let out = build_options(rows, &[], &settings());
    assert_eq!(ids(&out.serving), vec!["a-55", "a-61", "a-32"]);
}

#[test]
fn an_accusation_nothing_touches_is_not_in_the_short_list() {
    // The short list is "what this scenario already serves". An untouched
    // accusation is a legitimate target — it is one click away under "Show all" —
    // but putting all 120 in the short list is the choice overload the design's
    // research names as the enemy.
    let rows = vec![row("a-55", "55", 11), row("a-99", "99", 0)];
    let out = build_options(rows, &[], &settings());
    assert_eq!(ids(&out.serving), vec!["a-55"]);
    assert_eq!(ids(&out.others), vec!["a-99"]);
}

#[test]
fn the_short_list_is_capped_by_the_stored_parameter() {
    let rows: Vec<AllegationOptionRow> = (1..=20)
        .map(|n| row(&format!("a-{n}"), &n.to_string(), i64::from(30 - n)))
        .collect();

    let mut narrow = settings();
    narrow.link_short_list_max = 3;
    let out = build_options(rows.clone(), &[], &narrow);
    assert_eq!(out.serving.len(), 3);

    // R4/§2b, as a test: the number is a stored parameter, so raising it needs no
    // rebuild. A `const 8` here would pass the test above and fail this one.
    let mut wide = settings();
    wide.link_short_list_max = 12;
    assert_eq!(build_options(rows, &[], &wide).serving.len(), 12);
}

#[test]
fn an_accusation_appears_in_exactly_one_of_the_two_lists() {
    // A human scanning the full list must not meet ¶55 twice and wonder which one
    // to tick.
    let rows = vec![
        row("a-41", "41", 0),
        row("a-55", "55", 11),
        row("a-99", "99", 0),
    ];
    let out = build_options(rows, &["a-41".to_string()], &settings());

    for served in &out.serving {
        assert!(
            !out.others
                .iter()
                .any(|o| o.allegation_id == served.allegation_id),
            "{} is in both lists",
            served.allegation_id
        );
    }
    assert_eq!(out.total, 3);
    assert_eq!(out.serving.len() + out.others.len(), out.total);
}

#[test]
fn everything_the_short_list_drops_is_still_reachable() {
    // The cap must never hide an accusation. It moves it behind "Show all".
    let rows: Vec<AllegationOptionRow> = (1..=20)
        .map(|n| row(&format!("a-{n}"), &n.to_string(), i64::from(30 - n)))
        .collect();
    let mut narrow = settings();
    narrow.link_short_list_max = 3;

    let out = build_options(rows, &[], &narrow);
    assert_eq!(out.serving.len() + out.others.len(), 20);
    assert_eq!(out.total, 20);
}

#[test]
fn the_full_list_keeps_the_graphs_paragraph_order() {
    // The rows arrive ordered by paragraph (numerically, in Cypher). `others` must
    // not re-sort them — the human is scanning a complaint, and ¶9 comes before
    // ¶10 there.
    let rows = vec![
        row("a-9", "9", 0),
        row("a-10", "10", 0),
        row("a-100", "100", 0),
    ];
    let out = build_options(rows, &[], &settings());
    assert_eq!(ids(&out.others), vec!["a-9", "a-10", "a-100"]);
}

#[test]
fn a_case_with_no_accusations_yields_two_empty_lists_and_a_zero() {
    // A real state (nothing extracted yet), reported as itself. The panel has its
    // own sentence for it — `empty_options_notice` — which is why this must not be
    // conflated with "the filter matched nothing".
    let out = build_options(Vec::new(), &[], &settings());
    assert!(out.serving.is_empty());
    assert!(out.others.is_empty());
    assert_eq!(out.total, 0);
    assert!(!out.wording.empty_options_notice.is_empty());
}

// ─── The wording ─────────────────────────────────────────────────────────────

#[test]
fn the_show_all_label_carries_the_real_count() {
    // The design drafted "Show all 38"; the measured complaint holds 120. The
    // count is filled from what the graph returned, so it cannot go stale.
    let rows: Vec<AllegationOptionRow> = (1..=7)
        .map(|n| row(&format!("a-{n}"), &n.to_string(), 0))
        .collect();
    let out = build_options(rows, &[], &settings());
    assert_eq!(out.wording.show_all_label, "Show all 7");
}

#[test]
fn every_word_the_panel_shows_comes_from_the_store() {
    // R4, as a test. Change the stored strings and the panel changes with them —
    // no rebuild, no literal anywhere in this module.
    let mut custom = settings();
    custom.wording.link_allegations_heading = "Which accusations?".to_string();
    custom.wording.link_save_label = "Save it".to_string();
    custom.wording.link_show_all_label = "All {count} of them".to_string();

    let out = build_options(vec![row("a-1", "1", 0)], &[], &custom);
    assert_eq!(out.wording.allegations_heading, "Which accusations?");
    assert_eq!(out.wording.save_label, "Save it");
    assert_eq!(out.wording.show_all_label, "All 1 of them");
}

// ─── The label index the card path uses ──────────────────────────────────────

#[test]
fn the_label_index_spells_an_accusation_the_way_the_panel_does() {
    // The card labels a human's links from this index, and the panel labels the
    // checkbox from `to_option`. Two spellings of one accusation on one screen
    // would read as two different accusations.
    let rows = vec![row("a-41", "41", 0)];
    let index = label_index(&rows);
    let out = build_options(rows, &["a-41".to_string()], &settings());

    assert_eq!(index.get("a-41"), Some(&out.serving[0].label));
}
