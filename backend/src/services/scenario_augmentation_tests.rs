//! Unit tests for [`super`] — the augmentation service (task 1.4, v2 §8).
//!
//! Two things are pinned here: the validation law for human content, and the
//! BEHAVIOURAL half of §8 — that editing human content never triggers a gather.
//! The structural half (no scan path writes human tables) lives in
//! `scenario_human_facts_tests`, beside the table it protects.
//!
//! Everything is pure or source-scanning; the transactional writes need a live
//! Postgres and are DEV-verified, the same convention as the sibling services.

use super::*;

fn fact<'a>(
    text: &'a str,
    date: Option<NaiveDate>,
    date_type: Option<&'a str>,
) -> NewHumanFact<'a> {
    NewHumanFact {
        scenario_id: Uuid::nil(),
        text,
        occurred_on: date,
        date_type,
        person_refs: &[],
        authored_by: "Roman",
    }
}

fn a_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2009, 4, 21).expect("a real date")
}

// ── Human-fact validation ────────────────────────────────────────────────────

#[test]
fn a_fact_with_text_alone_is_valid() {
    // The common case: knowledge with no date and nobody named. A human fact is
    // not required to be dated — most are not.
    assert!(validate_human_fact(&fact(
        "Switching counsel meant emailing attachments",
        None,
        None
    ))
    .is_ok());
}

#[test]
fn an_empty_fact_is_refused() {
    assert!(matches!(
        validate_human_fact(&fact("", None, None)),
        Err(AugmentationError::EmptyText)
    ));
}

#[test]
fn a_whitespace_only_fact_is_refused() {
    // The field looked filled in. It records nothing.
    assert!(matches!(
        validate_human_fact(&fact("   \n\t ", None, None)),
        Err(AugmentationError::EmptyText)
    ));
}

#[test]
fn a_date_type_without_a_date_is_refused() {
    // "Around" what? A qualifier with nothing to qualify is a claim about
    // precision that has no subject.
    let result = validate_human_fact(&fact("something happened", None, Some("around")));
    assert!(matches!(
        result,
        Err(AugmentationError::DateTypeWithoutDate { .. })
    ));
}

#[test]
fn an_unknown_date_type_is_refused_before_it_is_stored() {
    // Parse-don't-validate at the write boundary: a token this build cannot read
    // back would render as an unqualified date later, overstating precision.
    let result = validate_human_fact(&fact("x", Some(a_date()), Some("someday")));
    assert!(matches!(
        result,
        Err(AugmentationError::UnknownDateType { .. })
    ));
}

#[test]
fn every_known_date_type_is_accepted_with_a_date() {
    for kind in DateType::ALL {
        assert!(
            validate_human_fact(&fact("x", Some(a_date()), Some(kind.code()))).is_ok(),
            "{} must be accepted with a date",
            kind.code()
        );
    }
}

#[test]
fn a_date_with_no_type_is_accepted() {
    // A plain date is an exact date; the type is the optional qualifier, not the
    // other way round.
    assert!(validate_human_fact(&fact("x", Some(a_date()), None)).is_ok());
}

// ── The talking-points cap ───────────────────────────────────────────────────

#[test]
fn a_list_within_the_cap_is_kept_in_order() {
    let points = vec!["first".to_string(), "second".to_string()];
    let kept = check_talking_points(&points).expect("within the cap");
    assert_eq!(kept, vec!["first".to_string(), "second".to_string()]);
}

#[test]
fn exceeding_the_cap_is_refused_and_the_message_names_the_limit() {
    let over: Vec<String> = (0..talking_points_cap() + 1)
        .map(|i| format!("point {i}"))
        .collect();

    let Err(error) = check_talking_points(&over) else {
        panic!("a list over the cap must be refused");
    };
    let message = error.to_string();
    // The human needs to know the limit AND what to do instead.
    assert!(
        message.contains(&talking_points_cap().to_string()),
        "the refusal must name the cap: {message}"
    );
    assert!(
        message.contains("shorten or replace"),
        "the refusal must offer a way forward: {message}"
    );
}

#[test]
fn exactly_the_cap_is_allowed() {
    // Off-by-one guard: the cap is inclusive, so three points fit a cap of three.
    let at_cap: Vec<String> = (0..talking_points_cap())
        .map(|i| format!("point {i}"))
        .collect();
    assert!(check_talking_points(&at_cap).is_ok());
}

#[test]
fn blank_points_are_dropped_rather_than_counted() {
    // A trailing empty box in the editor is not a talking point. Counting it
    // would refuse a legitimate third point because a fourth field was open.
    let mut points: Vec<String> = (0..talking_points_cap())
        .map(|i| format!("point {i}"))
        .collect();
    points.push("   ".to_string());

    let kept = check_talking_points(&points).expect("the blank does not count");
    assert_eq!(kept.len(), talking_points_cap());
}

#[test]
fn points_are_trimmed_so_stored_text_matches_what_was_meant() {
    let kept = check_talking_points(&["  padded  ".to_string()]).expect("valid");
    assert_eq!(kept, vec!["padded".to_string()]);
}

#[test]
fn an_empty_list_is_allowed_and_clears_the_points() {
    // Deleting every point is a legitimate edit — the human decided none of them
    // were right. It must not be an error.
    assert_eq!(
        check_talking_points(&[]).expect("valid"),
        Vec::<String>::new()
    );
}

// ── The refusal messages ─────────────────────────────────────────────────────
//
// Each of these reaches the client verbatim — the human who just typed the thing
// is the only one who can act on it. A `matches!` on the variant proves the right
// branch was taken; it does not prove the message still says anything, and a
// blanked template would pass every variant check above.

#[test]
fn the_empty_text_refusal_says_what_is_missing() {
    let Err(error) = validate_human_fact(&fact("", None, None)) else {
        panic!("refused");
    };
    let message = error.to_string();
    assert!(message.contains("needs some text"), "{message}");
    assert!(message.contains("records nothing"), "{message}");
}

#[test]
fn the_date_type_refusal_names_the_type_and_the_two_ways_out() {
    let Err(error) = validate_human_fact(&fact("x", None, Some("around"))) else {
        panic!("refused");
    };
    let message = error.to_string();
    // The offending token, so the human sees which field is the problem…
    assert!(message.contains("around"), "{message}");
    // …and both legitimate fixes, since either is fine.
    assert!(message.contains("add the date"), "{message}");
    assert!(message.contains("drop the type"), "{message}");
}

#[test]
fn the_unknown_date_type_refusal_lists_the_vocabulary() {
    let Err(error) = validate_human_fact(&fact("x", Some(a_date()), Some("someday"))) else {
        panic!("refused");
    };
    let message = error.to_string();
    assert!(message.contains("someday"), "the bad token: {message}");
    // Listing the four is what makes the message actionable rather than a scold.
    for known in DateType::ALL {
        assert!(
            message.contains(known.code()),
            "{} must be offered: {message}",
            known.code()
        );
    }
}

// ── The behavioural half of §8 ───────────────────────────────────────────────

/// Editing human content never triggers re-gathering (v2 §8, second invariant).
///
/// ## Why this matters more than it looks
///
/// Gather MEMOIZES candidate ordinals — it is the one read path allowed to write.
/// If an augmentation edit called it, typing a sentence about a scenario would
/// mint candidate identity as a side effect, and a human fact would silently
/// change what the card queue shows. The invariant is cheap to hold and
/// impossible to notice breaking, which is exactly the kind that needs a test.
#[test]
fn augmentation_never_gathers() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/services/scenario_augmentation.rs"),
    )
    .expect("the augmentation service is readable");

    // Strip comments so the prose in this module's own header — which discusses
    // gathering at length — does not trip the scan.
    let code: String = source
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            !(t.starts_with("//") || t.starts_with('*'))
        })
        .collect::<Vec<_>>()
        .join("\n");

    for forbidden in [
        "gather_scenario_candidates",
        "all_evidence_about_subject",
        "assign_candidate_ordinals",
        "ensure_candidate_ordinals",
    ] {
        assert!(
            !code.contains(forbidden),
            "the augmentation service calls {forbidden} — editing human content \
             must never trigger re-gathering (v2 §8). Gather mints candidate \
             identity as a side effect, so this would make typing a sentence \
             change what the card queue shows."
        );
    }
}

/// The service writes human content and nothing else.
///
/// The mirror of the scan-path allowlist in `scenario_human_facts_tests`: that
/// one proves the scan cannot reach human tables, this one proves the
/// augmentation path cannot reach the scan's.
#[test]
fn augmentation_writes_only_human_authored_tables() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/services/scenario_augmentation.rs"),
    )
    .expect("readable");

    let code: String = source
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            !(t.starts_with("//") || t.starts_with('*'))
        })
        .collect::<Vec<_>>()
        .join("\n");

    for forbidden in [
        "scenario_fact_refs",
        "scan_runs",
        "scan_run_verdicts",
        "scan_run_merges",
        "scenario_candidate_ordinals",
    ] {
        assert!(
            !code.contains(forbidden),
            "the augmentation service touches {forbidden}, which belongs to the \
             scan/gather paths. Human content and machine content are kept apart \
             on purpose (v2 §8)."
        );
    }
}
