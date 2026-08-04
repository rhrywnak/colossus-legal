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
        &HashMap::new(),
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
        &HashMap::new(),
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
