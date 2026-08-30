//! Tests for `services::chronology_subset_validate`.
//!
//! Every rule here is reachable without a database, which is the whole reason
//! the judgement was split out of the handlers. What is NOT reachable is the
//! part that needs one — the name clash and the event existence are both read
//! from Postgres and handed in as a `bool` and a set — so those two are tested
//! by stating the world rather than by querying it, and the integration proof
//! (`tests/timeline_subsets_integration.rs`) is what pins the queries.

use super::*;

/// A convenient id that is stable across a test's assertions.
fn id(n: u8) -> Uuid {
    Uuid::from_bytes([n; 16])
}

fn submitted(event: Uuid, position: i32, note: Option<&str>) -> SubmittedSubsetEvent<'_> {
    SubmittedSubsetEvent {
        event_id: event,
        position,
        note,
    }
}

#[test]
fn a_name_is_trimmed_and_kept() {
    let name = validate_name("  The $50,000  ", false).expect("a real name is accepted");
    assert_eq!(name, "The $50,000");
}

#[test]
fn a_blank_name_is_refused_and_is_not_a_conflict() {
    // The distinction matters: a blank name is a 400 the form fixes, and a taken
    // name is a 409 the author fixes by choosing a different one. Collapsing
    // them sends somebody to look in the wrong place.
    let refusal = validate_name("   ", false).expect_err("a blank name must be refused");
    assert_eq!(refusal, SubsetWriteRefusal::BlankName);
    assert!(!refusal.is_conflict());
    assert!(!refusal.is_unprocessable());
    assert_eq!(refusal.field(), Some("name"));
}

#[test]
fn a_taken_name_is_a_conflict_that_quotes_the_name() {
    let refusal =
        validate_name(" The $50,000 ", true).expect_err("a duplicate live name must be refused");
    assert!(refusal.is_conflict());
    assert!(!refusal.is_unprocessable());
    // The TRIMMED name is quoted, because that is the name that would have been
    // stored and the one the author has to change.
    assert_eq!(refusal.value(), Some("The $50,000"));
    assert!(refusal.to_string().contains("The $50,000"));
}

#[test]
fn the_blank_check_runs_before_the_clash_check() {
    // A blank name cannot clash with anything — the lookup is skipped in the
    // handler — so if both were somehow true the answer must still be "blank",
    // which is the one the author can act on.
    let refusal = validate_name("", true).expect_err("blank wins");
    assert_eq!(refusal, SubsetWriteRefusal::BlankName);
}

#[test]
fn an_ordered_set_is_kept_in_the_order_it_arrived() {
    // ⚑ The positions are NOT re-derived, sorted or renumbered. The picker sends
    // the story order it drew, and the read orders by the column. A validator
    // that "helpfully" renumbered would silently overrule a human's drag.
    let known: HashSet<Uuid> = [id(1), id(2), id(3)].into_iter().collect();
    let valid = validate_events(
        &[
            submitted(id(1), 3, None),
            submitted(id(2), 1, Some(" why ")),
            submitted(id(3), 2, None),
        ],
        &known,
    )
    .expect("three known events at three positions");

    assert_eq!(valid.len(), 3);
    assert_eq!(valid[0].event_id, id(1));
    assert_eq!(valid[0].position, 3);
    // The note is trimmed: three spaces and nothing are the same thing to a
    // reader, and storing the first renders an empty line with height.
    assert_eq!(valid[1].note, "why");
    // An absent note becomes "", matching the column's NOT NULL DEFAULT ''.
    assert_eq!(valid[0].note, "");
}

#[test]
fn an_empty_set_is_legal() {
    // Naming a story before choosing its events is a real thing an author does,
    // and the picker is a second screen.
    let valid = validate_events(&[], &HashSet::new()).expect("no events is a real subset");
    assert!(valid.is_empty());
}

#[test]
fn the_same_event_twice_is_refused_by_id() {
    let known: HashSet<Uuid> = [id(1)].into_iter().collect();
    let refusal = validate_events(
        &[submitted(id(1), 1, None), submitted(id(1), 2, None)],
        &known,
    )
    .expect_err("an event is in a story once or not at all");
    assert_eq!(
        refusal,
        SubsetWriteRefusal::DuplicateEvent {
            supplied: id(1).to_string()
        }
    );
    assert!(!refusal.is_conflict());
    assert!(!refusal.is_unprocessable());
}

#[test]
fn two_events_at_one_position_are_refused_by_position() {
    let known: HashSet<Uuid> = [id(1), id(2)].into_iter().collect();
    let refusal = validate_events(
        &[submitted(id(1), 4, None), submitted(id(2), 4, None)],
        &known,
    )
    .expect_err("a story order with a tie has no order");
    assert_eq!(
        refusal,
        SubsetWriteRefusal::DuplicatePosition {
            supplied: "4".to_string()
        }
    );
    assert_eq!(refusal.field(), Some("position"));
}

#[test]
fn an_unknown_event_is_unprocessable_and_names_the_id() {
    // 422, not 400: the shape is right and the VALUE names something this case
    // does not have. The form can only fix it by offering different choices,
    // which is a different instruction from "you sent the wrong shape".
    let known: HashSet<Uuid> = [id(1)].into_iter().collect();
    let refusal = validate_events(
        &[submitted(id(1), 1, None), submitted(id(9), 2, None)],
        &known,
    )
    .expect_err("an event that is not in this case must be refused");
    assert!(refusal.is_unprocessable());
    assert_eq!(refusal.value(), Some(id(9).to_string()).as_deref());
    assert!(refusal.to_string().contains(&id(9).to_string()));
}

#[test]
fn the_first_failure_is_the_one_reported() {
    // A list of five complaints about one paste is worse to read than the one
    // that has to be fixed first. The duplicate is at index 1 and the unknown id
    // at index 2; the duplicate is what comes back.
    let known: HashSet<Uuid> = [id(1)].into_iter().collect();
    let refusal = validate_events(
        &[
            submitted(id(1), 1, None),
            submitted(id(1), 2, None),
            submitted(id(9), 3, None),
        ],
        &known,
    )
    .expect_err("the first rule to fail decides");
    assert!(matches!(refusal, SubsetWriteRefusal::DuplicateEvent { .. }));
}

#[test]
fn every_refusal_names_a_field_a_form_can_highlight() {
    // ⚑ Derived rather than a hand-listed table: a variant added without a
    // `field()` arm would leave a form with nothing to point at, and the
    // exhaustive match in `field()` is only exhaustive against the variants that
    // existed when it was written.
    let all = [
        SubsetWriteRefusal::BlankName,
        SubsetWriteRefusal::NameTaken {
            supplied: "x".into(),
        },
        SubsetWriteRefusal::UnknownEvent {
            supplied: "x".into(),
        },
        SubsetWriteRefusal::DuplicatePosition {
            supplied: "1".into(),
        },
        SubsetWriteRefusal::DuplicateEvent {
            supplied: "x".into(),
        },
    ];
    for refusal in &all {
        assert!(
            refusal.field().is_some(),
            "{refusal:?} names no field, so a form has nothing to highlight"
        );
        assert!(
            !refusal.to_string().is_empty(),
            "{refusal:?} renders as an empty sentence"
        );
    }
    // Exactly one of the three statuses applies to each — never both, never
    // neither, which is what makes the mapping in `support::refusal` total.
    for refusal in &all {
        assert!(
            !(refusal.is_conflict() && refusal.is_unprocessable()),
            "{refusal:?} claims to be both a 409 and a 422"
        );
    }
}
