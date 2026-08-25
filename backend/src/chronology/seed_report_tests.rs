//! The count proof is an artifact an operator reads before typing `--apply`, so
//! its content is asserted rather than assumed.

use super::*;
use crate::chronology::seed::{PlannedEvent, PlannedLink};
use crate::chronology::seed_execute::{SeedMode, SeedOutcome};
use chrono::NaiveDate;

fn planned(source_id: &str, link: Option<PlannedLink>, unlinkable: Option<&str>) -> PlannedEvent {
    PlannedEvent {
        source_id: source_id.to_string(),
        event_date: NaiveDate::from_ymd_opt(2012, 4, 12).expect("a real date"),
        approximate: false,
        phase: "appeals".to_string(),
        title: "Judge Tighe Issues Post-Appeal Order".to_string(),
        fact: None,
        attributes: serde_json::json!({"tags": ["court_action"], "source_id": "e016"}),
        link,
        unlinkable_target: unlinkable.map(str::to_string),
    }
}

#[test]
fn the_report_quotes_the_whole_repoint_map_and_the_four_absences() {
    let plan = SeedPlan { events: vec![] };
    let out = render_report(&plan, "a_case", "roman", false);

    for (from, to) in REPOINT_MAP {
        assert!(out.contains(from), "the map must quote {from}");
        assert!(out.contains(to), "the map must quote {to}");
    }
    for id in NO_DOCUMENT_YET {
        assert!(out.contains(id), "the absences must name {id}");
    }
}

#[test]
fn a_re_pointed_link_is_marked_re_pointed_and_an_unchanged_one_is_not() {
    let plan = SeedPlan {
        events: vec![
            planned(
                "e019",
                Some(PlannedLink {
                    original_target_id: "doc-awad-complaint".to_string(),
                    target_id: "doc-awad-v-catholic-family-complaint-11-1-13".to_string(),
                    label: Some("View Complaint".to_string()),
                }),
                None,
            ),
            planned(
                "e010",
                Some(PlannedLink {
                    original_target_id: "doc-sabrina-morris-affidavit".to_string(),
                    target_id: "doc-sabrina-morris-affidavit".to_string(),
                    label: Some("Morris Affidavit".to_string()),
                }),
                None,
            ),
        ],
    };
    let out = render_report(&plan, "a_case", "roman", false);

    assert!(out.contains("link (RE-POINTED): doc-awad-v-catholic-family-complaint-11-1-13"));
    assert!(out.contains("link (unchanged): doc-sabrina-morris-affidavit"));
}

#[test]
fn an_event_with_no_document_says_so_rather_than_going_quiet() {
    let plan = SeedPlan {
        events: vec![planned("e013", None, Some("doc-penzien-coa-brief-300891"))],
    };
    let out = render_report(&plan, "a_case", "roman", false);
    assert!(out.contains("NO LINK — no document exists for doc-penzien-coa-brief-300891"));
}

#[test]
fn the_mode_line_and_the_tense_change_between_a_dry_run_and_an_apply() {
    let plan = SeedPlan { events: vec![] };

    let dry = render_report(&plan, "a_case", "roman", false);
    assert!(dry.contains("DRY RUN — nothing written"));
    assert!(dry.contains("events to write"));

    let applied = render_report(&plan, "a_case", "roman", true);
    assert!(applied.contains("mode       : APPLIED"));
    assert!(applied.contains("events written"));
    assert!(
        !applied.contains("to write"),
        "after --apply the proof must not still say 'would'"
    );
}

#[test]
fn the_totals_count_links_and_absences_separately() {
    let plan = SeedPlan {
        events: vec![
            planned(
                "e019",
                Some(PlannedLink {
                    original_target_id: "doc-awad-complaint".to_string(),
                    target_id: "doc-awad-v-catholic-family-complaint-11-1-13".to_string(),
                    label: None,
                }),
                None,
            ),
            planned("e013", None, Some("doc-penzien-coa-brief-300891")),
            planned("e001", None, None),
        ],
    };
    let out = render_report(&plan, "a_case", "roman", false);

    assert!(out.contains("events to write      : 3"));
    assert!(out.contains("link rows to write   : 1"));
    assert!(out.contains("events with no document yet : 1"));
}

fn outcome(rolled_back: bool) -> SeedOutcome {
    SeedOutcome {
        events_written: 22,
        links_written: 7,
        phases_present: 4,
        rolled_back,
    }
}

#[test]
fn the_outcome_section_records_that_the_targets_were_checked() {
    // The whole reason this section exists: a report file that stopped at the
    // plan would read as a clean dry run even when the target check had failed.
    let rendered = render_outcome(&outcome(false), SeedMode::DryRun);
    assert!(rendered.contains("targets checked    : OK"), "{rendered}");
    assert!(rendered.contains("phase rows present : 4"), "{rendered}");
    assert!(rendered.contains("events             : 22"), "{rendered}");
    assert!(rendered.contains("link rows          : 7"), "{rendered}");
}

#[test]
fn each_mode_states_a_different_result() {
    assert!(
        render_outcome(&outcome(false), SeedMode::DryRun).contains("DRY RUN — nothing was written")
    );
    assert!(render_outcome(&outcome(false), SeedMode::Apply).contains("COMMITTED"));

    let proved = render_outcome(&outcome(true), SeedMode::ProveInTransaction);
    assert!(proved.contains("PROVED, then ROLLED BACK — nothing was kept"));
    assert!(
        proved.contains("executed for real and were discarded"),
        "a proof that says nothing was kept must also say the writes really ran: {proved}"
    );
}

#[test]
fn a_committed_run_never_claims_to_have_rolled_back() {
    let committed = render_outcome(&outcome(false), SeedMode::Apply);
    assert!(!committed.contains("ROLLED BACK"), "{committed}");
    assert!(!committed.contains("discarded"), "{committed}");
}
