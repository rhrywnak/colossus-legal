//! Unit tests for [`super`] — the card endpoint's pure assembly and partition.
//!
//! The per-card §7 contract is tested in `services::scenario_card_tests`. What is
//! tested here is what this module owns: the partition into pool/set-aside, the
//! stable ordering, the page-text join, and the loud status decode.

use super::*;
use crate::bias::dto::DocumentRef;
use crate::repositories::pipeline_repository::ScenarioFactRefRecord;

fn instance(id: &str, page: Option<i64>) -> BiasInstance {
    BiasInstance {
        evidence_id: id.to_string(),
        title: String::new(),
        verbatim_quote: Some("I do not recall.".to_string()),
        question: None,
        page_number: page,
        pattern_tags: Vec::new(),
        stated_by: None,
        about: Vec::new(),
        document: Some(DocumentRef {
            id: "doc-7".to_string(),
            title: "Deposition".to_string(),
            document_type: None,
        }),
    }
}

fn fact_ref(node: &str, status: &str) -> ScenarioFactRefRecord {
    ScenarioFactRefRecord {
        scenario_id: uuid::Uuid::nil(),
        graph_node_id: node.to_string(),
        role_in_this_scenario: None,
        status: status.to_string(),
        note: None,
        confidence: None,
        source_run_id: None,
        tagged_at: chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
        defer_reason: None,
        // Task 2.13 defaults: a reference nobody has weighed or placed.
        tier: crate::domain::fact_tier::FactTier::DEFAULT
            .code()
            .to_string(),
        sort_ordinal: None,
        // Task 2.13c: nobody has weighed or placed these fixtures, which is
        // the honest state for a reference nothing has curated.
        tier_updated_by: None,
        tier_updated_at: None,
        order_updated_by: None,
        order_updated_at: None,
    }
}

fn ordinals(pairs: &[(&str, i32)]) -> HashMap<String, i32> {
    pairs
        .iter()
        .map(|(id, n)| ((*id).to_string(), *n))
        .collect()
}

/// The bare-card count is what makes the residue of D3 visible (task 1.7A).
///
/// A card whose quote spans a page boundary grounds fine and renders bare, so it
/// looks like every other grounded card. The count on the "served scenario cards"
/// line is the only place that state is observable, which makes an off-by-one
/// here a silent failure of its own.
#[test]
fn the_bare_card_count_sees_context_less_cards_and_ignores_the_rest() {
    let settings = crate::domain::settings::Settings::for_test();
    let page = "BEFORE. I do not recall. AFTER.".to_string();
    let mut page_text = HashMap::new();
    page_text.insert(page_key("doc-7", 14), page);

    // ev-1 sits on page 14, whose text is loaded — it gets context.
    // ev-2 claims page 99, for which no text was read — it cannot.
    let pool = vec![instance("ev-1", Some(14)), instance("ev-2", Some(99))];
    let response = assemble(
        pool,
        &HashMap::new(),
        &HashMap::new(),
        &ordinals(&[("ev-1", 1), ("ev-2", 2)]),
        &page_text,
        &settings,
        HumanTouchIndex {
            question_overrides: &HashMap::new(),
            links: &HashMap::new(),
        },
    );

    assert_eq!(response.pool.len(), 2, "both candidates are served");
    assert_eq!(
        cards_without_context(&response),
        1,
        "exactly the card with no page text is counted"
    );
}

/// A quote-less item is not a missing-context item.
///
/// It has nothing to be shown in context, and it already carries its own defer
/// reason. Counting it here would inflate the number that is supposed to mean
/// "quotes we could not place on their page".
#[test]
fn the_bare_card_count_ignores_an_item_with_no_quote_at_all() {
    let settings = crate::domain::settings::Settings::for_test();
    let mut quoteless = instance("ev-3", Some(14));
    quoteless.verbatim_quote = None;

    let response = assemble(
        vec![quoteless],
        &HashMap::new(),
        &HashMap::new(),
        &ordinals(&[("ev-3", 3)]),
        &HashMap::new(),
        &settings,
        HumanTouchIndex {
            question_overrides: &HashMap::new(),
            links: &HashMap::new(),
        },
    );

    assert_eq!(response.pool.len(), 1);
    assert_eq!(cards_without_context(&response), 0);
}

#[test]
fn an_unknown_status_token_is_a_loud_failure_not_a_default() {
    // Standing Rule 1. Bucketing an unrecognized status as "undecided" would show
    // the human a card labelled "Not yet decided" for an item already ruled on.
    let result = build_ref_states(vec![fact_ref("ev-1", "archived")]);
    assert!(
        matches!(result, Err(AppError::Internal { .. })),
        "an undefined status token must fail loudly"
    );
}

#[test]
fn an_unknown_tier_token_is_a_loud_failure_not_a_default() {
    // The sibling of the status test above, for the column task 2.13 added.
    //
    // Standing Rule 1, and the reason the decode is not `unwrap_or(Backup)`:
    // collapsing an unreadable tier onto the default would print "Backup" on a
    // card somebody had deliberately marked as carrying the scenario. The screen
    // would be confidently wrong, the human would have no way to tell, and
    // nothing would be in the log — the precise failure this project's first
    // standing rule exists to prevent.
    let mut row = fact_ref("ev-1", "included");
    row.tier = "critical".to_string();

    let result = build_ref_states(vec![row]);

    assert!(
        matches!(result, Err(AppError::Internal { .. })),
        "an undefined tier token must fail loudly, never default to backup"
    );
}

#[test]
fn every_defined_tier_token_decodes_on_the_card_path() {
    // The other half of the boundary: the three real tokens must all survive the
    // decode. A test that only proved the failure could pass against a decode
    // that rejected everything.
    for tier in crate::domain::fact_tier::FactTier::ALL {
        let mut row = fact_ref("ev-1", "included");
        row.tier = tier.code().to_string();

        let states = build_ref_states(vec![row]).expect("a defined tier decodes");

        assert_eq!(
            states.get("ev-1").and_then(|s| s.tier),
            Some(*tier),
            "{tier:?} must reach the card as itself",
        );
    }
}

// ── Task 2.13c: only INCLUDED facts get a place in the list ─────────────────

#[test]
fn only_included_facts_receive_a_display_position() {
    // `apply_display_order`'s filter is the ONLY thing stopping a dropped or
    // undecided candidate from being handed a position in the facts list. Invert
    // or delete that predicate and set-aside cards would sort into a list they do
    // not belong to — with nothing else in the codebase to notice.
    let mut states = build_ref_states(vec![
        fact_ref("ev-in-1", "included"),
        fact_ref("ev-in-2", "included"),
        fact_ref("ev-dropped", "dropped"),
        fact_ref("ev-undecided", "undecided"),
    ])
    .expect("every token here is one this build defines");

    apply_display_order(&mut states, &ordinals(&[("ev-in-1", 1), ("ev-in-2", 2)]));

    assert!(
        states["ev-in-1"].display_ordinal.is_some(),
        "an included fact is in the list and must have a place in it",
    );
    assert!(states["ev-in-2"].display_ordinal.is_some());
    assert_eq!(
        states["ev-dropped"].display_ordinal, None,
        "a set-aside candidate has no position in a list it does not appear in",
    );
    assert_eq!(
        states["ev-undecided"].display_ordinal, None,
        "an unruled candidate is not in the facts list at all",
    );
}

#[test]
fn the_positions_it_assigns_are_distinct_and_ordered_by_candidate_number() {
    // Two facts sharing a position would make the list's order depend on hash
    // iteration — different on different runs, which reads as the list shuffling
    // itself. `apply_display_order` walks a HashMap, so this is a real hazard and
    // not a theoretical one.
    let mut states = build_ref_states(vec![
        fact_ref("ev-second", "included"),
        fact_ref("ev-first", "included"),
    ])
    .expect("valid tokens");

    apply_display_order(&mut states, &ordinals(&[("ev-first", 1), ("ev-second", 2)]));

    let first = states["ev-first"].display_ordinal.expect("placed");
    let second = states["ev-second"].display_ordinal.expect("placed");
    assert_ne!(first, second, "two facts must not share one position");
    assert!(
        first < second,
        "an untouched list follows the candidate numbers: C-1 before C-2",
    );
}

/// The card queue's half of the same guarantee.
///
/// This is the surface the 2026-08-07 defect was SEEN on: 148 cards gathered
/// over a subject nobody chose. An empty payload here without the notice would
/// leave the human with no cards and no reason, which is the second-worst
/// outcome after the borrowed ones.
#[test]
fn a_scenario_with_no_target_is_told_why_its_queue_is_empty() {
    let notice = "No target defined — this scenario cannot gather evidence.";
    let response = no_target_response(notice);

    assert_eq!(
        response.no_target_notice.as_deref(),
        Some(notice),
        "an empty queue must carry its own explanation, not just be empty"
    );
    assert!(response.pool.is_empty() && response.set_aside.is_empty());
    assert!(
        response.link_progress.is_none(),
        "there is no stuck pile to report progress against — '0 of 0 linked' \
         would be a true sentence about a question nobody asked"
    );
}
