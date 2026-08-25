//! Behavioural tests for the seed plan.
//!
//! Every case names its input and states the expected output. The plan is pure,
//! so all of this runs with no database and no file system — the REAL file is
//! exercised separately by `guard_tests`, which is the permanent guard.

use super::*;

/// A source event with everything defaulted to something valid, so each test
/// varies exactly the one field it is about.
fn event(id: &str, category: &str, phase: &str, document_id: Option<&str>) -> SourceEvent {
    SourceEvent {
        id: id.to_string(),
        phase: phase.to_string(),
        date: "2010-02-12".to_string(),
        approximate: false,
        title: "Caregiver Affidavits Filed".to_string(),
        description: Some("Sabrina Morris and Jeffrey Humphrey file affidavits.".to_string()),
        category: category.to_string(),
        document_id: document_id.map(str::to_string),
        document_label: document_id.map(|_| "Morris Affidavit".to_string()),
    }
}

/// A source document holding exactly the events given, with the four real phases.
fn source(events: Vec<SourceEvent>) -> SourceTimeline {
    let phase = |id: &str| SourcePhase {
        id: id.to_string(),
        label: id.to_uppercase(),
        date_range: "2008–2009".to_string(),
        color: "#b45309".to_string(),
        description: None,
    };
    SourceTimeline {
        phases: vec![
            phase("estate"),
            phase("probate"),
            phase("appeals"),
            phase("civil_lawsuit"),
        ],
        events,
        categories: BTreeMap::new(),
    }
}

#[test]
fn an_event_carries_its_category_as_a_tag_and_its_phase_as_an_attribute() {
    let plan = build_plan(&source(vec![event("e010", "filing", "probate", None)]))
        .expect("a valid event plans");
    let attrs = &plan.events[0].attributes;

    assert_eq!(attrs["tags"], serde_json::json!(["filing"]));
    assert_eq!(attrs["phase"], serde_json::json!("probate"));
    assert_eq!(attrs["source"], serde_json::json!("legacy_json"));
    // The LABEL is never what is stored — the colour and display name are both
    // looked up by the key.
    assert_ne!(attrs["tags"], serde_json::json!(["Filing"]));
}

#[test]
fn the_date_is_parsed_and_the_description_becomes_the_fact() {
    let plan = build_plan(&source(vec![event("e010", "filing", "probate", None)])).expect("plans");
    let planned = &plan.events[0];

    assert_eq!(
        planned.event_date,
        NaiveDate::from_ymd_opt(2010, 2, 12).expect("a real date")
    );
    assert_eq!(
        planned.fact.as_deref(),
        Some("Sabrina Morris and Jeffrey Humphrey file affidavits.")
    );
    assert_eq!(planned.title, "Caregiver Affidavits Filed");
}

#[test]
fn a_near_miss_document_id_is_re_pointed_and_keeps_its_label() {
    let plan = build_plan(&source(vec![event(
        "e019",
        "filing",
        "civil_lawsuit",
        Some("doc-awad-complaint"),
    )]))
    .expect("plans");

    let link = plan.events[0].link.as_ref().expect("a link is planned");
    assert_eq!(link.original_target_id, "doc-awad-complaint");
    assert_eq!(
        link.target_id,
        "doc-awad-v-catholic-family-complaint-11-1-13"
    );
    assert_eq!(link.label.as_deref(), Some("Morris Affidavit"));
    assert_eq!(plan.events[0].unlinkable_target, None);
}

#[test]
fn the_one_already_real_id_maps_to_itself() {
    let plan = build_plan(&source(vec![event(
        "e010",
        "filing",
        "probate",
        Some("doc-sabrina-morris-affidavit"),
    )]))
    .expect("plans");

    let link = plan.events[0].link.as_ref().expect("a link is planned");
    assert_eq!(link.target_id, "doc-sabrina-morris-affidavit");
    assert_eq!(link.original_target_id, link.target_id);
}

#[test]
fn a_reference_with_no_document_yields_an_event_and_no_link() {
    let plan = build_plan(&source(vec![event(
        "e013",
        "filing",
        "appeals",
        Some("doc-penzien-coa-brief-300891"),
    )]))
    .expect("plans");

    assert_eq!(plan.events.len(), 1, "the EVENT is still seeded");
    assert!(plan.events[0].link.is_none(), "but no link row is written");
    assert_eq!(
        plan.events[0].unlinkable_target.as_deref(),
        Some("doc-penzien-coa-brief-300891")
    );
    assert_eq!(plan.link_count(), 0);
    assert_eq!(
        plan.unlinkable(),
        vec![("e013", "doc-penzien-coa-brief-300891")]
    );
}

#[test]
fn an_event_with_no_document_reference_is_neither_linked_nor_flagged() {
    let plan =
        build_plan(&source(vec![event("e001", "financial", "estate", None)])).expect("plans");
    assert!(plan.events[0].link.is_none());
    assert_eq!(
        plan.events[0].unlinkable_target, None,
        "no reference is not the same state as a reference that cannot resolve"
    );
}

#[test]
fn a_document_in_neither_list_stops_the_run_and_names_the_event() {
    let err = build_plan(&source(vec![event(
        "e099",
        "filing",
        "probate",
        Some("doc-something-nobody-mapped"),
    )]))
    .expect_err("an unmapped document must refuse");

    assert_eq!(
        err,
        SeedError::UnmappedDocument {
            source_id: "e099".to_string(),
            document_id: "doc-something-nobody-mapped".to_string(),
        }
    );
    assert!(err
        .to_string()
        .contains("guessing which is not this tool's call"));
}

#[test]
fn a_bad_date_stops_the_run() {
    let mut bad = event("e005", "financial", "estate", None);
    bad.date = "12 April 2009".to_string();
    let err = build_plan(&source(vec![bad])).expect_err("a non-ISO date must refuse");
    assert!(matches!(err, SeedError::BadDate { .. }), "got {err:?}");
    assert!(
        err.to_string().contains("e005"),
        "the event id must be named"
    );
}

#[test]
fn an_unknown_tag_stops_the_run() {
    let err = build_plan(&source(vec![event("e007", "hearsay", "estate", None)]))
        .expect_err("an unknown tag must refuse");
    assert_eq!(
        err,
        SeedError::UnknownTag {
            source_id: "e007".to_string(),
            tag: "hearsay".to_string(),
        }
    );
}

#[test]
fn an_unknown_phase_stops_the_run() {
    let err = build_plan(&source(vec![event("e007", "personal", "mediation", None)]))
        .expect_err("an unknown phase must refuse");
    assert_eq!(
        err,
        SeedError::UnknownPhase {
            source_id: "e007".to_string(),
            phase: "mediation".to_string(),
        }
    );
}

#[test]
fn target_ids_are_distinct_and_sorted() {
    let plan = build_plan(&source(vec![
        event(
            "e019",
            "filing",
            "civil_lawsuit",
            Some("doc-awad-complaint"),
        ),
        event(
            "e010",
            "filing",
            "probate",
            Some("doc-sabrina-morris-affidavit"),
        ),
        // The same document referenced twice yields ONE target.
        event(
            "e020",
            "discovery",
            "civil_lawsuit",
            Some("doc-awad-complaint"),
        ),
    ]))
    .expect("plans");

    assert_eq!(
        plan.target_ids(),
        vec![
            "doc-awad-v-catholic-family-complaint-11-1-13".to_string(),
            "doc-sabrina-morris-affidavit".to_string(),
        ]
    );
    assert_eq!(plan.link_count(), 3, "three links, two distinct targets");
}

#[test]
fn the_repoint_map_and_the_no_document_list_do_not_overlap() {
    // An id in both lists would make `plan_link` order-dependent: whichever
    // branch ran first would silently win.
    for (from, _) in REPOINT_MAP {
        assert!(
            !NO_DOCUMENT_YET.contains(from),
            "{from} is in both the re-point map and the no-document list"
        );
    }
}

#[test]
fn parse_source_names_the_file_when_the_json_is_not_the_expected_document() {
    let err = parse_source("/tmp/broken.json", "{\"phases\": 3}")
        .expect_err("a wrong-shaped document must refuse");
    assert!(matches!(err, SeedError::Unparseable { .. }), "got {err:?}");
    assert!(
        err.to_string().contains("/tmp/broken.json"),
        "the message must name the file, got: {err}"
    );
}
