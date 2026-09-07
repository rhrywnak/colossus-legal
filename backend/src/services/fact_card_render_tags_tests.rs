// Tests for `services::fact_card_render` — the tags, the backing line and the
// draft marks.
//
// Split from `fact_card_render_tests.rs` for the module-size limit (Rule 17).
// The fixtures both halves build on live in `fact_card_render_fixtures.rs`.

use super::render_fixtures::*;
use super::*;

// ─── The count tags ──────────────────────────────────────────────────────────

/// Two accusations of ONE count produce ONE tag.
///
/// The bar names which count this card is for; printing "Count 1" twice says
/// nothing the first said not.
#[test]
fn two_accusations_of_one_count_produce_one_tag() {
    let mut record = bare_record();
    record.supports = Some(supports_value(&[
        ("a-21", CardStance::Supports),
        ("a-44", CardStance::Supports),
    ]));
    assert_eq!(
        render(&record).count_tags,
        vec!["Count 1 — Breach of Fiduciary Duty"]
    );
}

/// Two accusations of two counts produce two tags, in the order named.
#[test]
fn two_counts_produce_two_tags_in_the_order_named() {
    let mut record = bare_record();
    record.supports = Some(supports_value(&[
        ("a-61", CardStance::Rebuts),
        ("a-21", CardStance::Supports),
    ]));
    assert_eq!(
        render(&record).count_tags,
        vec![
            "Count 4 — Abuse of Process",
            "Count 1 — Breach of Fiduciary Duty"
        ]
    );
}

/// An accusation wired to no element carries no tag, and does not suppress the
/// line.
///
/// 25 of the case's 120 allegations are in this state, measured. The card names
/// the accusation and the title bar simply carries no tag.
#[test]
fn an_accusation_with_no_count_carries_no_tag_but_keeps_its_line() {
    let mut record = bare_record();
    record.supports = Some(supports_value(&[("a-unwired", CardStance::Supports)]));
    let block = render(&record);
    assert!(block.count_tags.is_empty());
    assert_eq!(block.supports.len(), 1);
    assert!(block.supports[0].contains("A-9"));
}

// ─── The Backs line ──────────────────────────────────────────────────────────

/// A card that backs a point reads as "Point n — <its text>".
#[test]
fn a_backed_card_names_the_point_and_quotes_it() {
    let mut record = bare_record();
    record.backs_position = Some(2);
    let block = render(&record);
    assert_eq!(
        block.backs.as_deref(),
        Some("Point 2 — My sisters' claim rested on hearsay.")
    );
    assert_eq!(block.backs_position, Some(2));
}

/// A STALE position renders nothing — never "Point 7 — " with an empty tail.
///
/// `set_talking_points` deletes and re-inserts the whole list on an ordinary
/// edit, so a card naming position 3 of a two-point list is a state that WILL
/// occur. The number survives on `backs_position` so an editor can show what was
/// stored and a human can fix it.
#[test]
fn a_stale_backs_position_renders_no_line_but_keeps_its_number() {
    let mut record = bare_record();
    record.backs_position = Some(7);
    let block = render(&record);
    assert_eq!(block.backs, None);
    assert_eq!(
        block.backs_position,
        Some(7),
        "the editor must be able to show what was stored"
    );
}

// ─── The draft marks ─────────────────────────────────────────────────────────

/// A machine-written field is a draft; a human-written one is not.
#[test]
fn a_machine_field_is_a_draft_and_a_human_field_is_not() {
    let mut record = bare_record();
    record.title_authored_by = Some(MACHINE.to_string());
    record.answer_authored_by = Some("chuck".to_string());
    let drafts = render(&record).drafts;
    assert!(drafts.title);
    assert!(!drafts.answer);
}

/// EDITING ONE FIELD DOES NOT CLEAR ANOTHER'S MARK.
///
/// The whole reason authorship is per-column. The mockup's own card shows an
/// edited Answer beside a still-drafted Watch out, and a row-level author would
/// make that state unrepresentable.
#[test]
fn editing_one_field_leaves_the_other_marks_alone() {
    let mut record = bare_record();
    for author in [
        &mut record.title_authored_by,
        &mut record.backs_position_authored_by,
        &mut record.supports_authored_by,
        &mut record.watch_out_authored_by,
        &mut record.answer_authored_by,
    ] {
        *author = Some(MACHINE.to_string());
    }
    // Chuck rewrites the answer, and nothing else.
    record.answer_authored_by = Some("chuck".to_string());
    record.authored_by = "chuck".to_string();

    let drafts = render(&record).drafts;
    assert!(!drafts.answer, "the edited field loses its mark");
    assert!(drafts.title, "the others keep theirs");
    assert!(drafts.backs);
    assert!(drafts.supports);
    assert!(drafts.watch_out);
}

/// A field nobody has written carries no mark — absent is not a draft.
#[test]
fn an_unwritten_field_is_not_a_draft() {
    assert!(!render(&bare_record()).drafts.any());
}

/// A LATER drafting job still marks its fields as drafts.
#[test]
fn a_later_drafting_job_is_still_a_draft() {
    let mut record = bare_record();
    record.watch_out_authored_by = Some("machine:job_c_v2".to_string());
    assert!(render(&record).drafts.watch_out);
}

// ─── The words come from the store ───────────────────────────────────────────

/// Re-wording the templates re-words the card, with no code change.
#[test]
fn the_lines_are_built_from_the_stored_templates() {
    let mut record = bare_record();
    record.backs_position = Some(1);
    record.supports = Some(supports_value(&[("a-21", CardStance::Supports)]));

    let mut custom = words();
    custom.backs_template = "[{position}] {text}".to_string();
    custom.supports_template = "{code} ({verb}): {text}".to_string();
    custom.stance_supports_verb = "FOR".to_string();

    let allegations = allegations();
    let points = points();
    let block = render_card(
        &record,
        RenderContext {
            allegations: &allegations,
            talking_points: &points,
            wording: &custom,
        },
    );
    assert_eq!(
        block.backs.as_deref(),
        Some("[1] I never refused to pay for the funeral.")
    );
    assert_eq!(
        block.supports,
        vec!["A-21 (FOR): CFS could have returned the money."]
    );
}
