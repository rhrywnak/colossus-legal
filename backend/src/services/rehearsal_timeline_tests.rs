//! Tests for [`super`] — the conditional timeline.
//!
//! The block exists to show the accusation repeating over time. Every test here
//! is about the condition: when it may draw, when it must not, and what it says
//! instead. Drawing a "timeline" the placed set cannot support is the failure —
//! it would put a shape on the page that the record does not have.

use super::*;
use crate::domain::human_authored::HumanFactKind;
use crate::domain::settings::Settings;
use crate::services::scenario_accusation::{derive, StoredJudgment};
use std::collections::HashSet;

fn fact(id: &str, quote: &str, who: &str, when: Option<&str>) -> RehearsalFactRow {
    RehearsalFactRow {
        graph_node_id: id.to_string(),
        quote: Some(quote.to_string()),
        speaker: Some(who.to_string()),
        statement_type: Some("attorney_argument".to_string()),
        question: None,
        occurred_on: when.map(str::to_string),
        document_id: Some("doc-1".to_string()),
        document_title: Some("Hearing".to_string()),
        page: Some(1),
    }
}

fn facts(rows: Vec<RehearsalFactRow>) -> HashMap<String, RehearsalFactRow> {
    rows.into_iter()
        .map(|r| (r.graph_node_id.clone(), r))
        .collect()
}

fn instance(anchor: &str) -> StoredJudgment {
    StoredJudgment {
        kind: HumanFactKind::AccusationInstance,
        anchor_graph_node_id: anchor.to_string(),
        answers_graph_node_id: None,
    }
}

fn pairing(anchor: &str, answer: &str) -> StoredJudgment {
    StoredJudgment {
        kind: HumanFactKind::AnswerPairing,
        anchor_graph_node_id: anchor.to_string(),
        answers_graph_node_id: Some(answer.to_string()),
    }
}

fn timeline(
    judgments: &[StoredJudgment],
    ids: &[&str],
    rows: HashMap<String, RehearsalFactRow>,
) -> RehearsalTimeline {
    let settings = Settings::for_test();
    let included: HashSet<String> = ids.iter().map(|s| (*s).to_string()).collect();
    let state = derive(judgments, &included);
    build_timeline(&state, &rows, &settings)
}

// ── When it draws ────────────────────────────────────────────────────────────

#[test]
fn two_distinct_dates_are_enough_to_draw() {
    // The seeded threshold. Two points make a line; one makes a dot.
    let t = timeline(
        &[instance("ev-a"), instance("ev-b")],
        &["ev-a", "ev-b"],
        facts(vec![
            fact(
                "ev-a",
                "They refused.",
                "George Phillips",
                Some("2009-11-16"),
            ),
            fact("ev-b", "Again.", "George Phillips", Some("2009-12-15")),
        ]),
    );

    assert_eq!(t.entries.len(), 2);
    assert_eq!(t.notice, None);
}

#[test]
fn both_sides_are_interleaved_in_date_order() {
    // The design's shape: accusation, answer, accusation — so the repetition and
    // our responses read as one story rather than two lists.
    let t = timeline(
        &[instance("ev-a"), pairing("ev-a", "ev-answer")],
        &["ev-a", "ev-answer"],
        facts(vec![
            fact(
                "ev-a",
                "They refused.",
                "George Phillips",
                Some("2009-12-15"),
            ),
            fact("ev-answer", "I am open.", "Marie Awad", Some("2009-11-16")),
        ]),
    );

    assert_eq!(t.entries.len(), 2);
    // Chronological, so OUR earlier answer comes first — which is exactly the
    // point a witness makes with it.
    assert_eq!(t.entries[0].when, "16 Nov 2009");
    assert_eq!(t.entries[0].side, SIDE_OUR_ANSWER);
    assert_eq!(t.entries[1].side, SIDE_THEIR_WORDS);
}

#[test]
fn the_two_sides_are_tokens_a_client_can_style_without_reading_prose() {
    let t = timeline(
        &[instance("ev-a"), pairing("ev-a", "ev-answer")],
        &["ev-a", "ev-answer"],
        facts(vec![
            fact(
                "ev-a",
                "They refused.",
                "George Phillips",
                Some("2009-12-15"),
            ),
            fact("ev-answer", "I am open.", "Marie Awad", Some("2009-11-16")),
        ]),
    );
    let sides: HashSet<&str> = t.entries.iter().map(|e| e.side.as_str()).collect();
    assert_eq!(sides.len(), 2, "the two sides must be distinguishable");
}

#[test]
fn the_order_is_total_so_two_renders_cannot_disagree() {
    // Two entries on one date sort by their words. Without the tiebreak the order
    // would depend on map iteration, and a witness working down a list they expect
    // to stay put would lose their place between reloads.
    let rows = facts(vec![
        fact("ev-a", "Zebra.", "George Phillips", Some("2009-12-15")),
        fact("ev-b", "Apple.", "George Phillips", Some("2009-12-15")),
        fact("ev-c", "Later.", "George Phillips", Some("2010-01-01")),
    ]);
    let t = timeline(
        &[instance("ev-a"), instance("ev-b"), instance("ev-c")],
        &["ev-a", "ev-b", "ev-c"],
        rows,
    );

    let quotes: Vec<&str> = t.entries.iter().map(|e| e.quote.as_str()).collect();
    assert_eq!(quotes, vec!["Apple.", "Zebra.", "Later."]);
}

// ── When it refuses to draw ──────────────────────────────────────────────────

#[test]
fn one_distinct_date_draws_nothing_and_says_so() {
    // A timeline from one date is a dot. The block says what it is not showing
    // rather than rendering a shape the record does not have.
    let t = timeline(
        &[instance("ev-a"), instance("ev-b")],
        &["ev-a", "ev-b"],
        facts(vec![
            fact(
                "ev-a",
                "They refused.",
                "George Phillips",
                Some("2009-12-15"),
            ),
            fact("ev-b", "Again.", "George Phillips", Some("2009-12-15")),
        ]),
    );

    assert!(t.entries.is_empty());
    assert_eq!(
        t.notice.as_deref(),
        Some("Not enough dated items to draw a timeline — 0 of 2 carry no date.")
    );
}

#[test]
fn the_notice_counts_the_placed_items_that_carry_no_date() {
    // The design promises the block states how many items it is NOT showing. A
    // bare "not enough dates" hides the size of the gap.
    let t = timeline(
        &[instance("ev-a"), instance("ev-b"), instance("ev-c")],
        &["ev-a", "ev-b", "ev-c"],
        facts(vec![
            fact(
                "ev-a",
                "They refused.",
                "George Phillips",
                Some("2009-12-15"),
            ),
            fact("ev-b", "Again.", "George Phillips", None),
            fact("ev-c", "And again.", "George Phillips", None),
        ]),
    );

    assert!(t.entries.is_empty());
    assert_eq!(
        t.notice.as_deref(),
        Some("Not enough dated items to draw a timeline — 2 of 3 carry no date.")
    );
}

#[test]
fn nothing_placed_draws_nothing_and_still_says_so() {
    // The state S-2 is in today, measured: zero marked, zero paired. The block
    // must not be silent — an empty section and one that failed to load look
    // identical, and they are very different things.
    let t = timeline(&[], &[], facts(vec![]));
    assert!(t.entries.is_empty());
    assert_eq!(
        t.notice.as_deref(),
        Some("Not enough dated items to draw a timeline — 0 of 0 carry no date.")
    );
}

#[test]
fn the_threshold_reads_what_is_placed_and_never_the_included_pool() {
    // Ruled 2026-08-06, and the measurement that provoked it: S-2 has four
    // distinct dates in its included pool and zero placed items. A pool-based
    // threshold would have drawn an empty "timeline" here.
    let pool_is_rich = facts(vec![
        fact("ev-a", "One.", "George Phillips", Some("2009-10-02")),
        fact("ev-b", "Two.", "George Phillips", Some("2009-11-16")),
        fact("ev-c", "Three.", "George Phillips", Some("2009-12-15")),
        fact("ev-d", "Four.", "George Phillips", Some("2010-10-04")),
    ]);
    // Everything is INCLUDED; nothing is MARKED.
    let t = timeline(&[], &["ev-a", "ev-b", "ev-c", "ev-d"], pool_is_rich);

    assert!(t.entries.is_empty(), "nothing was placed, so nothing draws");
    assert_eq!(
        t.notice.as_deref(),
        Some("Not enough dated items to draw a timeline — 0 of 0 carry no date.")
    );
}

#[test]
fn a_statement_the_record_cannot_produce_is_not_a_timeline_entry() {
    // The six dead refs measured on S-2. They are named in the accusation block's
    // gap list; here they simply are not drawable.
    let t = timeline(
        &[instance("ev-a"), instance("ev-gone")],
        &["ev-a", "ev-gone"],
        facts(vec![fact(
            "ev-a",
            "They refused.",
            "George Phillips",
            Some("2009-12-15"),
        )]),
    );
    assert_eq!(
        t.notice.as_deref(),
        Some("Not enough dated items to draw a timeline — 0 of 1 carry no date.")
    );
}

// ── The person filter ────────────────────────────────────────────────────────

#[test]
fn the_filter_offers_everyone_who_appears_in_first_appearance_order() {
    // A reader reaches for the filter after MEETING a name in the list, so the
    // name they just read is near the top of the control. Alphabetical would put
    // one they have not met there instead.
    let t = timeline(
        &[
            instance("ev-a"),
            pairing("ev-a", "ev-answer"),
            instance("ev-c"),
        ],
        &["ev-a", "ev-answer", "ev-c"],
        facts(vec![
            fact(
                "ev-a",
                "They refused.",
                "George Phillips",
                Some("2009-12-15"),
            ),
            fact("ev-answer", "I am open.", "Marie Awad", Some("2009-11-16")),
            fact("ev-c", "Later.", "William B. Murphy", Some("2010-10-04")),
        ]),
    );

    assert_eq!(
        t.people,
        vec![
            "Marie Awad".to_string(),
            "George Phillips".to_string(),
            "William B. Murphy".to_string()
        ]
    );
}

#[test]
fn one_person_appearing_twice_is_listed_once() {
    let t = timeline(
        &[instance("ev-a"), instance("ev-b")],
        &["ev-a", "ev-b"],
        facts(vec![
            fact("ev-a", "One.", "George Phillips", Some("2009-11-16")),
            fact("ev-b", "Two.", "George Phillips", Some("2009-12-15")),
        ]),
    );
    assert_eq!(t.people, vec!["George Phillips".to_string()]);
}

#[test]
fn a_block_that_draws_nothing_offers_no_filter_options() {
    // A filter over an empty list is a control that cannot do anything, and a
    // control that cannot do anything reads as one that is broken.
    let t = timeline(&[], &[], facts(vec![]));
    assert!(t.people.is_empty());
    // Its words still travel, so the client never composes them itself.
    assert_eq!(t.filter_all_label, "Everyone");
    assert_eq!(t.filter_prompt, "Show");
}

// ── The order survived the date formatting (ruled 2026-08-06) ────────────────

#[test]
fn the_order_is_chronological_even_where_the_rendered_dates_sort_the_other_way() {
    // THE TRAP the raw sort key exists for. As strings, "1 Dec 2009" sorts BEFORE
    // "15 Nov 2009" — so formatting before sorting would put December ahead of
    // November, on the one block whose entire purpose is the chronology. Sorting
    // on the stored ISO value is what keeps time in order.
    let t = timeline(
        &[instance("ev-nov"), instance("ev-dec")],
        &["ev-nov", "ev-dec"],
        facts(vec![
            fact("ev-nov", "November.", "George Phillips", Some("2009-11-15")),
            fact("ev-dec", "December.", "George Phillips", Some("2009-12-01")),
        ]),
    );

    assert_eq!(t.entries.len(), 2);
    assert_eq!(
        t.entries[0].quote, "November.",
        "November comes first in time"
    );
    assert_eq!(t.entries[0].when, "15 Nov 2009");
    assert_eq!(t.entries[1].when, "1 Dec 2009");
    // And the rendered strings really do sort the wrong way, so this test is
    // testing the trap rather than describing a coincidence.
    assert!(t.entries[1].when < t.entries[0].when);
}

#[test]
fn two_entries_sharing_one_date_cannot_draw_a_timeline() {
    // Renamed from a name that overclaimed (test-auditor, 2026-08-06): this proves
    // the THRESHOLD, not that the count reads the stored value — both spellings of
    // one date are identical, so it would pass either way.
    //
    // That the count reads the STORED value is guarded STRUCTURALLY instead: the
    // `HashSet` in `build_timeline` is built over the tuple's first element, which
    // is the raw ISO string, and the rendered value lives in the second. There is
    // no code path by which a formatted string could enter the distinct count.
    let t = timeline(
        &[instance("ev-a"), instance("ev-b")],
        &["ev-a", "ev-b"],
        facts(vec![
            fact("ev-a", "One.", "George Phillips", Some("2009-12-15")),
            fact("ev-b", "Two.", "George Phillips", Some("2009-12-15")),
        ]),
    );

    assert!(
        t.entries.is_empty(),
        "one distinct date cannot draw a timeline"
    );
    assert!(t.notice.is_some());
}

#[test]
fn an_unparseable_date_still_places_an_entry_and_shows_the_stored_value() {
    // A malformed stored value is a data-integrity question for task 2.5, not a
    // reason to drop the statement off the page. It renders verbatim and takes its
    // place in the order — where an ISO-shaped sibling puts it.
    let t = timeline(
        &[instance("ev-real"), instance("ev-odd")],
        &["ev-real", "ev-odd"],
        facts(vec![
            fact("ev-real", "Real.", "George Phillips", Some("2009-12-15")),
            fact("ev-odd", "Odd.", "George Phillips", Some("circa 2009")),
        ]),
    );

    let rendered: Vec<&str> = t.entries.iter().map(|e| e.when.as_str()).collect();
    assert!(rendered.contains(&"15 Dec 2009"), "{rendered:?}");
    assert!(
        rendered.contains(&"circa 2009"),
        "the stored value survives verbatim: {rendered:?}"
    );
}

// ── The drawn timeline's row furniture (task 2.11 C, ruling C7) ──────────────

/// A timeline with BOTH sides on it, so the two assertions below are not proving
/// one rule twice.
fn drawn_timeline() -> RehearsalTimeline {
    timeline(
        &[
            instance("ev-a"),
            instance("ev-b"),
            pairing("ev-b", "ev-answer"),
        ],
        &["ev-a", "ev-b", "ev-answer"],
        facts(vec![
            fact(
                "ev-a",
                "They refused.",
                "George Phillips",
                Some("2009-12-15"),
            ),
            fact("ev-b", "Again.", "George Phillips", Some("2010-03-22")),
            fact(
                "ev-answer",
                "I am open to dividing it amicably.",
                "Marie Awad",
                Some("2009-11-12"),
            ),
        ]),
    )
}

#[test]
fn each_row_carries_the_stored_word_for_its_side_beside_the_token() {
    // Both travel. The TOKEN decides the colour and the LABEL is what a human
    // reads — a client matching on the prose would stop telling the sides apart
    // the day Roman reworded one, silently, on a page whose whole point is that
    // the two sides are distinguishable at a glance.
    let timeline = drawn_timeline();

    for entry in &timeline.entries {
        let expected = if entry.side == SIDE_OUR_ANSWER {
            "OUR ANSWER"
        } else {
            "THEY SAY"
        };
        assert_eq!(
            entry.side_label, expected,
            "side {} mislabelled",
            entry.side
        );
    }
    // Anti-vacuity: both sides must actually appear, or the loop proves one rule.
    assert!(timeline.entries.iter().any(|e| e.side == SIDE_OUR_ANSWER));
    assert!(timeline.entries.iter().any(|e| e.side == SIDE_THEIR_WORDS));
}

#[test]
fn each_row_carries_the_pinpoint_a_witness_needs_to_produce_the_document() {
    // Ruling C7: a timeline row IS an instance or an answer, chronologically
    // arranged. The research the 2026-08-06 source ruling rests on is about a
    // witness producing the source on the spot, which does not care which block
    // she was reading when she was asked.
    let timeline = drawn_timeline();

    for entry in &timeline.entries {
        assert!(
            !entry.source.label.is_empty(),
            "a row with no source label reads as a rendering fault"
        );
        assert_eq!(entry.source.open_label, "Open");
    }
}
