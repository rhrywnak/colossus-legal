// Tests for the loader's plan.
//
// The plan is what the dry run PRINTS and what the apply WRITES, so every rule
// here is one a reader of the dry run is trusting. The two that matter most: an
// absent draft is not written as a clear, and an unresolvable accusation is named
// rather than dropped.

use super::super::model::{Pick, RunReport, SupportEntry, TalkingPoint};
use super::*;
use colossus_legal_backend::domain::fact_card::CardStance;

fn allegations() -> HashMap<String, String> {
    HashMap::from([
        (
            "45984d77".to_string(),
            "doc-complaint:allegation:45984d77".to_string(),
        ),
        (
            "08c0731b".to_string(),
            "doc-complaint:allegation:08c0731b".to_string(),
        ),
    ])
}

fn card() -> DraftedCard {
    DraftedCard {
        title: "The court ordered the $50,000 back".to_string(),
        backs: None,
        supports: Vec::new(),
        watch_out: None,
        answer_draft: None,
        card_id: "doc-tighe:evidence:8a240a75".to_string(),
        c_code: Some("C98".to_string()),
        over_supports_cap: false,
        dropped_not_in_list: Vec::new(),
        date: None,
        speaker: None,
    }
}

fn fields_for(plan: &CardPlan, field: CardField) -> Vec<&PlannedField> {
    plan.fields.iter().filter(|f| f.field == field).collect()
}

/// A full card plans all five fields.
#[test]
fn a_full_card_plans_every_field() {
    let mut c = card();
    c.backs = Some(2);
    c.watch_out = Some("They will say the judge approved it.".to_string());
    c.answer_draft = Some("DRAFT: The money was Dad's.".to_string());
    c.supports = vec![SupportEntry {
        allegation_id: "45984d77".to_string(),
        stance: CardStance::Supports,
    }];

    let plan = plan_cards(&[c], &allegations(), 14).expect("the card plans");
    assert_eq!(plan.cards, 1);
    assert_eq!(plan.fields.len(), 5);
    assert!(plan.unresolved_accusations.is_empty());
    assert_eq!(
        fields_for(&plan, CardField::BacksPosition)[0]
            .value
            .as_deref(),
        Some("2")
    );
}

/// AN ABSENT DRAFT IS NOT WRITTEN AS A CLEAR.
///
/// Job B leaves `watch_out` and `answer_draft` out on some cards. Writing those
/// as NULL would stamp `machine:job_b_v1` on a field the machine never wrote, and
/// the card would show a draft mark over an em dash — a mark claiming the machine
/// had drafted an emptiness.
#[test]
fn an_absent_draft_plans_no_field_at_all() {
    let plan = plan_cards(&[card()], &allegations(), 14).expect("plans");
    assert_eq!(plan.fields.len(), 1, "the title alone");
    assert_eq!(plan.fields[0].field, CardField::Title);
    assert!(fields_for(&plan, CardField::Answer).is_empty());
    assert!(fields_for(&plan, CardField::WatchOut).is_empty());
}

/// A blank draft is the same as an absent one.
#[test]
fn a_blank_draft_plans_no_field() {
    let mut c = card();
    c.watch_out = Some("   ".to_string());
    let plan = plan_cards(&[c], &allegations(), 14).expect("plans");
    assert!(fields_for(&plan, CardField::WatchOut).is_empty());
}

/// The short accusation id is RESOLVED to the full graph id.
///
/// Job B wrote `"45984d77"`; the graph holds
/// `"doc-complaint:allegation:45984d77"`. Storing the short form would make every
/// card claim a link nothing in the graph answers.
#[test]
fn a_short_accusation_id_is_resolved_to_the_full_node_id() {
    let mut c = card();
    c.supports = vec![SupportEntry {
        allegation_id: "45984d77".to_string(),
        stance: CardStance::Rebuts,
    }];
    let plan = plan_cards(&[c], &allegations(), 14).expect("plans");
    let stored = fields_for(&plan, CardField::Supports)[0]
        .value
        .as_deref()
        .expect("a value");
    assert!(
        stored.contains("doc-complaint:allegation:45984d77"),
        "{stored}"
    );
    assert!(stored.contains("rebuts"), "the stance survives: {stored}");
}

/// AN UNRESOLVABLE ACCUSATION IS NAMED, and the rest of the card is still
/// written.
///
/// A card claiming a link nothing answers is the stale-pointer defect of
/// 2026-07-24, and the loader is where it is cheapest to see. Refusing the whole
/// card would lose four good sentences over one bad pointer.
#[test]
fn an_unresolvable_accusation_is_named_and_the_card_still_loads() {
    let mut c = card();
    c.supports = vec![
        SupportEntry {
            allegation_id: "45984d77".to_string(),
            stance: CardStance::Supports,
        },
        SupportEntry {
            allegation_id: "deadbeef".to_string(),
            stance: CardStance::Supports,
        },
    ];
    let plan = plan_cards(&[c], &allegations(), 14).expect("plans");
    assert_eq!(plan.unresolved_accusations.len(), 1);
    assert!(plan.unresolved_accusations[0].contains("deadbeef"));
    assert!(
        plan.unresolved_accusations[0].contains("8a240a75"),
        "names the card"
    );

    let stored = fields_for(&plan, CardField::Supports)[0]
        .value
        .as_deref()
        .expect("the resolvable one still writes");
    assert!(stored.contains("45984d77"));
    assert!(!stored.contains("deadbeef"));
}

/// A card whose ONLY accusation is unresolvable plans no supports field —
/// rather than an empty list, which would read as "names none" instead of
/// "named one we could not find".
#[test]
fn a_card_whose_only_accusation_is_unresolvable_plans_no_supports() {
    let mut c = card();
    c.supports = vec![SupportEntry {
        allegation_id: "deadbeef".to_string(),
        stance: CardStance::Supports,
    }];
    let plan = plan_cards(&[c], &allegations(), 14).expect("plans");
    assert!(fields_for(&plan, CardField::Supports).is_empty());
    assert_eq!(plan.unresolved_accusations.len(), 1);
}

/// A long title stops the whole plan — nothing is written.
#[test]
fn a_long_title_stops_the_plan() {
    let mut c = card();
    c.title =
        "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen"
            .to_string();
    assert!(plan_cards(&[c], &allegations(), 14).is_err());
}

/// The author is the machine token, and it carries the machine prefix.
#[test]
fn the_loader_writes_as_the_machine() {
    use colossus_legal_backend::domain::fact_card::{CardAuthor, MACHINE_AUTHOR_PREFIX};
    assert!(LOADER_AUTHOR.starts_with(MACHINE_AUTHOR_PREFIX));
    assert!(CardAuthor(LOADER_AUTHOR).is_draft());
}

// ─── Talking points ──────────────────────────────────────────────────────────

fn points_file(positions: &[i32]) -> TalkingPointsFile {
    TalkingPointsFile {
        scenario_code: "S-9".to_string(),
        scenario_id: uuid::Uuid::nil(),
        talking_points: positions
            .iter()
            .map(|p| TalkingPoint {
                position: *p,
                text: format!("point {p}"),
                backed_by: Vec::new(),
                backed_by_c_codes: Vec::new(),
                backed_by_card_ids: Vec::new(),
                why_these_cards: None,
            })
            .collect(),
        run_report: RunReport::default(),
    }
}

/// Positions are translated from 1-based to the 0-based `item_index` storage
/// uses.
#[test]
fn talking_point_positions_become_zero_based_item_indexes() {
    let planned = plan_points(&[points_file(&[1, 2, 3])]).expect("plans");
    assert_eq!(
        planned[0].items,
        vec![
            (0, "point 1".to_string()),
            (1, "point 2".to_string()),
            (2, "point 3".to_string())
        ]
    );
}

/// A GAP in the positions stops the load.
///
/// Every card names a point by NUMBER (`backs_position`), so a gap would make a
/// card point at the wrong sentence — on a rehearsal page, under oath.
#[test]
fn a_gap_in_the_positions_stops_the_load() {
    let err = plan_points(&[points_file(&[1, 3])]).expect_err("2 is missing");
    let message = format!("{err}");
    assert!(message.contains("S-9"), "{message}");
    assert!(message.contains("backs_position"), "says why: {message}");
}

/// A REPEATED position stops the load, for the same reason.
#[test]
fn a_repeated_position_stops_the_load() {
    assert!(plan_points(&[points_file(&[1, 1, 2])]).is_err());
}

/// Positions out of order are fine — they are sorted before checking.
#[test]
fn positions_out_of_order_are_accepted() {
    assert!(plan_points(&[points_file(&[3, 1, 2])]).is_ok());
}

// ─── Picks ───────────────────────────────────────────────────────────────────

fn candidates(numbers: &[i32], reasons: bool) -> CandidatesFile {
    CandidatesFile {
        scenario_code: "S-1".to_string(),
        scenario_id: uuid::Uuid::nil(),
        picks: numbers
            .iter()
            .map(|n| Pick {
                pick: *n,
                graph_node_id: format!("doc-x:evidence:{n}"),
                title: format!("title {n}"),
                reason: reasons.then(|| format!("because {n}")),
                c_code: None,
                gather_rank: None,
                k: None,
            })
            .collect(),
        talking_points: Vec::new(),
        run_report: RunReport::default(),
    }
}

/// Picks are ordered by their pick number and spaced by the ordering module's
/// own step.
///
/// The step must be that module's, not a number of this loader's: a pick placed
/// outside the scheme's spacing would leave no room for a human to drag a card
/// between two picks.
#[test]
fn picks_are_ordered_and_spaced_by_the_ordering_modules_step() {
    use colossus_legal_backend::services::scenario_fact_order::ORDINAL_STEP;
    let planned = plan_picks(&[candidates(&[2, 1, 3], false)]).expect("plans");
    let ordinals: Vec<i32> = planned[0].picks.iter().map(|(_, o, _)| *o).collect();
    assert_eq!(
        ordinals,
        vec![ORDINAL_STEP, ORDINAL_STEP * 2, ORDINAL_STEP * 3]
    );
    assert!(planned[0].picks[0].0.ends_with(":1"), "pick 1 leads");
}

/// A gap in the pick numbers stops the load — the number IS the display order.
#[test]
fn a_gap_in_the_pick_numbers_stops_the_load() {
    let err = plan_picks(&[candidates(&[1, 2, 4], false)]).expect_err("3 is missing");
    assert!(format!("{err}").contains("display order"));
}

/// PICK REASONS ARE COUNTED, NOT STORED AND NOT MIS-FILED.
///
/// §1's card has no field for "why the ranker picked this", and mapping it onto
/// `watch_out` would put a ranking note where a witness expects to read how the
/// other side will use the fact. The count is what makes the omission visible in
/// the dry run rather than silent.
#[test]
fn pick_reasons_are_counted_rather_than_stored() {
    let planned = plan_picks(&[candidates(&[1, 2], true)]).expect("plans");
    assert_eq!(planned[0].unstored_reasons, 2);
    for (_, _, title) in &planned[0].picks {
        assert!(
            !title.contains("because"),
            "a reason must not become a title"
        );
    }
}

/// A file with no reasons reports none.
#[test]
fn a_file_without_reasons_reports_none_unstored() {
    assert_eq!(
        plan_picks(&[candidates(&[1], false)]).expect("plans")[0].unstored_reasons,
        0
    );
}
