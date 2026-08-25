//! The count proof is an artifact an operator reads before typing `--apply`, so
//! its content is asserted rather than assumed.

use super::*;
use crate::chronology::seed::{PlannedEvent, PlannedLink};
use chrono::NaiveDate;

fn planned(source_id: &str, link: Option<PlannedLink>, unlinkable: Option<&str>) -> PlannedEvent {
    PlannedEvent {
        source_id: source_id.to_string(),
        event_date: NaiveDate::from_ymd_opt(2012, 4, 12).expect("a real date"),
        approximate: false,
        title: "Judge Tighe Issues Post-Appeal Order".to_string(),
        fact: None,
        attributes: serde_json::json!({"tags": ["court_action"], "phase": "appeals"}),
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
